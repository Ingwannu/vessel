//! Schedule forms own their opened revision and typed fields. The app uses
//! separate operations for a new-session definition and a heartbeat cadence,
//! so a session's ongoing prompt cannot be reassigned through this editor.
pub mod cadence;
pub mod fields;
pub mod heartbeat;
pub mod io;
mod preview;

use vessel_proto::{
    Creator, HarnessRef, ModelId, OptionChoice, SessionMode, SessionSpec,
    automation::{
        AfterRun, Cadence, CatchUp, CreateSchedule, EndReason, ScheduleDefinition, ScheduleInfo,
        ScheduleState, ScheduleTarget, TimeZoneName, UpdateSchedule, UpdateScheduleCadence,
    },
    workspace::SessionPlace,
};

/// The write a validated schedule or heartbeat form can send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Save {
    /// Insert a schedule on the selected machine.
    Create(CreateSchedule),
    /// Replace a definition at the revision the person opened.
    Update(UpdateSchedule),
    /// Change cadence at the opened revision while keeping the heartbeat's owner.
    Heartbeat(UpdateScheduleCadence),
}

/// Validated fields and original facts for a new-session schedule.
#[derive(Debug)]
pub struct Draft {
    /// Shared name, prompt and optional limit.
    pub fields: fields::Fields,
    /// Editable cron or the exact stored elapsed cadence.
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
            cadence: cadence::Draft::open(&info.definition.cadence, zone),
            opened: Some(Box::new(info.clone())),
            spec: Some(spec.clone()),
            place: spec.place.clone(),
            archive,
        })
    }

    /// Open an existing-session schedule in its own cadence-only form.
    #[must_use]
    pub fn open_heartbeat(info: &ScheduleInfo) -> Option<Self> {
        super::is_heartbeat(info).then(|| Self {
            fields: fields::Fields::open(&info.definition),
            cadence: cadence::Draft::open(&info.definition.cadence, None),
            opened: Some(Box::new(info.clone())),
            spec: None,
            place: None,
            archive: false,
        })
    }

    /// The heartbeat record supplies the original target and saved rule.
    #[must_use]
    pub fn heartbeat(&self) -> Option<&ScheduleInfo> {
        self.opened().filter(|info| super::is_heartbeat(info))
    }

    /// Format a cadence for the opened editor. A heartbeat and its row
    /// share the same elapsed representation.
    /// # Errors
    /// Locale formatting failed.
    pub fn describe_cadence(
        &self,
        rule: &Cadence,
        catalog: &crate::localization::Translator,
    ) -> Result<String, crate::localization::FormatError> {
        if self.heartbeat().is_some() {
            super::locale::cadence(rule, catalog)
        } else {
            cadence::description(rule, catalog)
        }
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

    /// The renderer can check readiness without cloning the draft.
    #[must_use]
    pub fn ready(&self) -> bool {
        if let Some(opened) = self.heartbeat() {
            return self
                .cadence
                .checked()
                .is_ok_and(|rule| heartbeat::changed(opened, rule));
        }
        self.spec.is_some()
            && (self.opened.is_some() || self.place.is_some())
            && self.fields.error().is_none()
    }

    /// Build the request from the validated rule, using the opened elapsed
    /// cadence verbatim when the person has left it alone.
    #[must_use]
    pub fn save(&self, cadence: &Cadence) -> Option<Save> {
        if let Some(opened) = self.heartbeat() {
            return heartbeat::changed(opened, cadence)
                .then(|| Save::Heartbeat(UpdateScheduleCadence {
                    schedule: opened.id,
                    revision: opened.revision,
                    cadence: cadence.clone(),
                }));
        }
        let mut spec = self.spec.clone()?;
        if self.opened.is_none() && self.place.is_none() {
            return None;
        }
        spec.place.clone_from(&self.place);
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
