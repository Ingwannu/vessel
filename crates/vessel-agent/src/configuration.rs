//! One mutation owner per process, shared by old single setters and bundles.
//!
//! A driver may spawn an asynchronous setter and return to its event loop.
//! Serializing at that loop therefore does not serialize actual completion.
//! This owner retains the fence after callers disappear, while leaving
//! permission answers, output and process cancellation on the driver's lane.

use tokio::sync::{mpsc, oneshot};
use vessel_proto::launch_profiles::{ConfigurationNotice, LaunchChoice, LaunchChoices};
use vessel_proto::{OptionChoice, SessionOption};

use crate::InputError;
use crate::input::Command;

/// An acknowledged bundle's prefix and first refusal; no rollback is implied.
#[derive(Debug)]
#[must_use]
pub struct ConfigurationOutcome {
    applied: Vec<LaunchChoice>,
    options: Option<Vec<SessionOption>>,
    notice: Option<ConfigurationNotice>,
    failure: Option<ConfigurationFailure>,
}

impl ConfigurationOutcome {
    /// Consume a completed result for the node's existing error mapping.
    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        Vec<LaunchChoice>,
        Option<ConfigurationNotice>,
        Option<ConfigurationFailure>,
    ) {
        (self.applied, self.notice, self.failure)
    }
    /// Acknowledged choices, in order, bounded by the submitted bundle.
    #[must_use]
    pub fn applied(&self) -> &[LaunchChoice] {
        &self.applied
    }
    /// Latest actual driver answer, absent when no setter acknowledged.
    #[must_use]
    pub fn options(&self) -> Option<&[SessionOption]> {
        self.options.as_deref()
    }
    /// First explicit driver notice, retained without skipping later choices.
    #[must_use]
    pub const fn notice(&self) -> Option<&ConfigurationNotice> {
        self.notice.as_ref()
    }
    /// The first failed choice; no following choice was sent.
    #[must_use]
    pub const fn failure(&self) -> Option<&ConfigurationFailure> {
        self.failure.as_ref()
    }
}

/// A failed step, alongside the real driver error rather than a fabricated ack.
#[derive(Debug)]
pub struct ConfigurationFailure {
    choice: LaunchChoice,
    error: InputError,
}
impl ConfigurationFailure {
    /// Move the original cause to the session's ordinary wire boundary.
    #[must_use]
    pub fn into_parts(self) -> (LaunchChoice, InputError) {
        (self.choice, self.error)
    }
    /// The choice whose completion failed.
    #[must_use]
    pub const fn choice(&self) -> &LaunchChoice {
        &self.choice
    }
    /// Original error, including its source chain.
    #[must_use]
    pub const fn error(&self) -> &InputError {
        &self.error
    }
}

/// Driver completion, kept internal so vendor-specific notices are parsed
/// only at the driver boundary. Existing protocols report no notice.
#[derive(Debug)]
pub(crate) struct Configured {
    pub(crate) options: Vec<SessionOption>,
    pub(crate) notice: Option<ConfigurationNotice>,
}
impl Configured {
    pub(crate) const fn without_notice(options: Vec<SessionOption>) -> Self {
        Self {
            options,
            notice: None,
        }
    }
}

impl From<Vec<SessionOption>> for Configured {
    fn from(options: Vec<SessionOption>) -> Self {
        Self::without_notice(options)
    }
}

#[derive(Debug)]
enum Mutation {
    Single {
        choice: OptionChoice,
        reply: oneshot::Sender<Result<Vec<SessionOption>, InputError>>,
    },
    Bundle {
        choices: LaunchChoices,
        reply: oneshot::Sender<ConfigurationOutcome>,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct Configuration(mpsc::Sender<Mutation>);
impl Configuration {
    pub(crate) fn spawn(commands: mpsc::Sender<Command>) -> Self {
        // Reuse the process command budget. Full admission applies bounded
        // backpressure, never an unbounded queue of provider mutations.
        let (send, receive) = mpsc::channel(crate::process::COMMANDS);
        drop(tokio::spawn(run(commands, receive)));
        Self(send)
    }

    pub(crate) async fn single(
        &self,
        choice: OptionChoice,
    ) -> Result<Vec<SessionOption>, InputError> {
        let (reply, result) = oneshot::channel();
        self.0
            .send(Mutation::Single { choice, reply })
            .await
            .map_err(|_closed| InputError::Closed)?;
        result.await.unwrap_or(Err(InputError::Closed))
    }

    pub(crate) async fn bundle(
        &self,
        choices: LaunchChoices,
    ) -> Result<ConfigurationOutcome, InputError> {
        let (reply, result) = oneshot::channel();
        self.0
            .send(Mutation::Bundle { choices, reply })
            .await
            .map_err(|_closed| InputError::Closed)?;
        result.await.map_err(|_closed| InputError::Closed)
    }
}

async fn run(commands: mpsc::Sender<Command>, mut receive: mpsc::Receiver<Mutation>) {
    loop {
        // External AgentInput handles may live longer than the driver. Its
        // receiver closure must end this owner even while those handles live.
        let mutation = tokio::select! {
            biased;
            () = commands.closed() => return,
            mutation = receive.recv() => match mutation { Some(value) => value, None => return },
        };
        match mutation {
            Mutation::Single { choice, reply } => {
                let result = step(&commands, choice).await.map(|done| done.options);
                let _gone = reply.send(result);
            }
            Mutation::Bundle { choices, reply } => {
                let result = bundle(&commands, choices).await;
                let _gone = reply.send(result);
            }
        }
    }
}

async fn step(
    commands: &mpsc::Sender<Command>,
    choice: OptionChoice,
) -> Result<Configured, InputError> {
    let (reply, result) = oneshot::channel();
    commands
        .send(Command::SetOption { choice, reply })
        .await
        .map_err(|_closed| InputError::Closed)?;
    // Closure, unlike a caller timeout, proves that the old process can no
    // longer apply changes. A hung live provider retains its own fence.
    tokio::select! {
        biased;
        () = commands.closed() => Err(InputError::Closed),
        result = result => result.unwrap_or(Err(InputError::Closed)),
    }
}

async fn bundle(commands: &mpsc::Sender<Command>, choices: LaunchChoices) -> ConfigurationOutcome {
    let mut outcome = ConfigurationOutcome {
        applied: Vec::with_capacity(choices.as_slice().len()),
        options: None,
        notice: None,
        failure: None,
    };
    for choice in choices.as_slice() {
        match step(commands, choice.as_choice().clone()).await {
            Ok(done) => {
                outcome.applied.push(choice.clone());
                outcome.options = Some(done.options);
                if outcome.notice.is_none() {
                    outcome.notice = done.notice;
                }
            }
            Err(error) => {
                outcome.failure = Some(ConfigurationFailure {
                    choice: choice.clone(),
                    error,
                });
                break;
            }
        }
    }
    outcome
}

#[cfg(test)]
mod tests;
