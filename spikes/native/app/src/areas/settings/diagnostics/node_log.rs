//! A machine's "Log file" row on Settings › Overview and its "Node logs"
//! dialog (ADR 0111, TRACKER 569), as Paseo's desktop daemon card
//! (`desktop-updates-section.tsx`: "Log file", "Copy path", "Open logs",
//! `DaemonLogsModal`).
//!
//! [Decision Log]
//! - Purpose: Paseo shows its own daemon's log. The app shows the chosen
//!   machine's node log, for a connection that may administer it
//!   (`node.log` needs `admin`; a refusal reads "Log path unavailable").
//! - Constraint: `node.log` exists from protocol 33. The row shows only on a
//!   connected machine that speaks it, so an older one is never asked.
//! - Choice:
//!   - The row asks for one line to learn the path, once per chosen machine,
//!     when Settings loads its page (`Shell::agent_load`, the page's one load
//!     hook).
//!   - The dialog asks for the node's default of 100 lines when it opens.
//!   - The dialog is Paseo's modal: the path line, 12 below it the contents,
//!     wrapped, muted, 12 px monospace on 18 px lines.
//!   - The coordinator's truthfulness rule: a failed call shows why, and
//!     "(log file is empty)" stands only for an empty file
//!     (`view::diagnostics::log_contents`).
//!   - The row reads again after an answer that may change (no connection,
//!     no answer in time, a busy node) and on a new connection; a path or a
//!     refusal such as `forbidden` is kept (the app reviewer's R2 on row 337).
//! - Stated difference: a Copy icon in the dialog's title row copies the
//!   lines shown, with the diagnostics' copied toast. Paseo's log text is
//!   selectable, and GPUI text is not (the coordinator's ruling on row 337).

use vessel_app_kit::dialog::{DialogContent, DialogSize, DialogState, dialog as draw};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, ClipboardItem, Context, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, outline_button, settings};
use vessel_app_link::Command;
use vessel_app_link::diagnostics::{Ask, Request};
use vessel_app_view::board::{Machine, Reach};
use vessel_app_view::connection::ConnectionEpoch;
use vessel_app_view::devices::{MachineKey, Ticket};
use vessel_app_view::diagnostics::{LogContents, Problem, log_contents};
use vessel_app_view::settings::Section;
use vessel_proto::diagnostics::{DIAGNOSTICS_WIRE_PROTOCOL, LogSink, NodeLog};

use crate::shell::Shell;

/// One read of a machine's log: its ticket while asked, then its answer.
struct Read {
    machine: String,
    /// The connection it was asked over (the row's): a new connection to
    /// the machine reads again.
    epoch: Option<ConnectionEpoch>,
    asking: Option<Ticket>,
    answer: Option<Result<NodeLog, Problem>>,
}

impl Read {
    fn asks(&self, machine: &MachineKey, ticket: Ticket) -> bool {
        self.machine == machine.as_str() && self.asking == Some(ticket)
    }
}

/// Whether the row asks `machine` (over `epoch`) again, given its last read:
/// none for it yet, another connection since, or an answer that may change
/// on asking (`Problem::transient`). A path, or a refusal such as
/// `forbidden`, is kept; a read still asked is waited for (the app
/// reviewer's R2 on row 337).
fn asks_again(read: Option<&Read>, machine: &str, epoch: Option<ConnectionEpoch>) -> bool {
    let Some(read) = read.filter(|read| read.machine == machine && read.epoch == epoch) else {
        return true;
    };
    read.asking.is_none() && matches!(&read.answer, Some(Err(problem)) if problem.transient())
}

/// The row's read, the dialog's read and its dialog.
pub(super) struct State {
    row: Option<Read>,
    open: Option<Read>,
    dialog: DialogState,
}

impl State {
    pub(super) fn new(cx: &mut Context<'_, Shell>) -> Self {
        Self {
            row: None,
            open: None,
            dialog: DialogState::new(cx),
        }
    }

    /// Take an answer for the row or the dialog. False when neither asked.
    pub(super) fn answered(
        &mut self,
        machine: &MachineKey,
        ticket: Ticket,
        result: Result<NodeLog, Problem>,
    ) -> bool {
        let read = [self.row.as_mut(), self.open.as_mut()]
            .into_iter()
            .flatten()
            .find(|read| read.asks(machine, ticket));
        let Some(read) = read else {
            return false;
        };
        read.asking = None;
        read.answer = Some(result);
        true
    }
}

/// Whether `machine` is connected and answers `node.log`.
fn speaks(machine: &Machine) -> bool {
    matches!(machine.reach, Reach::Online)
        && machine
            .protocol
            .zip(DIAGNOSTICS_WIRE_PROTOCOL)
            .is_some_and(|(spoken, since)| spoken >= since)
}

impl Shell {
    /// On Overview, ask the chosen machine for its log's path: once per
    /// machine and connection, and again after an answer that may change
    /// (`asks_again`). A machine that does not answer `node.log` now (not
    /// connected) drops what the row knew of it.
    pub(crate) fn log_row_load(&mut self) {
        if self.areas.settings.section != Section::Overview {
            return;
        }
        let Some(machine) = super::super::choices::chosen_machine(self) else {
            return;
        };
        let (name, epoch, speaks) = (
            machine.name.clone(),
            machine.connection_epoch,
            speaks(machine),
        );
        let state = &mut self.areas.settings.diagnostics;
        if !speaks {
            if state
                .log
                .row
                .as_ref()
                .is_some_and(|read| read.machine == name)
            {
                state.log.row = None;
            }
            return;
        }
        if !asks_again(state.log.row.as_ref(), &name, epoch) {
            return;
        }
        let Ok(ticket) = state.tickets.next_ticket() else {
            return;
        };
        state.log.row = Some(Read {
            machine: name.clone(),
            epoch,
            asking: Some(ticket),
            answer: None,
        });
        self.command(Command::Diagnostics(Ask {
            machine: MachineKey::from_book(&name),
            ticket,
            request: Request::Log { lines: Some(1) },
        }));
    }

    /// Open the dialog for `machine`, asking for the default lines.
    pub(super) fn open_node_log(
        &mut self,
        machine: &str,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let state = &mut self.areas.settings.diagnostics;
        let Ok(ticket) = state.tickets.next_ticket() else {
            return;
        };
        state.log.open = Some(Read {
            machine: machine.to_owned(),
            epoch: None,
            asking: Some(ticket),
            answer: None,
        });
        state.log.dialog.focus(window, cx);
        self.command(Command::Diagnostics(Ask {
            machine: MachineKey::from_book(machine),
            ticket,
            request: Request::Log { lines: None },
        }));
        cx.notify();
    }

    /// Close the dialog, giving focus back to what opened it (the app
    /// reviewer's R1 on row 337: `DialogState::dismissed`).
    pub(super) fn close_node_log(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let state = &mut self.areas.settings.diagnostics.log;
        state.open = None;
        state.dialog.dismissed(window, cx);
        cx.notify();
    }

    fn copy_log_path(&mut self, path: &str, cx: &mut Context<'_, Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(path.to_owned()));
        let said = self.words.diagnostics().log_copied().to_owned();
        self.toast(said, vessel_app_view::toasts::ToastTone::Copied, cx);
    }
}

/// Where a node without a log file writes, or its file's path.
fn path_line(log: &NodeLog, shell: &Shell) -> String {
    let words = shell.words.diagnostics();
    match log {
        NodeLog::File { path, .. } => path.display().to_string(),
        NodeLog::NoFile {
            writes_to: LogSink::Terminal,
        } => words.log_terminal().to_owned(),
        NodeLog::NoFile {
            writes_to: LogSink::Elsewhere,
        } => words.log_elsewhere().to_owned(),
    }
}

/// The Overview section with the "Log file" row, on a machine that answers
/// `node.log`.
pub(in crate::areas::settings) fn log_row(
    machine: &Machine,
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    if !speaks(machine) {
        return None;
    }
    let words = shell.words.diagnostics();
    let read = shell
        .areas
        .settings
        .diagnostics
        .log
        .row
        .as_ref()
        .filter(|read| read.machine == machine.name);
    let answer = read.and_then(|read| read.answer.as_ref());
    let path = match answer {
        Some(Ok(NodeLog::File { path, .. })) => Some(path.display().to_string()),
        Some(Ok(NodeLog::NoFile { .. }) | Err(_)) | None => None,
    };
    let hint = match answer {
        Some(Ok(log)) => path_line(log, shell),
        Some(Err(_)) | None => words.log_unavailable().to_owned(),
    };
    let copied = path.clone();
    let copy = outline_button(
        "node-log-copy-path",
        words.log_copy_path(),
        path.is_some(),
        tokens,
    )
    .on_click(cx.listener(move |shell, _event, _window, cx| {
        if let Some(path) = &copied {
            shell.copy_log_path(path, cx);
        }
    }));
    let name = machine.name.clone();
    let open = outline_button("node-log-open", words.log_open(), path.is_some(), tokens).on_click(
        cx.listener(move |shell, _event, window, cx| {
            shell.open_node_log(&name, window, cx);
        }),
    );
    let controls = div()
        .flex()
        .items_center()
        .gap(px(spacing::S2))
        .child(copy)
        .child(open);
    let card = settings::card(tokens).child(settings::row_at(
        words.log_title(),
        Some(hint.into()),
        None,
        Some(controls.into_any_element()),
        true,
        compact,
        tokens,
    ));
    Some(settings::section(words.log_section(), card, tokens).into_any_element())
}

/// The "Node logs" dialog, while it is open.
pub(super) fn dialog(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.settings.diagnostics.log;
    let read = state.open.as_ref()?;
    let words = shell.words.diagnostics();
    let muted = |text: String| {
        div()
            .type_(Type::Meta)
            .text_color(tokens.neutral.foreground_muted)
            .child(text)
    };
    let body = match &read.answer {
        None => div()
            .flex()
            .items_center()
            .gap(px(spacing::S2))
            .child(vessel_app_kit::slider::spinner(
                "node-log-loading",
                14.0,
                tokens.neutral.foreground_muted,
            ))
            .child(muted(words.log_loading().to_owned())),
        Some(answer) => {
            let hint = match answer {
                Ok(log) => path_line(log, shell),
                Err(_) => words.log_unavailable().to_owned(),
            };
            let contents = match log_contents(answer) {
                LogContents::Lines(text) => Some(output(text, tokens)),
                LogContents::Empty => Some(output(words.log_empty().to_owned(), tokens)),
                LogContents::Error(problem) => Some(
                    div()
                        .type_(Type::Meta)
                        .text_color(tokens.status().error.foreground)
                        .child(super::problem_words(problem, shell))
                        .into_any_element(),
                ),
                LogContents::None => None,
            };
            div()
                .flex()
                .flex_col()
                .gap(px(spacing::S3))
                .pb(px(spacing::S2))
                .child(muted(hint))
                .children(contents)
        }
    };
    let compact = crate::areas::draft::compact(window);
    // Paseo's log text is selectable; GPUI text is not, so a Copy of the
    // lines shown stands in for it in the title row (a stated difference,
    // the coordinator's ruling on row 337).
    let shown = if let Some(LogContents::Lines(text)) = read.answer.as_ref().map(log_contents) {
        Some(text)
    } else {
        None
    };
    let copy = super::copy_action("node-log-copy", shown, shell, tokens, compact, cx);
    let content = DialogContent {
        title: words.log_dialog().into(),
        body: body.into_any_element(),
        footer: Vec::new(),
        size: DialogSize::Wide,
    };
    Some(draw(
        &state.dialog,
        content.with_header_actions(copy),
        tokens,
        window,
        compact,
        Shell::close_node_log,
        cx,
    ))
}

/// Paseo's `logOutput`: muted, 12 px monospace on 18 px lines, wrapped: the
/// kit's code type (12 at the default code size) on lines one and a half
/// times its size, so both follow the person's code size.
fn output(text: String, tokens: &Tokens) -> AnyElement {
    div()
        .type_(Type::Mono)
        .line_height(vessel_app_kit::ui::relative(1.5))
        .text_color(tokens.neutral.foreground_muted)
        .child(text)
        .into_any_element()
}

#[cfg(test)]
mod tests;
