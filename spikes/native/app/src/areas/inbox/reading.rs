//! The app reports real pane/composer transitions to the pure read policy.
//! Its passive writes use the same authenticated source and ticket boundary
//! as the inbox; the conversation's legacy timer yields when this owns it.
use crate::{areas::Areas, routes::Route, shell::Shell};
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::{Command, inbox::Ask};
use vessel_app_view::inbox::reading::{Action, Focus, Observation, Requests, Target, Trigger};

pub(crate) fn start(areas: &Areas, window: &mut Window, cx: &mut Context<'_, Shell>) {
    let focus = areas.session.composer.focus_handle(cx);
    cx.on_focus(&focus, window, |shell, window, _cx| {
        shell.inbox_read_frame(window);
        shell.inbox_read_trigger(Trigger::InputFocus);
    })
    .detach();
}
impl Shell {
    pub(super) fn inbox_read_frame(&mut self, window: &Window) {
        let observation = self.inbox_read_observation(window);
        let change = self.areas.inbox.reading.observe(observation);
        for action in [change.leaving, change.entering].into_iter().flatten() {
            self.inbox_read_send(action);
        }
    }
    /// Pin an explicit open to the shared welcome identity. Reusing a route
    /// name for a different node can never retarget a passive read operation.
    pub(crate) fn inbox_read_open(&mut self, machine: &str, session: vessel_proto::SessionId) {
        // Settle the departing view using its original admission before
        // installing the explicitly opened source. Never rebind a deferred read.
        let leaving = self.areas.inbox.reading.observe(None).leaving;
        if let Some(action) = leaving {
            self.inbox_read_send(action);
        }
        self.areas.inbox.read_source = self
            .fleet
            .machines
            .iter()
            .find(|item| item.name == machine)
            .and_then(|item| item.connection_context())
            .and_then(|context| vessel_app_view::inbox::Source::new(context).ok());
        self.areas.inbox.read_open = self.areas.inbox.read_source.as_ref().map(|source| Target {
            node: source.node(),
            session,
        });
        self.areas.inbox.read_last = None;
    }
    fn inbox_read_observation(&self, window: &Window) -> Option<Observation> {
        if self.route != Route::Session {
            return None;
        }
        let open = self
            .areas
            .session
            .open
            .as_ref()
            .filter(|open| open.structured)?;
        let target = self
            .areas
            .inbox
            .read_open
            .filter(|target| target.session == open.session)?;
        let source = self.areas.inbox.read_source.as_ref()?;
        if source.node() != target.node
            || self
                .fleet
                .machines
                .iter()
                .find(|machine| machine.name == open.machine)
                .and_then(|machine| machine.connection_context())
                .as_ref()
                != Some(source.context())
        {
            return None;
        }
        let area = &self.areas.conversation;
        let info = area.info.as_ref().filter(|info| info.id == open.session)?;
        Some(Observation {
            target: Target {
                node: source.node(),
                session: open.session,
            },
            focus: if window.is_window_active() && window.is_visible() {
                Focus::Active
            } else {
                Focus::Inactive
            },
            read: if self
                .inbox_session_attention(source.name().as_str(), open.session)
                .map_or(info.marks.unread, |attention| {
                    attention.explicit_unread || attention.unread.get() != 0
                }) {
                vessel_app_view::inbox::Read::Unread
            } else {
                vessel_app_view::inbox::Read::Read
            },
            requests: if area.live.permissions.is_empty() && area.live.questions.is_empty() {
                Requests::None
            } else {
                Requests::Waiting
            },
            through: area.timeline.next(),
        })
    }
    pub(crate) fn inbox_read_trigger(&mut self, trigger: Trigger) {
        if let Some(action) = self.areas.inbox.reading.trigger(trigger) {
            self.inbox_read_send(action);
        }
    }
    pub(crate) fn inbox_read_owns_current(&self) -> bool {
        self.route == Route::Session && self.areas.session.open.is_some()
    }
    pub(crate) fn inbox_hold_read(&mut self, machine: &str, session: vessel_proto::SessionId) {
        if let Some(source) = self
            .areas
            .inbox
            .read_source
            .as_ref()
            .filter(|source| source.name().as_str() == machine)
        {
            self.areas.inbox.reading.hold(
                Target {
                    node: source.node(),
                    session,
                },
                self.areas.conversation.timeline.next(),
            );
        }
    }

    fn inbox_read_send(&mut self, action: Action) {
        let Some(source) = self
            .areas
            .inbox
            .read_source
            .as_ref()
            .filter(|source| {
                self.areas.inbox.read_open == Some(action.target)
                    && source.node() == action.target.node
            })
            .cloned()
        else {
            return;
        };
        let request = match self.areas.inbox.store.ticket() {
            Ok(request) => request,
            Err(problem) => {
                self.areas.inbox.read_problem = Some(problem);
                return;
            }
        };
        self.areas.inbox.read_last = Some((request, action.target));
        self.command(Command::Inbox(Ask::ReadSession {
            source,
            session: action.target.session,
            through: action.through,
            request,
        }));
    }
}
