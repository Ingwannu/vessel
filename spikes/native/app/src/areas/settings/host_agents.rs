//! Settings › Agents for the chosen machine, as Paseo's `HostAgentsPage`
//! (`screens/settings/host-page.tsx`): the "Agents" section with its
//! cards, here "Release idle agents" (`IdleRuntimeCleanupCard`), read from
//! and written to the machine's own settings (`view::idle_release`,
//! `link::node_settings`); while the machine is not connected, Paseo's
//! one card that says to connect.
//!
//! [Decision Log]
//! - Purpose: Paseo's host cards that Vessel's node already has a setting
//!   for; the agents themselves (installed, signed in) are Providers'.
//! - Choice: Paseo's row: the title with its info tip (`SettingsInfoTip`,
//!   the kit's tooltip on an info glyph), the hint, an error under it when
//!   the node refused a change, the kit's switch, waiting while a change
//!   is made. The page reads when it is shown for a machine and drops what
//!   it read when it is left, so coming back reads again (Paseo's query).
//! - Not shown, nothing behind them in Vessel yet: Paseo's "Enable Paseo
//!   tools", "Browser tools", "Append system prompt", "Resume agents after
//!   restart" and agent profiles. Tools and Skills live on Overview.
//! - Differences: Paseo draws nothing for a node too old for the setting;
//!   Vessel says so in the row, as nothing shown may mislead and an empty
//!   section would. The info tip shows on a pointer only (the kit's
//!   tooltip; Paseo's opens on touch too).

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::settings::{self, switch};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::tooltip::Place;
use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Tooltips, Type, TypeExt as _, glyph};
use vessel_app_link::Command;
use vessel_app_link::node_settings::{Answer, Ask};
use vessel_app_view::devices::{MachineKey, Tickets};
use vessel_app_view::idle_release::{Card, Page, Problem, Switch};
use vessel_app_view::settings::Section;
use vessel_app_view::{Machine, Reach};
use vessel_proto::ErrorCode;

use crate::routes::Route;
use crate::shell::Shell;

/// The info mark's hit area is padded a step and pulled back as far, so
/// the glyph sits on the title's line while the target is larger.
const TIP_PULL: f32 = -4.0;

/// The page's state: what was read for which machine.
#[derive(Default)]
pub(crate) struct State {
    page: Option<Page>,
    tickets: Tickets,
    /// Changes made, for the switch's slide.
    turns: u64,
    tips: Tooltips,
    info: Place,
}

impl Shell {
    /// Before drawing: read the chosen machine's settings when the page
    /// is shown for it and has not yet; drop them when it is left.
    pub(crate) fn settle_host_agents(&mut self, window: &Window) {
        let shown = self.route == Route::Settings
            && self.areas.settings.section == Section::Agents
            && (!vessel_app_kit::breakpoint::phone(window) || self.areas.settings.pushed);
        let online = super::choices::chosen_machine(self)
            .filter(|machine| shown && machine.reach == Reach::Online)
            .map(|machine| MachineKey::from_book(&machine.name));
        let state = &mut self.areas.settings.host_agents;
        let Some(machine) = online else {
            if !shown {
                state.page = None;
            }
            return;
        };
        if state
            .page
            .as_ref()
            .is_some_and(|page| page.machine() == &machine)
        {
            return;
        }
        let Ok(ticket) = state.tickets.next_ticket() else {
            return;
        };
        state.page = Some(Page::reading(machine.clone(), ticket));
        state.tips = Tooltips::new();
        self.command(Command::NodeSettings(Ask {
            machine,
            ticket,
            change: None,
        }));
    }

    /// The switch was pressed: send the change.
    fn idle_toggle(&mut self, cx: &mut Context<'_, Self>) {
        let state = &mut self.areas.settings.host_agents;
        let (Some(page), Ok(ticket)) = (state.page.as_mut(), state.tickets.next_ticket()) else {
            return;
        };
        let Some(change) = page.toggle(ticket) else {
            return;
        };
        let machine = page.machine().clone();
        state.turns = state.turns.saturating_add(1);
        self.command(Command::NodeSettings(Ask {
            machine,
            ticket,
            change: Some(change),
        }));
        cx.notify();
    }

    /// The machine answered.
    pub(crate) fn node_settings_answer(&mut self, answer: Answer) {
        let Answer {
            machine,
            ticket,
            result,
        } = answer;
        if let Some(page) = self.areas.settings.host_agents.page.as_mut() {
            let _kept = page.answered(&machine, ticket, result);
        }
    }
}

/// The Agents page.
pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.t.lang.settings();
    let machine = super::choices::chosen_machine(shell);
    let state = &shell.areas.settings.host_agents;
    // Connected: the "Agents" section and its cards; otherwise Paseo's one
    // card saying to connect, under no heading.
    let body = match (machine, state.page.as_ref()) {
        (Some(machine), Some(page)) if machine.reach == Reach::Online => {
            match idle_card(machine, page, shell, tokens, compact, cx) {
                Ok(card) => card.map(|card| {
                    settings::section(words.section(Section::Agents, shell.words), card, tokens)
                }),
                Err(Offline) => Some(unavailable(words.agents_unavailable, tokens)),
            }
        }
        (Some(_) | None, _) => Some(unavailable(words.agents_unavailable, tokens)),
    };
    div()
        .flex()
        .flex_col()
        .children(body)
        .children(state.tips.render(tokens))
        .into_any_element()
}

/// The node could not be reached for its settings.
struct Offline;

/// The idle-release card, or nothing while it is read (as Paseo's).
fn idle_card(
    machine: &Machine,
    page: &Page,
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Result<Option<gpui::Div>, Offline> {
    let words = shell.t.lang.settings();
    let row = match page.card(machine.protocol) {
        Card::Reading => return Ok(None),
        Card::Failed(Problem::Offline) => return Err(Offline),
        Card::TooOld => settings::row_titled(
            title(shell, tokens, compact),
            Some(words.idle_too_old.into()),
            None,
            None,
            true,
            compact,
            tokens,
        ),
        Card::Failed(problem @ Problem::Refused(_)) => settings::row_titled(
            title(shell, tokens, compact),
            Some(words.idle_hint(None).into()),
            Some(refusal(problem, shell).into()),
            None,
            true,
            compact,
            tokens,
        ),
        Card::Switch(switch_state) => switch_row(switch_state, shell, tokens, compact, cx),
    };
    Ok(Some(settings::card(tokens).child(row)))
}

/// The row with its switch.
fn switch_row(
    state: Switch,
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> gpui::Div {
    let words = shell.t.lang.settings();
    let mut control = switch(
        "settings-idle-release",
        state.on,
        !state.pending,
        compact,
        shell.areas.settings.host_agents.turns,
        tokens,
    );
    if !state.pending {
        control = control
            .cursor_pointer()
            .on_click(cx.listener(|shell, _event, _window, cx| shell.idle_toggle(cx)));
    }
    settings::row_titled(
        title(shell, tokens, compact),
        Some(words.idle_hint(state.minutes).into()),
        state.refused.map(|problem| refusal(problem, shell).into()),
        Some(control.into_any_element()),
        true,
        compact,
        tokens,
    )
}

/// "Release idle agents" and its info tip (Paseo's `SettingsInfoTip`: a
/// 16 info glyph, muted, 4 around and pulled 4 to the left, 8 after the
/// title).
fn title(shell: &Shell, tokens: &Tokens, compact: bool) -> AnyElement {
    let words = shell.t.lang.settings();
    let state = &shell.areas.settings.host_agents;
    let tip = div()
        .id("settings-idle-release-info")
        .p(px(spacing::S1))
        .ml(px(TIP_PULL))
        .child(glyph(Glyph::Info, 16.0, tokens.neutral.foreground_muted));
    let tip = state
        .tips
        .on(tip, &state.info, words.idle_release_info, compact);
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::S2))
        .child(
            div()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground)
                .child(words.idle_release),
        )
        .child(tip)
        .into_any_element()
}

/// A refused read or change, in words (a node's own text is never shown).
fn refusal(problem: Problem, shell: &Shell) -> &'static str {
    let words = shell.t.lang.settings();
    match problem {
        Problem::Refused(ErrorCode::Forbidden | ErrorCode::Unauthorized) => words.idle_forbidden,
        Problem::Offline | Problem::Refused(_) => words.idle_refused,
    }
}

/// Paseo's card for a host it cannot reach: one muted sentence, centred.
fn unavailable(text: &'static str, tokens: &Tokens) -> gpui::Div {
    settings::card(tokens).child(
        div()
            .p(px(spacing::S4))
            .flex()
            .justify_center()
            .type_(Type::Body)
            .text_color(tokens.neutral.foreground_muted)
            .child(text),
    )
}
