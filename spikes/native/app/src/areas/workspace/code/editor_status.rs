//! The editor's caret and selection, in a view of their own: the caret's
//! line and column (Paseo's toolbar, "Ln 3, Col 7") and, while the
//! selection covers lines, "Add lines 12–20 to chat".
//!
//! Why a view of its own: the editor redraws on every caret move and on
//! each blink of the caret; a view that follows it redraws as often, and
//! the panel's whole screen must not (it would be drawn twice a second
//! while nothing else moves). The rest of the toolbar (the save state) is
//! the panel's, drawn when the model changes.
//!
//! The button comes in (`QUICK`, `appear`) as a selection starts.

use gpui::AppContext as _;
use vessel_app_kit::code_editor::CodeEditor;
use vessel_app_kit::motion::QUICK;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    Context, Entity, IntoElement, ParentElement as _, Render, StatefulInteractiveElement as _,
    Styled as _, Subscription, WeakEntity, Window, div, px,
};
use vessel_app_kit::{Type, TypeExt as _};

use super::appear::appear;
use crate::shell::Shell;

/// The caret's line and column, and the selection's button.
pub(super) struct Status {
    editor: CodeEditor,
    shell: WeakEntity<Shell>,
    _follow: Subscription,
}

impl Status {
    pub(super) fn new(editor: CodeEditor, cx: &mut Context<'_, Shell>) -> Entity<Self> {
        let shell = cx.entity().downgrade();
        cx.new(|cx| {
            let follow = editor.observe(cx, |_status, cx| cx.notify());
            Self {
                editor,
                shell,
                _follow: follow,
            }
        })
    }
}

impl Render for Status {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = vessel_app_kit::tokens(cx);
        let neutral = tokens.neutral;
        let (line, column) = self.editor.cursor(cx);
        let selected = self.editor.selected_lines(cx);
        let words = self.shell.upgrade().map(|shell| shell.read(cx).words());
        let button = selected.zip(words).map(|((first, last), words)| {
            let label = words.add_lines(
                usize::try_from(first).unwrap_or(usize::MAX),
                usize::try_from(last).unwrap_or(usize::MAX),
            );
            let shell = self.shell.clone();
            let button = vessel_app_kit::button("code-editor-add-lines", label, true, &tokens)
                .on_click(move |_event, window, cx| {
                    let _gone = shell.update(cx, |shell, cx| {
                        shell.add_editor_lines((first, last), window, cx);
                    });
                });
            // Comes in as a selection starts; its id does not name the
            // lines, so widening the selection does not play it again.
            appear("code-editor-add-lines-in", QUICK, button)
        });
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(spacing::S3))
            .child(
                div()
                    .type_(Type::Meta)
                    .text_color(neutral.foreground_extra_muted)
                    .child(format!("Ln {line}, Col {column}")),
            )
            .children(button)
    }
}
