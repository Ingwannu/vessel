//! Attention rows use existing controls. Opening a real `TerminalId` pane stays
//! at C2/W12's owner boundary; selecting an alert can always reveal this row.
use crate::shell::Shell;
use vessel_app_kit::theme::spacing::{S0, S2, S3};
use vessel_app_kit::ui::{
    AnyElement, Context, ElementId, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, outline_button};
use vessel_app_link::inbox::terminals::Action;
use vessel_app_view::inbox::{Listing, terminals::Key, words};

pub(super) fn render(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> AnyElement {
    let words = shell.words.inbox();
    let mut body = div().flex().flex_col().gap(px(S2)).min_w(px(S0));
    let mut any = false;
    for node in shell.areas.inbox.store.nodes() {
        for row in node
            .snapshot
            .iter()
            .flat_map(|snapshot| snapshot.terminals.iter().flatten())
        {
            if !any {
                body = body.child(div().type_(Type::Label).child(words.terminals()));
                any = true;
            }
            let key = Key {
                node: node.source.node(),
                terminal: row.terminal,
            };
            let mut line = div()
                .flex()
                .flex_col()
                .gap(px(S2))
                .p(px(S3))
                .rounded(px(vessel_app_kit::theme::radius::R2))
                .bg(if shell.areas.inbox.terminals.selected == Some(key) {
                    tokens.neutral.interaction_highlight
                } else {
                    tokens.neutral.surface1
                })
                .child(
                    div().type_(Type::Label).child(
                        row.title
                            .as_ref()
                            .map_or(words.terminal_untitled(), |title| title.as_str())
                            .to_owned(),
                    ),
                )
                .child(
                    div()
                        .type_(Type::Meta)
                        .child(node.source.name().as_str().to_owned()),
                )
                .child(
                    div()
                        .type_(Type::Meta)
                        .child(words::terminal(row, shell.words)),
                );
            if let Some(state) = row
                .attention
                .filter(|state| vessel_app_view::inbox::terminals::announces(*state))
            {
                let source = node.source.clone();
                let pending = shell.areas.inbox.terminals.pending.iter().any(|(known, action, _)| *known == source && matches!(action, Action::Clear { terminal, .. } if *terminal == row.terminal));
                let enabled = node.listing == Listing::Ready && !pending;
                line = line.child(
                    outline_button(
                        ElementId::Name(
                            format!("terminal-clear-{}-{}", key.node, key.terminal).into(),
                        ),
                        words.read(),
                        enabled,
                        tokens,
                    )
                    .on_click(cx.listener(
                        move |shell, _event, _window, cx| {
                            if !enabled {
                                return;
                            }
                            shell.inbox_terminal_action(
                                source.clone(),
                                Action::Clear {
                                    terminal: state.terminal,
                                    revision: state.revision,
                                },
                            );
                            cx.notify();
                        },
                    )),
                );
            }
            body = body.child(line);
        }
    }
    body.into_any_element()
}
