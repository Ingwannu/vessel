//! Diagnostics, as Paseo's (`screens/settings-screen.tsx`
//! `DiagnosticsSection`, `components/app-diagnostic-sheet.tsx`): one row,
//! "App diagnostic", whose Run collects the report and shows it in a
//! dialog (a sheet on a phone) with Copy and Refresh.
//!
//! [Decision Log]
//! - Purpose: Paseo's way to hand over what the app knows when something
//!   is wrong, in one copyable block (`view::diagnostics`).
//! - Choice: Paseo's row and its secondary Run; the report is collected
//!   from what the app already holds (the book's machines as last read,
//!   the window's renderer, where preferences and the log are), so Run is
//!   instant and asks no machine anything new; Refresh collects it again.
//! - Not shown: Paseo's "Test audio" (Vessel plays no sound yet).
//! - Difference: Paseo's progress rows (one per host while it probes)
//!   have nothing to show here, as nothing is waited for.

use vessel_app_kit::dialog::{DialogContent, DialogSize, DialogState, dialog};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, ClipboardItem, Context, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, outline_button, settings};
use vessel_app_view::diagnostics::{self, AppFacts, Renderer};
use vessel_app_view::prefs::Kept;

use crate::shell::Shell;

/// The report, while its dialog is open.
pub(crate) struct State {
    report: Option<String>,
    dialog: DialogState,
    /// Last copy notice, retained while it may still own the global frame foot.
    copied_notice: Option<&'static str>,
}

impl State {
    pub(crate) fn new(cx: &mut Context<'_, Shell>) -> Self {
        Self {
            report: None,
            dialog: DialogState::new(cx),
            copied_notice: None,
        }
    }
}

impl Shell {
    /// Copy the report the button actually showed; retain only notice wording.
    fn copy_diagnostic(&mut self, report: &str, cx: &mut Context<'_, Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(report.to_owned()));
        let copied = self.words.diagnostics().copied();
        self.areas.settings.diagnostics.copied_notice = Some(copied);
        self.notice = Some(copied.to_owned());
        cx.notify();
    }

    /// Refresh an owned copy notice without recollecting or translating data.
    pub(crate) fn translate_diagnostic_notice(&mut self) {
        let state = &mut self.areas.settings.diagnostics;
        if let Some(previous) = state.copied_notice {
            let copied = self.words.diagnostics().copied();
            if self.notice.as_deref() == Some(previous) {
                self.notice = Some(copied.to_owned());
            }
            state.copied_notice = Some(copied);
        }
    }

    /// Collect the report (again) and show it.
    fn run_diagnostic(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let facts = app_facts(self, window);
        let state = &mut self.areas.settings.diagnostics;
        state.report = Some(diagnostics::report(&facts, &self.fleet.machines));
        state.dialog.focus(window, cx);
        cx.notify();
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

/// The report's dialog, while it is open: the text as code, Copy and
/// Refresh under it.
pub(crate) fn dialogs(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.settings.diagnostics;
    let report = state.report.clone()?;
    let words = shell.words.diagnostics();
    let copied = report.clone();
    let copy = outline_button("settings-diagnostic-copy", words.copy(), true, tokens).on_click(
        cx.listener(move |shell, _event, _window, cx| {
            shell.copy_diagnostic(&copied, cx);
        }),
    );
    let refresh = outline_button("settings-diagnostic-refresh", words.refresh(), true, tokens)
        .on_click(cx.listener(|shell, _event, window, cx| shell.run_diagnostic(window, cx)));
    // The report as Paseo's sheet shows it: one block of code text that
    // wraps, on the code surface, selectable by copying the whole.
    let body = div()
        .p(px(spacing::S3))
        .rounded(px(vessel_app_kit::theme::radius::R2))
        .bg(tokens.neutral.surface1)
        .border_1()
        .border_color(tokens.neutral.border)
        .type_(Type::Mono)
        .text_color(tokens.neutral.foreground)
        .child(report);
    Some(dialog(
        &state.dialog,
        DialogContent {
            title: words.report_title().into(),
            body: body.into_any_element(),
            footer: vec![refresh.into_any_element(), copy.into_any_element()],
            size: DialogSize::Wide,
        },
        tokens,
        window,
        crate::areas::draft::compact(window),
        |shell: &mut Shell, _window, _cx| shell.areas.settings.diagnostics.report = None,
        cx,
    ))
}

#[cfg(test)]
mod language_tests;
