//! An agent's Diagnostic (ADR 0111; TRACKER 568, 67, 87), as Paseo's
//! `provider-diagnostic-sheet.tsx`: a "Diagnostic" button at the agent's
//! details, and its sheet, asked when it opens and on Refresh.
//!
//! [Decision Log]
//! - Purpose: a person asks why an agent does not work on a machine and
//!   gets Paseo's block to read or paste: the program, where it is, its
//!   version, a new probe's steps and the catalogue's status.
//! - Constraint: `agent.diagnostic` exists from protocol 33. The button
//!   shows only on a connected machine that speaks it, so an older one is
//!   never asked.
//! - Choice:
//!   - The text is the node's typed facts rendered by vessel-proto
//!     (`AgentDiagnostic::text`), the same bytes the page and CLI print.
//!   - The body is Paseo's: running while there is no text, the text, or
//!     "No diagnostic available".
//!   - A failure takes the text's place, as Paseo writes a failed fetch's
//!     message there, but in the app's words for its code.
//! - The probe behind it is the node's: a refresh within 5 s of the last
//!   one is answered with the same probe (row 334's floor).

use vessel_app_kit::dialog::{DialogContent, DialogSize, DialogState, dialog as draw};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, outline_button};
use vessel_app_link::Command;
use vessel_app_link::diagnostics::{Ask, Request};
use vessel_app_view::board::Reach;
use vessel_app_view::devices::{MachineKey, Ticket};
use vessel_app_view::diagnostics::{Problem, Sheet, SheetBody};
use vessel_proto::HarnessRef;
use vessel_proto::diagnostics::DIAGNOSTICS_WIRE_PROTOCOL;

use crate::shell::Shell;

/// The open sheet, if any, and its dialog.
pub(super) struct State {
    open: Option<Open>,
    dialog: DialogState,
}

struct Open {
    machine: MachineKey,
    agent: HarnessRef,
    sheet: Sheet,
}

impl State {
    pub(super) fn new(cx: &mut Context<'_, Shell>) -> Self {
        Self {
            open: None,
            dialog: DialogState::new(cx),
        }
    }

    /// Take an answer for the open sheet. False when it is not its own.
    pub(super) fn answered(&mut self, ticket: Ticket, result: Result<String, Problem>) -> bool {
        self.open
            .as_mut()
            .is_some_and(|open| open.sheet.answered(ticket, result))
    }
}

impl Shell {
    /// Open the sheet for `agent` on `machine`, asking at once.
    fn open_agent_diagnostic(
        &mut self,
        machine: MachineKey,
        agent: HarnessRef,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let state = &mut self.areas.settings.diagnostics;
        let Ok(ticket) = state.tickets.next_ticket() else {
            return;
        };
        state.agent.open = Some(Open {
            machine: machine.clone(),
            agent: agent.clone(),
            sheet: Sheet::asking(ticket),
        });
        state.agent.dialog.focus(window, cx);
        self.command(Command::Diagnostics(Ask {
            machine,
            ticket,
            request: Request::Agent {
                agent,
                profile: None,
            },
        }));
        cx.notify();
    }

    /// Refresh: ask again, keeping the text until the answer.
    fn refresh_agent_diagnostic(&mut self, cx: &mut Context<'_, Self>) {
        let state = &mut self.areas.settings.diagnostics;
        let Some(open) = state.agent.open.as_mut() else {
            return;
        };
        if open.sheet.loading() {
            return;
        }
        let Ok(ticket) = state.tickets.next_ticket() else {
            return;
        };
        open.sheet.refresh(ticket);
        let ask = Ask {
            machine: open.machine.clone(),
            ticket,
            request: Request::Agent {
                agent: open.agent.clone(),
                profile: None,
            },
        };
        self.command(Command::Diagnostics(ask));
        cx.notify();
    }
}

/// The "Diagnostic" button beside `agent`'s details, on the Agents page's
/// machine when it is connected and speaks `agent.diagnostic`.
pub(in crate::areas::settings) fn diagnostic_button(
    agent: &HarnessRef,
    machine: &str,
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let known = shell
        .fleet
        .machines
        .iter()
        .find(|known| known.name == machine)?;
    let speaks = matches!(known.reach, Reach::Online)
        && known
            .protocol
            .zip(DIAGNOSTICS_WIRE_PROTOCOL)
            .is_some_and(|(spoken, since)| spoken >= since);
    if !speaks {
        return None;
    }
    let key = MachineKey::from_book(machine);
    let agent = agent.clone();
    let button = outline_button(
        "agent-diagnostic",
        shell.words.diagnostics().agent(),
        true,
        tokens,
    )
    .on_click(cx.listener(move |shell, _event, window, cx| {
        shell.open_agent_diagnostic(key.clone(), agent.clone(), window, cx);
    }));
    Some(button.into_any_element())
}

/// The sheet's dialog, while it is open.
pub(super) fn dialog(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.settings.diagnostics.agent;
    let open = state.open.as_ref()?;
    let words = shell.words.diagnostics();
    let compact = crate::areas::draft::compact(window);
    let body = match open.sheet.body() {
        SheetBody::Running => note(
            Some(vessel_app_kit::slider::spinner(
                "agent-diagnostic-running",
                14.0,
                tokens.neutral.foreground_muted,
            )),
            words.running(),
            tokens,
        ),
        SheetBody::Text(text) => {
            super::code_surface("agent-diagnostic-text", text.to_owned(), tokens)
        }
        SheetBody::Failed(problem) => super::code_surface(
            "agent-diagnostic-text",
            super::problem_words(problem, shell),
            tokens,
        ),
        SheetBody::None => note(None, words.agent_none(), tokens),
    };
    let actions = super::title_actions(
        "agent-diagnostic-actions",
        (open.sheet.text().map(str::to_owned), open.sheet.loading()),
        shell,
        tokens,
        compact,
        cx,
        |shell, _window, cx| shell.refresh_agent_diagnostic(cx),
    );
    let content = DialogContent {
        title: words.agent().into(),
        body,
        footer: Vec::new(),
        size: DialogSize::Wide,
    };
    Some(draw(
        &state.dialog,
        content.with_header_actions(actions),
        tokens,
        window,
        compact,
        |shell: &mut Shell, _window, _cx| shell.areas.settings.diagnostics.agent.open = None,
        cx,
    ))
}

/// Paseo's surface card holding one muted line, with a spinner before it
/// while it runs.
fn note(spinner: Option<AnyElement>, words: &str, tokens: &Tokens) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap(px(spacing::S2))
        .p(px(spacing::S4))
        .rounded(px(vessel_app_kit::theme::radius::R2))
        .bg(tokens.neutral.surface1)
        .border_1()
        .border_color(tokens.neutral.border)
        .children(spinner)
        .child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(words.to_owned()),
        )
        .into_any_element()
}
