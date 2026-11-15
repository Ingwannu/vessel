//! Session navigation resolves node identity before following an existing log.
use crate::shell::Shell;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::{
    Command,
    sessions::{Answer, Ask, Failure},
};
use vessel_app_view::sessions::{NavigationPurpose, NavigationTarget, SessionAddress};
use vessel_proto::{NodeId, SessionId};
impl Shell {
    /// The desktop OS delivery queue calls this after its host readiness boundary.
    #[expect(
        dead_code,
        reason = "desktop delivery registers this typed session entry in its package"
    )]
    pub(crate) fn open_session_address(
        &mut self,
        node: NodeId,
        session: SessionId,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.navigate_session(
            NavigationTarget::Address(SessionAddress { node, session }),
            NavigationPurpose::Open,
            cx,
        );
    }
    /// Copy the originally admitted address, including while its host is offline.
    pub(crate) fn copy_session_link(&mut self, cx: &mut Context<'_, Self>) {
        let Some(open) = &self.areas.session.open else {
            return;
        };
        let address = open
            .binding
            .as_ref()
            .and_then(|binding| self.opened_session_address(&binding.host, open.session));
        let Some(address) = address else {
            self.notice = Some(self.words.sessions().identity_unavailable().to_owned());
            cx.notify();
            return;
        };
        self.copy_session_address(address, cx);
        cx.notify();
    }
    fn copy_session_address(&mut self, address: SessionAddress, cx: &mut Context<'_, Self>) {
        let link = vessel_app_view::desktop::NavigationTarget::Session {
            node: address.node,
            session: address.session,
        }
        .to_url();
        cx.write_to_clipboard(vessel_app_kit::ui::ClipboardItem::new_string(link));
        self.notice = Some(self.words.sessions().link_copied().to_owned());
    }
    fn navigate_session(
        &mut self,
        target: NavigationTarget,
        purpose: NavigationPurpose,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(ticket) = self.areas.sessions.navigation.begin(purpose) else {
            self.notice = Some(self.words.sessions().navigation_busy().to_owned());
            cx.notify();
            return;
        };
        self.command(Command::Sessions(Ask { ticket, target }));
        cx.notify();
    }
    pub(crate) fn sessions_navigation_answer(
        &mut self,
        answer: Answer,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(purpose) = self.areas.sessions.navigation.finish(answer.ticket) else {
            return;
        };
        match answer.result {
            Ok(located) => match purpose {
                NavigationPurpose::Open => self.areas.sessions.pending_open = Some(located),
                NavigationPurpose::CopyLink => {
                    self.copy_session_address(located.address, cx);
                }
            },
            Err(failure) => {
                self.notice = Some(
                    match failure {
                        Failure::Offline => self.words.sessions().navigation_offline(),
                        Failure::Refused(_) | Failure::InvalidHost | Failure::WrongSession => {
                            self.words.sessions().navigation_refused()
                        }
                        Failure::TimedOut => self.words.sessions().navigation_timeout(),
                        Failure::Busy => self.words.sessions().navigation_busy(),
                    }
                    .to_owned(),
                );
            }
        }
        cx.notify();
    }
    /// Window-dependent focus occurs only after the correlated lookup succeeded.
    pub(crate) fn settle_sessions(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if let Some(located) = self.areas.sessions.pending_open.take() {
            let mut open = crate::areas::session::Open::of(&located.card);
            open.binding = Some(vessel_app_link::sessions::Binding {
                host: located.host,
                address: located.address,
            });
            self.open_session(open, window, cx);
        }
    }
    /// A manual route/session change revokes any older pending deep link.
    pub(crate) fn cancel_session_navigation(&mut self) {
        self.areas.sessions.lifecycle.navigation_changed();
        self.areas.sessions.navigation.cancel();
        self.areas.sessions.pending_open = None;
    }
}
