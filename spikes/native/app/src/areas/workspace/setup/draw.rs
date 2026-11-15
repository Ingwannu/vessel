//! Structured setup projection using shared kit controls and canonical values.
//! Effects require a separately admitted controller; this renderer grants none.
use crate::shell::Shell;
use vessel_app_kit::ui::{
    Context, Div, ElementId, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, Start, Tokens, Type, TypeExt as _, badge, button_with_kind,
    output_box, theme::spacing,
};
use vessel_app_view::workspace::setup_observation::{Address, Model, Support, elapsed, output};
use vessel_proto::SetupRunId;
use vessel_proto::workspace::{
    CancelSetupOutcome, SetupCommandIndex, SetupCommandProgress, SetupCommandResult,
    SetupCommandState, SetupRunState,
};

const LOG_MAX_HEIGHT: f32 = 400.0;
/// Callbacks are provided only by a later admitted area controller.
#[derive(Clone, Copy)]
pub(super) struct Dispatch {
    pub(super) toggle: fn(
        &mut Shell,
        &Address,
        SetupRunId,
        SetupCommandIndex,
        &mut Window,
        &mut Context<'_, Shell>,
    ),
    pub(super) cancel: fn(&mut Shell, &Address, SetupRunId, &mut Window, &mut Context<'_, Shell>),
}

pub(super) struct Card<'a> {
    pub(super) shell: &'a Shell,
    pub(super) model: &'a Model,
    pub(super) dispatch: Option<Dispatch>,
    pub(super) tokens: &'a Tokens,
}
pub(super) fn render(card: &Card<'_>, window: &mut Window, cx: &mut Context<'_, Shell>) -> Div {
    let Card {
        shell,
        model,
        dispatch,
        tokens,
    } = card;
    let dispatch = *dispatch;
    let words = shell.words.setupobs();
    let mut body = div()
        .debug_selector(|| "setup-observation".to_owned())
        .flex()
        .flex_col()
        .gap(px(spacing::S3))
        .p(px(spacing::S4));
    if model.support == Support::Unavailable {
        return body.child(notice(words.unsupported(), tokens));
    }
    if model.failure.is_some()
        || matches!(
            model.read,
            vessel_app_view::workspace::git::Reading::Failed(_)
        )
    {
        body = body.child(notice(words.read_failed(), tokens));
    }
    if let Some(outcome) = model.cancellation {
        let label = match outcome {
            CancelSetupOutcome::Requested => words.cancel_requested(),
            CancelSetupOutcome::NotRunning => words.cancel_not_running(),
            CancelSetupOutcome::Stale => words.cancel_stale(),
            CancelSetupOutcome::Unknown => words.cancel_unknown(),
        };
        body = body.child(div().type_(Type::Meta).child(label.to_owned()));
    }
    let Some(snapshot) = model.snapshot() else {
        return body.child(div().type_(Type::Body).child(words.absent().to_owned()));
    };
    let status = match snapshot.state {
        SetupRunState::Running => words.running(),
        SetupRunState::Succeeded => words.completed(),
        SetupRunState::Failed => words.failed(),
        SetupRunState::Cancelled => words.cancelled(),
        SetupRunState::Unknown => words.unknown(),
    };
    body = body.child(div().type_(Type::Body).child(status.to_owned()));
    if snapshot.commands.is_empty() {
        body = body.child(
            div()
                .type_(Type::Meta)
                .child(words.no_commands().to_owned()),
        );
    }
    let Some(address) = &model.address else {
        return body;
    };
    for row in &snapshot.commands {
        body = body.child(command(card, row, window, cx));
    }
    let mut cancel = button_with_kind(
        format!("setup-observation-cancel-{}", snapshot.cursor.run),
        words.cancel(),
        ButtonKind::Secondary,
        if model.can_cancel() && dispatch.is_some() {
            ButtonState::Enabled
        } else {
            ButtonState::Disabled
        },
        tokens,
    )
    .debug_selector(|| "setup-observation-cancel".to_owned());
    if model.can_cancel()
        && let Some(dispatch) = dispatch
    {
        let address = address.clone();
        let run = snapshot.cursor.run;
        cancel = cancel.on_click(cx.listener(move |shell, _event, window, cx| {
            (dispatch.cancel)(shell, &address, run, window, cx)
        }));
    }
    body.child(cancel)
}
fn command(
    card: &Card<'_>,
    row: &SetupCommandProgress,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let Card {
        shell,
        model,
        dispatch,
        tokens,
    } = card;
    let dispatch = *dispatch;
    let words = shell.words.setupobs();
    let (label, tone, duration) = match &row.state {
        SetupCommandState::Waiting => (words.waiting(), badge::Variant::Muted, None),
        SetupCommandState::Running => (words.running(), badge::Variant::Muted, None),
        SetupCommandState::Unknown => (words.unknown(), badge::Variant::Warning, None),
        SetupCommandState::Finished { elapsed, result } => {
            let (label, tone) = match result {
                SetupCommandResult::Exited { code: Some(0) } => {
                    (words.completed(), badge::Variant::Success)
                }
                SetupCommandResult::Exited { code: Some(_) } => {
                    (words.failed(), badge::Variant::Error)
                }
                SetupCommandResult::Exited { code: None } => {
                    (words.exit_unknown(), badge::Variant::Warning)
                }
                SetupCommandResult::SpawnFailed => (words.spawn_failed(), badge::Variant::Error),
                SetupCommandResult::TimedOut => (words.timed_out(), badge::Variant::Error),
                SetupCommandResult::Cancelled => (words.cancelled(), badge::Variant::Muted),
                SetupCommandResult::IoFailed => (words.io_failed(), badge::Variant::Error),
                SetupCommandResult::Unknown => (words.unknown(), badge::Variant::Warning),
            };
            (label, tone, Some(*elapsed))
        }
    };
    let mut head = div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(spacing::S2))
        .child(div().min_w_0().flex_1().type_(Type::Mono).child(format!(
            "{}. {}",
            row.index.get(),
            row.command
        )))
        .child(badge::badge(
            label.to_owned(),
            tone,
            badge::Size::ExtraSmall,
            None,
            tokens,
        ));
    if let Some(duration) = duration {
        head = head.child(div().type_(Type::Meta).child(elapsed(duration)));
    }
    let run = model.snapshot().map(|snapshot| snapshot.cursor.run);
    let text = output(row.output.bytes());
    let expandable = row.state != SetupCommandState::Running || !text.trim().is_empty();
    let mut header = div()
        .debug_selector(|| "setup-command".to_owned())
        .id(ElementId::Name(
            format!("setup-command-{run:?}-{}", row.index.get()).into(),
        ))
        .p(px(spacing::S3))
        .child(head);
    if expandable
        && let Some(dispatch) = dispatch
        && let (Some(address), Some(snapshot)) = (&model.address, model.snapshot())
    {
        let address = address.clone();
        let run = snapshot.cursor.run;
        let index = row.index;
        header = header.on_click(cx.listener(move |shell, _event, window, cx| {
            (dispatch.toggle)(shell, &address, run, index, window, cx)
        }));
    }
    let mut card = div()
        .flex()
        .flex_col()
        .border_1()
        .border_color(tokens.neutral.border)
        .child(header);
    if model.expanded(row.index) {
        if row.truncated {
            card = card.child(div().type_(Type::Meta).child(words.truncated().to_owned()));
        }
        if text.trim().is_empty() {
            card = card.child(
                div()
                    .p(px(spacing::S3))
                    .type_(Type::Meta)
                    .child(words.no_output().to_owned()),
            );
        } else {
            card = card.child(div().max_h(px(LOG_MAX_HEIGHT)).child(output_box(
                ElementId::Name(format!("setup-output-{run:?}-{}", row.index.get()).into()),
                &text,
                Start::End,
                tokens,
                window,
                cx,
            )));
        }
    }
    card
}

fn notice(text: &str, tokens: &Tokens) -> Div {
    vessel_app_kit::alert::alert(
        vessel_app_kit::alert::Content {
            title: None,
            description: Some(div().child(text.to_owned()).into_any_element()),
            action: None,
        },
        vessel_app_kit::alert::Variant::Warning,
        vessel_app_kit::alert::Size::Small,
        tokens,
    )
}
