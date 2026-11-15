//! The existing kit autocomplete handles rows, empty/error/loading and scroll.
use super::{Context, DraftKey, Root, Shell, Window};
use vessel_app_kit::autocomplete::{AutocompleteRow, Listing, Motion, RowKind, autocomplete};
use vessel_app_kit::ui::{
    AnyElement, FluentBuilder as _, IntoElement as _, ParentElement as _, SharedString, Styled,
    div, px,
};
use vessel_app_kit::{Tokens, anchor_probe, chrome, motion};
use vessel_app_view::composer::mentions::{Phase, SuggestionKind};
const LIFT: f32 = 4.0;
const OFFSET: f32 = 8.0;
const UNDER_BAR: f32 = -48.0;
const AT_TOP: f32 = -12.0;
const ROWS: Motion = Motion {
    ease: motion::ease,
    length: motion::QUICK,
};

pub(in crate::areas::composer) fn menu(
    shell: &Shell,
    target: DraftKey,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    shell.observe_file_menu(target, cx);
    let owner = shell.file_menu(target);
    let state = owner.borrow();
    let current = listing(shell, target, cx);
    let now = shell.areas.composer.now();
    let mut reveal = state.reveal.borrow_mut();
    reveal.follow(
        current.is_some(),
        now,
        if motion::held(cx) {
            0.0
        } else {
            motion::STANDARD.as_secs_f32()
        },
    );
    let Some(shown) = reveal.shown(now) else {
        drop(state.last.borrow_mut().take());
        return None;
    };
    if reveal.moving(now) {
        window.request_animation_frame();
    }
    drop(reveal);
    let (listing, chosen) = if let Some(current) = current {
        *state.last.borrow_mut() = Some(current.clone());
        current
    } else {
        state.last.borrow().clone()?
    };
    let on_choose =
        move |shell: &mut Shell, index, window: &mut Window, cx: &mut Context<'_, Shell>| {
            shell.choose_mention(target, index, window, cx);
        };
    let id = match target {
        DraftKey::New => "draft-files",
        DraftKey::Session { .. } => "session-files",
        DraftKey::Workspace { .. } => "workspace-files",
    };
    let list = autocomplete(
        id,
        &listing,
        chosen,
        &state.list,
        on_choose,
        ROWS,
        tokens,
        cx,
    );
    let room = state.drawn.get().map(|bounds| {
        px(f32::from(bounds.bottom())
            .mul_add(
                1.0,
                if chrome::system_titlebar() {
                    AT_TOP
                } else {
                    UNDER_BAR
                },
            )
            .max(0.0))
    });
    if room.is_none() {
        window.request_animation_frame();
    }
    Some(
        div()
            .absolute()
            .left_0()
            .right_0()
            .bottom_full()
            .mb(px(LIFT.mul_add(shown, OFFSET)))
            .flex()
            .flex_col()
            .when_some(room, Styled::max_h)
            .opacity(shown)
            .child(anchor_probe(&state.drawn))
            .child(list)
            .into_any_element(),
    )
}

fn listing(
    shell: &Shell,
    target: DraftKey,
    cx: &vessel_app_kit::ui::App,
) -> Option<(Listing, Option<usize>)> {
    let owner = shell.file_menu(target);
    let state = owner.borrow();
    let words = shell.words.composer();
    match state.model.phase() {
        Phase::Hidden => {
            let token = shell.active_file_token(target, cx);
            if token.is_some() {
                match &*shell.areas.composer.mention_root.borrow() {
                    Root::Reading(ask) if matches!(target, DraftKey::Session { session } if session == ask.session) => {
                        Some((
                            Listing::Loading(words.files_loading().to_owned().into()),
                            None,
                        ))
                    }
                    Root::Ready(ask, Err(_)) if matches!(target, DraftKey::Session { session } if session == ask.session) => {
                        Some((
                            Listing::Failed(words.files_failed().to_owned().into()),
                            None,
                        ))
                    }
                    Root::Exhausted => Some((
                        Listing::Failed(words.files_failed().to_owned().into()),
                        None,
                    )),
                    Root::Missing | Root::Reading(_) | Root::Ready(_, _) => None,
                }
            } else {
                None
            }
        }
        Phase::Debouncing { .. } | Phase::Loading { .. } if state.model.rows().is_empty() => {
            Some((
                Listing::Loading(words.files_loading().to_owned().into()),
                None,
            ))
        }
        Phase::Empty(_) => Some((Listing::Empty(words.files_empty().to_owned().into()), None)),
        Phase::Invalid(_) | Phase::Failed { .. } => Some((
            Listing::Failed(words.files_failed().to_owned().into()),
            None,
        )),
        Phase::Rows { .. } | Phase::Debouncing { .. } | Phase::Loading { .. } => {
            let rows = state
                .model
                .rows()
                .iter()
                .rev()
                .map(|row| AutocompleteRow {
                    label: SharedString::from(row.path.as_str().to_owned()),
                    detail: None,
                    description: None,
                    kind: match row.kind {
                        SuggestionKind::File => RowKind::File,
                        SuggestionKind::Directory => RowKind::Directory,
                    },
                })
                .collect();
            Some((Listing::Rows(rows), state.model.selected()))
        }
    }
}
