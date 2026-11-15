//! Settings rows and the adaptive report viewer use existing kit controls.
use crate::shell::Shell;
use vessel_app_kit::dialog::{DialogContent, DialogSize, dialog};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, ClipboardItem, Context, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, outline_button, settings};
use vessel_app_view::devices::MachineKey;
use vessel_app_view::diagnostics::{collection::Phase, words};

pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let labels = shell.words.diagnostic_flow();
    let legacy = shell.t.lang.settings();
    let run = outline_button("settings-diagnostic-run", legacy.run, true, tokens)
        .on_click(cx.listener(|shell, _event, window, cx| shell.run_diagnostic(None, window, cx)));
    let app = settings::card(tokens).child(settings::row_at(
        legacy.app_diagnostic,
        Some(labels.privacy().into()),
        None,
        Some(run.into_any_element()),
        true,
        compact,
        tokens,
    ));
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
            shell.run_diagnostic(Some(key.clone()), window, cx)
        }));
        hosts = hosts.child(settings::row_at(
            machine.name.clone(),
            Some(labels.partial().into()),
            None,
            Some(run.into_any_element()),
            index == 0,
            compact,
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
            compact,
            tokens,
        ));
    }
    let mut sources = settings::card(tokens);
    let sections = [
        (labels.daemon(), labels.daemon_note()),
        (labels.provider(), labels.provider_note()),
        (labels.process(), labels.process_note()),
        (labels.logs(), labels.logs_note()),
    ];
    for (index, (label, note)) in sections.into_iter().enumerate() {
        sources = sources.child(settings::row_at(
            label,
            Some(note.into()),
            None,
            Some(
                div()
                    .type_(Type::Caption)
                    .text_color(tokens.neutral.foreground_muted)
                    .child(labels.unavailable_badge())
                    .into_any_element(),
            ),
            index == 0,
            compact,
            tokens,
        ));
    }
    div()
        .flex()
        .flex_col()
        .child(settings::section(
            legacy.section(vessel_app_view::settings::Section::Diagnostics),
            app,
            tokens,
        ))
        .child(settings::section(labels.hosts(), hosts, tokens))
        .child(settings::section(labels.partial(), sources, tokens))
        .into_any_element()
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
    let legacy = shell.t.lang.settings();
    let labels = shell.words.diagnostic_flow();
    let report = state.collection.report().cloned();
    let copy = outline_button(
        "settings-diagnostic-copy",
        legacy.copy,
        report.is_some(),
        tokens,
    )
    .on_click(cx.listener(move |shell, _event, _window, cx| {
        if let Some(report) = &report {
            cx.write_to_clipboard(ClipboardItem::new_string(report.as_str().to_owned()));
            shell.notice = Some(shell.t.lang.settings().diagnostic_copied.to_owned());
            cx.notify();
        }
    }));
    let refresh = outline_button("settings-diagnostic-refresh", legacy.refresh, true, tokens)
        .on_click(cx.listener(|shell, _event, window, cx| {
            shell.run_diagnostic(shell.areas.settings.diagnostics.target.clone(), window, cx)
        }));
    let mut body = div().flex().flex_col().gap(px(spacing::S3)).min_w_0();
    body = body.child(status_row(labels.client(), labels.done(), tokens));
    body = body.child(status_row(labels.desktop(), labels.unavailable(), tokens));
    for row in state.collection.rows() {
        body = body.child(status_row(
            row.machine.as_str(),
            words::progress(&row.progress, shell.words),
            tokens,
        ));
    }
    if let Phase::Failed(problem) = state.collection.phase() {
        body = body.child(
            div()
                .type_(Type::Body)
                .child(words::problem(*problem, shell.words)),
        );
    }
    if let Some(report) = state.collection.report() {
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
    Some(dialog(
        &state.dialog,
        DialogContent {
            title: legacy.app_diagnostic.into(),
            body: body.into_any_element(),
            footer: vec![refresh.into_any_element(), copy.into_any_element()],
            size: DialogSize::Wide,
        },
        tokens,
        window,
        crate::areas::draft::compact(window),
        |shell: &mut Shell, _window, _cx| shell.areas.settings.diagnostics.collection.close(),
        cx,
    ))
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
                .type_(Type::Caption)
                .text_color(tokens.neutral.foreground_muted)
                .child(status.to_owned()),
        )
}
