//! Terminal attention is a metadata projection of the same Inbox read. The
//! terminal owner alone may supply a resident/focus snapshot or reveal a pane.
use crate::shell::Shell;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::{
    Command,
    inbox::{
        Ask,
        terminals::{Action, ResultState},
    },
};
use vessel_app_view::inbox::{
    Problem, Request, Source,
    admission::Admission,
    delivery::{Channel, Delivery},
    notices::MAX_PENDING_NOTICES,
    presence::Focus,
    terminals::Key,
};
use vessel_proto::notifications::TerminalAttention;

#[derive(Default)]
pub(super) struct Terminals {
    pub(super) queued: Vec<(Source, TerminalAttention)>,
    pub(super) pending: Vec<(Source, Action, Request)>,
    pub(super) granted: Vec<(Source, TerminalAttention)>,
    pub(super) selected: Option<Key>,
}
impl Shell {
    pub(super) fn inbox_terminal_action(&mut self, source: Source, action: Action) {
        let area = &mut self.areas.inbox;
        let (terminal, revision) = match action {
            Action::Claim { terminal, revision } | Action::Clear { terminal, revision } => {
                (terminal, revision)
            }
        };
        let current = area.targets.iter().any(|target| matches!(target, vessel_app_link::inbox::Target::Online(known) if *known == source))
            && area.store.nodes().iter().any(|node| node.source == source && node.listing == vessel_app_view::inbox::Listing::Ready
                && node.snapshot.iter().flat_map(|snapshot| snapshot.terminals.iter().flatten()).any(|row| row.terminal == terminal && row.attention.is_some_and(|state| state.revision == revision && vessel_app_view::inbox::terminals::announces(state))));
        if !current {
            area.problem = Some(Problem::Refused(vessel_proto::ErrorCode::Conflict));
            return;
        }
        if area.terminals.pending.len() >= MAX_PENDING_NOTICES {
            area.problem = Some(Problem::Refused(vessel_proto::ErrorCode::Unavailable));
            return;
        }
        if area
            .terminals
            .pending
            .iter()
            .any(|(known, pending, _)| *known == source && *pending == action)
        {
            return;
        }
        let request = match area.store.ticket() {
            Ok(request) => request,
            Err(problem) => {
                area.problem = Some(problem);
                return;
            }
        };
        area.terminals
            .pending
            .push((source.clone(), action, request));
        self.command(Command::Inbox(Ask::Terminal {
            source,
            action,
            request,
        }));
    }
    pub(super) fn inbox_terminal_answer(
        &mut self,
        source: Source,
        action: Action,
        request: Request,
        result: Result<ResultState, Problem>,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        let area = &mut self.areas.inbox;
        let Some(index) = area
            .terminals
            .pending
            .iter()
            .position(|(known, pending, ticket)| {
                *known == source && *pending == action && *ticket == request
            })
        else {
            return false;
        };
        let _settled = area.terminals.pending.remove(index);
        if !area.targets.iter().any(|target| matches!(target, vessel_app_link::inbox::Target::Online(known) if *known == source)) { return false; }
        match (action, result) {
            (Action::Claim { terminal, revision }, Ok(ResultState::Claim(Admission::Admitted))) => {
                if let Some(state) = area
                    .store
                    .nodes()
                    .iter()
                    .filter(|node| node.source == source)
                    .flat_map(|node| node.snapshot.iter())
                    .flat_map(|snapshot| snapshot.terminals.iter().flatten())
                    .find(|row| row.terminal == terminal)
                    .and_then(|row| row.attention)
                    .filter(|state| state.revision == revision)
                    && area.terminals.granted.len() < MAX_PENDING_NOTICES
                {
                    area.terminals.granted.push((source, state));
                }
                self.inbox_notice_update(cx);
            }
            (Action::Claim { .. }, Ok(ResultState::Claim(Admission::Suppressed))) => return false,
            (Action::Clear { terminal, .. }, Ok(ResultState::Cleared(state)))
                if state.terminal == terminal =>
            {
                self.inbox_refresh();
            }
            (Action::Claim { .. } | Action::Clear { .. }, Err(problem)) => {
                area.problem = Some(problem);
            }
            (Action::Clear { .. }, Ok(ResultState::Claim(_)))
            | (Action::Claim { .. } | Action::Clear { .. }, Ok(ResultState::Cleared(_))) => {
                area.problem = Some(Problem::InvalidReply);
            }
        }
        true
    }
    pub(super) fn inbox_terminal_claims(&mut self, window: &Window) {
        let delivery = Delivery::of(self.prefs.settings.notifications.alerts);
        let queued = std::mem::take(&mut self.areas.inbox.terminals.queued);
        if delivery.desktop == Channel::Disabled && delivery.sound == Channel::Disabled {
            return;
        }
        for (source, state) in queued {
            let visible = window.is_window_active()
                && window.is_visible()
                && self.areas.inbox.presence.snapshot().focus
                    == Focus::Terminal {
                        node: source.node(),
                        terminal: state.terminal,
                    };
            if !visible {
                self.inbox_terminal_action(
                    source,
                    Action::Claim {
                        terminal: state.terminal,
                        revision: state.revision,
                    },
                );
            }
        }
    }
    pub(super) fn inbox_terminal_deliver(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let delivery = Delivery::of(self.prefs.settings.notifications.alerts);
        let granted = std::mem::take(&mut self.areas.inbox.terminals.granted);
        let mut sound = false;
        for (source, state) in granted {
            let current = self.areas.inbox.store.nodes().iter().any(|node| {
                node.source == source
                    && node.listing == vessel_app_view::inbox::Listing::Ready
                    && node
                        .snapshot
                        .iter()
                        .flat_map(|snapshot| snapshot.terminals.iter().flatten())
                        .any(|row| !row.failed && row.attention == Some(state))
            });
            let visible = window.is_window_active()
                && window.is_visible()
                && self.areas.inbox.presence.snapshot().focus
                    == Focus::Terminal {
                        node: source.node(),
                        terminal: state.terminal,
                    };
            if !current || visible || !self.areas.inbox.targets.iter().any(|target| matches!(target, vessel_app_link::inbox::Target::Online(known) if *known == source)) { continue; }
            if delivery.desktop == Channel::Enabled {
                let token = vessel_app_view::inbox::notices::DeliveryTag::new(
                    uuid::Uuid::new_v4().into_bytes(),
                );
                super::notification_routes::remember(
                    token,
                    source.clone(),
                    vessel_app_view::inbox::notices::Destination::Terminal(Key {
                        node: source.node(),
                        terminal: state.terminal,
                    }),
                    window,
                    cx,
                );
                cx.show_system_notification(vessel_app_kit::ui::SystemNotification {
                    tag: super::notifications::tag(token).into(),
                    title: self.t.app.into(),
                    body: self.words.inbox().notification_body().to_owned().into(),
                    actions: Vec::new(),
                });
            }
            sound |= delivery.sound == Channel::Enabled;
        }
        if sound {
            window.play_system_bell();
        }
    }
}
