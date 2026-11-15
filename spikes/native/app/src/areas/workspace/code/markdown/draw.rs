//! Preview chrome and the shared Markdown renderer; no filesystem reads here.
use super::super::{Shown, Viewer, buffers::FileKey};
use super::{Mode, Stage};
use crate::shell::Shell;
use std::sync::Arc;
use vessel_app_kit::chat_markdown::{
    Options,
    actions::{Actions, CopyStyle},
    chat_markdown_with,
};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, button};
use vessel_app_view::workspace::file_ticket::FileTicket;
use vessel_app_view::workspace::markdown_preview::MarkdownDocument;
pub(in crate::areas::workspace::code) fn toolbar(
    shell: &Shell,
    viewer: &Viewer,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let preview = viewer.markdown.as_ref()?;
    let key = FileKey::new(&viewer.scope, viewer.path.clone());
    let labels = shell.words().labels();
    Some(
        div()
            .flex()
            .gap(px(spacing::S1))
            .children(
                [
                    (Mode::Source, "file-mode-source", labels.file_mode_source),
                    (Mode::Preview, "file-mode-preview", labels.file_mode_preview),
                ]
                .map(|(mode, id, label)| {
                    let key = key.clone();
                    button(id, label, preview.mode == mode, tokens).on_click(cx.listener(
                        move |shell, _, _, cx| {
                            if let Some(preview) = shell
                                .areas
                                .workspace
                                .code
                                .viewer_for(&key)
                                .and_then(|viewer| viewer.markdown.as_mut())
                            {
                                preview.mode = mode;
                            }
                            cx.notify();
                        },
                    ))
                }),
            )
            .into_any_element(),
    )
}

pub(in crate::areas::workspace::code) fn body(
    shell: &Shell,
    viewer: &Viewer,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let preview = viewer.markdown.as_ref()?;
    if preview.mode == Mode::Source {
        return None;
    }
    let loading = viewer.edit.as_ref().map_or_else(
        || matches!(viewer.text, Shown::Loading),
        |editing| {
            editing.model.status() == vessel_app_view::workspace::editor::EditorStatus::Loading
        },
    );
    if loading {
        return Some(
            super::super::frame::note(shell.words().labels().loading, tokens).into_any_element(),
        );
    }
    if matches!(
        viewer.text,
        Shown::Failed(_) | Shown::Binary | Shown::TooLarge
    ) && viewer.edit.is_none()
    {
        return None;
    }
    if viewer.edit.as_ref().is_some_and(|editing| {
        editing.model.status() == vessel_app_view::workspace::editor::EditorStatus::Failed
    }) {
        return None;
    }
    let key = FileKey::new(&viewer.scope, viewer.path.clone());
    let content = stage_element(shell, viewer, key, &preview.stage, tokens, window, cx);
    Some(
        div()
            .id("markdown-preview-frame")
            .size_full()
            .overflow_y_scroll()
            .p(px(spacing::S3))
            .type_(Type::Body)
            .text_color(tokens.neutral.foreground)
            .child(content)
            .into_any_element(),
    )
}

fn document_element(
    key: FileKey,
    ticket: FileTicket,
    document: &MarkdownDocument,
    unavailable: &'static str,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let weak = cx.entity().downgrade();
    let actions = Actions::new(move |href, _, app| {
        let _gone = weak.update(app, |shell, cx| {
            shell.markdown_link(&key, href.as_str(), cx);
        });
    })
    .with_images(move |_| {
        gpui::ImageSource::Custom(Arc::new(move |_, _| {
            Some(Err(gpui::ImageCacheError::Asset(unavailable.into())))
        }))
    });
    let table = document.front_matter.iter().map(|row| {
        div()
            .flex()
            .gap(px(spacing::S3))
            .border_b_1()
            .border_color(tokens.neutral.border)
            .child(div().flex_1().child(row.key.clone()))
            .child(div().flex_1().child(row.value.clone()))
    });
    div()
        .flex()
        .flex_col()
        .gap(px(spacing::S3))
        .children(table)
        .child(chat_markdown_with(
            "file-markdown-body",
            &document.body,
            ticket.0,
            Options {
                marks: None,
                actions: Some(actions),
                copy: CopyStyle::MarkdownCodeAware,
            },
            tokens,
            window,
            cx,
        ))
        .into_any_element()
}

fn stage_element(
    shell: &Shell,
    viewer: &Viewer,
    key: FileKey,
    stage: &Stage,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let labels = shell.words().labels();
    match stage {
        Stage::Idle => {
            // Loading a file is not a terminal failure; wait for its first answer.
            if viewer
                .edit
                .as_ref()
                .is_some_and(|editing| editing.model.version().is_some())
                || matches!(viewer.text, Shown::Text { .. })
            {
                cx.defer_in(window, move |shell, _, cx| shell.prepare_markdown(&key, cx));
            }
            div().child(labels.loading).into_any_element()
        }
        Stage::Parsing(_) => div().child(labels.loading).into_any_element(),
        Stage::Incomplete => div()
            .child(labels.file_preview_incomplete)
            .into_any_element(),
        Stage::Ready { ticket, document } => document_element(
            key,
            *ticket,
            document,
            labels.file_image_unavailable,
            tokens,
            window,
            cx,
        ),
    }
}
