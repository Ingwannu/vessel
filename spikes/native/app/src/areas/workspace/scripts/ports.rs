//! Observed ports are separate from declarations and from service health.
use super::draw::Card;
use crate::shell::Shell;
use vessel_app_kit::ui::{
    Context, Div, ElementId, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, div, px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, Tokens, Type, TypeExt as _, alert, button_with_kind, theme::spacing,
};
use vessel_app_view::workspace::scripts::{Preview, listeners};
use vessel_proto::ports::PortScan;
use vessel_proto::terminals::TerminalInfo;

pub(super) fn notice(text: &str, tokens: &Tokens) -> Div {
    alert::alert(
        alert::Content {
            title: None,
            description: Some(div().child(text.to_owned()).into_any_element()),
            action: None,
        },
        alert::Variant::Warning,
        alert::Size::Small,
        tokens,
    )
}
pub(super) fn scan(card: &Card<'_>, tokens: &Tokens) -> Div {
    let shell = card.shell;
    let words = shell.words.scripts();
    let Some(snapshot) = card.model.shown() else {
        return div();
    };
    let ports = match &snapshot.ports {
        Ok(ports) => ports,
        Err(_problem) => return notice(words.scan_failed(), tokens),
    };
    let mut column = div().flex().flex_col().gap(px(spacing::S2));
    match &ports.scan {
        PortScan::Pending => column = column.child(notice(words.scan_pending(), tokens)),
        PortScan::Unavailable { .. } | PortScan::Unknown => {
            column = column.child(notice(words.scan_failed(), tokens));
        }
        PortScan::Scanned { .. } => {}
    }
    if ports.more {
        column = column.child(notice(words.scan_more(), tokens));
    }
    column
}
pub(super) fn for_run(
    card: &Card<'_>,
    run: Option<&TerminalInfo>,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let shell = card.shell;
    let words = shell.words.scripts();
    let model = card.model;
    let Some(snapshot) = model.shown() else {
        return div();
    };
    let Ok(ports) = &snapshot.ports else {
        return div();
    };
    if !matches!(ports.scan, PortScan::Scanned { .. }) {
        return div();
    }
    let mut column = div().flex().flex_col().gap(px(spacing::S2)).child(
        div()
            .type_(Type::Meta)
            .text_color(tokens.neutral.foreground_muted)
            .child(words.ports()),
    );
    let observed = listeners(run, ports);
    if observed.is_empty() {
        return column.child(div().type_(Type::Meta).child(words.no_ports()));
    }
    for (index, port) in observed.into_iter().enumerate() {
        let Some(run) = run else {
            continue;
        };
        let preview_request = Preview::observed(run, port.clone());
        let target = model.target.clone();
        let destination = target
            .as_ref()
            .filter(|target| shell.script_card_is_current(target))
            .ok_or(vessel_app_view::workspace::scripts::PreviewProblem::Stale)
            .and_then(|_| model.preview_url(&preview_request, &card.host.text(cx)));
        let mut preview = button_with_kind(
            ElementId::Name(format!("script-preview-{}-{index}", port.port).into()),
            words.preview(),
            ButtonKind::Secondary,
            if destination.is_ok() {
                ButtonState::Enabled
            } else {
                ButtonState::Disabled
            },
            tokens,
        )
        .min_h(px(vessel_app_kit::breakpoint::TOUCH_TARGET));
        let mut copy = button_with_kind(
            ElementId::Name(format!("script-copy-{}-{index}", port.port).into()),
            words.copy(),
            ButtonKind::Secondary,
            if destination.is_ok() {
                ButtonState::Enabled
            } else {
                ButtonState::Disabled
            },
            tokens,
        )
        .min_h(px(vessel_app_kit::breakpoint::TOUCH_TARGET));
        if destination.is_ok() {
            let copy_target = target.clone();
            let copy_preview = preview_request.clone();
            copy = copy.on_click(cx.listener(move |shell, _event, window, cx| {
                if let Some(target) = &copy_target {
                    shell.script_preview_for(
                        target,
                        &copy_preview,
                        super::UrlAction::Copy,
                        window,
                        cx,
                    );
                }
            }));
            preview = preview.on_click(cx.listener(move |shell, _event, window, cx| {
                if let Some(target) = &target {
                    shell.script_preview_for(
                        target,
                        &preview_request,
                        super::UrlAction::Open,
                        window,
                        cx,
                    );
                }
            }));
        }
        match destination {
            Ok(url) => {
                column = column.child(div().type_(Type::Meta).child(url.as_str().to_owned()));
            }
            Err(problem) => {
                column = column.child(
                    div().type_(Type::Meta).child(
                        vessel_app_view::workspace::scripts::words::issue(
                            shell.words,
                            &vessel_app_view::workspace::scripts::Issue::Preview(problem),
                        )
                        .to_owned(),
                    ),
                );
            }
        }
        column = column.child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(spacing::S2))
                .child(
                    div()
                        .type_(Type::Meta)
                        .child(format!("{}:{}", port.address, port.port)),
                )
                .child(preview)
                .child(copy),
        );
    }
    column
}
