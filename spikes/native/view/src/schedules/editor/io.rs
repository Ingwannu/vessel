//! Form replies are tied to their exact operation, ticket and connection.
//! Each field group can invalidate its own read without retiring a save.
use crate::{
    connection::ConnectionContext,
    schedules::load::{Failure, Ticket, Tickets},
};
use vessel_proto::{
    GetHarnessOptions, Harness, HarnessOptions, Scope,
    agents::AgentInfo,
    automation::{Cadence, PreviewSchedule, ScheduleInfo, SchedulePreview, TimeZoneName},
    workspace::{ProjectInfo, WorktreeInfo},
};

use super::{Draft, Save, preview};

/// Facts returned by the selected host, plus the native viewer zone.
#[derive(Debug)]
pub struct Targets {
    /// Project choices on this machine.
    pub projects: Vec<ProjectInfo>,
    /// Existing worktrees that can be selected as a place.
    pub worktrees: Vec<WorktreeInfo>,
    /// Installed harnesses.
    pub harnesses: Vec<Harness>,
    /// Configured agents on this machine.
    pub agents: Vec<AgentInfo>,
    /// Confirmed grants or the typed reason a device report is unavailable.
    pub scopes: Result<Vec<Scope>, Failure>,
    /// Zone reported by the machine.
    pub machine_zone: Option<TimeZoneName>,
    /// IANA zone reported by the device.
    pub device_zone: Option<TimeZoneName>,
}

/// One typed unary operation on the original physical admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ask {
    /// Projects, agents and zone facts for the chosen machine.
    Targets,
    /// Only time-zone and authority facts; an existing session needs no picker catalogues.
    HeartbeatFacts,
    /// Options for the selected provider and profile.
    Options(GetHarnessOptions),
    /// Node projection for this exact rule.
    Preview(PreviewSchedule),
    /// The write built at the original revision.
    Save(Box<Save>),
}

impl Ask {
    /// A dispatched write can complete without its reply reaching the app.
    #[must_use]
    pub const fn interrupted(&self, reason: Failure) -> Failure {
        if matches!(self, Self::Save(_)) {
            Failure::Unconfirmed
        } else {
            reason
        }
    }
}

/// Immutable operation identity and arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Exact original physical admission.
    pub connection: ConnectionContext,
    /// Identifier scoped to this form owner.
    pub ticket: Ticket,
    /// The original operation and typed parameters.
    pub ask: Ask,
}

/// Successful typed result from the connection.
#[derive(Debug)]
pub enum Value {
    /// Target facts for the form.
    Targets(Box<Targets>),
    /// Agent-advertised choices.
    Options(Box<HarnessOptions>),
    /// Complete node preview.
    Preview(Box<SchedulePreview>),
    /// Confirmed created or updated definition.
    Saved(Box<ScheduleInfo>),
}

#[derive(Debug, Default)]
struct Slot<T> {
    waiting: Option<Request>,
    result: Option<Result<T, Failure>>,
}
impl<T> Slot<T> {
    fn empty() -> Self {
        Self {
            waiting: None,
            result: None,
        }
    }
    fn begin(&mut self, request: &Request) {
        self.waiting = Some(request.clone());
        self.result = None;
    }
    fn settle(&mut self, request: &Request, result: Result<T, Failure>) -> bool {
        if self.waiting.as_ref() != Some(request) {
            return false;
        }
        self.waiting = None;
        self.result = Some(result);
        true
    }
}

/// A live form visit and its independently correlated replies.
#[derive(Debug)]
pub struct Form {
    /// The only editable version of this visit's definition.
    pub draft: Draft,
    connection: ConnectionContext,
    heartbeat_editable: bool,
    targets: Slot<Box<Targets>>,
    options: Slot<Box<HarnessOptions>>,
    options_for: Option<GetHarnessOptions>,
    retained_options: Option<(GetHarnessOptions, Result<Box<HarnessOptions>, Failure>)>,
    preview: Slot<Box<SchedulePreview>>,
    save: Slot<Box<ScheduleInfo>>,
}

impl Form {
    /// Open a blank or revision-bound draft for the selected admission.
    #[must_use]
    pub fn new(connection: ConnectionContext, draft: Draft, heartbeat_editable: bool) -> Self {
        Self {
            draft,
            connection,
            heartbeat_editable,
            targets: Slot::empty(),
            options: Slot::empty(),
            options_for: None,
            retained_options: None,
            preview: Slot::empty(),
            save: Slot::empty(),
        }
    }

    /// The form never silently moves across a reconnection.
    #[must_use]
    pub const fn connection(&self) -> &ConnectionContext {
        &self.connection
    }

    /// Reserve a new operation before it crosses the command queue.
    pub fn issue(&mut self, tickets: &mut Tickets, ask: Ask) -> Option<Request> {
        if self.saving() || matches!(self.save.result, Some(Err(Failure::Unconfirmed))) {
            return None;
        }
        let request = Request {
            connection: self.connection.clone(),
            ticket: tickets.issue()?,
            ask,
        };
        match &request.ask {
            Ask::Targets | Ask::HeartbeatFacts => self.targets.begin(&request),
            Ask::Options(options) => {
                if self.options_for.as_ref().is_some_and(|current| {
                    current.harness != options.harness
                        && self.draft.spec().is_some_and(|spec| {
                            spec.harness == current.harness && spec.profile == current.profile
                        })
                }) && let (Some(current), Some(Ok(report))) =
                    (self.options_for.take(), self.options.result.take())
                {
                    self.retained_options = Some((current, Ok(report)));
                }
                self.options_for = Some(options.clone());
                self.options.begin(&request);
            }
            Ask::Preview(_) => self.preview.begin(&request),
            Ask::Save(_) => self.save.begin(&request),
        }
        Some(request)
    }

    /// Only the exact outstanding request can change this visit.
    pub fn accept(&mut self, request: &Request, result: Result<Value, Failure>) -> bool {
        match &request.ask {
            Ask::Targets | Ask::HeartbeatFacts => {
                let result = result.and_then(|value| match value {
                    Value::Targets(facts) => Ok(facts),
                    Value::Options(_) | Value::Preview(_) | Value::Saved(_) => {
                        Err(Failure::Unexpected)
                    }
                });
                if !self.targets.settle(request, result) {
                    return false;
                }
                if let Some(Ok(facts)) = &self.targets.result {
                    self.draft
                        .cadence
                        .default_zone(facts.device_zone.as_ref().or(facts.machine_zone.as_ref()));
                }
                true
            }
            Ask::Options(_) => self.options.settle(
                request,
                result.and_then(|value| match value {
                    Value::Options(options) => Ok(options),
                    Value::Targets(_) | Value::Preview(_) | Value::Saved(_) => {
                        Err(Failure::Unexpected)
                    }
                }),
            ),
            Ask::Preview(input) => self.preview.settle(
                request,
                result.and_then(|value| match value {
                    Value::Preview(preview) if preview::valid_input(input, &preview) => Ok(preview),
                    Value::Targets(_) | Value::Options(_) | Value::Preview(_) | Value::Saved(_) => {
                        Err(Failure::Unexpected)
                    }
                }),
            ),
            Ask::Save(save) => self.save.settle(
                request,
                result.and_then(|value| match value {
                    Value::Saved(info) if valid_saved(&self.draft, save, &info) => Ok(info),
                    Value::Targets(_) | Value::Options(_) | Value::Preview(_) | Value::Saved(_) => {
                        Err(Failure::Unconfirmed)
                    }
                }),
            ),
        }
    }

    /// A changed cadence makes a pending preview irrelevant.
    pub fn invalidate_preview(&mut self) {
        self.preview = Slot::empty();
    }

    /// Present loading, terminal failure and success separately.
    #[must_use]
    pub fn targets(&self) -> Option<&Result<Box<Targets>, Failure>> {
        self.targets.result.as_ref()
    }

    /// Only the selected provider and profile may supply option choices.
    #[must_use]
    pub fn options(
        &self,
        expected: &GetHarnessOptions,
    ) -> Option<&Result<Box<HarnessOptions>, Failure>> {
        if self.options_for.as_ref() == Some(expected) && self.options.result.is_some() {
            return self.options.result.as_ref();
        }
        self.retained_options
            .as_ref()
            .filter(|(ask, _)| ask == expected)
            .map(|(_, result)| result)
    }

    /// Node-generated run times and effective schedule rule.
    #[must_use]
    pub fn preview(&self) -> Option<&Result<Box<SchedulePreview>, Failure>> {
        self.preview.result.as_ref()
    }

    /// The typed terminal save result, including ambiguity.
    #[must_use]
    pub fn saved(&self) -> Option<&Result<Box<ScheduleInfo>, Failure>> {
        self.save.result.as_ref()
    }

    /// Until the one save finishes, the form cannot issue another operation.
    #[must_use]
    pub const fn saving(&self) -> bool {
        self.save.waiting.is_some()
    }

    /// Confirmed Operate grants enable writes. An older device query leaves
    /// grant enforcement to the existing schedule RPC contract.
    #[must_use]
    pub fn can_operate(&self) -> bool {
        self.targets().is_some_and(|facts| {
            facts.as_ref().is_ok_and(|facts| match &facts.scopes {
                Ok(scopes) => scopes.contains(&Scope::Operate),
                Err(Failure::Older) => self.draft.heartbeat().is_none(),
                Err(_) => false,
            })
        })
    }

    /// This exact admitted row offers the heartbeat cadence editor.
    #[must_use]
    pub const fn heartbeat_editable(&self) -> bool {
        self.heartbeat_editable
    }

    /// Whether the form may build and dispatch a save.
    #[must_use]
    pub fn can_save(&self) -> bool {
        self.draft.ready() && self.cadence_for_save().is_some()
    }

    /// A verified preview admits the original typed rule for save. A stored
    /// elapsed cadence keeps its original anchor. Older nodes can save the
    /// typed rule directly.
    #[must_use]
    pub fn save_request(&self) -> Option<Save> {
        self.draft.save(self.cadence_for_save()?)
    }

    fn cadence_for_save(&self) -> Option<&Cadence> {
        if self.saving()
            || !self.can_operate()
            || self.draft.heartbeat().is_some() && !self.heartbeat_editable
            || matches!(self.save.result, Some(Err(Failure::Unconfirmed)))
        {
            return None;
        }
        let cadence = self.draft.cadence.checked().ok()?;
        match self.preview()? {
            Ok(preview) if cadence_matches(cadence, &preview.cadence) => Some(cadence),
            Err(Failure::Older) if self.draft.heartbeat().is_none() => Some(cadence),
            Ok(_) | Err(_) => None,
        }
    }
}

fn cadence_matches(request: &Cadence, canonical: &Cadence) -> bool {
    request == canonical
        || matches!((request, canonical),
            (Cadence::Every { seconds, anchor: None }, Cadence::Every { seconds: resolved, anchor: Some(_) })
                if seconds == resolved)
}

fn valid_saved(draft: &Draft, request: &Save, info: &ScheduleInfo) -> bool {
    match request {
        Save::Create(_) => matches!(
            info.definition.target,
            vessel_proto::automation::ScheduleTarget::NewSession { .. }
        ),
        Save::Update(update) => {
            info.id == update.schedule && info.revision == update.revision.wrapping_add(1)
        }
        Save::Heartbeat(update) => draft.heartbeat().is_some_and(|opened| {
            super::heartbeat::confirmed(opened, update, info)
        }),
    }
}
