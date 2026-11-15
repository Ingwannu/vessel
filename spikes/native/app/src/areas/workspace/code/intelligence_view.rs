//! Language results show their actual saved/cache freshness. Empty
//! diagnostics are never described as a clean or complete workspace.

use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, button};
use vessel_app_link::workspace::IntelligenceResult;
use vessel_app_view::workspace::intelligence::{
    IntelligenceAction, IntelligenceFailure, language_for,
};
use vessel_proto::lsp::{Freshness, Location};

use super::buffers::FileKey;
use super::intelligence::{Refusal, State};
use crate::shell::Shell;

pub(super) fn panel(
    shell: &Shell,
    viewer: &super::Viewer,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let _language = language_for(viewer.path.as_str())?;
    let key = FileKey::new(&viewer.scope, viewer.path.clone());
    let labels = shell.words().labels();
    let actions = [
        (IntelligenceAction::Hover, labels.language_hover),
        (IntelligenceAction::Definition, labels.language_definition),
        (IntelligenceAction::Diagnostics, labels.language_diagnostics),
    ];
    let toolbar = div()
        .flex()
        .flex_row()
        .flex_wrap()
        .gap(px(spacing::S1))
        .children(
            actions
                .into_iter()
                .enumerate()
                .map(|(index, (action, label))| {
                    let key = key.clone();
                    button(("file-language", index), label, false, tokens).on_click(cx.listener(
                        move |shell, _, window, cx| {
                            shell.language_query_for(&key, action, window, cx);
                        },
                    ))
                }),
        );
    let mut body = div()
        .flex()
        .flex_col()
        .gap(px(spacing::S1))
        .p(px(spacing::S2))
        .child(toolbar);
    match &viewer.intelligence {
        State::Idle => {}
        State::Loading(_) => body = body.child(labels.loading),
        State::Refused(refusal) => {
            body = body.child(match refusal {
                Refusal::Unsaved => labels.language_save_first,
                Refusal::Workspace | Refusal::Path | Refusal::Language => {
                    labels.language_unavailable
                }
            });
        }
        State::Ready { result, .. } => {
            body = body.child(result_body(shell, &key, result, tokens, cx));
        }
    }
    Some(
        body.id("file-language-panel")
            .max_h(px(spacing::S32))
            .overflow_y_scroll()
            .flex_shrink_0()
            .type_(Type::Meta)
            .text_color(tokens.neutral.foreground_muted)
            .into_any_element(),
    )
}

fn failure(shell: &Shell, failure: &IntelligenceFailure) -> String {
    match failure {
        IntelligenceFailure::Call(problem) => shell.words().problem(problem),
        IntelligenceFailure::Deadline => shell.words().labels().language_timeout.to_owned(),
    }
}

fn result_body(
    shell: &Shell,
    key: &FileKey,
    result: &IntelligenceResult,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let labels = shell.words().labels();
    let mut body = div().flex().flex_col().gap(px(spacing::S1));
    let (freshness, next, truncated) = match result {
        IntelligenceResult::Hover(Ok(hover)) => {
            // Plain display deliberately never interprets server HTML or
            // links as executable UI; the supplied content remains visible.
            body = body.child(if hover.text.is_empty() {
                labels.language_no_hover.to_owned()
            } else {
                hover.text.clone()
            });
            (hover.context.freshness, None, hover.truncated)
        }
        IntelligenceResult::Definitions(Ok(page)) => {
            if page.items.is_empty() {
                body = body.child(labels.language_no_locations);
            }
            for (index, location) in page.items.iter().enumerate() {
                body = body.child(location_row(key.clone(), index, location, None, tokens, cx));
            }
            (page.context.freshness, page.next, page.truncated)
        }
        IntelligenceResult::Diagnostics(Ok(page)) => {
            if page.items.is_empty() {
                body = body.child(labels.language_no_diagnostics);
            }
            for (index, diagnostic) in page.items.iter().enumerate() {
                body = body.child(location_row(
                    key.clone(),
                    index,
                    &diagnostic.location,
                    Some(&diagnostic.message),
                    tokens,
                    cx,
                ));
            }
            (page.context.freshness, page.next, page.truncated)
        }
        IntelligenceResult::Hover(Err(error))
        | IntelligenceResult::Definitions(Err(error))
        | IntelligenceResult::Diagnostics(Err(error)) => {
            return body.child(failure(shell, error)).into_any_element();
        }
    };
    body = body.child(match freshness {
        Freshness::Saved => labels.language_saved,
        Freshness::Cached => labels.language_cached,
        Freshness::Unknown => labels.language_unknown,
    });
    if truncated {
        body = body.child(labels.language_truncated);
    }
    if let Some(offset) = next {
        let key = key.clone();
        body = body.child(
            button("file-language-more", labels.show_rest, false, tokens).on_click(
                cx.listener(move |shell, _, _, cx| shell.language_next_for(&key, offset, cx)),
            ),
        );
    }
    body.into_any_element()
}

fn location_row(
    key: FileKey,
    index: usize,
    location: &Location,
    message: Option<&str>,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let target = location.clone();
    let path = location.file.as_path().display();
    let line = location.range.start.line;
    let column = location.range.start.column;
    let text = message.map_or_else(
        || format!("{path}:{line}:{column}"),
        |message| format!("{path}:{line}:{column} {message}"),
    );
    button(("file-language-location", index), text, false, tokens)
        .on_click(
            cx.listener(move |shell, _, _, cx| shell.language_location_for(&key, &target, cx)),
        )
        .into_any_element()
}
