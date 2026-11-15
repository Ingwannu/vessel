//! Sender/next concurrency and one completion gate; stdout belongs to the command.
use super::{
    client::Client,
    error::{Cleanup, DictateError, Failure},
    file::FileInput,
    send,
};
use std::{
    future::{Future, pending},
    io,
    pin::Pin,
    time::Duration,
};
use tokio::{
    sync::oneshot,
    time::{Instant, sleep_until},
};
use vessel_client::CallError;
use vessel_proto::{
    Method,
    dictation::{
        CancelDictation, DictationId, DictationUpdate, NextDictation, StartDictation, Transcript,
        WithinMs,
    },
};

/// Cleanup cannot keep an otherwise failed file command alive indefinitely.
const CANCEL_TIMEOUT: Duration = Duration::from_secs(5);

enum Phase {
    Uploading,
    AwaitingAcceptance(Instant),
    AwaitingFinal(Instant),
}
impl Phase {
    fn deadline(&self) -> Option<Instant> {
        match self {
            Self::Uploading => None,
            Self::AwaitingAcceptance(until) | Self::AwaitingFinal(until) => Some(*until),
        }
    }
    fn expired(&self) -> Failure {
        match self {
            Self::Uploading | Self::AwaitingAcceptance(_) => Failure::AcceptanceTimedOut,
            Self::AwaitingFinal(_) => Failure::FinalTimedOut,
        }
    }
}
fn deadline(budget: WithinMs) -> Result<Instant, Failure> {
    Instant::now()
        .checked_add(Duration::from_millis(u64::from(budget.get())))
        .ok_or(Failure::Clock)
}
async fn until(at: Option<Instant>) {
    match at {
        Some(at) => sleep_until(at).await,
        None => pending().await,
    }
}
pub(super) fn interrupted(result: io::Result<()>) -> Failure {
    match result {
        Ok(()) => Failure::Cancelled,
        Err(source) => Failure::Signal(source),
    }
}

/// Failed local admission never sent start; every ambiguous failure attempts one cancel.
pub(super) async fn cancel_after_failure(
    client: &Client,
    id: &DictationId,
    primary: Failure,
) -> DictateError {
    if let Failure::Call {
        method: Method::StartDictation,
        source: CallError::UnassignedProtocol { .. } | CallError::NotInProtocol { .. },
    } = &primary
    {
        return DictateError::new(primary);
    }
    match tokio::time::timeout(
        CANCEL_TIMEOUT,
        client.call(CancelDictation { id: id.clone() }),
    )
    .await
    {
        Ok(Ok(_ack)) => DictateError::new(primary),
        Ok(Err(source)) => DictateError::with_cleanup(primary, Cleanup::Call(source)),
        Err(source) => DictateError::with_cleanup(primary, Cleanup::TimedOut(source)),
    }
}

pub(super) async fn transcribe<F>(
    client: &Client,
    id: &DictationId,
    mut input: FileInput,
    mut interrupt: Pin<&mut F>,
) -> Result<Transcript, DictateError>
where
    F: Future<Output = io::Result<()>>,
{
    let result = async {
        tokio::select! {
            biased;
            result = interrupt.as_mut() => return Err(interrupted(result)),
            result = send::call(client, StartDictation { id: id.clone(), format: input.format() }) => { let _ready = result?; }
        }
        complete(client, id, &mut input, interrupt.as_mut()).await
    }.await;
    match result {
        Ok(text) => Ok(text),
        Err(primary) => Err(cancel_after_failure(client, id, primary).await),
    }
}

async fn complete<F>(
    client: &Client,
    id: &DictationId,
    input: &mut FileInput,
    mut interrupt: Pin<&mut F>,
) -> Result<Transcript, Failure>
where
    F: Future<Output = io::Result<()>>,
{
    let (finishing, mut first_finish) = oneshot::channel();
    let uploading = send::upload(client, id, input, finishing);
    let receiving = final_text(client, id);
    tokio::pin!(uploading, receiving);
    let mut phase = Phase::Uploading;
    let mut text = None;
    loop {
        if matches!(phase, Phase::AwaitingFinal(_))
            && let Some(text) = text.take()
        {
            return Ok(text);
        }
        tokio::select! {
            // Upload first observes its finish notice before an early Final can
            // be classified; these futures run in this task, not detached jobs.
            biased;
            result = interrupt.as_mut() => return Err(interrupted(result)),
            result = &mut uploading, if !matches!(phase, Phase::AwaitingFinal(_)) => {
                phase = Phase::AwaitingFinal(deadline(result?.within_ms)?);
            }
            sent = &mut first_finish, if matches!(phase, Phase::Uploading) => {
                sent.map_err(Failure::FinishNotice)?;
                phase = Phase::AwaitingAcceptance(deadline(WithinMs::MAX)?);
            }
            result = &mut receiving, if text.is_none() => {
                let final_text = result?;
                if matches!(phase, Phase::Uploading) { return Err(Failure::EarlyFinal); }
                text = Some(final_text);
            }
            () = until(phase.deadline()) => return Err(phase.expired()),
        }
    }
}
async fn final_text(client: &Client, id: &DictationId) -> Result<Transcript, Failure> {
    loop {
        match send::call(client, NextDictation { id: id.clone() }).await? {
            DictationUpdate::Partial { .. } => {}
            DictationUpdate::Final { text } => return Ok(text),
            DictationUpdate::Failed { reason } => return Err(Failure::Stream(reason)),
            DictationUpdate::Unknown => return Err(Failure::UnknownUpdate),
        }
    }
}

#[cfg(test)]
mod tests;
