//! A file's find/replace UI observes only that editor, so a caret blink
//! does not repaint the shell. Replacement explicitly reaches the guarded
//! save model; the engine's replacement method emits no change event.

#[cfg(test)]
#[path = "editor_find_tests.rs"]
mod tests;

use gpui::AppContext as _;
use vessel_app_kit::code_editor::{
    CodeEditor, FindCase, FindDirection, FindRevision, FindState, ReplaceScope,
};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    Context, Div, Entity, InteractiveElement as _, IntoElement, KeyDownEvent, ParentElement as _,
    Render, StatefulInteractiveElement as _, Styled as _, Subscription, WeakEntity, Window, div,
    px,
};
use vessel_app_kit::{TextField, Type, TypeExt as _, button};

use super::buffers::FileKey;
use crate::shell::Shell;

/// Paseo's match counter display cap; the engine still navigates all matches.
const MATCH_STATUS_LIMIT: usize = 10_000;

pub(super) struct FindPanel {
    editor: CodeEditor,
    shell: WeakEntity<Shell>,
    key: FileKey,
    query: TextField,
    replacement: TextField,
    case: FindCase,
    activation: Option<FindRevision>,
    _follow: Subscription,
}

impl FindPanel {
    pub(super) fn new(
        editor: CodeEditor,
        key: FileKey,
        labels: &'static vessel_app_view::workspace::words::Labels,
        window: &mut Window,
        cx: &mut Context<'_, Shell>,
    ) -> Entity<Self> {
        let shell = cx.entity().downgrade();
        cx.new(|cx| {
            let query = TextField::with_change(
                labels.file_find,
                |this: &mut Self, _, _, cx| this.move_match(FindDirection::Next, cx),
                |this: &mut Self, text, cx| this.editor.find_query(&text, this.case, cx),
                window,
                cx,
            );
            let replacement = TextField::new(
                labels.file_replace,
                |this: &mut Self, _, window, cx| this.replace(ReplaceScope::Current, window, cx),
                window,
                cx,
            );
            let follow = editor.observe(cx, |_, cx| cx.notify());
            Self {
                editor,
                shell,
                key,
                query,
                replacement,
                case: FindCase::default(),
                activation: None,
                _follow: follow,
            }
        })
    }

    pub(super) fn has_focus(&self, window: &Window, cx: &gpui::App) -> bool {
        self.query.focus_handle(cx).is_focused(window)
            || self.replacement.focus_handle(cx).is_focused(window)
    }

    fn move_match(&mut self, direction: FindDirection, cx: &mut Context<'_, Self>) {
        self.editor.find_move(direction, cx);
        cx.notify();
    }

    fn replace(&mut self, scope: ReplaceScope, window: &mut Window, cx: &mut Context<'_, Self>) {
        let text = self.replacement.text(cx);
        if self.editor.find_replace(&text, scope, window, cx).0 != 0 {
            let _gone = self.shell.update(cx, |shell, cx| {
                shell.editor_changed_for(&self.key, window, cx);
            });
        }
        cx.notify();
    }
}

impl Render for FindPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let FindState::Open {
            revision,
            query,
            current,
            total,
        } = self.editor.find_state(cx)
        else {
            return div();
        };
        let Some(shell) = self.shell.upgrade() else {
            return div();
        };
        let labels = shell.read(cx).words().labels();
        if self.activation != Some(revision) {
            self.activation = Some(revision);
            self.query.set_text(&query, window, cx);
            self.query.focus(window, cx);
        }
        let tokens = vessel_app_kit::tokens(cx);
        let replacement = (self.editor.editability(cx) == vessel_app_kit::Editability::Editable)
            .then(|| self.replacement_row(labels, &tokens, cx));
        let count = if total.0 == 0 {
            labels.file_find_no_matches.to_owned()
        } else {
            format!(
                "{} / {}{}",
                current.map_or(0, std::num::NonZeroUsize::get),
                total.0.min(MATCH_STATUS_LIMIT),
                if total.0 > MATCH_STATUS_LIMIT {
                    "+"
                } else {
                    ""
                }
            )
        };
        div()
            .flex()
            .flex_col()
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    this.editor.close_find(window, cx);
                    cx.stop_propagation();
                }
            }))
            .gap(px(spacing::S1))
            .p(px(spacing::S2))
            .type_(Type::Meta)
            .text_color(tokens.neutral.foreground_muted)
            .child(self.search_row(labels, &tokens, count, cx))
            .children(replacement)
    }
}

impl FindPanel {
    fn search_row(
        &self,
        labels: &vessel_app_view::workspace::words::Labels,
        tokens: &vessel_app_kit::Tokens,
        count: String,
        cx: &mut Context<'_, Self>,
    ) -> Div {
        div()
            .flex()
            .flex_row()
            .flex_wrap()
            .gap(px(spacing::S1))
            .child(div().flex_1().min_w_0().child(self.query.input(tokens, cx)))
            .child(count)
            .child(
                button(
                    "source-find-previous",
                    labels.file_find_previous,
                    false,
                    tokens,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.move_match(FindDirection::Previous, cx);
                })),
            )
            .child(
                button("source-find-next", labels.file_find_next, false, tokens).on_click(
                    cx.listener(|this, _, _, cx| {
                        this.move_match(FindDirection::Next, cx);
                    }),
                ),
            )
            .child(
                button(
                    "source-find-case",
                    labels.file_find_case,
                    self.case == FindCase::Sensitive,
                    tokens,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.case = match this.case {
                        FindCase::Sensitive => FindCase::Insensitive,
                        FindCase::Insensitive => FindCase::Sensitive,
                    };
                    this.editor.find_query(&this.query.text(cx), this.case, cx);
                })),
            )
            .child(
                button("source-find-close", labels.dismiss, false, tokens).on_click(cx.listener(
                    |this, _, window, cx| {
                        this.editor.close_find(window, cx);
                    },
                )),
            )
    }

    fn replacement_row(
        &self,
        labels: &vessel_app_view::workspace::words::Labels,
        tokens: &vessel_app_kit::Tokens,
        cx: &mut Context<'_, Self>,
    ) -> Div {
        div()
            .flex()
            .flex_row()
            .flex_wrap()
            .gap(px(spacing::S1))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(self.replacement.input(tokens, cx)),
            )
            .child(
                button("source-find-replace", labels.file_replace, false, tokens).on_click(
                    cx.listener(|this, _, window, cx| {
                        this.replace(ReplaceScope::Current, window, cx);
                    }),
                ),
            )
            .child(
                button(
                    "source-find-replace-all",
                    labels.file_replace_all,
                    false,
                    tokens,
                )
                .on_click(cx.listener(|this, _, window, cx| {
                    this.replace(ReplaceScope::All, window, cx);
                })),
            )
    }
}

impl Shell {
    /// The existing global Find dispatcher yields only to this actual focused
    /// source owner. Displayed path or selected-session state alone cannot win.
    pub(crate) fn file_find_at_focus(
        &mut self,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        if self.route != crate::routes::Route::Session {
            return false;
        }
        let panel = &self.areas.workspace.code;
        let active = panel.file_key();
        let mut keys = active
            .clone()
            .into_iter()
            .chain(panel.files.retained.keys().cloned());
        let editor = keys.find_map(|key| {
            let viewer = panel.viewer_at(&key)?;
            let legacy = panel.open
                && panel.tab == super::Tab::Files
                && active.as_ref() == Some(&key)
                && !panel.file_tabs.contains(&key)
                && panel.scope().as_ref() == Some(&viewer.scope);
            if !legacy && !self.file_visible_in_layout(&key, window) {
                return None;
            }
            let editing = viewer.edit.as_ref()?;
            let editor = editing.editor.as_ref()?;
            let focused = editor.has_focus(window, cx)
                || editing.find.as_ref().is_some_and(|find| {
                    let find = find.read(cx);
                    find.key == key && find.has_focus(window, cx)
                });
            focused.then(|| editor.clone())
        });
        let Some(editor) = editor else {
            return false;
        };
        editor.open_find(cx);
        true
    }
}
