//! The commit form's scope: what the node stages before committing
//! (staged only, every tracked change, or everything with new files),
//! as the kit's select (Paseo's `SettingsSelect`) and its list.

use vessel_app_kit::picker::ComboRow;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, combobox, select};
use vessel_app_view::workspace::git::{commit_files, commit_scopes};
use vessel_proto::workspace::CommitStage;

use crate::shell::Shell;

impl Shell {
    /// A scope was chosen from the select's list.
    fn git_scope_chosen(&mut self, stage: CommitStage, cx: &mut Context<'_, Self>) {
        let git = &mut self.areas.git;
        git.scope_list.close();
        let enabled = commit_scopes(&git.status, &git.writing)
            .iter()
            .any(|scope| scope.stage == stage && scope.enabled);
        if enabled {
            git.stage = stage;
            if let Some(status) = git.status.shown() {
                git.files.reset(commit_files(status, stage).count());
            }
        }
        cx.notify();
    }
}

/// The node's scope as a select: what goes into the commit.
pub(super) fn scope(
    shell: &Shell,
    tokens: &Tokens,
    narrow: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let git = &shell.areas.git;
    let lang = shell.t.lang;
    let trigger = select::trigger(
        "git-commit-scope",
        lang.git_commit_scope(git.stage),
        &git.scope_anchor,
        git.scope_list.is_open(),
        !git.writing.busy(),
        narrow,
        tokens,
    )
    .on_click(cx.listener(|shell, _event, window, cx| {
        let git = &mut shell.areas.git;
        if git.scope_list.is_open() {
            git.scope_list.close();
        } else {
            let bounds = git.scope_anchor.get();
            git.scope_list.set_anchor(bounds);
            git.scope_list.open(None, window, cx);
        }
        cx.notify();
    }));
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .justify_between()
        .gap(px(spacing::S2))
        .child(
            div()
                .type_(Type::Body)
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(tokens.neutral.foreground)
                .child(lang.git_labels().commit_scope),
        )
        .child(trigger)
        .into_any_element()
}

/// The scope's list, when open (over the form).
pub(super) fn scope_list(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let git = &shell.areas.git;
    if !git.scope_list.is_open() {
        return None;
    }
    let lang = shell.t.lang;
    let scopes = commit_scopes(&git.status, &git.writing);
    let rows: Vec<ComboRow> = scopes
        .iter()
        .map(|scope| ComboRow {
            checked: scope.stage == git.stage,
            disabled: !scope.enabled,
            ..ComboRow::new(lang.git_commit_scope(scope.stage))
        })
        .collect();
    let handlers = vessel_app_kit::picker::ComboHandlers {
        state: |shell: &mut Shell| &mut shell.areas.git.scope_list,
        chose: std::rc::Rc::new(move |shell: &mut Shell, index, _window, cx| {
            if let Some(scope) = scopes.get(index) {
                shell.git_scope_chosen(scope.stage, cx);
            }
        }),
        back: None,
    };
    combobox(
        "git-commit-scope-list",
        &git.scope_list,
        &select::header(),
        &rows,
        &handlers,
        select::WIDTH,
        tokens,
        window,
        cx,
    )
}
