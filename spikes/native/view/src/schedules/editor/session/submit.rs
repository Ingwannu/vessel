//! Per-slot response correlation and canonical-preview save preparation.
use super::super::definition::{Issue, Save};
use super::*;
use chrono::{DateTime, Utc};
use vessel_proto::HarnessOptions;
use vessel_proto::automation::{PreviewCount, PreviewSchedule, SchedulePreview};

/// A successful save closes the model before another activation can repeat it.
#[derive(Debug)]
pub enum Received {
    /// Old visit/context/ticket/method, or no open form.
    Ignored,
    /// Current visible operation state changed.
    Changed,
    /// Authoritative node result; the app ACKs actual UI dismissal for this visit.
    Saved {
        visit: Visit,
        info: Box<ScheduleInfo>,
    },
}
impl Model {
    /// Only node-authored target choices for this visit's physical machine.
    #[must_use]
    pub fn targets(&self) -> Option<&io::Targets> {
        let State::Active(active) = &self.state else {
            return None;
        };
        match active.targets.value() {
            Some(io::Value::Targets(value)) => Some(value),
            Some(io::Value::Options(_) | io::Value::Preview(_) | io::Value::Saved(_)) | None => {
                None
            }
        }
    }
    /// Dynamic agent options; absence/error is never an invented default catalog.
    #[must_use]
    pub fn options(&self) -> Option<&HarnessOptions> {
        let State::Active(active) = &self.state else {
            return None;
        };
        match active.options.value() {
            Some(io::Value::Options(value)) => Some(value),
            Some(io::Value::Targets(_) | io::Value::Preview(_) | io::Value::Saved(_)) | None => {
                None
            }
        }
    }
    /// Current accepted node preview only; no local cron computation.
    #[must_use]
    pub fn projected(&self) -> Option<&SchedulePreview> {
        let State::Active(active) = &self.state else {
            return None;
        };
        match active.preview.value() {
            Some(io::Value::Preview(value)) => Some(value),
            Some(io::Value::Targets(_) | io::Value::Options(_) | io::Value::Saved(_)) | None => {
                None
            }
        }
    }
    /// The exact request/ready/failure state for each visible form operation.
    #[must_use]
    pub fn mailbox(&self, operation: &io::Operation) -> Option<&io::Mailbox> {
        let State::Active(active) = &self.state else {
            return None;
        };
        Some(match operation {
            io::Operation::Targets => &active.targets,
            io::Operation::Options(_) => &active.options,
            io::Operation::Preview(_) => &active.preview,
            io::Operation::Save(_) => &active.saving,
        })
    }
    /// Called after the UI's cancellable input debounce, never while painting.
    /// # Errors
    /// The named preview count cannot silently become an unchecked wire value.
    pub fn preview(
        &mut self,
        expected: Visit,
        after: DateTime<Utc>,
    ) -> Result<Option<io::Request>, vessel_proto::automation::OutOfRange> {
        if self.visit() != Some(expected) || self.blocked() {
            return Ok(None);
        }
        let Some(connection) = self.connection().cloned() else {
            return Ok(None);
        };
        let Some(cadence) = self
            .draft()
            .and_then(|draft| draft.cadence.checked().ok())
            .cloned()
        else {
            return Ok(None);
        };
        let count = PreviewCount::try_from(super::super::preview::NEXT_RUNS)?;
        let Some(ticket) = self.ticket() else {
            return Ok(None);
        };
        let request = io::Request::on(
            connection,
            ticket,
            io::Operation::Preview(PreviewSchedule {
                cadence,
                after: Some(after),
                count,
            }),
        );
        if let State::Active(active) = &mut self.state {
            active.preview.begin(request.clone());
        }
        Ok(Some(request))
    }
    /// Build only a ready definition, reusing the matching node cadence anchor.
    /// Older nodes may save the parsed rule without any projected run times.
    /// # Errors
    /// Field/cadence/editor failures retain their accepted typed sources.
    pub fn prepare_save(&self) -> Result<Option<Save>, Issue<'_>> {
        let State::Active(active) = &self.state else {
            return Ok(None);
        };
        if active.blocked() {
            return Ok(None);
        }
        let mut save = active.draft.save()?;
        let cadence = active.draft.cadence.checked().map_err(Issue::Cadence)?;
        if let Some(io::Value::Preview(value)) = active.preview.value() {
            let Some(request) = active.preview.request() else {
                return Err(Issue::Preview);
            };
            if request.connection() != &active.connection
                || !matches!(request.operation(), io::Operation::Preview(input) if &input.cadence == cadence)
            {
                return Err(Issue::Preview);
            }
            match &mut save {
                Save::Create(request) => request.definition.cadence = value.cadence.clone(),
                Save::Update(request) => request.definition.cadence = value.cadence.clone(),
            }
        } else if active.preview.failure() != Some(&Failure::Older) {
            return Err(Issue::Preview);
        }
        Ok(Some(save))
    }
    /// The prepared candidate must still equal the current form before dispatch.
    /// This also rejects a captured old save callback without reparsing text.
    pub fn save(&mut self, expected: Visit, candidate: Save) -> Option<io::Request> {
        if self.visit() != Some(expected)
            || self.prepare_save().ok().flatten().as_ref() != Some(&candidate)
        {
            return None;
        }
        let connection = self.connection()?.clone();
        let request = io::Request::on(
            connection,
            self.ticket()?,
            io::Operation::Save(Box::new(candidate)),
        );
        if let State::Active(active) = &mut self.state {
            active.saving.begin(request.clone());
        }
        Some(request)
    }
    /// Every response must match the current admission and exact operation slot.
    pub fn received(
        &mut self,
        request: &io::Request,
        answer: Result<io::Value, Failure>,
    ) -> Received {
        let State::Active(active) = &mut self.state else {
            return Received::Ignored;
        };
        if request.connection() != &active.connection {
            return Received::Ignored;
        }
        let slot = match request.operation() {
            io::Operation::Targets => &mut active.targets,
            io::Operation::Options(_) => &mut active.options,
            io::Operation::Preview(_) => &mut active.preview,
            io::Operation::Save(_) => &mut active.saving,
        };
        if !slot.settle(request, answer) {
            return Received::Ignored;
        }
        if let Some(io::Value::Saved(info)) = slot.value() {
            let saved = Received::Saved {
                visit: active.visit,
                info: info.clone(),
            };
            self.state = State::Closed;
            return saved;
        }
        Received::Changed
    }
}
