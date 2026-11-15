//! Cooperative one-chunk advancement and detached commit ownership.

use super::{
    Arc, Current, Duration, Input, Instant, Jobs, Life, Seal, Segment, Stream, StreamError,
    StreamResult, WithinMs,
};
use crate::dictation::{budget, updates::End};

impl Stream {
    pub(super) fn can_progress(&self) -> bool {
        if self.life != Life::Live {
            return false;
        }
        if self.deferred.is_some() {
            return self.updates.can_partial();
        }
        let capacity = self.jobs.as_ref().is_some_and(|jobs| !jobs.full());
        let forward = self.input.as_ref().is_some_and(Input::forwardable);
        if forward && capacity {
            return true;
        }
        if let Some(finishing) = &self.finishing {
            match &finishing.seal {
                Seal::Open if self.input.as_ref().is_some_and(Input::all_forwarded) => return true,
                Seal::Waiting(_) if capacity => return true,
                Seal::Open | Seal::Waiting(_) | Seal::Sealed => {}
            }
            if matches!(finishing.seal, Seal::Sealed) && self.text.finished() {
                return true;
            }
        }
        match &self.current {
            Some(Current::Chunk(_)) => !forward,
            Some(Current::Finish(_)) => {
                !forward
                    && self
                        .finishing
                        .as_ref()
                        .is_some_and(|state| !matches!(state.seal, Seal::Waiting(_)))
            }
            None => false,
        }
    }

    pub(super) fn progress(&mut self) -> StreamResult<()> {
        if let Some(completion) = self.deferred.take() {
            return self.fold(completion);
        }
        let waiting = self.finishing.as_mut().and_then(|state| {
            if matches!(state.seal, Seal::Waiting(_))
                && self.jobs.as_ref().is_some_and(|jobs| !jobs.full())
            {
                Some(std::mem::replace(&mut state.seal, Seal::Sealed))
            } else {
                None
            }
        });
        if let Some(Seal::Waiting(segment)) = waiting {
            self.submit(segment)?;
            return Ok(());
        }
        if self.jobs.as_ref().is_some_and(Jobs::full) {
            if self.seal()? {
                return Ok(());
            }
            return self.answer();
        }
        if self.advance_audio()? {
            return Ok(());
        }
        if self.seal()? {
            return Ok(());
        }
        self.answer()?;
        if self
            .finishing
            .as_ref()
            .is_some_and(|state| matches!(state.seal, Seal::Sealed))
            && self.text.finished()
        {
            let text = self
                .text
                .join()
                .map_err(|e| Arc::new(StreamError::Text(e)))?;
            self.updates
                .finish(End::Final(text))
                .map_err(|e| Arc::new(StreamError::Terminal(e)))?;
            self.life = Life::Terminal;
        }
        Ok(())
    }

    fn advance_audio(&mut self) -> StreamResult<bool> {
        let Some(pcm) = self.input.as_mut().and_then(Input::pop) else {
            return Ok(false);
        };
        let audio = self
            .audio
            .as_mut()
            .ok_or_else(|| Arc::new(StreamError::Gone))?;
        let chunk = audio
            .resampler
            .push(&pcm)
            .map_err(|e| Arc::new(StreamError::Audio(e)))?;
        audio
            .buffer
            .append(&chunk)
            .map_err(|e| Arc::new(StreamError::Audio(e)))?;
        if audio.buffer.ready() {
            let segment = audio.buffer.seal();
            if let Some(segment) = segment {
                self.submit(segment)?;
            }
        }
        Ok(true)
    }

    fn seal(&mut self) -> StreamResult<bool> {
        if !self.input.as_ref().is_some_and(Input::all_forwarded) {
            return Ok(false);
        }
        let Some(state) = &mut self.finishing else {
            return Ok(false);
        };
        if !matches!(state.seal, Seal::Open) {
            return Ok(false);
        }
        let audio = self
            .audio
            .as_mut()
            .ok_or_else(|| Arc::new(StreamError::Gone))?;
        state.seal = match audio.buffer.seal() {
            Some(segment) => Seal::Waiting(segment),
            None => Seal::Sealed,
        };
        Ok(true)
    }

    fn submit(&mut self, segment: Segment) -> StreamResult<()> {
        let index = self
            .text
            .reserve()
            .map_err(|e| Arc::new(StreamError::Text(e)))?;
        self.jobs
            .as_mut()
            .ok_or_else(|| Arc::new(StreamError::Gone))?
            .spawn(index, segment)
            .map_err(|_returned| {
                // The returned capacity container owns audio, not a foreign
                // diagnostic cause. Drop it before retaining terminal failure.
                Arc::new(StreamError::Capacity)
            })
    }

    fn answer(&mut self) -> StreamResult<()> {
        let Some(current) = self.current.take() else {
            return Ok(());
        };
        match current {
            Current::Chunk(reply) => {
                let ack = self.input.as_ref().and_then(Input::ack);
                drop(reply.send(Ok(ack)));
            }
            Current::Finish(reply) => {
                let result = self.accept_finish();
                match result {
                    Ok(within) => drop(reply.send(Ok(within))),
                    Err(error) => {
                        drop(reply.send(Err(Arc::clone(&error))));
                        return Err(error);
                    }
                }
            }
        }
        Ok(())
    }

    fn accept_finish(&mut self) -> StreamResult<WithinMs> {
        let state = self
            .finishing
            .as_ref()
            .ok_or_else(|| Arc::new(StreamError::Gone))?;
        if let Some((within, _)) = state.accepted {
            return Ok(within);
        }
        let input = self
            .input
            .as_ref()
            .ok_or_else(|| Arc::new(StreamError::Gone))?;
        let last = input
            .final_seq()
            .ok_or_else(|| Arc::new(StreamError::Gone))?;
        let audio = self
            .audio
            .as_ref()
            .ok_or_else(|| Arc::new(StreamError::Gone))?;
        let duration = audio
            .buffer
            .duration()
            .map_err(|e| Arc::new(StreamError::Audio(e)))?;
        let within = budget::compute(self.text.pending(), duration, input.ack(), last)
            .map_err(|e| Arc::new(StreamError::Budget(e)))?;
        let at = Instant::now()
            .checked_add(Duration::from_millis(u64::from(within.get())))
            .ok_or_else(|| Arc::new(StreamError::Clock))?;
        if let Some(state) = &mut self.finishing {
            state.accepted = Some((within, at));
        }
        Ok(within)
    }
}
