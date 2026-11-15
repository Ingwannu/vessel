//! Adaptive kit dialogs and the existing canonical conversation rows.
use crate::shell::Shell;
use vessel_app_kit::theme::spacing::{S0, S2, S3};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, DialogContent, DialogSize, Tokens, Type, TypeExt as _,
    button_with_kind,
};
use vessel_app_view::subagents::TrackLoad;
use vessel_proto::subagents::ChildHistory;

#[derive(Clone, Copy)]
enum Action {
    Close,
    Refresh,
    HideFinished,
    Back,
    Older,
    CancelDetach,
    Detach,
}
pub(crate) fn with_header(
    shell: &Shell,
    body: AnyElement,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    div()
        .flex_1()
        .min_h(px(S0))
        .flex()
        .flex_col()
        .children(header(shell, tokens, cx))
        .child(body)
        .into_any_element()
}
fn header(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Option<AnyElement> {
    shell.areas.subagents.track.as_ref()?;
    let words = shell.words.subagents();
    let parent = shell
        .areas
        .subagents
        .info
        .as_ref()
        .and_then(|info| info.parentage().ok())
        .and_then(vessel_proto::subagents::Parentage::parent);
    Some(
        div()
            .flex()
            .items_center()
            .gap(px(S2))
            .px(px(S3))
            .child(
                button_with_kind(
                    "subagents-open",
                    shell.areas.subagents.pill.clone(),
                    ButtonKind::Ghost,
                    ButtonState::Enabled,
                    tokens,
                )
                .on_click(cx.listener(|shell, _, window, cx| shell.open_subagents(window, cx))),
            )
            .when(parent.is_some(), |row| {
                row.child(
                    button_with_kind(
                        "subagent-parent",
                        words.back(),
                        ButtonKind::Ghost,
                        ButtonState::Enabled,
                        tokens,
                    )
                    .on_click(
                        cx.listener(|shell, _, window, cx| shell.subagent_parent(window, cx)),
                    ),
                )
            })
            .into_any_element(),
    )
}
pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    if shell.areas.subagents.pending_open.is_some() {
        cx.defer_in(window, Shell::subagents_open_pending);
    }
    if !shell.areas.subagents.open {
        return None;
    }
    let words = shell.words.subagents();
    let content = DialogContent {
        title: words.title().into(),
        body: body(shell, tokens, window, cx).into_any_element(),
        footer: vec![
            button(Action::Refresh, shell, tokens, cx),
            button(Action::Close, shell, tokens, cx),
        ],
        size: DialogSize::Wide,
    };
    let dialog = vessel_app_kit::dialog(
        &shell.areas.subagents.dialog,
        content,
        tokens,
        window,
        crate::areas::draft::compact(window),
        Shell::close_subagents,
        cx,
    );
    Some(
        div()
            .child(dialog)
            .children(confirm(shell, tokens, window, cx))
            .into_any_element(),
    )
}
fn body(shell: &Shell, tokens: &Tokens, window: &mut Window, cx: &mut Context<'_, Shell>) -> Div {
    let area = &shell.areas.subagents;
    let words = shell.words.subagents();
    let mut body = div().flex().flex_col().min_w_0().gap(px(S3));
    if let Some(problem) = area.failure {
        body = body.child(note(problem.words(shell.words), tokens));
    }
    if let Some(track) = &area.track {
        if let TrackLoad::Failed(problem) = track.load() {
            body = body.child(note(problem.words(shell.words), tokens));
        }
        if matches!(track.load(), TrackLoad::Loading(_)) {
            body = body.child(note(words.loading(), tokens));
        }
        if area.partial || matches!(track.coverage(), ChildHistory::Partial { .. }) {
            body = body.child(note(words.partial(), tokens));
        }
    }
    if let Some(pane) = &area.pane {
        body = body
            .child(button(Action::Back, shell, tokens, cx))
            .child(super::breadcrumbs::render(shell, tokens, cx))
            .child(note(words.read_only(), tokens));
        if let Some(problem) = pane.failure() {
            body = body.child(note(problem.words(shell.words), tokens));
        }
        if pane.loading() {
            body = body.child(note(words.loading(), tokens));
        }
        if matches!(
            pane.history(),
            ChildHistory::Partial { .. } | ChildHistory::Missing | ChildHistory::Unsupported
        ) {
            body = body.child(note(words.partial(), tokens));
        }
        body = body
            .child(
                div()
                    .h(px(f32::from(window.viewport_size().height) * PANE_SHARE))
                    .min_w_0()
                    .child(crate::areas::conversation::timeline::rows(
                        shell,
                        super::CHILD_SEAT,
                        tokens,
                        window,
                        cx,
                    )),
            )
            .child(button(Action::Older, shell, tokens, cx));
        if !area.rows.is_empty() {
            body = body.child(super::rows::list(shell, window, cx));
        }
    } else if area.rows.is_empty() {
        body = body.child(note(words.empty(), tokens));
    } else {
        body = body
            .child(button(Action::HideFinished, shell, tokens, cx))
            .child(super::rows::list(shell, window, cx));
    }
    body
}
const PANE_SHARE: f32 = 0.3;
fn confirm(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    shell.areas.subagents.confirm.as_ref()?;
    let words = shell.words.subagents();
    let content = DialogContent {
        title: words.detach_title().into(),
        body: note(words.detach_body(), tokens).into_any_element(),
        footer: vec![
            button(Action::CancelDetach, shell, tokens, cx),
            button(Action::Detach, shell, tokens, cx),
        ],
        size: DialogSize::Normal,
    };
    Some(vessel_app_kit::dialog(
        &shell.areas.subagents.confirm_dialog,
        content,
        tokens,
        window,
        crate::areas::draft::compact(window),
        Shell::subagent_cancel_detach,
        cx,
    ))
}
fn button(
    action: Action,
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.words.subagents();
    let area = &shell.areas.subagents;
    let (id, label) = match action {
        Action::Close => ("subagents-close", words.close()),
        Action::Refresh => ("subagents-refresh", words.refresh()),
        Action::HideFinished => ("subagents-finished", words.archive_finished()),
        Action::Back => ("subagents-back", words.back()),
        Action::Older => ("subagents-older", words.older()),
        Action::CancelDetach => ("subagent-cancel", words.close()),
        Action::Detach => ("subagent-confirm", words.detach()),
    };
    let scope = DrawScope::of(area);
    let enabled = match action {
        Action::Older => area
            .pane
            .as_ref()
            .is_some_and(|pane| !pane.loading() && pane.before().is_some()),
        Action::Detach => area.detaching.is_none() && area.uncertain.is_none(),
        Action::HideFinished => area
            .track
            .as_ref()
            .is_some_and(|track| track.counts().finished > 0),
        Action::Close | Action::Refresh | Action::Back | Action::CancelDetach => true,
    };
    button_with_kind(
        id,
        label,
        ButtonKind::Secondary,
        if enabled {
            ButtonState::Enabled
        } else {
            ButtonState::Disabled
        },
        tokens,
    )
    .min_h(px(vessel_app_kit::breakpoint::TOUCH_TARGET))
    .on_click(cx.listener(move |shell, _, window, cx| {
        if !enabled || !scope.current(shell, action) {
            return;
        }
        match action {
            Action::Close => shell.close_subagents(window, cx),
            Action::Refresh => shell.subagents_refresh(),
            Action::HideFinished => shell.subagents_hide_finished(),
            Action::Back => shell.subagent_provider_back(),
            Action::Older => shell.subagent_older(),
            Action::CancelDetach => shell.subagent_cancel_detach(window, cx),
            Action::Detach => shell.subagent_detach(),
        }
        cx.notify();
    }))
    .into_any_element()
}
fn note(text: &str, tokens: &Tokens) -> Div {
    div()
        .type_(Type::Meta)
        .text_color(tokens.neutral.foreground_muted)
        .child(text.to_owned())
}
pub(super) fn row_number(index: usize) -> u64 {
    u64::try_from(index).unwrap_or(u64::MAX)
}

struct DrawScope {
    target: Option<vessel_app_view::subagents::Target>,
    pane: Option<vessel_proto::subagents::ProviderChildRef>,
    choice: Option<vessel_app_view::subagents::DetachChoice>,
    revision: Option<vessel_app_view::devices::Ticket>,
}
impl DrawScope {
    fn of(area: &super::SubagentArea) -> Self {
        Self {
            target: area.track.as_ref().map(|track| track.target().clone()),
            pane: area
                .pane
                .as_ref()
                .map(vessel_app_view::subagents::ChildPane::identity),
            choice: area.confirm.clone(),
            revision: area
                .pane
                .as_ref()
                .and_then(|pane| pane.projection_revision()),
        }
    }
    fn current(&self, shell: &Shell, action: Action) -> bool {
        let area = &shell.areas.subagents;
        area.track.as_ref().map(|track| track.target()) == self.target.as_ref()
            && (!matches!(action, Action::Older | Action::Back)
                || area
                    .pane
                    .as_ref()
                    .map(vessel_app_view::subagents::ChildPane::identity)
                    == self.pane
                    && area
                        .pane
                        .as_ref()
                        .and_then(|pane| pane.projection_revision())
                        == self.revision)
            && (!matches!(action, Action::Detach) || area.confirm == self.choice)
    }
}
