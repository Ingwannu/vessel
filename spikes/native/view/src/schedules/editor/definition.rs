//! A form is either a new-chat definition or a cadence-only recurring prompt.
//! Preserve the opened revision and every nonedited field; never reconstruct an
//! existing `SessionSpec` from a few visible selectors.
use super::{
    EditError, cadence,
    fields::{FieldError, Fields},
};
use vessel_proto::automation::{
    AfterRun, Cadence, CatchUp, CreateSchedule, ScheduleDefinition, ScheduleInfo, ScheduleTarget,
    TimeZoneName, UpdateSchedule,
};
use vessel_proto::{SessionMode, SessionSpec};

/// The archive switch cannot select the wire's unknown future policy.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Finish {
    /// Archive the new session after its first completed turn (source default).
    #[default]
    Archive,
    /// Keep the new session live until it ends.
    Keep,
}
impl Finish {
    fn wire(self) -> AfterRun {
        match self {
            Self::Archive => AfterRun::Archive,
            Self::Keep => AfterRun::Keep,
        }
    }
}
/// Typed new-chat target selected through project/agent/model controls.
#[derive(Debug, Clone)]
pub struct NewTarget {
    spec: Box<SessionSpec>,
    finish: Finish,
}
impl NewTarget {
    /// Validate a composed spec; terminal/resume sessions cannot run repeatedly.
    /// # Errors
    /// Returns the accepted editor error for unsupported automatic sessions.
    pub fn new(spec: SessionSpec, finish: Finish) -> Result<Self, EditError> {
        match spec.mode {
            SessionMode::Structured { resume: None, .. } => Ok(Self {
                spec: Box::new(spec),
                finish,
            }),
            SessionMode::Structured {
                resume: Some(_), ..
            }
            | SessionMode::Terminal { .. } => Err(EditError::SessionMode),
        }
    }
    /// Preserve full known spec metadata for rendering/selective editing.
    #[must_use]
    pub fn spec(&self) -> &SessionSpec {
        &self.spec
    }
    /// Current archive switch meaning.
    #[must_use]
    pub const fn finish(&self) -> Finish {
        self.finish
    }
    /// This switch changes only archive behavior, never place or model.
    pub fn set_finish(&mut self, finish: Finish) {
        self.finish = finish;
    }
    fn wire(&self) -> ScheduleTarget {
        ScheduleTarget::NewSession {
            spec: self.spec.clone(),
            after: self.finish.wire(),
        }
    }
}
#[derive(Debug)]
enum Kind {
    Create {
        fields: Fields,
        target: Option<NewTarget>,
    },
    EditNew {
        opened: Box<ScheduleInfo>,
        fields: Fields,
        target: Option<NewTarget>,
    },
    EditRecurring {
        opened: Box<ScheduleInfo>,
    },
}
/// The node request this specific form visit can submit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Save {
    /// A new definition on the explicitly selected machine.
    Create(CreateSchedule),
    /// Replacement with the opened node revision, never a refreshed revision.
    Update(UpdateSchedule),
}
/// Borrowed validation meaning; typed sources remain in their field cache.
#[derive(Debug)]
pub enum Issue<'a> {
    /// A cached invalid name/prompt/run count.
    Field(&'a FieldError),
    /// A cached invalid cadence input.
    Cadence(&'a super::CadenceError),
    /// No valid structured new-chat target was selected.
    Target,
    /// No complete current node preview or explicit older-node fallback yet.
    Preview,
    /// An existing definition cannot be replaced safely.
    Edit(EditError),
}
// The cache owns these errors; borrowing them avoids reparsing or cloning a
// form merely to prepare a save. `thiserror` rejects borrowed source fields,
// while their concrete error types are static and can be exposed unchanged.
impl std::fmt::Display for Issue<'_> {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Field(error) => write!(out, "invalid schedule field: {error}"),
            Self::Cadence(error) => write!(out, "invalid schedule cadence: {error}"),
            Self::Target => out.write_str("a scheduled chat target is required"),
            Self::Preview => out.write_str("preview the current schedule rule before saving"),
            Self::Edit(error) => write!(out, "cannot edit schedule: {error}"),
        }
    }
}
impl std::error::Error for Issue<'_> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Field(error) => Some(*error),
            Self::Cadence(error) => Some(*error),
            Self::Edit(error) => Some(error),
            Self::Target | Self::Preview => None,
        }
    }
}
impl Issue<'_> {
    /// One shared Alert meaning for a definition that cannot be opened/built.
    #[must_use]
    pub fn notice(
        &self,
        catalog: &crate::localization::Translator,
    ) -> crate::schedules::presentation::Notice {
        crate::schedules::presentation::Notice {
            label: self.label(catalog).into(),
            variant: crate::alert::Variant::Error,
        }
    }
    /// Use the accepted cadence/edit table and new form catalog once each.
    #[must_use]
    pub fn label(&self, catalog: &crate::localization::Translator) -> String {
        let lang = crate::Lang::from_tag(catalog.locale().tag());
        match self {
            Self::Field(error) => error.label(catalog),
            Self::Cadence(error) => lang.cadence_error(error),
            Self::Preview => catalog.schedules().form_preview_required().to_owned(),
            Self::Target => catalog.schedules().form_target_required().to_owned(),
            Self::Edit(error) => lang.edit_error(*error).to_owned(),
        }
    }
}
/// Form-owned original revision, typed cadence and selectively editable fields.
#[derive(Debug)]
pub struct Draft {
    kind: Kind,
    /// Parsed cadence inputs; drawing/save never reparses their raw text.
    pub cadence: cadence::Draft,
}
impl Draft {
    /// Start from the chosen typed default cadence and viewer zone.
    /// # Errors
    /// Unknown cadences cannot be opened by this client.
    pub fn create(initial: Cadence, viewer: TimeZoneName) -> Result<Self, EditError> {
        Ok(Self {
            kind: Kind::Create {
                fields: Fields::empty(),
                target: None,
            },
            cadence: cadence::Draft::open(initial, viewer)?,
        })
    }
    /// Unknown state/reason/definition is refused before editable controls.
    /// # Errors
    /// Refuses unknown state/target/policy or an unsupported automatic session.
    pub fn open(opened: ScheduleInfo, viewer: TimeZoneName) -> Result<Self, EditError> {
        let _checked = super::replacement(&opened, opened.definition.clone())?;
        let cadence = cadence::Draft::open(opened.definition.cadence.clone(), viewer)?;
        let kind = match &opened.definition.target {
            ScheduleTarget::NewSession { spec, after } => {
                let finish = match after {
                    AfterRun::Archive => Finish::Archive,
                    AfterRun::Keep => Finish::Keep,
                    AfterRun::Unknown => return Err(EditError::Unknown),
                };
                let target = Some(NewTarget::new((**spec).clone(), finish)?);
                let fields = Fields::open(&opened.definition);
                Kind::EditNew {
                    opened: Box::new(opened),
                    fields,
                    target,
                }
            }
            ScheduleTarget::Session { .. } => Kind::EditRecurring {
                opened: Box::new(opened),
            },
            ScheduleTarget::Unknown => return Err(EditError::Unknown),
        };
        Ok(Self { kind, cadence })
    }
    /// Recurring prompt forms expose no writable name/prompt/limit fields.
    #[must_use]
    pub fn fields(&self) -> Option<&Fields> {
        match &self.kind {
            Kind::Create { fields, .. } | Kind::EditNew { fields, .. } => Some(fields),
            Kind::EditRecurring { .. } => None,
        }
    }
    /// Mutable text fields are available only to a new-chat form.
    pub fn fields_mut(&mut self) -> Option<&mut Fields> {
        match &mut self.kind {
            Kind::Create { fields, .. } | Kind::EditNew { fields, .. } => Some(fields),
            Kind::EditRecurring { .. } => None,
        }
    }
    /// A project/model selection cannot replace a recurring prompt's target.
    pub fn target_mut(&mut self) -> Option<&mut Option<NewTarget>> {
        match &mut self.kind {
            Kind::Create { target, .. } | Kind::EditNew { target, .. } => Some(target),
            Kind::EditRecurring { .. } => None,
        }
    }
    /// Target facts for the form; no spec is synthesized while painting.
    #[must_use]
    pub fn target(&self) -> Option<&NewTarget> {
        match &self.kind {
            Kind::Create { target, .. } | Kind::EditNew { target, .. } => target.as_ref(),
            Kind::EditRecurring { .. } => None,
        }
    }
    /// The read-only original definition; its id/revision remain stable even if
    /// a list refresh reports a newer edit while this form is open.
    #[must_use]
    pub fn opened(&self) -> Option<&ScheduleInfo> {
        match &self.kind {
            Kind::Create { .. } => None,
            Kind::EditNew { opened, .. } | Kind::EditRecurring { opened } => Some(opened),
        }
    }
    /// Build from cached typed values. The caller may replace the cadence with
    /// the matching node preview's canonical anchor immediately before sending.
    /// # Errors
    /// Retains the exact invalid field, missing target or accepted editor error.
    pub fn save(&self) -> Result<Save, Issue<'_>> {
        let cadence = self.cadence.checked().map_err(Issue::Cadence)?.clone();
        match &self.kind {
            Kind::Create { fields, target } => {
                // A new chat needs a node-selected location. Opening an older edit
                // still preserves its full stored spec, including legacy cwd.
                let target = target
                    .as_ref()
                    .filter(|target| target.spec().place.is_some() || target.spec().cwd.is_some());
                let definition = definition(fields, target, cadence, None)?;
                super::check_definition(&definition).map_err(Issue::Edit)?;
                Ok(Save::Create(CreateSchedule {
                    definition,
                    paused: false,
                }))
            }
            Kind::EditNew {
                opened,
                fields,
                target,
            } => {
                let definition =
                    definition(fields, target.as_ref(), cadence, Some(&opened.definition))?;
                super::replacement(opened, definition)
                    .map(Save::Update)
                    .map_err(Issue::Edit)
            }
            Kind::EditRecurring { opened } => {
                let mut definition = opened.definition.clone();
                definition.cadence = cadence;
                super::replacement(opened, definition)
                    .map(Save::Update)
                    .map_err(Issue::Edit)
            }
        }
    }
}
fn definition<'a>(
    fields: &'a Fields,
    target: Option<&NewTarget>,
    cadence: Cadence,
    old: Option<&ScheduleDefinition>,
) -> Result<ScheduleDefinition, Issue<'a>> {
    let values = fields.values().map_err(Issue::Field)?;
    let target = target.ok_or(Issue::Target)?;
    let target = target.wire();
    Ok(ScheduleDefinition {
        name: values.name,
        prompt: values.prompt,
        max_runs: values.max_runs,
        cadence,
        target,
        catch_up: old.map_or(CatchUp::Once, |old| old.catch_up),
        ends_at: old.and_then(|old| old.ends_at),
    })
}

#[cfg(test)]
mod tests;
