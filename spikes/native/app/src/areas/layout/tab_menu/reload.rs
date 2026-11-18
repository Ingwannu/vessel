//! The tab menu's "Reload agent" (TRACKER 803, ADR 0110): Paseo's
//! `reload-agent` row (`workspace-tab-menu.ts`) and `handleReloadAgent`
//! (`workspace-screen.tsx`), over the node's `session.reload`.
//!
//! [Decision Log]
//! - Purpose and intent: a person starts an agent again on its
//!   conversation so it picks up skills, MCP servers and its login; the
//!   node adds what the agent's own history has beyond the log.
//! - Existing implementation and constraints: the tab menu's builder
//!   (`tab_menu::entries`) serves the strip and the switcher; session calls
//!   go through `Act` and come back as `Event::Acted`; the toast host shows
//!   one toast at a time.
//! - Alternatives considered: a park and wake from the client (not Paseo's
//!   reload, and racing a running turn: the coordinator's ruling for 797).
//! - Chosen approach: the row after Close other tabs on an agent tab, with
//!   Paseo's rotate-cw and tooltip, offered as `session::reload_offer` says
//!   (hidden on a connected node older than the protocol and for an agent
//!   that cannot load its conversation; shown while disconnected, where a
//!   press says "Host is not connected", the coordinator's ruling). A press
//!   holds "Reloading agent..." until the answer, which this module takes
//!   before the conversation does: "Reloaded agent", or the node's words,
//!   else "Failed to reload agent".
//! - Pros, cons and impact: the answer is said for any agent tab, open or
//!   not; a refusal is the toast's alone (no refusal line in the
//!   conversation), as Paseo's.

use vessel_app_kit::MenuItem;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::ui::Context;
use vessel_app_link::{Act, ActAnswer, ActKind, Acted, Command};
use vessel_app_view::layout::{TabId, Target};
use vessel_app_view::session::{ReloadOffer, reload_offer};
use vessel_app_view::toasts::ToastTone;
use vessel_proto::SessionId;

use super::super::menu::Action;
use super::{led, target_of};
use crate::shell::Shell;

/// `session`'s machine and what Reload agent does for it: from its card,
/// else from the open session (a card not listed yet).
fn reload_target(shell: &Shell, session: SessionId) -> Option<(String, ReloadOffer)> {
    let card = shell
        .fleet
        .machines
        .iter()
        .flat_map(|machine| machine.sessions.iter())
        .find(|card| card.session == session);
    if let Some(card) = card {
        let protocol = shell.fleet.protocol_of(&card.machine);
        let offer = if card.structured {
            reload_offer(protocol, card.detail.cannot_load)
        } else {
            ReloadOffer::Hidden
        };
        return Some((card.machine.clone(), offer));
    }
    let open = shell
        .areas
        .session
        .open
        .as_ref()
        .filter(|open| open.session == session && open.structured)?;
    let cannot_load = shell
        .areas
        .conversation
        .info
        .as_deref()
        .filter(|info| info.id == session)
        .and_then(|info| info.abilities.as_ref())
        .is_some_and(|abilities| !abilities.resume);
    let protocol = shell.fleet.protocol_of(&open.machine);
    Some((open.machine.clone(), reload_offer(protocol, cannot_load)))
}

/// The Reload agent row for an agent tab's `session`, unless hidden.
pub(super) fn row(shell: &Shell, session: SessionId) -> Option<(Action, MenuItem)> {
    let (_, offer) = reload_target(shell, session)?;
    if offer == ReloadOffer::Hidden {
        return None;
    }
    let words = shell.words.layout();
    let item = MenuItem::new(words.reload_agent().to_owned())
        .with_tooltip(words.reload_agent_tooltip().to_owned());
    Some((Action::Reload, led(item, Glyph::RotateCw)))
}

impl Shell {
    /// Reload agent pressed: "Host is not connected" while its host is
    /// away, else the act sent and "Reloading agent..." held until the
    /// answer.
    pub(in crate::areas::layout) fn tab_menu_reload(
        &mut self,
        tab: TabId,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(Target::Agent(session)) = target_of(self, tab) else {
            return;
        };
        let Some((machine, offer)) = reload_target(self, session) else {
            return;
        };
        let translator = self.words;
        let words = translator.layout();
        match offer {
            ReloadOffer::Hidden => {}
            ReloadOffer::Disconnected => {
                self.toast(words.host_disconnected(), ToastTone::Error, cx);
            }
            ReloadOffer::Ready => {
                self.toast_hold(words.reloading_agent(), ToastTone::Plain, cx);
                self.command(Command::Act {
                    machine,
                    session,
                    act: Act::Reload,
                });
            }
        }
    }

    /// A reload's answer: its toast in place of "Reloading agent...". A
    /// success goes on to the conversation (the session as it now stands);
    /// a failure is the toast's alone.
    pub(crate) fn tab_menu_reload_acted(
        &mut self,
        answer: ActAnswer,
        cx: &mut Context<'_, Self>,
    ) -> Option<ActAnswer> {
        if answer.kind != ActKind::Reload {
            return Some(answer);
        }
        let translator = self.words;
        let words = translator.layout();
        match &answer.result {
            Ok(acted) => {
                // The session as it now stands in its card (a reload
                // unarchives it), as the board's own acts put theirs.
                if let Acted::Session(info) = acted {
                    self.fleet.put_session(&answer.machine, info);
                }
                self.toast(words.reloaded_agent(), ToastTone::Ok, cx);
                Some(answer)
            }
            Err(failed) => {
                let said = failed.message.trim();
                let text = if said.is_empty() {
                    words.failed_to_reload_agent().to_owned()
                } else {
                    said.to_owned()
                };
                self.toast(text, ToastTone::Error, cx);
                None
            }
        }
    }
}
