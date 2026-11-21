//! Prepared row content, compared outside paint. Every meaning comes from the
//! page model; GPUI only draws these words/types and binds the returned actions.
use super::{
    Action, Status, locale, metadata,
    page::{EntryState, Feedback, Page, Row, RowKey},
    target::{self, Line, Missing},
};
use crate::{
    Fleet, alert,
    localization::{FormatError, Translator},
};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use vessel_proto::HarnessRef;

/// Whole row feedback; no duplicate badge for the same failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    /// Complete localized sentence.
    pub label: Arc<str>,
    /// Shared Alert semantics selected by the pure rule.
    pub variant: alert::Variant,
}
/// One already labeled menu action in the reference's order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuEntry {
    /// Stable action identity, never an index reinterpreted after refresh.
    pub action: Action,
    /// Exact input and feedback state from the page.
    pub state: EntryState,
    /// Product-specific idle words from the accepted table.
    pub label: Arc<str>,
    /// Exact pending action words from the new catalog.
    pub pending: Arc<str>,
}
/// Owned, comparable drawing inputs. Repeated identical reads cause no frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedRow {
    /// Stable machine/schedule identity.
    pub key: RowKey,
    /// Original admitted row identity, captured before the trigger is painted.
    pub intent: Option<super::page::Intent>,
    /// The accepted title precedence, intact for wrapping.
    pub title: Arc<str>,
    /// Whole target line prepared outside paint for the source single-line label.
    pub target_label: Arc<str>,
    /// Exact known agent identity, absent when the target cannot be resolved.
    pub agent: Option<HarnessRef>,
    /// Node-only state, for its shared badge variant and accepted words.
    pub status: Status,
    /// Cadence → creation → last result → next run, one complete phrase.
    pub metadata: Arc<str>,
    /// One row-local refusal/failure explanation, if any.
    pub notice: Option<Notice>,
    /// Accessible menu name for the target category.
    pub menu_label: Arc<str>,
    /// Only wired actions, with pending identity retained across polls.
    pub menu: Vec<MenuEntry>,
}
impl PreparedRow {
    /// A row with action controls on its current admission.
    #[must_use]
    pub fn has_menu(&self) -> bool {
        !self.menu.is_empty()
    }
}
/// Prepare all visible rows using cached target facts and the viewer calendar.
#[must_use]
pub fn rows(
    page: &Page,
    fleet: &Fleet,
    now: DateTime<Utc>,
    calendar: metadata::Calendar,
    catalog: &Translator,
) -> Vec<PreparedRow> {
    page.visible_rows()
        .into_iter()
        .map(|row| prepare(&row, fleet, now, calendar, catalog))
        .collect()
}
fn prepare(
    row: &Row<'_>,
    fleet: &Fleet,
    now: DateTime<Utc>,
    calendar: metadata::Calendar,
    catalog: &Translator,
) -> PreparedRow {
    let target = target_for(row, fleet);
    let target_label = display_target(&target.line, catalog);
    let meta = metadata::prepare(
        row.schedule,
        row.host.as_ref().map(super::load::MachineKey::as_str),
        now,
        calendar,
        catalog,
        row.status() == Status::Active,
    );
    let notice = match row.feedback() {
        Some(feedback) => Some(Notice {
            label: match &feedback {
                Feedback::ReadOnly => catalog.schedules().read_only().to_owned(),
                Feedback::Failed(failure) => locale::failure(failure, catalog).to_owned(),
            }
            .into(),
            variant: feedback.variant(),
        }),
        None => meta
            .as_ref()
            .err()
            .or_else(|| target_label.as_ref().err())
            .map(|_| Notice {
                label: catalog.schedules().display_unavailable().into(),
                variant: alert::Variant::Info,
            }),
    };
    let heartbeat = super::is_heartbeat(row.schedule);
    PreparedRow {
        key: row.key.clone(),
        intent: row.intent(),
        title: super::title(&row.schedule.definition).into(),
        target_label: target_label
            .unwrap_or_else(|_| catalog.schedules().display_unavailable().into()),
        agent: target.agent.cloned(),
        status: row.status(),
        metadata: meta.unwrap_or_default().into(),
        notice,
        menu_label: if heartbeat {
            catalog.schedules().more_heartbeat().into()
        } else {
            catalog.schedules().more().into()
        },
        menu: row
            .menu_entries()
            .into_iter()
            .map(|entry| MenuEntry {
                action: entry.action,
                state: entry.state,
                label: locale::action_label(entry.action, heartbeat, catalog).into(),
                pending: locale::pending(entry.action, catalog).into(),
            })
            .collect(),
    }
}
fn target_for<'a>(row: &'a Row<'_>, fleet: &'a Fleet) -> target::Target<'a> {
    let session = match row.schedule.definition.target {
        vessel_proto::automation::ScheduleTarget::Session { session } => Some(session),
        vessel_proto::automation::ScheduleTarget::NewSession { .. }
        | vessel_proto::automation::ScheduleTarget::Unknown => None,
    };
    let chat = fleet
        .machines
        .iter()
        .find(|machine| machine.name == row.key.machine.as_str())
        .and_then(|machine| {
            machine
                .sessions
                .iter()
                .find(|card| Some(card.session) == session)
        });
    target::resolve(
        &row.schedule.definition.target,
        chat,
        row.projects.as_deref().ok(),
        row.worktrees.as_deref().ok(),
    )
}
fn display_target(line: &Line<'_>, catalog: &Translator) -> Result<Arc<str>, FormatError> {
    let words = catalog.schedules();
    let label = match line {
        Line::Chat(title) => *title,
        Line::UntitledChat => words.target_chat_untitled(),
        Line::DefaultFolder => words.target_default_folder(),
        Line::Project { name, dir: None } => name.as_str(),
        Line::Project {
            name,
            dir: Some(dir),
        } => {
            return words
                .project_folder(dir.as_str(), name.as_str())
                .map(Into::into);
        }
        Line::Path(path) => return Ok(target::display_path(path).as_ref().into()),
        Line::Unavailable(missing) => match missing {
            Missing::Chat => words.target_chat_unavailable(),
            Missing::Project => words.target_project_unavailable(),
            Missing::Worktree => words.target_worktree_unavailable(),
        },
        Line::Unknown => words.newer_value(),
    };
    Ok(label.into())
}

/// `HostFilter` leading, independent of a schedule load/mutation failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineLeading {
    /// All machines uses the reference Server glyph.
    All,
    /// Actual connection evidence; missing evidence is muted, never guessed.
    Status(crate::badge::Variant),
}
/// Map only the connection's evidence, never a cached schedule read's success.
#[must_use]
pub fn machine_leading(filter: &super::page::MachineFilter, fleet: &Fleet) -> MachineLeading {
    use crate::{Reach, badge::Variant};
    match filter {
        super::page::MachineFilter::All => MachineLeading::All,
        super::page::MachineFilter::One(key) => MachineLeading::Status(
            fleet
                .machines
                .iter()
                .find(|machine| machine.name == key.as_str())
                .map_or(Variant::Muted, |machine| match machine.reach {
                    Reach::Online => Variant::Success,
                    Reach::Unreachable(_) => Variant::Error,
                }),
        ),
    }
}

/// One area clock task at the fastest visible text tier; old absolute-only
/// rows run no timer. An active countdown needs minute updates even when its
/// creation and previous-run ages are both static.
#[must_use]
pub fn refresh_tier(page: &Page, now: DateTime<Utc>) -> Option<super::time::RefreshTier> {
    use super::time::{Age, RefreshTier};
    page.visible_rows()
        .iter()
        .flat_map(|row| {
            let info = row.schedule;
            [
                Age::of(info.created_at, now).refresh_tier(),
                info.last_run
                    .as_ref()
                    .and_then(|run| Age::of(run.at, now).refresh_tier()),
                super::next_due(info).map(|_| RefreshTier::Minute),
            ]
            .into_iter()
            .flatten()
        })
        .min()
}
