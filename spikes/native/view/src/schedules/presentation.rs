//! Prepared row content, compared outside paint. Every meaning comes from the
//! page model; GPUI only draws these words/types and binds the returned actions.
use super::{
    Action, Status,
    clock::Clock,
    locale, metadata,
    page::{EntryState, Feedback, Page, Row, RowKey},
    target::{self, Line, Missing},
};
use crate::{Fleet, Lang, alert, localization::Translator};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use vessel_proto::HarnessRef;

/// A complete target label with its semantically appropriate typeface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetLine {
    /// Identity or localized unavailable/default-folder sentence.
    Text(Arc<str>),
    /// A node path, shortened for display only.
    Path(Arc<str>),
    /// Project identity and its separately styled node-relative path.
    Project {
        /// Body-face project label.
        name: Arc<str>,
        /// Fixed-width relative folder; no path joining on the viewer.
        dir: Arc<str>,
    },
}
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
    pub pending: Option<Arc<str>>,
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
    /// Reference target line.
    pub target: TargetLine,
    /// Exact known agent identity, absent when the target cannot be resolved.
    pub agent: Option<HarnessRef>,
    /// Node-only state, for its shared badge variant and accepted words.
    pub status: Status,
    /// Cadence → creation → last result → next run, one complete phrase.
    pub metadata: Arc<str>,
    /// One row-local refusal/failure explanation, if any.
    pub notice: Option<Notice>,
    /// Only wired actions, with pending identity retained across polls.
    pub menu: Vec<MenuEntry>,
    /// Read-only history entry, including ended and read-only rows.
    pub history: Option<Arc<str>>,
}
impl PreparedRow {
    /// A read-only inspection entry keeps the menu available without inventing
    /// write authority for an ended/read-only row.
    #[must_use]
    pub fn has_menu(&self) -> bool {
        !self.menu.is_empty() || self.history.is_some()
    }
}
/// Prepare all visible rows using cached target facts and the viewer calendar.
#[must_use]
pub fn rows(
    page: &Page,
    fleet: &Fleet,
    now: DateTime<Utc>,
    clock: &Clock,
    catalog: &Translator,
) -> Vec<PreparedRow> {
    page.visible_rows()
        .into_iter()
        .map(|row| prepare(&row, fleet, now, clock, catalog))
        .collect()
}
fn prepare(
    row: &Row<'_>,
    fleet: &Fleet,
    now: DateTime<Utc>,
    clock: &Clock,
    catalog: &Translator,
) -> PreparedRow {
    let lang = Lang::from_tag(catalog.locale().tag());
    let target = target_for(row, fleet);
    let meta = metadata::prepare(
        row.schedule,
        row.host.as_ref().map(super::load::MachineKey::as_str),
        now,
        clock,
        catalog,
    );
    let notice = match row.feedback() {
        Some(feedback) => Some(Notice {
            label: match &feedback {
                Feedback::ReadOnly => lang.schedules().read_only.to_owned(),
                Feedback::Failed(failure) => locale::failure(failure, catalog).to_owned(),
            }
            .into(),
            variant: feedback.variant(),
        }),
        None => meta.as_ref().err().map(|error| Notice {
            label: error.label(catalog).into(),
            variant: alert::Variant::Info,
        }),
    };
    let product = super::product(&row.schedule.definition.target);
    PreparedRow {
        key: row.key.clone(),
        intent: row.intent(),
        title: super::title(&row.schedule.definition).into(),
        target: target_line(&target.line, catalog),
        agent: target.agent.cloned(),
        status: Status::of(&row.schedule.state),
        metadata: meta.unwrap_or_default().into(),
        notice,
        history: row
            .connection
            .map(|_| catalog.schedules().history_menu().into()),
        menu: row
            .menu_entries()
            .into_iter()
            .map(|entry| MenuEntry {
                action: entry.action,
                state: entry.state,
                label: lang.schedule_action(product, entry.action).into(),
                pending: locale::pending(entry.action, catalog).map(Arc::from),
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
fn target_line(line: &Line<'_>, catalog: &Translator) -> TargetLine {
    let words = catalog.schedules();
    let text = match line {
        Line::Chat(title) => (*title).to_owned(),
        Line::UntitledChat => words.target_chat_untitled().to_owned(),
        Line::DefaultFolder => words.target_default_folder().to_owned(),
        Line::Project { name, dir: None } => name.as_str().to_owned(),
        Line::Project {
            name,
            dir: Some(dir),
        } => {
            return TargetLine::Project {
                name: name.as_str().into(),
                dir: dir.as_str().into(),
            };
        }
        Line::Path(path) => {
            return TargetLine::Path(target::display_path(path).into_owned().into());
        }
        Line::Unavailable(missing) => match missing {
            Missing::Chat => words.target_chat_unavailable(),
            Missing::Project => words.target_project_unavailable(),
            Missing::Worktree => words.target_worktree_unavailable(),
        }
        .to_owned(),
        Line::Unknown => Lang::from_tag(catalog.locale().tag())
            .schedules()
            .newer_machine
            .to_owned(),
    };
    TargetLine::Text(text.into())
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

#[cfg(test)]
mod tests;
