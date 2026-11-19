//! Diagnostics, as Paseo's (`screens/settings-screen.tsx`
//! `DiagnosticsSection`, `components/app-diagnostic-sheet.tsx`): one row,
//! "App diagnostic". Its Run asks each machine for its own report
//! (`view::diagnostics::Run`) and shows a progress row per step while it
//! asks, then the whole text, with Copy and Refresh in the title row.
//!
//! [Decision Log]
//! - Purpose: Paseo's way to hand over what the app and each host know when
//!   something is wrong, in one copyable block.
//! - Before (to row 335): Run collected only what the app held and asked no
//!   machine, so it had no progress to show, and a machine's own state never
//!   reached the report.
//! - Choice:
//!   - Paseo's row and its secondary Run.
//!   - Paseo's sheet: the progress card while any machine is asked, then
//!     only the text, never both (the coordinator's ruling on row 334's page).
//!   - Copy and Refresh are the title row's icon actions, as Paseo's header
//!     actions.
//! - Not shown: Paseo's "Test audio" (Vessel plays no sound yet).
//! - An agent's Diagnostic (`agent`) and a machine's log (`node_log`) belong
//!   to this area too: one ticket source and one router for the answers.

use vessel_app_kit::dialog::{DialogContent, DialogSize, DialogState, dialog};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::toolbar_button::{ToolbarButton, ToolbarConfig, ToolbarIcon, ToolbarInk};
use vessel_app_kit::ui::{
    AnyElement, ClipboardItem, Context, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonState, ControlSizing, Tokens, Type, TypeExt as _, assets::Glyph, outline_button, settings,
};
use vessel_app_link::Command;
use vessel_app_link::diagnostics::{Answer, Ask, Reply, Request};
use vessel_app_view::devices::Tickets;
use vessel_app_view::diagnostics::{AppFacts, Problem, Renderer, Run, Step, StepOf, StepState};
use vessel_app_view::prefs::Kept;

use crate::shell::Shell;

mod agent;
mod node_log;

pub(super) use agent::diagnostic_button;
pub(super) use node_log::log_row;

/// The area's runs, sheets and their dialogs.
pub(crate) struct State {
    run: Option<Run>,
    dialog: DialogState,
    tickets: Tickets,
    agent: agent::State,
    log: node_log::State,
}

impl State {
    pub(crate) fn new(cx: &mut Context<'_, Shell>) -> Self {
        Self {
            run: None,
            dialog: DialogState::new(cx),
            tickets: Tickets::default(),
            agent: agent::State::new(cx),
            log: node_log::State::new(cx),
        }
    }
}

impl Shell {
    /// Copy the report the button actually showed, then Paseo's copied
    /// toast (app-diagnostic-sheet.tsx 149: "Copied diagnostic").
    fn copy_diagnostic(&mut self, report: &str, cx: &mut Context<'_, Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(report.to_owned()));
        self.toast_copied(self.words.diagnostics().copied_label(), cx);
    }

    /// Start a run (again): ask each machine that can answer, and show the
    /// dialog in its progress.
    fn run_diagnostic(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let facts = app_facts(self, window);
        let state = &mut self.areas.settings.diagnostics;
        let (run, asks) = Run::start(facts, &self.fleet.machines, &mut state.tickets);
        state.run = Some(run);
        state.dialog.focus(window, cx);
        for (machine, ticket) in asks {
            self.command(Command::Diagnostics(Ask {
                machine,
                ticket,
                request: Request::Report,
            }));
        }
        cx.notify();
    }

    /// A diagnostics call answered: to the run, the agent's sheet or the
    /// log, whichever asked with its ticket (one source for all three).
    pub(crate) fn diagnostics_answer(&mut self, answer: Answer) {
        let Answer {
            machine,
            ticket,
            result,
        } = answer;
        let state = &mut self.areas.settings.diagnostics;
        match result {
            Ok(Reply::Report(report)) => {
                if let Some(run) = state.run.as_mut() {
                    let _taken = run.answered(&machine, ticket, Ok(*report));
                }
            }
            Ok(Reply::Agent(diagnostic)) => {
                // The shared renderer ends the block with a newline; the surface
                // draws no empty last line.
                let _taken = state
                    .agent
                    .answered(ticket, Ok(diagnostic.text().trim_end().to_owned()));
            }
            Ok(Reply::Log(log)) => {
                let _taken = state.log.answered(&machine, ticket, Ok(log));
            }
            Err(problem) => {
                let taken = state
                    .run
                    .as_mut()
                    .is_some_and(|run| run.answered(&machine, ticket, Err(problem)));
                if !taken && !state.agent.answered(ticket, Err(problem)) {
                    let _taken = state.log.answered(&machine, ticket, Err(problem));
                }
            }
        }
    }
}

/// What the app knows of itself now.
fn app_facts(shell: &Shell, window: &Window) -> AppFacts {
    let renderer = window.gpu_specs().map_or(Renderer::Unknown, |specs| {
        if specs.is_software_emulated {
            Renderer::Software {
                device: specs.device_name,
            }
        } else {
            Renderer::Gpu {
                device: specs.device_name,
                driver: specs.driver_name,
            }
        }
    });
    let preferences = match &shell.prefs.kept {
        Kept::File { path } => path.clone(),
        Kept::ThisLaunch { .. } => shell
            .t
            .lang
            .settings()
            .kept_why(&shell.prefs.kept)
            .unwrap_or_default(),
    };
    // A browser has no log file and no home folder (`vessel-app-log` and
    // `dirs` are the desktop's): the report leaves both out.
    #[cfg(not(target_arch = "wasm32"))]
    let (log, home) = (
        vessel_app_log::dir().map(|dir| dir.display().to_string()),
        dirs::home_dir().map(|home| home.display().to_string()),
    );
    #[cfg(target_arch = "wasm32")]
    let (log, home) = (None, None);
    AppFacts {
        collected_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        label: crate::LABEL.to_owned(),
        platform: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        renderer,
        preferences,
        log,
        home,
    }
}

/// The Diagnostics page.
pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.words.diagnostics();
    let run = outline_button("settings-diagnostic-run", words.run(), true, tokens)
        .on_click(cx.listener(|shell, _event, window, cx| shell.run_diagnostic(window, cx)));
    let card = settings::card(tokens).child(settings::row_at(
        words.report_title(),
        Some(words.description().into()),
        None,
        Some(run.into_any_element()),
        true,
        compact,
        tokens,
    ));
    settings::section(words.title(), card, tokens).into_any_element()
}

/// The area's dialogs that are open: the app diagnostic, an agent's
/// Diagnostic, a machine's Node logs.
pub(crate) fn dialogs(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Vec<AnyElement> {
    [
        run_dialog(shell, tokens, window, cx),
        agent::dialog(shell, tokens, window, cx),
        node_log::dialog(shell, tokens, window, cx),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// The app diagnostic's dialog: its progress, or its text once every
/// machine answered.
fn run_dialog(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.settings.diagnostics;
    let run = state.run.as_ref()?;
    let words = shell.words.diagnostics();
    let compact = crate::areas::draft::compact(window);
    let text = run.text();
    let body = text.clone().map_or_else(
        || progress(&run.steps(), shell, tokens),
        |text| code_surface(text, tokens),
    );
    let actions = title_actions(
        "settings-diagnostic",
        (text, run.running()),
        shell,
        tokens,
        compact,
        cx,
        Shell::run_diagnostic,
    );
    let content = DialogContent {
        title: words.report_title().into(),
        body,
        footer: Vec::new(),
        size: DialogSize::Wide,
    };
    Some(dialog(
        &state.dialog,
        content.with_header_actions(actions),
        tokens,
        window,
        compact,
        |shell: &mut Shell, _window, _cx| shell.areas.settings.diagnostics.run = None,
        cx,
    ))
}

/// Paseo's progress card: a row per step, a spinner while it runs, else a
/// dot, then "label: state".
fn progress(steps: &[Step], shell: &Shell, tokens: &Tokens) -> AnyElement {
    let words = shell.words.diagnostics();
    let status = tokens.status();
    let rows = steps.iter().enumerate().map(|(index, step)| {
        let label = match &step.of {
            StepOf::App => words.step_app().to_owned(),
            StepOf::Machine(name) => name.clone(),
        };
        let (mark, said) = match step.state {
            StepState::Running => (
                vessel_app_kit::slider::spinner(
                    ("diagnostic-step", index),
                    14.0,
                    tokens.neutral.foreground_muted,
                ),
                words.step_running(&label),
            ),
            StepState::Done => (dot(status.success.foreground), words.step_done(&label)),
            StepState::Failed => (dot(status.error.foreground), words.step_failed(&label)),
        };
        div()
            .flex()
            .items_center()
            .gap(px(spacing::S2))
            .child(mark)
            .child(
                div()
                    .type_(Type::Meta)
                    .text_color(tokens.neutral.foreground_muted)
                    .child(said.unwrap_or(label)),
            )
    });
    div()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .p(px(spacing::S4))
        .rounded(px(vessel_app_kit::theme::radius::R2))
        .bg(tokens.neutral.surface1)
        .border_1()
        .border_color(tokens.neutral.border)
        .children(rows)
        .into_any_element()
}

/// A step's dot.
fn dot(colour: vessel_app_kit::ui::Hsla) -> AnyElement {
    div()
        .size(px(spacing::S2))
        .flex_none()
        .rounded_full()
        .bg(colour)
        .into_any_element()
}

/// A diagnostic's text as Paseo's code surface: one block of code text
/// that wraps, selectable by copying the whole.
fn code_surface(text: String, tokens: &Tokens) -> AnyElement {
    div()
        .p(px(spacing::S3))
        .rounded(px(vessel_app_kit::theme::radius::R2))
        .bg(tokens.neutral.surface1)
        .border_1()
        .border_color(tokens.neutral.border)
        .type_(Type::Mono)
        .text_color(tokens.neutral.foreground)
        .child(text)
        .into_any_element()
}

/// Paseo's title-row actions for a diagnostic: Copy (only with text), then
/// Refresh (a spinner while it runs). `shown`: the text, and whether it runs.
fn title_actions(
    id: &'static str,
    shown: (Option<String>, bool),
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
    refresh: impl Fn(&mut Shell, &mut Window, &mut Context<'_, Shell>) + 'static,
) -> AnyElement {
    let (text, running) = shown;
    let words = shell.words.diagnostics();
    let state = |enabled: bool| {
        ToolbarConfig::new(
            ControlSizing::from_compact(compact),
            if enabled {
                ButtonState::Enabled
            } else {
                ButtonState::Disabled
            },
        )
    };
    let copy = ToolbarButton::new(
        (id, 0_usize),
        words.copy_label(),
        ToolbarIcon::new(Glyph::Copy, ToolbarInk::ExtraMuted).adaptive(),
        state(text.is_some()),
        tokens,
    )
    .on_click(cx.listener(move |shell, _event, _window, cx| {
        if let Some(text) = &text {
            shell.copy_diagnostic(text, cx);
        }
    }));
    let again = if running {
        ToolbarButton::loading(
            (id, 1_usize),
            words.refreshing_label(),
            ToolbarInk::ExtraMuted,
            state(false),
            tokens,
        )
    } else {
        ToolbarButton::new(
            (id, 1_usize),
            words.refresh_label(),
            ToolbarIcon::new(Glyph::RotateCw, ToolbarInk::ExtraMuted).adaptive(),
            state(true),
            tokens,
        )
    }
    .on_click(cx.listener(move |shell, _event, window, cx| refresh(shell, window, cx)));
    div()
        .flex()
        .items_center()
        .gap(px(spacing::S2))
        .child(copy)
        .child(again)
        .into_any_element()
}

/// A call's failure, as the screen says it.
fn problem_words(problem: Problem, shell: &Shell) -> String {
    let words = shell.words.diagnostics();
    match problem {
        Problem::Offline => words.problem_offline().to_owned(),
        Problem::Refused(code) => {
            let code = vessel_proto::diagnostics::text::words(&code);
            words.problem_refused(&code).unwrap_or(code)
        }
        Problem::TimedOut => words.problem_timed_out().to_owned(),
    }
}

#[cfg(test)]
mod language_tests;
