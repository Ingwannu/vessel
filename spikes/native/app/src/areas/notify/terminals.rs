//! Terminal notices (claims row 3; `vessel_app_view::notices::terminals`
//! holds the Decision Log). A machine that admits claims has its terminal
//! list followed by the link. What that list brings is announced under
//! Settings' alerts and claimed by its exact attention revision. Only an
//! admitted claim is shown, sounded and counted. A click opens the terminal
//! in the workspace of a session that holds it. While ADR 0089's protocol
//! is unassigned, no list is followed and nothing here runs.

use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::Command;
use vessel_app_link::claims::{TerminalAnswer, TerminalAsk};
use vessel_app_view::layout::{AgentRole, OpenIntent, Placement, Target};
use vessel_app_view::notices::claims::{Asked, claims};
use vessel_app_view::notices::terminals::{
    TerminalAnnounce, TerminalNotice, announce_terminals, to_clear,
};
use vessel_app_view::notices::{NoticeKind, title_with};
use vessel_app_view::terminal_tabs::place_for;
use vessel_proto::TerminalId;
use vessel_proto::notifications::NotificationClaim;
use vessel_proto::terminals::TerminalPlace;

use gpui::SystemNotification;

use super::About;
use crate::shell::Shell;

impl Shell {
    /// A machine's followed terminal list brought notices: ask the machine
    /// about the ones Settings' alerts would tell.
    pub(crate) fn terminal_notices_found(&self, machine: String, notices: Vec<TerminalNotice>) {
        let told = announce_terminals(notices, self.prefs.settings.notifications.alerts);
        if told.is_empty() {
            return;
        }
        self.command(Command::TerminalClaims(TerminalAsk { machine, told }));
    }

    /// The machine answered: show, sound and count what it admitted.
    pub(crate) fn terminal_notices_claimed(
        &mut self,
        answer: TerminalAnswer,
        cx: &mut Context<'_, Self>,
    ) {
        let told: Vec<TerminalAnnounce> = answer
            .asked
            .into_iter()
            .filter(|(_one, asked)| *asked == Asked::Answered(NotificationClaim::Admitted))
            .map(|(one, _asked)| one)
            .collect();
        if told.is_empty() {
            return;
        }
        let counts = self.prefs.settings.notifications.title;
        let area = &mut self.areas.notify;
        if area.unseen.found(told.len(), area.active, counts) {
            area.title = Some(title_with(&area.base, area.unseen.count()).into());
            cx.notify();
        }
        self.tell_terminals(&told, cx);
    }

    /// Show `told`'s system notifications, titled in Paseo's words with
    /// the terminal's title as the body, and play one sound for them.
    fn tell_terminals(&mut self, told: &[TerminalAnnounce], cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.notify;
        let untitled = self.words.terminal_tabs().fallback();
        for one in told.iter().filter(|one| one.system) {
            let notice = &one.notice;
            let body = if notice.title.is_empty() {
                untitled.to_owned()
            } else {
                notice.title.clone()
            };
            let about = About::Terminal {
                terminal: notice.terminal,
                place: notice.place.clone(),
            };
            cx.show_system_notification(SystemNotification {
                tag: area.tag(&notice.machine, about, notice.kind),
                title: self.t.lang.notices().terminal_title(notice.kind).into(),
                body: body.into(),
                actions: Vec::new(),
            });
        }
        let sound = told
            .iter()
            .filter(|one| one.sound)
            .map(|one| one.notice.kind)
            .min_by_key(|kind| match kind {
                NoticeKind::NeedsYou => 0,
                NoticeKind::Finished => 1,
            });
        if let Some(kind) = sound {
            vessel_app_link::sound::play(kind);
        }
    }

    /// The person is looking at `terminal`'s tab in a visible window: clear
    /// its attention at the exact revision shown, once per revision
    /// (`vessel_app_view::notices::terminals::to_clear`). Only a machine
    /// that admits claims is asked.
    pub(crate) fn terminal_attention_seen(&mut self, machine: &str, terminal: TerminalId) {
        let protocol = self
            .fleet
            .machines
            .iter()
            .find(|known| known.name == machine)
            .and_then(|known| known.protocol);
        if !claims(protocol) {
            return;
        }
        let Some(clear) =
            crate::areas::terminal_tabs::listed_info(self, terminal).and_then(to_clear)
        else {
            return;
        };
        let seen = (terminal, clear.revision);
        if self.areas.notify.cleared == Some(seen) {
            return;
        }
        self.areas.notify.cleared = Some(seen);
        self.command(Command::ClearTerminalAttention {
            machine: machine.to_owned(),
            clear,
        });
    }

    /// A click on a terminal notice: open a session whose workspace holds
    /// `place`, then the terminal's tab there, as Paseo's tap opens the
    /// terminal tab (`session-context.tsx:658-677`). Nothing opens when no
    /// session the fleet lists holds it.
    pub(super) fn terminal_notice_clicked(
        &mut self,
        machine: &str,
        terminal: TerminalId,
        place: &TerminalPlace,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let open = self
            .fleet
            .machines
            .iter()
            .filter(|known| known.name == machine)
            .find_map(|known| {
                known
                    .sessions
                    .iter()
                    .find(|card| place_for(card, known.protocol) == *place)
            })
            .map(crate::areas::session::Open::of);
        let Some(open) = open else {
            tracing::debug!("a terminal notice's click found no session holding it");
            return;
        };
        self.open_session(open, window, cx);
        let Ok(workspace) = crate::areas::layout::current_key(self) else {
            return;
        };
        let mode = crate::areas::layout::mode(window);
        let opened = self.update_layout(
            &workspace,
            |layout| {
                layout.open(
                    Target::Terminal(terminal),
                    OpenIntent::New,
                    Placement::Focused,
                    AgentRole::Unknown,
                    mode,
                )
            },
            cx,
        );
        if opened.is_err() {
            tracing::debug!("a terminal notice's tab could not be opened");
        }
    }
}

#[cfg(test)]
mod tests;
