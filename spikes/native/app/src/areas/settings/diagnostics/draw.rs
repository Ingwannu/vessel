//! Settings rows and the adaptive report viewer use existing kit controls.
use crate::shell::Shell;
use vessel_app_kit::dialog::{DialogContent, DialogSize, dialog};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, outline_button, settings};
use vessel_app_view::devices::MachineKey;
use vessel_app_view::diagnostics::{
    collection::{Collection, Phase},
    words,
};
use vessel_app_view::localization::Translator;

pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let labels = shell.words.diagnostic_flow();
    let framing = shell.words.diagnostics();
    let run = outline_button("settings-diagnostic-run", framing.run(), true, tokens)
        .on_click(cx.listener(|shell, _event, window, cx| shell.run_diagnostic(None, window, cx)));
    let app = settings::card(tokens).child(settings::row_at(
        framing.report_title(),
        Some(labels.privacy().into()),
        None,
        Some(run.into_any_element()),
        true,
        compact,
        tokens,
    ));
    let layout = if compact {
        PageLayout::Compact
    } else {
        PageLayout::Wide
    };
    div()
        .flex()
        .flex_col()
        .child(settings::section(framing.title(), app, tokens))
        .child(settings::section(
            labels.hosts(),
            host_card(shell, tokens, layout, cx),
            tokens,
        ))
        .child(settings::section(
            labels.partial(),
            source_card(shell, tokens, layout),
            tokens,
        ))
        .into_any_element()
}
#[derive(Clone, Copy)]
enum PageLayout {
    Compact,
    Wide,
}
impl PageLayout {
    const fn compact(self) -> bool {
        matches!(self, Self::Compact)
    }
}
fn host_card(
    shell: &Shell,
    tokens: &Tokens,
    layout: PageLayout,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let labels = shell.words.diagnostic_flow();
    let mut hosts = settings::card(tokens);
    for (index, machine) in shell.fleet.machines.iter().enumerate() {
        let key = MachineKey::from_book(&machine.name);
        let run = outline_button(
            format!("diagnostic-host-{index}"),
            labels.report(),
            true,
            tokens,
        )
        .on_click(cx.listener(move |shell, _event, window, cx| {
            shell.run_diagnostic(Some(key.clone()), window, cx);
        }));
        hosts = hosts.child(settings::row_at(
            machine.name.clone(),
            Some(labels.partial().into()),
            None,
            Some(run.into_any_element()),
            index == 0,
            layout.compact(),
            tokens,
        ));
    }
    if shell.fleet.machines.is_empty() {
        hosts = hosts.child(settings::row_at(
            labels.no_hosts(),
            None,
            None,
            None,
            true,
            layout.compact(),
            tokens,
        ));
    }
    hosts
}
fn source_card(shell: &Shell, tokens: &Tokens, layout: PageLayout) -> Div {
    let labels = shell.words.diagnostic_flow();
    let mut sources = settings::card(tokens);
    let sections = [
        (
            labels.daemon(),
            labels.daemon_note(),
            labels.host_dependent(),
        ),
        (
            labels.process(),
            labels.process_note(),
            labels.host_dependent(),
        ),
        (
            labels.logs(),
            labels.logs_note(),
            labels.unavailable_badge(),
        ),
    ];
    for (index, (label, note, availability)) in sections.into_iter().enumerate() {
        sources = sources.child(settings::row_at(
            label,
            Some(note.into()),
            None,
            Some(
                div()
                    .type_(Type::Meta)
                    .text_color(tokens.neutral.foreground_muted)
                    .child(availability)
                    .into_any_element(),
            ),
            index == 0,
            layout.compact(),
            tokens,
        ));
    }
    sources
}

pub(crate) fn dialogs(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.settings.diagnostics;
    if state.collection.phase() == &Phase::Hidden {
        return None;
    }
    let framing = shell.words.diagnostics();
    Some(dialog(
        &state.dialog,
        DialogContent {
            title: framing.report_title().into(),
            body: report_body(&state.collection, shell.words, tokens).into_any_element(),
            footer: report_actions(shell, tokens, cx),
            size: DialogSize::Wide,
        },
        tokens,
        window,
        crate::areas::draft::compact(window),
        |shell: &mut Shell, _window, _cx| shell.areas.settings.diagnostics.collection.close(),
        cx,
    ))
}
fn report_actions(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Vec<AnyElement> {
    let framing = shell.words.diagnostics();
    let report = shell
        .areas
        .settings
        .diagnostics
        .collection
        .report()
        .cloned();
    let copy = outline_button(
        "settings-diagnostic-copy",
        framing.copy(),
        report.is_some(),
        tokens,
    )
    .on_click(cx.listener(move |shell, _event, _window, cx| {
        if let Some(report) = &report {
            shell.copy_diagnostic(report, cx);
        }
    }));
    let refresh = outline_button(
        "settings-diagnostic-refresh",
        framing.refresh(),
        shell.areas.settings.diagnostics.collection.can_start(),
        tokens,
    )
    .on_click(cx.listener(|shell, _event, window, cx| {
        shell.refresh_diagnostic(window, cx);
    }));
    vec![refresh.into_any_element(), copy.into_any_element()]
}
fn report_body(run: &Collection, catalog: &'static Translator, tokens: &Tokens) -> Div {
    let labels = catalog.diagnostic_flow();
    let mut body = div().flex().flex_col().gap(px(spacing::S3)).min_w_0();
    let client_status = if run.has_client_report() {
        labels.done()
    } else {
        labels.unavailable()
    };
    body = body.child(status_row(labels.client(), client_status, tokens));
    body = body.child(status_row(labels.desktop(), labels.unavailable(), tokens));
    for row in run.rows() {
        body = body.child(status_row(
            row.machine.as_str(),
            words::progress(&row.progress, catalog),
            tokens,
        ));
    }
    if let Phase::Failed(problem) = run.phase() {
        body = body.child(
            div()
                .type_(Type::Body)
                .child(words::problem(*problem, catalog)),
        );
    }
    if let Some(report) = run.report() {
        body = body.child(
            div()
                .p(px(spacing::S3))
                .rounded(px(vessel_app_kit::theme::radius::R2))
                .bg(tokens.neutral.surface1)
                .type_(Type::Mono)
                .text_color(tokens.neutral.foreground)
                .child(report.as_str().to_owned()),
        );
    }
    body
}

fn status_row(label: &str, status: &str, tokens: &Tokens) -> impl vessel_app_kit::ui::IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(spacing::S1))
        .min_w_0()
        .child(
            div()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground)
                .child(label.to_owned()),
        )
        .child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(status.to_owned()),
        )
}
