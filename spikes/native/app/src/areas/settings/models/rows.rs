//! The provider sheet's model/id/provider order through shared furniture.
//! These shared settings labels have no text-selection control. The existing
//! copy button preserves whole identifiers without a second editing control.
use super::*;
use vessel_app_kit::copy::{CopyLook, copy_button};
use vessel_app_kit::ui::{App, FluentBuilder as _, InteractiveElement as _};
use vessel_app_view::models::page::Snapshot;
use vessel_proto::ModelRoute;

pub(super) fn card<'a>(
    routes: impl IntoIterator<Item = &'a ModelRoute>,
    snapshot: &Snapshot,
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    window: &Window,
    cx: &App,
) -> Div {
    settings::card(tokens).children(
        routes
            .into_iter()
            .map(|route| row(route, snapshot, shell, tokens, compact, window, cx)),
    )
}
fn row(
    route: &ModelRoute,
    snapshot: &Snapshot,
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    window: &Window,
    cx: &App,
) -> AnyElement {
    let words = shell.words.models();
    let title = div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(spacing::S3))
        .child(
            div()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground)
                .child(route.name.to_string()),
        )
        .child(identifier(
            "model",
            route.model.as_str(),
            tokens,
            compact,
            window,
            cx,
        ));
    let facts = div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(spacing::S3))
        .child(text(route.provider.to_string(), tokens).type_(Type::Meta))
        .child(text(words.api_name(route.api), tokens).type_(Type::Meta))
        .children(route.small_model.as_ref().map(|small| {
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(spacing::S2))
                .child(text(words.small(), tokens).type_(Type::Meta))
                .child(identifier(
                    "small-model",
                    small.as_str(),
                    tokens,
                    compact,
                    window,
                    cx,
                ))
        }));
    let contents = div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .child(title)
        .child(facts)
        .children(usage(route, snapshot, shell, tokens));
    // modelRow uses vertical8/horizontal16 and a top hairline even first.
    // The existing card/frame retain their radius, colour and other insets.
    settings::row_with_content(contents.into_any_element(), None, false, compact, tokens)
        .id(gpui::SharedString::from(route.name.to_string()))
        .py(px(spacing::S2))
        .items_start()
        .into_any_element()
}
fn usage(route: &ModelRoute, snapshot: &Snapshot, shell: &Shell, tokens: &Tokens) -> Option<Div> {
    let words = shell.words.models();
    let line = div().flex().flex_wrap().items_center().gap(px(spacing::S2));
    match models::usage(route, snapshot) {
        models::Usage::Unavailable => {
            Some(line.child(text(words.usage_unavailable(), tokens).type_(Type::Meta)))
        }
        models::Usage::Unused => None,
        models::Usage::Profiles { first, rest } => Some(
            div()
                .flex()
                .flex_col()
                .gap(px(spacing::S1))
                .child(text(words.used_by(), tokens).type_(Type::Meta))
                .children(std::iter::once(first).chain(rest).map(|user| {
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap(px(spacing::S2))
                        .children(
                            user.distinct_label()
                                .map(|label| text(label.to_owned(), tokens).type_(Type::Meta)),
                        )
                        .child(code(user.profile.as_str(), tokens))
                        .child(text(user.agent_label, tokens).type_(Type::Meta))
                })),
        ),
    }
}

fn identifier(
    id: &'static str,
    value: &str,
    tokens: &Tokens,
    compact: bool,
    window: &Window,
    cx: &App,
) -> Div {
    div()
        .min_w_0()
        .max_w_full()
        .flex()
        .items_center()
        .gap(px(spacing::S1))
        .child(code(value, tokens))
        .child(
            copy_button(
                id,
                value.to_owned().into(),
                CopyLook {
                    glyph: tokens.neutral.foreground_muted,
                    hover: tokens.neutral.surface2,
                },
                window,
                cx,
            )
            .when(compact, |button| {
                button.size(px(vessel_app_kit::breakpoint::TOUCH_TARGET))
            }),
        )
}
fn code(value: &str, tokens: &Tokens) -> Div {
    div()
        .min_w_0()
        .max_w_full()
        .whitespace_normal()
        .type_(Type::Mono)
        .text_color(tokens.neutral.foreground_muted)
        .child(value.to_owned())
}

#[cfg(test)]
mod tests;
