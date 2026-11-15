//! Side panel layout, using the shared chat rows, request cards and composer.

use super::{SIDE, SidePlace};
use crate::shell::Shell;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    KeyDownEvent, ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window, div,
    px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, SendKind, SendOn, Tokens, Type, TypeExt as _, alert, breakpoint,
    button_with_kind, composer_card, glyph, send_button,
};
use vessel_app_view::prefs::SendKey;
use vessel_app_view::side_question::{Operation, Problem, Question};

/// A readable side chat column, independent of the main chat's content width.
const PANEL_WIDTH: f32 = 400.0;

pub(crate) fn toggle(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let open = shell.areas.session.open.as_ref()?;
    if !open.structured {
        return None;
    }
    let area = &shell.areas.side_question;
    let label = if area.state.target().is_some() {
        shell.words.side_question().close()
    } else {
        shell.words.side_question().open()
    };
    let target = if breakpoint::phone(window) {
        breakpoint::TOUCH_TARGET
    } else {
        spacing::S7
    };
    let button = button_with_kind(
        "side-toggle",
        "",
        ButtonKind::Ghost,
        ButtonState::Enabled,
        tokens,
    )
    .size(px(target))
    .flex_shrink_0()
    .child(glyph(
        Glyph::Message,
        spacing::S4,
        tokens.neutral.foreground_muted,
    ))
    .on_click(cx.listener(|shell, _, window, cx| shell.side_toggle(window, cx)));
    let (tips, place) = &area.toggle_tip;
    Some(
        div()
            .child(tips.on(button, place, label.to_owned(), breakpoint::phone(window)))
            .children(tips.render(tokens))
            .into_any_element(),
    )
}

pub(crate) fn beside(
    shell: &Shell,
    main: Div,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let Some(target) = shell.areas.side_question.state.target() else {
        return main;
    };
    if !shell.areas.session.open.as_ref().is_some_and(|open| {
        open.machine == target.machine.as_str() && open.session == target.parent
    }) {
        return main;
    }
    let side = panel(shell, tokens, window, cx);
    let frame = div().relative().size_full().min_w_0().min_h_0();
    if f32::from(window.viewport_size().width) < breakpoint::INSPECTOR {
        frame.child(main).child(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left_0()
                .right_0()
                .occlude()
                .child(side),
        )
    } else {
        frame
            .flex()
            .child(div().flex_1().min_w_0().min_h_0().child(main))
            .child(
                div()
                    .w(px(PANEL_WIDTH))
                    .flex_shrink_0()
                    .min_h_0()
                    .child(side),
            )
    }
}

fn panel(shell: &Shell, tokens: &Tokens, window: &mut Window, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.words.side_question();
    let area = &shell.areas.side_question;
    let busy = area.state.pending().map(|pending| match pending.operation {
        Operation::Open => words.opening(),
        Operation::Reset => words.resetting(),
        Operation::Prompt => words.sending(),
    });
    let ready = area.state.side().is_some();
    let content = div()
        .flex_1()
        .min_h_0()
        .child(super::super::conversation::timeline::rows(
            shell, SIDE, tokens, window, cx,
        ));
    div()
        .id("side-question-panel")
        .relative()
        .size_full()
        .flex()
        .flex_col()
        .min_w_0()
        .min_h_0()
        .bg(tokens.neutral.surface0)
        .border_l_1()
        .border_color(tokens.neutral.border)
        .on_key_down(cx.listener(|shell, event: &KeyDownEvent, window, cx| {
            if event.keystroke.key == "escape"
                && !vessel_app_kit::TextField::focused_composing(window, cx)
            {
                shell.side_toggle(window, cx);
                cx.stop_propagation();
            }
        }))
        .child(header(shell, tokens, window, cx))
        .child(
            div()
                .px(px(spacing::S3))
                .py(px(spacing::S2))
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(words.hint().to_owned()),
        )
        .children(busy.map(|text| {
            div()
                .px(px(spacing::S3))
                .type_(Type::Meta)
                .child(text.to_owned())
        }))
        .children(
            area.state
                .problem()
                .map(|problem| failure(shell, problem, tokens, cx)),
        )
        .when(ready, |panel| panel.child(content))
        .when(!ready, |panel| panel.child(div().flex_1()))
        .when(ready, |panel| {
            panel.child(composer(shell, tokens, window, cx))
        })
}

fn header(shell: &Shell, tokens: &Tokens, window: &Window, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.words.side_question();
    let area = &shell.areas.side_question;
    let enabled = area.state.side().is_some() && area.state.pending().is_none();
    let state = if enabled {
        ButtonState::Enabled
    } else {
        ButtonState::Disabled
    };
    div()
        .flex()
        .items_center()
        .gap(px(spacing::S1))
        .px(px(spacing::S2))
        .py(px(spacing::S1))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .type_(Type::Body)
                .child(words.title().to_owned()),
        )
        .child(
            button_with_kind(
                "side-reset",
                words.reset().to_owned(),
                ButtonKind::Ghost,
                state,
                tokens,
            )
            .when(enabled, |button| {
                button.on_click(cx.listener(|shell, _, window, cx| shell.side_reset(window, cx)))
            }),
        )
        .child(
            button_with_kind(
                "side-close",
                words.close().to_owned(),
                ButtonKind::Ghost,
                ButtonState::Enabled,
                tokens,
            )
            .child(glyph(
                Glyph::Close,
                spacing::S4,
                tokens.neutral.foreground_muted,
            ))
            .on_click(cx.listener(|shell, _, window, cx| shell.side_toggle(window, cx))),
        )
        .when(breakpoint::phone(window), |header| header.flex_wrap())
}

fn composer(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let area = &shell.areas.side_question;
    let field = &area.composer;
    field.set_placeholder(shell.words.side_question().placeholder(), window, cx);
    field.set_send_on(match shell.prefs.settings.composer.send {
        SendKey::Enter => SendOn::Enter,
        SendKey::ModEnter => SendOn::ModEnter,
    });
    if let Some(sent) = area.acknowledged.borrow_mut().take()
        && field.text(cx) == sent
    {
        field.clear(window, cx);
    }
    let running = area.conversation.timeline.in_turn();
    let enabled = area.state.pending().is_none()
        && (running || (area.state.may_prompt() && Question::parse(&field.text(cx)).is_ok()));
    let button = send_button(
        "side-send",
        if running {
            SendKind::Stop
        } else {
            SendKind::Send
        },
        enabled,
        breakpoint::phone(window),
        tokens,
    )
    .when(enabled, |button| {
        button.on_click(cx.listener(move |shell, _, window, cx| {
            if running {
                shell.side_stop();
            } else {
                let text = shell.areas.side_question.composer.text(cx);
                shell.side_submit(text, window, cx);
            }
        }))
    });
    let requests =
        super::super::conversation::dock::requests::<SidePlace>(shell, tokens, window, cx);
    div()
        .id("side-question-composer")
        .px(px(spacing::S3))
        .pb(px(spacing::S3))
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .children(requests)
        .child(
            composer_card(tokens, breakpoint::phone(window))
                .child(field.input(tokens, cx))
                .child(div().flex().justify_end().child(button)),
        )
}

fn failure(shell: &Shell, problem: Problem, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.words.side_question();
    let text = match &problem {
        Problem::Closed | Problem::NoSide => words.recover(),
        Problem::Busy => words.busy(),
        Problem::Exhausted => words.exhausted(),
        Problem::InvalidQuestion(_) => words.invalid(),
        Problem::Slash => words.slash(),
        Problem::Offline => words.offline(),
        Problem::HistoryGap => words.history_gap(),
        Problem::Unsupported => words.unsupported(),
        Problem::Uncertain => words.uncertain(),
        Problem::Unsent => words.unsent(),
        Problem::InvalidReply => words.invalid_reply(),
        Problem::Refused(_) => words.refused(),
    };
    let recover = shell.areas.side_question.state.pending().is_none()
        && !matches!(
            problem,
            Problem::Busy
                | Problem::Exhausted
                | Problem::InvalidQuestion(_)
                | Problem::Slash
                | Problem::HistoryGap
        );
    let action = recover.then(|| {
        button_with_kind(
            "side-recover",
            words.recover().to_owned(),
            ButtonKind::Ghost,
            ButtonState::Enabled,
            tokens,
        )
        .on_click(cx.listener(|shell, _, _, cx| {
            shell.side_open();
            cx.notify();
        }))
        .into_any_element()
    });
    div().p(px(spacing::S2)).child(alert::alert(
        alert::Content {
            title: None,
            description: Some(div().child(text.to_owned()).into_any_element()),
            action,
        },
        alert::Variant::Error,
        alert::Size::Small,
        tokens,
    ))
}
