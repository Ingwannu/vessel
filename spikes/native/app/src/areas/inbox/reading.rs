//! The app reports real pane/composer transitions to the pure read policy.
//! Its passive writes use the same authenticated source and ticket boundary
//! as the inbox; the conversation's legacy timer yields when this owns it.
use crate::{areas::Areas, routes::Route, shell::Shell};
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::{Command, inbox::Ask};
use vessel_app_view::inbox::reading::{Action, Observation, Target, Trigger};

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
    fn inbox_read_observation(&self, window: &Window) -> Option<Observation> {
        if self.route != Route::Session {
            return None;
        }
        let open = self.areas.session.open.as_ref()?;
        let source = &self
            .areas
            .inbox
            .store
            .nodes()
            .iter()
            .find(|node| node.source.name.as_str() == open.machine)?
            .source;
        let area = &self.areas.conversation;
        let info = area.info.as_ref().filter(|info| info.id == open.session)?;
        Some(Observation {
            target: Target {
                node: source.node,
                session: open.session,
            },
            focused: true,
            active: window.is_window_active() && window.is_visible(),
            unread: info.marks.unread,
            pending: !area.live.permissions.is_empty() || !area.live.questions.is_empty(),
            through: area.timeline.next(),
        })
    }
    pub(crate) fn inbox_read_trigger(&mut self, trigger: Trigger) {
        if let Some(action) = self.areas.inbox.reading.trigger(trigger) {
            self.inbox_read_send(action);
        }
    }
    pub(crate) fn inbox_read_owns_current(&self) -> bool {
        self.areas.conversation.info.as_ref().is_some_and(|info| {
            self.areas
                .inbox
                .reading
                .target()
                .is_some_and(|target| target.session == info.id)
        })
    }
    pub(crate) fn inbox_hold_read(&mut self, machine: &str, session: vessel_proto::SessionId) {
        if let Some(node) = self
            .areas
            .inbox
            .store
            .nodes()
            .iter()
            .find(|node| node.source.name.as_str() == machine)
        {
            self.areas.inbox.reading.hold(
                Target {
                    node: node.source.node,
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
            .store
            .nodes()
            .iter()
            .find(|node| node.source.node == action.target.node)
            .map(|node| node.source.clone())
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
