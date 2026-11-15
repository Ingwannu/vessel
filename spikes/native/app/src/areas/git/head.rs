//! The repository row at the head of the Changes tab: Paseo's
//! `ChangesRepositoryToolbar` (`git/diff-pane.tsx`), the branch switcher
//! leading and the git actions trailing; under it, what went wrong last.

use std::rc::Rc;

use gpui::AnimationExt as _;
use vessel_app_kit::split_button::{
    MenuPhase, PrimaryState, SplitContent, SplitHandlers, split_button,
};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::toolbar_label::{LabelLayout, State, trigger};
use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, breakpoint};
use vessel_app_view::workspace::git::{
    PanelWidth, PrimaryControl, Reading, head_activity, primary_control,
};
use vessel_proto::workspace::WorkspaceRoot;

use super::{Target, menu, name};

/// What the toolbar label trigger takes besides its words: its insets
/// (`spacing[1]` each side), the gap and the 12-point caret, plus a point
/// so a measured width never rounds the cut text past its box.
const TRIGGER_CHROME: f32 = 25.0;

/// Before the first measure: the toolbar's insets, the gap and an
/// icon-only split button.
const HEAD_ESTIMATE: f32 = 110.0;

/// The head row's least height: the toolbar trigger's, so the row keeps its
/// height when the trigger gives way to the spinner.
const HEAD_ROW: f32 = 26.0;
use crate::shell::Shell;

/// Track F's slot at the head of its Changes tab. The first frame on a
/// new repository only asks to follow it (a frame does not change state).
pub(crate) fn slot(
    shell: &Shell,
    machine: &str,
    root: &WorkspaceRoot,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let target = Target {
        machine: machine.to_owned(),
        root: *root,
    };
    let git = &shell.areas.git;
    if git.target.as_ref() != Some(&target) {
        cx.defer_in(window, move |shell, _window, cx| {
            shell.git_follow(target, cx);
            cx.notify();
        });
        return None;
    }
    let touch = breakpoint::phone(window);
    let mut head = div()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .w_full()
        .min_w_0();
    head = match (&git.status, git.status.shown()) {
        (_, Some(status)) => head.child(row(shell, status, tokens, touch, window, cx)),
        (Reading::Failed(problem), None) => {
            head.child(menu::read_failed(shell, problem, tokens, cx))
        }
        (Reading::Empty | Reading::Loading { .. } | Reading::Ready(_), None) => head.child(
            div()
                .flex()
                .items_center()
                .gap(px(spacing::S2))
                .min_h(px(HEAD_ROW))
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(vessel_app_kit::slider::spinner(
                    "git-reading",
                    14.0,
                    tokens.neutral.foreground_muted,
                ))
                .child(shell.t.lang.git_labels().reading),
        ),
    };
    let pulls_open = shell.areas.pulls.form.is_some() || shell.areas.pulls.facts.is_some();
    if git.open.is_none() && git.dropping.is_none() && git.confirm_switch.is_none() && !pulls_open {
        // The owner's word 39: a row that opens arrives in the surface
        // transition (opacity, 300 ms on the kit's curve; at once while
        // motion is held). Its element state lives while it is shown, so
        // each new appearance after none runs it again.
        head = head.children(menu::issue(shell, tokens, window, cx).map(|issue| {
            issue.with_animation(
                "git-issue-arrives",
                vessel_app_kit::motion::transition(vessel_app_kit::motion::STANDARD),
                Styled::opacity,
            )
        }));
    }
    Some(
        head.children(menu::overlay(shell, tokens, touch, cx))
            .children(super::branch::switcher(shell, tokens, window, cx))
            .children(super::commit::dialog(shell, tokens, window, cx))
            .children(super::commit_scope::scope_list(shell, tokens, window, cx))
            .children(super::branch::dialog(shell, tokens, window, cx))
            .children(super::branch::switch_confirm(shell, tokens, window, cx))
            .children(super::stashes::dialog(shell, tokens, window, cx))
            .children(crate::areas::pulls::form_dialog(shell, tokens, window, cx))
            .children(crate::areas::pulls::base_list(shell, tokens, window, cx))
            .children(crate::areas::pulls::facts_dialog(shell, tokens, window, cx))
            .children(shell.areas.pulls.tips.render(tokens))
            // A tooltip does not stay over a menu or a list it was the
            // trigger of: the caret stops the press before the tooltip
            // hears it.
            .children(
                (!git.menu.is_open() && !git.switcher.is_open())
                    .then(|| git.tips.render(tokens))
                    .flatten(),
            )
            .into_any_element(),
    )
}

/// The branch switcher and the split button on one line.
fn row(
    shell: &Shell,
    status: &vessel_proto::workspace::GitStatus,
    tokens: &Tokens,
    touch: bool,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let git = &shell.areas.git;
    let lang = shell.t.lang;
    let header = lang.git_header(status, head_activity(&git.status, &git.writing));
    let switcher_state = if git.switcher.is_open() {
        State::Open
    } else {
        State::Ready
    };
    let width = shell
        .code_git_width()
        .map_or(window.viewport_size().width, |width| {
            if touch {
                window.viewport_size().width
            } else {
                width
            }
        });
    // Paseo's `BranchSwitcher`: the name opens the list of branches; its
    // tooltip says so. A detached head shows where it is and still opens it.
    // The name is one line cut in its middle (`name.rs`); when cut, the
    // tooltip carries the whole name instead.
    let estimate = HEAD_ESTIMATE.mul_add(-1.0, f32::from(width));
    let room = name::room(&git.branch_room, TRIGGER_CHROME, estimate, window);
    let shown = name::one_line(window, &header.name, Type::Meta, room);
    let trigger = trigger(
        "git-branch",
        shown.text,
        switcher_state,
        LabelLayout::SingleLine,
        touch,
        tokens,
    );
    let trigger = if shown.cut {
        git.tips
            .on_code(trigger, &git.branch_tip, header.name.clone(), touch)
    } else {
        git.tips.on(
            trigger,
            &git.branch_tip,
            lang.git_labels().switch_title,
            touch,
        )
    };
    let branch = div()
        .flex()
        .flex_1()
        .min_w_0()
        .child(vessel_app_kit::picker::anchor_probe(&git.branch_room))
        .child(trigger.child(git.switcher.anchor()).on_click(cx.listener(
            |shell, _event, window, cx| {
                shell.git_toggle_switcher(window, cx);
            },
        )));
    let control = primary_control(&git.status, &git.writing, PanelWidth(f32::from(width)));
    div()
        .id("git-row")
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::S2))
        .w_full()
        .min_w_0()
        .child(branch)
        .children(crate::areas::pulls::link(shell, status, tokens, window, cx))
        .child(actions(shell, status, control, tokens, touch, cx))
        .into_any_element()
}

/// Paseo's split button: the action that makes sense now, then the menu.
/// With none (clean and in step), the primary half is the git glyph and
/// opens the same menu, as Paseo's menu-only trigger does.
fn actions(
    shell: &Shell,
    status: &vessel_proto::workspace::GitStatus,
    control: PrimaryControl,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let git = &shell.areas.git;
    let lang = shell.t.lang;
    let chosen = control.action;
    let state = if control.running {
        PrimaryState::Busy
    } else {
        PrimaryState::Ready
    };
    let label = match chosen {
        Some(action) if control.labelled => lang.git_primary_label(action, status),
        Some(_) | None => "",
    };
    let content = SplitContent {
        label: label.into(),
        icon: Some(chosen.map_or(vessel_app_kit::assets::Glyph::Branch, menu::glyph)),
        state,
        menu: if git.menu.is_open() {
            MenuPhase::Open
        } else {
            MenuPhase::Closed
        },
    };
    let handlers = SplitHandlers {
        primary: Rc::new(move |shell: &mut Shell, window, cx| match chosen {
            Some(action) => shell.git_choose(action, window, cx),
            None => shell.git_open_menu_at_trigger(window, cx),
        }),
        menu: Rc::new(|shell: &mut Shell, at, window, cx| {
            shell.git_open_menu(at, window, cx);
        }),
    };
    // Without room for its words, the primary half's action is named by
    // its tooltip (Paseo's `hideLabels` keeps it in the accessibility label).
    let tip = match chosen {
        Some(action) if !control.labelled => lang.git_primary_label(action, status),
        Some(_) | None => lang.git_labels().options_tip,
    };
    git.tips
        .on(
            split_button("git-actions", content, &handlers, tokens, touch, cx),
            &git.actions_tip,
            tip,
            touch,
        )
        .child(vessel_app_kit::picker::anchor_probe(&git.actions_anchor))
        .into_any_element()
}
