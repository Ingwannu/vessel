//! Observed ports are separate from declarations and from service health.
use crate::shell::Shell;
use vessel_app_kit::ui::{
    Context, Div, ElementId, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, alert, button, theme::spacing};
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
pub(super) fn scan(shell: &Shell, tokens: &Tokens) -> Div {
    let words = shell.words.scripts();
    let Some(snapshot) = shell.areas.workspace.scripts.model.read.shown() else {
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
            column = column.child(notice(words.scan_failed(), tokens))
        }
        PortScan::Scanned { .. } => {}
    }
    if ports.more {
        column = column.child(notice(words.scan_more(), tokens));
    }
    column
}
pub(super) fn for_run(
    shell: &Shell,
    run: Option<&TerminalInfo>,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words.scripts();
    let model = &shell.areas.workspace.scripts.model;
    let Some(snapshot) = model.read.shown() else {
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
        let preview_request = Preview {
            terminal: run.id,
            listener: port.clone(),
        };
        let destination = model.preview_url(
            &preview_request,
            &shell.areas.workspace.scripts.host.text(cx),
        );
        let mut preview = button(
            ElementId::Name(format!("script-preview-{}-{index}", port.port).into()),
            words.preview(),
            false,
            tokens,
        );
        if destination.is_ok() {
            preview = preview.on_click(cx.listener(move |shell, _event, _window, cx| {
                shell.script_preview(preview_request.clone(), cx)
            }));
        }
        match destination {
            Ok(url) => {
                column = column.child(div().type_(Type::Meta).child(url.as_str().to_owned()))
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
                )
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
                .child(preview),
        );
    }
    column
}
