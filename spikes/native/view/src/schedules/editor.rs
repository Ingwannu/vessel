//! Schedule forms own their opened revision and typed fields. The app uses
//! separate operations for a new-session definition and a heartbeat cadence,
//! so a session's ongoing prompt cannot be reassigned through this editor.
pub mod cadence;
pub mod fields;
pub mod io;
mod preview;

use vessel_proto::{
    Creator, HarnessRef, ModelId, OptionChoice, SessionMode, SessionSpec,
    automation::{
        AfterRun, Cadence, CatchUp, CreateSchedule, EndReason, ScheduleDefinition, ScheduleInfo,
        ScheduleState, ScheduleTarget, TimeZoneName, UpdateSchedule,
    },
    workspace::SessionPlace,
};

/// The write a validated new-session form can send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Save {
    /// Insert a schedule on the selected machine.
    Create(CreateSchedule),
    /// Replace a definition at the revision the person opened.
    Update(UpdateSchedule),
}

/// Validated fields and original facts for a new-session schedule.
#[derive(Debug)]
pub struct Draft {
    /// Shared name, prompt and optional limit.
    pub fields: fields::Fields,
    /// Editable wall-clock or exact elapsed cadence.
    pub cadence: cadence::Draft,
    opened: Option<Box<ScheduleInfo>>,
    spec: Option<Box<SessionSpec>>,
    place: Option<Box<SessionPlace>>,
    archive: bool,
}

impl Draft {
    /// Start a new schedule with the explicit viewer or machine zone.
    #[must_use]
    pub fn create(zone: Option<&TimeZoneName>) -> Self {
        Self {
            fields: fields::Fields::empty(),
            cadence: cadence::Draft::create(zone),
            opened: None,
            spec: None,
            place: None,
            archive: true,
        }
    }

    /// A whole-definition edit can only preserve compatible new-session data.
    #[must_use]
    pub fn can_open(info: &ScheduleInfo) -> bool {
        let ScheduleTarget::NewSession { spec, after } = &info.definition.target else {
            return false;
        };
        !matches!(info.defined_by, Creator::Agent { .. })
            && !matches!(
                info.state,
                ScheduleState::Unknown
                    | ScheduleState::Ended {
                        reason: EndReason::Unknown,
                        ..
                    }
            )
            && !matches!(info.definition.catch_up, CatchUp::Unknown)
            && !matches!(info.definition.cadence, Cadence::Unknown)
            && !matches!(after, AfterRun::Unknown)
            && matches!(spec.mode, SessionMode::Structured { resume: None, .. })
    }

    /// Open a compatible, person-owned new-session schedule. Keep every
    /// stored value absent from the controls, including the expiry and spec.
    #[must_use]
    pub fn open(info: &ScheduleInfo, zone: Option<&TimeZoneName>) -> Option<Self> {
        if !Self::can_open(info) {
            return None;
        }
        let ScheduleTarget::NewSession { spec, after } = &info.definition.target else {
            return None;
        };
        let archive = match after {
            AfterRun::Archive => true,
            AfterRun::Keep => false,
            AfterRun::Unknown => return None,
        };
        Some(Self {
            fields: fields::Fields::open(&info.definition),
            cadence: cadence::Draft::open(&info.definition.cadence, zone)?,
            opened: Some(Box::new(info.clone())),
            spec: Some(spec.clone()),
            place: spec.place.clone(),
            archive,
        })
    }

    /// The immutable original definition, for revision checks and disclosures.
    #[must_use]
    pub fn opened(&self) -> Option<&ScheduleInfo> {
        self.opened.as_deref()
    }

    /// Selected provider and options without synthesizing defaults in paint.
    #[must_use]
    pub fn spec(&self) -> Option<&SessionSpec> {
        self.spec.as_deref()
    }

    /// Editable structured spec. The form calls this only for a selected field.
    pub fn spec_mut(&mut self) -> Option<&mut SessionSpec> {
        self.spec.as_deref_mut()
    }

    /// Project or existing worktree.
    #[must_use]
    pub fn place(&self) -> Option<&SessionPlace> {
        self.place.as_deref()
    }

    /// Explicit selection can replace the old placement.
    pub fn set_place(&mut self, place: SessionPlace) {
        self.place = Some(Box::new(place));
        if let Some(spec) = self.spec_mut() {
            spec.cwd = None;
        }
    }

    /// Select both values together. Switching providers clears settings owned
    /// by the old provider and keeps the rest of the structured spec.
    pub fn select_model(&mut self, harness: HarnessRef, model: Option<ModelId>) {
        let spec = self.spec.get_or_insert_with(|| {
            Box::new(SessionSpec::new(
                harness.clone(),
                SessionMode::Structured {
                    model: None,
                    resume: None,
                    options: Vec::new(),
                },
            ))
        });
        if spec.harness != harness {
            spec.profile = None;
            if let SessionMode::Structured { options, .. } = &mut spec.mode {
                options.clear();
            }
            spec.harness = harness;
        }
        if let SessionMode::Structured { model: current, .. } = &mut spec.mode {
            *current = model;
        }
    }

    /// Host-specific choices belong to their original node.
    pub fn for_other_host(&mut self) {
        self.spec = None;
        self.place = None;
        self.cadence.host_changed();
    }

    /// Set one advertised option while preserving all unrelated choices.
    pub fn set_option(&mut self, choice: OptionChoice) {
        if let Some(spec) = self.spec_mut()
            && let SessionMode::Structured { options, .. } = &mut spec.mode
        {
            options.retain(|old| old.id != choice.id);
            options.push(choice);
        }
    }

    /// Whether completed runs archive their new session.
    #[must_use]
    pub const fn archive(&self) -> bool {
        self.archive
    }

    /// This switch only selects the post-run policy.
    pub const fn set_archive(&mut self, archive: bool) {
        self.archive = archive;
    }

    /// Build the request from a current, verified preview cadence or an explicit
    /// older-node fallback. Any elapsed anchor comes from that cadence.
    #[must_use]
    pub fn save(&self, cadence: &Cadence) -> Option<Save> {
        let mut spec = self.spec.clone()?;
        if self.opened.is_none() && self.place.is_none() {
            return None;
        }
        spec.place = self.place.clone();
        let text = self.fields.values().ok()?;
        let definition = ScheduleDefinition {
            name: text.name,
            prompt: text.prompt,
            max_runs: text.max_runs,
            cadence: cadence.clone(),
            target: ScheduleTarget::NewSession {
                spec,
                after: if self.archive {
                    AfterRun::Archive
                } else {
                    AfterRun::Keep
                },
            },
            catch_up: self
                .opened()
                .map_or(CatchUp::Once, |opened| opened.definition.catch_up),
            ends_at: self.opened().and_then(|opened| opened.definition.ends_at),
        };
        Some(match self.opened() {
            Some(opened) => Save::Update(UpdateSchedule {
                schedule: opened.id,
                revision: opened.revision,
                definition,
            }),
            None => Save::Create(CreateSchedule {
                definition,
                paused: false,
            }),
        })
    }
}

#[cfg(test)]
mod tests;
