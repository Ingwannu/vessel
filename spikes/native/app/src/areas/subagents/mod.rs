//! Subagent track and read-only child pane, sharing the conversation renderer.

mod actions;
mod answers;
mod breadcrumbs;
mod draw;
mod rows;
pub(crate) use draw::{render, with_header};

use crate::{areas::conversation::ConversationArea, shell::Shell};
use gpui::FocusHandle;
use vessel_app_kit::{DialogState, VirtualList, ui::Context};
use vessel_app_view::{
    devices::{Ticket, Tickets},
    subagents::{ChildPane, DetachChoice, Host, Target, Track, TrackProblem},
};
use vessel_proto::{SessionInfo, SessionRef, subagents::ProviderChildId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Row {
    Managed(SessionRef),
    Provider(ProviderChildId),
}
#[derive(Debug, Clone)]
pub(super) struct HostRead {
    pub host: Host,
    pub parent: SessionRef,
    pub ticket: Ticket,
}
pub(crate) struct SubagentArea {
    pub(super) track: Option<Track>,
    pub(super) info: Option<Box<SessionInfo>>,
    pub(super) lookup: Option<(Target, Ticket)>,
    pub(super) navigate: Option<(Target, Ticket)>,
    pub(super) hosts: Vec<HostRead>,
    pub(super) tickets: Tickets,
    pub(super) rows: Vec<Row>,
    pub(super) list: VirtualList,
    pub(super) open: bool,
    pub(super) dialog: DialogState,
    pub(super) confirm: Option<DetachChoice>,
    pub(super) confirm_dialog: DialogState,
    pub(super) detaching: Option<(DetachChoice, Ticket)>,
    pub(super) uncertain: Option<Target>,
    pub(super) pane: Option<ChildPane>,
    pub(super) readonly: ConversationArea,
    pub(super) readonly_source: Option<vessel_proto::subagents::ProviderChildRef>,
    pub(super) failure: Option<TrackProblem>,
    pub(super) partial: bool,
    pub(super) pill: String,
    pub(super) prior: Option<FocusHandle>,
    pub(super) pending_open: Option<crate::areas::session::Open>,
}
impl SubagentArea {
    pub(crate) fn new(cx: &mut Context<'_, Shell>) -> Self {
        Self {
            track: None,
            info: None,
            lookup: None,
            navigate: None,
            hosts: Vec::new(),
            tickets: Tickets::default(),
            rows: Vec::new(),
            list: VirtualList::new(false),
            open: false,
            dialog: DialogState::new(cx),
            confirm: None,
            confirm_dialog: DialogState::new(cx),
            detaching: None,
            uncertain: None,
            pane: None,
            readonly: ConversationArea::new(cx),
            readonly_source: None,
            failure: None,
            partial: true,
            pill: String::new(),
            prior: None,
            pending_open: None,
        }
    }
    pub(super) fn reproject(&mut self, words: &'static vessel_app_view::localization::Translator) {
        let Some(track) = &self.track else {
            self.rows.clear();
            return;
        };
        let provider_parent = self.pane.as_ref().map(|pane| &pane.query().child);
        let rows = track
            .managed()
            .iter()
            .filter(|_| provider_parent.is_none())
            .map(|row| Row::Managed(row.target.address))
            .chain(
                track
                    .providers(provider_parent)
                    .map(|row| Row::Provider(row.id.clone())),
            )
            .collect::<Vec<_>>();
        if rows != self.rows {
            self.list.reset(rows.len());
            self.rows = rows;
        }
        self.pill = u32::try_from(track.counts().total)
            .ok()
            .and_then(|count| {
                words
                    .subagents()
                    .count(vessel_app_view::localization::Number::count(count))
                    .ok()
            })
            .unwrap_or_else(|| words.subagents().title().to_owned());
    }
}

pub(super) const CHILD_SEAT: crate::areas::conversation::Seat = crate::areas::conversation::Seat {
    get: child_seat,
    get_mut: child_seat_mut,
};
fn child_seat(shell: &Shell) -> Option<&ConversationArea> {
    let pane = shell.areas.subagents.pane.as_ref()?;
    if shell.areas.subagents.readonly_source.as_ref() != Some(&pane.identity()) {
        return None;
    }
    Some(&shell.areas.subagents.readonly)
}
fn child_seat_mut(shell: &mut Shell) -> Option<&mut ConversationArea> {
    let pane = shell.areas.subagents.pane.as_ref()?;
    if shell.areas.subagents.readonly_source.as_ref() != Some(&pane.identity()) {
        return None;
    }
    Some(&mut shell.areas.subagents.readonly)
}
