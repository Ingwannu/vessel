//! Terminal selection is independent of output and provider backpressure.

use super::{
    Arc, CommitPhase, Completion, Current, Data, Finishing, JobError, Life, Seal, Stream,
    StreamError, StreamResult, Text, Waiter, Wake, mpsc,
};
use crate::dictation::{
    control::Retention, endpoint::ProviderError, input::InputError, provider_json::DecodeError,
    updates::End,
};
use vessel_proto::dictation::Transcript;

impl Stream {
    pub(super) async fn apply(&mut self, event: Wake) {
        let result = match event {
            Wake::Closed | Wake::Data(None) | Wake::Next(None) => {
                self.closed().await;
                return;
            }
            Wake::Retire => {
                self.retire().await;
                return;
            }
            Wake::Stop => {
                let error = self
                    .control
                    .chosen()
                    .unwrap_or_else(|| Arc::new(StreamError::Cancelled));
                self.fail(error).await;
                return;
            }
            Wake::Deadline => Err(Arc::new(StreamError::TimedOut)),
            Wake::Next(Some(reply)) => {
                self.updates.wait(reply);
                Ok(())
            }
            Wake::Data(Some(data)) => self.admit(data),
            Wake::Completed(Some(Ok(completion))) => self.fold(completion),
            Wake::Completed(Some(Err(error))) => Err(Arc::new(StreamError::Provider(error))),
            Wake::Completed(None) => Ok(()),
            Wake::Progress => self.progress(),
        };
        if let Err(error) = result {
            self.fail(error).await;
        }
        if self.life == Life::Terminal {
            self.release().await;
        }
    }

    fn admit(&mut self, data: Data) -> StreamResult<()> {
        let input = self
            .input
            .as_mut()
            .ok_or_else(|| Arc::new(StreamError::Gone));
        match data {
            Data::Chunk { seq, pcm, reply } => {
                match input.and_then(|input| {
                    input
                        .accept(seq, pcm)
                        .map_err(|e| Arc::new(StreamError::Input(e)))
                }) {
                    Ok(_) => {
                        self.current = Some(Current::Chunk(reply));
                        Ok(())
                    }
                    Err(error) => {
                        drop(reply.send(Err(Arc::clone(&error))));
                        self.input_refusal(error)
                    }
                }
            }
            Data::Finish { last, reply } => {
                match input.and_then(|input| {
                    input
                        .finish(last)
                        .map_err(|e| Arc::new(StreamError::Input(e)))
                }) {
                    Ok(()) => {
                        if self.finishing.is_none() {
                            self.finishing = Some(Finishing {
                                accepted: None,
                                seal: Seal::Open,
                            });
                            let _old = self.phase.send_replace(CommitPhase::Finishing);
                        }
                        self.current = Some(Current::Finish(reply));
                        Ok(())
                    }
                    Err(error) => {
                        drop(reply.send(Err(Arc::clone(&error))));
                        self.input_refusal(error)
                    }
                }
            }
        }
    }

    fn input_refusal(&mut self, error: Arc<StreamError>) -> StreamResult<()> {
        if matches!(
            error.as_ref(),
            StreamError::Input(InputError::BadAudio) | StreamError::Gone
        ) {
            return Ok(());
        }
        if matches!(error.as_ref(), StreamError::Input(InputError::NoAudio)) {
            self.updates.absent();
            self.life = Life::Consumed;
            return Ok(());
        }
        Err(error)
    }

    pub(super) fn fold(&mut self, completion: Completion) -> StreamResult<()> {
        let silent = matches!(
            &completion.result,
            Err(JobError::Endpoint(ProviderError::Decode(
                DecodeError::TooSmall
            )))
        ) && completion.phase == CommitPhase::Finishing;
        if !silent && completion.result.is_ok() && !self.updates.can_partial() {
            self.deferred = Some(completion);
            return Ok(());
        }
        let text = if silent {
            Transcript::try_from(String::new()).map_err(|e| Arc::new(StreamError::Transcript(e)))?
        } else {
            completion
                .result
                .map_err(|e| Arc::new(StreamError::Provider(e)))?
        };
        self.text
            .complete(completion.index, text)
            .map_err(|e| Arc::new(StreamError::Text(e)))?;
        if !silent {
            let joined = self
                .text
                .join()
                .map_err(|e| Arc::new(StreamError::Text(e)))?;
            self.updates
                .partial(joined)
                .map_err(|e| Arc::new(StreamError::Partial(e)))?;
        }
        Ok(())
    }

    pub(super) async fn fail(&mut self, error: Arc<StreamError>) {
        let chosen = self.control.stop(error, Retention::KeepTerminal);
        if self.life == Life::Live {
            if let Some(current) = self.current.take() {
                current.refuse(Arc::clone(&chosen));
            }
            // Only this owner changes Live to Terminal, so this cannot overwrite
            // a selected success or earlier failure.
            let _existing = self.updates.finish(End::Failed(chosen));
            self.life = Life::Terminal;
        }
        self.release().await;
    }

    pub(super) async fn closed(&mut self) {
        let _chosen = self
            .control
            .stop(Arc::new(StreamError::Cancelled), Retention::Forget);
        self.retire().await;
    }

    pub(super) async fn retire(&mut self) {
        let chosen = self
            .control
            .chosen()
            .unwrap_or_else(|| Arc::new(StreamError::Cancelled));
        if let Some(current) = self.current.take() {
            current.refuse(Arc::clone(&chosen));
        }
        self.updates.retire(End::Failed(chosen));
        let _delivery = self.updates.pump();
        self.life = Life::Consumed;
        self.release().await;
    }

    pub(super) fn drain_pending(
        &mut self,
        data: &mut mpsc::Receiver<Data>,
        next: &mut mpsc::Receiver<Waiter>,
    ) {
        let error = self
            .control
            .chosen()
            .unwrap_or_else(|| Arc::new(StreamError::Gone));
        while let Ok(command) = data.try_recv() {
            let current = match command {
                Data::Chunk { reply, .. } => Current::Chunk(reply),
                Data::Finish { reply, .. } => Current::Finish(reply),
            };
            current.refuse(Arc::clone(&error));
        }
        // A next already enqueued before retirement is still a real pending
        // call, even if the biased stop branch ran before its admission.
        while let Ok(reply) = next.try_recv() {
            self.updates.wait(reply);
            let _delivery = self.updates.pump();
        }
    }

    pub(super) async fn release(&mut self) {
        if let Some(mut jobs) = self.jobs.take()
            && let Err(source) = jobs.clear().await
        {
            let _chosen = self.control.stop(
                Arc::new(StreamError::Provider(source)),
                Retention::KeepTerminal,
            );
        }
        drop(self.audio.take());
        drop(self.input.take());
        drop(self.deferred.take());
        self.text = Text::default();
    }
}
