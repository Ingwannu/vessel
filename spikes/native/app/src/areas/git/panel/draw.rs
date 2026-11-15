//! The C3 registry skeleton: exact-scope repository facts and file summary.
//! Body editors are mounted here once the mapped document control is checked.
use crate::areas::layout::registry::PanelContext;
use crate::shell::Shell;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _};
use vessel_app_view::workspace::git::ReadState;
use vessel_app_view::workspace::git::panel::{PanelAddress, PanelBinding, PanelTarget, ReadIssue};

pub(crate) fn render_panel(
    shell: &Shell,
    panel: &PanelContext<'_>,
    target: PanelTarget,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let address = PanelAddress {
        workspace: panel.workspace.clone(),
        tab: panel.tab,
    };
    let entry = shell.areas.git.panels.entries.get(&address);
    if entry.is_none_or(|entry| entry.target != target) {
        cx.defer_in(window, move |shell, window, cx| {
            shell.git_panel_open(address, target, window, cx)
        });
        return loading(Some(shell.words.git_panel().checking()), tokens);
    }
    let retry_address = address.clone();
    let retry_target = target;
    let retry = vessel_app_kit::button(
        "git-panel-refresh",
        shell.words.git_panel().refresh(),
        false,
        tokens,
    )
    .on_click(cx.listener(move |shell, _event, window, cx| {
        shell.git_panel_open(retry_address.clone(), retry_target.clone(), window, cx);
    }));
    let body = entry.map_or_else(
        || loading(Some(shell.words.git_panel().checking()), tokens),
        |entry| {
            if let Some(why) = &entry.problem {
                return failure(shell, why, tokens, window, cx);
            }
            match &entry.binding {
                PanelBinding::Failed(why) => failure(shell, why, tokens, window, cx),
                PanelBinding::Idle | PanelBinding::Locating(_) => {
                    loading(Some(shell.words.git_panel().checking()), tokens)
                }
                PanelBinding::Ready { .. } => summary(shell, &address, tokens, window, cx),
            }
        },
    );
    div()
        .size_full()
        .min_h_0()
        .flex()
        .flex_col()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .p(px(spacing::S2))
                .child(shell.words.git_panel().title().to_owned())
                .child(retry),
        )
        .child(div().flex_1().min_h_0().child(body))
        .children(entry.and_then(|entry| entry.tips.render(tokens)))
        .into_any_element()
}

fn summary(
    shell: &Shell,
    address: &PanelAddress,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let Some(entry) = shell.areas.git.panels.entries.get(address) else {
        return loading(Some(shell.words.git_panel().checking()), tokens);
    };
    let Some(reads) = &entry.reads else {
        return loading(Some(shell.words.git_panel().checking()), tokens);
    };
    if let ReadState::Failed(why) = reads.status() {
        return failure(shell, why, tokens, window, cx);
    }
    if reads.status().shown().is_none() {
        return loading(Some(shell.words.git_panel().checking()), tokens);
    }
    match reads.summary() {
        ReadState::Failed(why) => failure(shell, why, tokens, window, cx),
        ReadState::Empty | ReadState::Loading { previous: None, .. } => loading(None, tokens),
        ReadState::Ready(data)
        | ReadState::Loading {
            previous: Some(data),
            ..
        } => {
            if data.files.is_empty() && !data.more {
                return div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .pt(px(spacing::S16))
                    .gap(px(spacing::S2))
                    .type_(Type::Body)
                    .text_color(tokens.neutral.foreground_muted)
                    .child(shell.words.git_panel().empty().to_owned())
                    .into_any_element();
            }
            let address = address.clone();
            let files = entry.files.element(cx.processor(
                move |shell: &mut Shell, index: usize, window, cx| {
                    let tokens = vessel_app_kit::tokens(cx);
                    file_row(shell, &address, index, &tokens, window)
                },
            ));
            div()
                .size_full()
                .min_h_0()
                .flex()
                .flex_col()
                .children(data.more.then(|| {
                    div()
                        .p(px(spacing::S2))
                        .child(shell.words.git_panel().partial().to_owned())
                }))
                .child(div().flex_1().min_h_0().child(files))
                .into_any_element()
        }
    }
}
fn file_row(
    shell: &mut Shell,
    address: &PanelAddress,
    index: usize,
    tokens: &Tokens,
    window: &mut Window,
) -> AnyElement {
    let Some(entry) = shell.areas.git.panels.entries.get_mut(address) else {
        return div().into_any_element();
    };
    let Some(file) = entry
        .reads
        .as_ref()
        .and_then(|reads| reads.summary().shown())
        .and_then(|summary| summary.files.get(index))
    else {
        return div().into_any_element();
    };
    let label = super::super::name::one_line(
        window,
        file.path.as_str(),
        Type::Mono,
        f32::from(entry.files.width()) - spacing::S4,
    );
    let place = entry.path_places.entry(file.path.clone()).or_default();
    let row = div()
        .id(("git-panel-file", index))
        .w_full()
        .min_w_0()
        .px(px(spacing::S2))
        .py(px(spacing::S1))
        .type_(Type::Mono)
        .text_color(tokens.neutral.foreground)
        .child(label.text);
    entry
        .tips
        .on_code(
            row,
            place,
            file.path.as_str().to_owned(),
            vessel_app_kit::breakpoint::phone(window),
        )
        .into_any_element()
}

fn loading(label: Option<&str>, tokens: &Tokens) -> AnyElement {
    // DiffBodyContent's repository phase includes its label; the subsequent
    // diff phase has only the same large ActivityIndicator. Neither is an alert.
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .pt(px(spacing::S16))
        .gap(px(spacing::S4))
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground_muted)
        .child(vessel_app_kit::loading::activity_indicator(
            "git-panel-reading",
            vessel_app_kit::loading::Size::Large,
            tokens.neutral.foreground_muted,
        ))
        .children(label.map(|text| div().child(text.to_owned())))
        .into_any_element()
}

fn failure(
    shell: &Shell,
    why: &vessel_app_view::workspace::git::panel::ReadFailure,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let issue = ReadIssue::for_failure(why);
    let notice = vessel_app_kit::alert::alert(
        vessel_app_kit::alert::Content {
            title: Some(shell.words.git_panel().failed_title().into()),
            description: Some(
                div()
                    .child(issue.words(shell.words).to_owned())
                    .into_any_element(),
            ),
            action: None,
        },
        crate::badge::alert_variant(issue.variant()),
        vessel_app_kit::alert::Size::Small,
        tokens,
    );
    let detail = match why {
        vessel_app_view::workspace::git::panel::ReadFailure::Call(
            vessel_app_view::workspace::problem::Problem::Node { message, .. }
            | vessel_app_view::workspace::problem::Problem::Gone(message),
        ) => Some(message.as_str()),
        vessel_app_view::workspace::git::panel::ReadFailure::Call(
            vessel_app_view::workspace::problem::Problem::Unsent
            | vessel_app_view::workspace::problem::Problem::Older,
        )
        | vessel_app_view::workspace::git::panel::ReadFailure::WorkspaceUnavailable
        | vessel_app_view::workspace::git::panel::ReadFailure::KeysExhausted
        | vessel_app_view::workspace::git::panel::ReadFailure::DestinationClosed
        | vessel_app_view::workspace::git::panel::ReadFailure::Capacity
        | vessel_app_view::workspace::git::panel::ReadFailure::TimedOut
        | vessel_app_view::workspace::git::panel::ReadFailure::DestinationChanged
        | vessel_app_view::workspace::git::panel::ReadFailure::UnexpectedNode { .. }
        | vessel_app_view::workspace::git::panel::ReadFailure::UnexpectedSession { .. }
        | vessel_app_view::workspace::git::panel::ReadFailure::UnexpectedFile { .. }
        | vessel_app_view::workspace::git::panel::ReadFailure::BaseMismatch { .. }
        | vessel_app_view::workspace::git::panel::ReadFailure::ComparisonUnavailable => None,
    };
    div()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .p(px(spacing::S4))
        .child(notice)
        .children(detail.map(|text| {
            vessel_app_kit::output_box(
                "git-panel-detail",
                text,
                vessel_app_kit::output::Start::Top,
                tokens,
                window,
                cx,
            )
        }))
        .into_any_element()
}
