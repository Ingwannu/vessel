//! The existing settings renderer owns every card, switch and button geometry.
use super::{Intent, on as intent_handler};
use crate::shell::Shell;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{
    ButtonContent, ButtonIcon, ButtonKind, ButtonState, ControlSizing, Tokens, Type, TypeExt as _,
    button_with_content, button_with_kind_at, settings,
};
use vessel_app_view::orchestration::{Page, Problem, has_selection};
use vessel_proto::mcp::Injection;
use vessel_proto::skills::{SkillOperation, SkillsState};

const DOCS: &str = "https://github.com/Ingwannu/vessel#agent-tools";

pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.words.orchestration();
    let machine = super::super::choices::chosen_machine(shell);
    let online = machine.is_some_and(|machine| machine.connection_context().is_some());
    let supported = machine.is_some_and(|machine| {
        vessel_proto::mcp::MCP_PARITY
            .is_some_and(|required| machine.protocol.is_some_and(|peer| peer >= required))
    });
    let state = &shell.areas.settings.orchestration;
    let mut body = div().flex().flex_col();
    if online {
        let card = if state.exhausted {
            message(words.exhausted(), tokens)
        } else if !supported {
            message(words.unsupported(), tokens)
        } else if let Some(page) = &state.page {
            tools(shell, page, tokens, compact, cx)
        } else {
            message(words.loading(), tokens)
        };
        body = body.child(settings::section(words.section(), card, tokens));
    }
    let skills = if !online {
        message(words.unavailable(), tokens)
    } else if state.exhausted {
        message(words.exhausted(), tokens)
    } else if !supported {
        message(words.unsupported(), tokens)
    } else if let Some(page) = &state.page {
        skills(shell, page, tokens, compact, cx)
    } else {
        message(words.loading(), tokens)
    };
    let docs = button_with_kind_at(
        "orchestration-docs",
        words.docs(),
        ButtonKind::Ghost,
        ButtonState::Enabled,
        ControlSizing::from_compact(compact),
        tokens,
    )
    .on_click(cx.listener(|_shell, _event, _window, cx| cx.open_url(DOCS)))
    .into_any_element();
    let mut actions = div().flex().flex_wrap().items_center().gap(px(spacing::S2));
    if let Some(page) = &state.page
        && (page.tools.problem().is_some() || page.skills.problem().is_some())
        && !state.exhausted
    {
        actions = actions.child(
            button_with_kind_at(
                "orchestration-status-refresh",
                words.refresh(),
                ButtonKind::Ghost,
                enabled(!page.tools.pending() && !page.skills.pending()),
                ControlSizing::from_compact(compact),
                tokens,
            )
            .when(!page.tools.pending() && !page.skills.pending(), |button| {
                button.on_click(intent_handler(page.scope(), Intent::Refresh, cx))
            }),
        );
    }
    let actions = actions.child(docs).into_any_element();
    body.child(settings::section_with_trailing(
        words.skills_section(),
        skills,
        Some(actions),
        tokens,
    ))
    .into_any_element()
}
fn tools(
    shell: &Shell,
    page: &Page,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words.orchestration();
    let Some(value) = page.tools.value() else {
        return waiting(shell, page, page.tools.problem(), tokens, compact, cx);
    };
    let enabled = !page.tools.writes_blocked();
    let switch = settings::switch(
        "orchestration-tools",
        value.inject == Injection::On,
        enabled,
        compact,
        shell.areas.settings.orchestration.turns,
        tokens,
    )
    .when(enabled, |switch| {
        switch.on_click(intent_handler(page.scope(), Intent::Tools, cx))
    });
    settings::card(tokens).child(settings::row_at(
        words.tools_title(),
        Some(words.tools_hint().into()),
        page.tools
            .problem()
            .map(|cause| problem(shell, cause).to_owned().into()),
        Some(switch.into_any_element()),
        true,
        compact,
        tokens,
    ))
}
fn skills(
    shell: &Shell,
    page: &Page,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words.orchestration();
    let Some(status) = page.skills.value() else {
        return waiting(shell, page, page.skills.problem(), tokens, compact, cx);
    };
    let working = page.skills.pending();
    let writable = !page.skills.writes_blocked();
    let installed = status.state != SkillsState::NotInstalled;
    let update = status.state == SkillsState::Drift
        && status.ops.iter().any(|op| {
            matches!(
                op,
                SkillOperation::Add { .. } | SkillOperation::Update { .. }
            )
        });
    let can_toggle = writable && (installed || has_selection(status));
    let mut controls = div().flex().items_center().flex_wrap().gap(px(spacing::S2));
    if update {
        controls = controls.child(
            button_with_kind_at(
                "orchestration-update",
                if working {
                    words.working()
                } else {
                    words.update()
                },
                ButtonKind::Outline,
                enabled(writable),
                ControlSizing::from_compact(compact),
                tokens,
            )
            .when(writable, |button| {
                button.on_click(intent_handler(page.scope(), Intent::Update, cx))
            }),
        );
    }
    controls = controls
        .child(
            button_with_content(
                "orchestration-choose",
                ButtonContent::Icon(ButtonIcon::muted(Glyph::Sliders)),
                ButtonKind::Ghost,
                enabled(!page.skills.pending()),
                ControlSizing::from_compact(compact),
                tokens,
            )
            .when(!page.skills.pending(), |button| {
                button.on_click(intent_handler(page.scope(), Intent::Choose, cx))
            }),
        )
        .child(
            settings::switch(
                "orchestration-skills",
                installed,
                can_toggle,
                compact,
                shell.areas.settings.orchestration.turns,
                tokens,
            )
            .when(can_toggle, |switch| {
                switch.on_click(intent_handler(page.scope(), Intent::Skills, cx))
            }),
        );
    settings::card(tokens).child(settings::row_at(
        words.skills_title(),
        Some(
            if update {
                words.update_available()
            } else {
                words.skills_hint()
            }
            .into(),
        ),
        page.skills
            .problem()
            .map(|cause| problem(shell, cause).to_owned().into()),
        Some(controls.into_any_element()),
        true,
        compact,
        tokens,
    ))
}
fn waiting(
    shell: &Shell,
    page: &Page,
    cause: Option<Problem>,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words.orchestration();
    let mut card = message(
        cause.map_or(words.loading(), |cause| problem(shell, cause)),
        tokens,
    );
    if cause.is_some_and(|cause| cause != Problem::Unsupported) {
        card = card.child(
            button_with_kind_at(
                "orchestration-refresh",
                words.refresh(),
                ButtonKind::Outline,
                ButtonState::Enabled,
                ControlSizing::from_compact(compact),
                tokens,
            )
            .on_click(intent_handler(page.scope(), Intent::Refresh, cx)),
        );
    }
    card
}
pub(super) fn message(text: &str, tokens: &Tokens) -> Div {
    settings::card(tokens)
        .p(px(spacing::S4))
        .type_(Type::Meta)
        .text_color(tokens.neutral.foreground_muted)
        .child(text.to_owned())
}
pub(super) const fn enabled(on: bool) -> ButtonState {
    if on {
        ButtonState::Enabled
    } else {
        ButtonState::Disabled
    }
}
pub(super) fn problem(shell: &Shell, cause: Problem) -> &str {
    let words = shell.words.orchestration();
    match cause {
        Problem::Offline => words.unavailable(),
        Problem::Unsupported => words.unsupported(),
        Problem::Changed => words.changed(),
        Problem::Busy => words.busy(),
        Problem::TimedOut => words.unconfirmed(),
        Problem::Refused(vessel_proto::ErrorCode::Forbidden) => words.access(),
        Problem::Refused(_) | Problem::InvalidReply => words.failed(),
    }
}
