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
use vessel_app_view::workspace::git::panel::{
    Body, ComparisonMode, Demand, EmptyAction, PanelAddress, PanelBinding, PanelTarget, ReadIssue,
};

pub(crate) fn render_panel(
    shell: &Shell,
    panel: &PanelContext<'_>,
    target: PanelTarget,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    if let Some(notice) = owner_notice(shell, tokens, window, cx) {
        return notice;
    }
    let address = PanelAddress {
        workspace: panel.workspace.clone(),
        tab: panel.tab,
    };
    let entry = shell.areas.git.panels.entries.get(&address);
    if entry.is_none_or(|entry| entry.target != target) {
        cx.defer_in(window, move |shell, window, cx| {
            shell.git_panel_open(address, target, window, cx);
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
        shell.git_panel_refresh(retry_address.clone(), retry_target.clone(), window, cx);
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

/// The legacy header shares the pane's startup refusal, before scheduling any
/// repository follow/read; this adds no second notice renderer.
pub(in crate::areas::git) fn owner_notice(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let why = shell.areas.git.panels.owner_problem()?;
    Some(failure(shell, &why, tokens, window, cx))
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
    match reads.body(Demand::Active) {
        Body::Inactive => div().into_any_element(),
        Body::Connecting => message(shell.words.git_panel().problem_connection(), tokens),
        Body::CheckingRepository => loading(Some(shell.words.git_panel().checking()), tokens),
        Body::RepositoryFailed(why) | Body::DiffFailed(why) => {
            failure(shell, why, tokens, window, cx)
        }
        Body::NotRepository => message(shell.words.git_panel().not_repository(), tokens),
        Body::ReadingDiff => loading(None, tokens),
        Body::DiffTooLarge => div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(spacing::S2))
            .pt(px(spacing::S16))
            .type_(Type::Body)
            .text_color(tokens.neutral.foreground_muted)
            .child(shell.words.git_panel().too_large_title().to_owned())
            .child(shell.words.git_panel().too_large_description().to_owned())
            .into_any_element(),
        Body::Empty(action) => empty(shell, address, entry, action, tokens, cx),
        Body::Files(data) => {
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
fn message(text: &str, tokens: &Tokens) -> AnyElement {
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .pt(px(spacing::S16))
        .gap(px(spacing::S2))
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground_muted)
        .child(text.to_owned())
        .into_any_element()
}
fn empty(
    shell: &Shell,
    address: &PanelAddress,
    entry: &super::Entry,
    action: Option<EmptyAction>,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let captured = entry
        .context
        .as_ref()
        .zip(entry.reads.as_ref())
        .zip(entry.instance)
        .and_then(|((context, reads), instance)| {
            Some(super::comparison::ComparisonChange {
                address: address.clone(),
                target: entry.target.clone(),
                instance,
                context: context.clone(),
                repository: reads.repository(),
                comparison: reads.comparison().clone(),
                summary: reads.summary_ticket()?.clone(),
            })
        });
    div()
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
        .children(action.zip(captured).map(|(action, captured)| {
            let mode = match action {
                EmptyAction::Uncommitted => ComparisonMode::Uncommitted,
                EmptyAction::Committed => ComparisonMode::Committed,
            };
            vessel_app_kit::button_with_kind(
                "git-panel-empty-mode",
                shell.words.git_panel().empty_action(action),
                vessel_app_kit::ButtonKind::Ghost,
                vessel_app_kit::ButtonState::Enabled,
                tokens,
            )
            .on_click(cx.listener(move |shell, _event, window, cx| {
                shell.git_panel_compare(&captured, mode, window, cx);
            }))
        }))
        .into_any_element()
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
    // List holds its viewport RefCell while asking for a row. Let the
    // existing text layout truncate against this row's real width instead of
    // querying entry.files.width() recursively from inside that callback.
    let place = entry.path_places.entry(file.path.clone()).or_default();
    let row = div()
        .id(("git-panel-file", index))
        .w_full()
        .min_w_0()
        .px(px(spacing::S2))
        .py(px(spacing::S1))
        .type_(Type::Mono)
        .text_color(tokens.neutral.foreground)
        .whitespace_nowrap()
        .overflow_hidden()
        .text_ellipsis_middle()
        .child(file.path.as_str().to_owned());
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
    let detail = why.detail(shell.words);
    div()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .p(px(spacing::S4))
        .child(notice)
        .children(detail.map(|text| {
            vessel_app_kit::output_box(
                "git-panel-detail",
                &text,
                vessel_app_kit::output::Start::Top,
                tokens,
                window,
                cx,
            )
        }))
        .into_any_element()
}

#[cfg(test)]
#[path = "draw_tests.rs"]
mod tests;
