//! The queue's rows as Paseo and GOAT draw them: each waiting message
//! with its grip (dragged, or moved by the arrow keys), its text or its
//! edit box, and its actions (send now, edit, remove). Its state and what
//! the actions do are the parent module's.

use gpui::AppContext as _;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::picker::glyph;
use vessel_app_kit::ui::{
    AnyElement, App, Context, Div, Entity, FluentBuilder as _, InteractiveElement as _,
    IntoElement, KeyDownEvent, MouseButton, ParentElement as _, Render, SharedString, Stateful,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, breakpoint, popup};
use vessel_app_view::ConversationWords;
use vessel_proto::{Delivery, PromptQueue, QueueChange, QueueEntryId, QueuedPrompt};

use crate::shell::Shell;

use super::{QueueEdit, text_of};

/// A row's round button (Paseo's 32).
const BUTTON: f32 = 32.0;
const BUTTON_NARROW: f32 = 40.0;

/// A row being dragged by its grip.
#[derive(Debug, Clone)]
struct Dragged {
    entry: QueueEntryId,
    text: SharedString,
    tokens: Tokens,
}

/// What follows the pointer while a row is dragged: its text on the
/// row's surface.
struct DragPreview(Dragged);

impl Render for DragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        let neutral = self.0.tokens.neutral;
        div()
            .w(px(280.0))
            .px(px(12.0))
            .py(px(8.0))
            .rounded(px(8.0))
            .border_1()
            .border_color(neutral.border)
            .bg(neutral.surface1)
            .shadow(popup::shadow_md(&self.0.tokens))
            .type_(Type::Body)
            .text_color(neutral.foreground)
            .truncate()
            .child(self.0.text.clone())
    }
}

/// The queue above the composer, when something waits.
pub(crate) fn queue(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let area = &shell.areas.conversation;
    let queue = area.timeline.facts().queue.as_ref()?;
    if queue.entries.is_empty() {
        return None;
    }
    let words = shell.t.lang.conversation();
    let narrow = breakpoint::phone(window);
    let running = shell.areas.session.conversation.turn_running();
    let steers = running
        && area
            .timeline
            .facts()
            .abilities
            .as_ref()
            .is_some_and(|abilities| abilities.steer);
    let mut track = div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(head(queue, *words, tokens, cx));
    for (at, prompt) in queue.entries.iter().enumerate() {
        let moves = moves_of(queue, at);
        track = track.child(row(
            shell,
            prompt,
            (running, steers),
            moves,
            tokens,
            (narrow, window),
            cx,
        ));
    }
    // The slot puts it in the composer's column, at the card's width.
    Some(track.pb(px(8.0)).into_any_element())
}

/// The queue's head: how many wait (and why they are held), Resume while
/// held, and Clear.
fn head(
    queue: &PromptQueue,
    words: ConversationWords,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(px(4.0))
        .type_(Type::Meta)
        .text_color(tokens.neutral.foreground_muted)
        .child(match queue.held {
            Some(why) => words.queued_held(queue.entries.len(), why),
            None => words.queued(queue.entries.len()),
        })
        .child(div().flex_1())
        .when(queue.held.is_some(), |head| {
            head.child(
                text_button("queue-resume", words.queue_resume(), tokens).on_click(cx.listener(
                    |shell, _event, _window, _cx| {
                        shell.queue_change(QueueChange::Resume);
                    },
                )),
            )
        })
        .child(
            text_button("queue-clear", words.queue_clear(), tokens).on_click(cx.listener(
                |shell, _event, _window, _cx| {
                    shell.queue_change(QueueChange::Clear);
                },
            )),
        )
}

/// Where the arrow keys move entry `at`: before the one above, or before
/// the one after the next (the end past the last), as GOAT does.
fn moves_of(queue: &PromptQueue, at: usize) -> Moves {
    Moves {
        up: at
            .checked_sub(1)
            .and_then(|above| queue.entries.get(above))
            .map_or(Move::Stay, |above| Move::Before(above.id)),
        down: if at.saturating_add(1) < queue.entries.len() {
            queue
                .entries
                .get(at.saturating_add(2))
                .map_or(Move::End, |after| Move::Before(after.id))
        } else {
            Move::Stay
        },
    }
}

/// Where a row's grip moves it by an arrow key.
#[derive(Debug, Clone, Copy)]
enum Move {
    /// It is at that end already.
    Stay,
    /// Before this entry.
    Before(QueueEntryId),
    /// To the end.
    End,
}

/// Where the up and down arrows move a row.
#[derive(Debug, Clone, Copy)]
struct Moves {
    up: Move,
    down: Move,
}

/// One waiting message: Paseo's row, GOAT's actions.
fn row(
    shell: &Shell,
    prompt: &QueuedPrompt,
    (running, steers): (bool, bool),
    moves: Moves,
    tokens: &Tokens,
    (narrow, window): (bool, &mut Window),
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let neutral = tokens.neutral;
    let entry = prompt.id;
    let text = text_of(prompt);
    let editing = shell
        .areas
        .conversation
        .queue_edit
        .as_ref()
        .filter(|edit| edit.entry == entry);
    let side = if narrow { BUTTON_NARROW } else { BUTTON };
    let grip = grip(
        Dragged {
            entry,
            text: text.clone().into(),
            tokens: *tokens,
        },
        moves,
        window,
        cx,
    );
    let body = body(editing, text, tokens, narrow, cx);
    let actions = actions(prompt, editing.is_none(), running, steers, side, tokens, cx);
    let drop_line = neutral.foreground_extra_muted;
    let row = div()
        .id(SharedString::from(format!("queue-row-{entry}")))
        .flex()
        .gap(px(8.0))
        .px(px(12.0))
        .py(px(8.0))
        .rounded(px(8.0))
        .border_1()
        .border_color(neutral.border)
        .bg(neutral.surface1)
        .drag_over::<Dragged>(move |style, _dragged, _window, _cx| style.border_color(drop_line))
        .on_drop(cx.listener(move |shell, dragged: &Dragged, _window, _cx| {
            if dragged.entry != entry {
                shell.queue_change(QueueChange::Move {
                    entry: dragged.entry,
                    before: Some(entry),
                });
            }
        }));
    // On a phone the four 40-point buttons would leave the text about
    // 100 points: they go on a line of their own under it (Paseo keeps
    // one line, with two buttons).
    if narrow {
        row.flex_col()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(grip)
                    .child(body),
            )
            .child(div().flex().justify_end().child(actions))
            .into_any_element()
    } else {
        row.items_center()
            .child(grip)
            .child(body)
            .child(actions)
            .into_any_element()
    }
}

/// A row's grip: dragged before another row; given the keys by a click,
/// its arrow keys move the row one place (GOAT's), and it shows it has
/// them in the foreground on `surface2`.
fn grip(
    dragged: Dragged,
    moves: Moves,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Stateful<Div> {
    let neutral = dragged.tokens.neutral;
    let entry = dragged.entry;
    let grip_id = SharedString::from(format!("queue-grip-{entry}"));
    // The grip's keys outlive a frame and follow the entry as it moves.
    let focus = window
        .use_keyed_state(grip_id.clone(), cx, |_, cx| cx.focus_handle())
        .read(cx)
        .clone();
    let keyed = focus.is_focused(window);
    div()
        .id(grip_id)
        .flex_none()
        .rounded(px(4.0))
        .cursor_grab()
        .track_focus(&focus)
        .when(keyed, |grip| grip.bg(neutral.surface2))
        .child(glyph(
            Glyph::Grip,
            14.0,
            if keyed {
                neutral.foreground
            } else {
                neutral.foreground_extra_muted
            },
        ))
        .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
            window.focus(&focus, cx);
        })
        .on_key_down(
            cx.listener(move |shell, event: &KeyDownEvent, _window, cx| {
                let before = match event.keystroke.key.as_str() {
                    "up" => moves.up,
                    "down" => moves.down,
                    _ => return,
                };
                cx.stop_propagation();
                let before = match before {
                    Move::Stay => return,
                    Move::Before(other) => Some(other),
                    Move::End => None,
                };
                shell.queue_change(QueueChange::Move { entry, before });
            }),
        )
        .on_drag(
            dragged,
            |dragged, _offset, _window, cx: &mut App| -> Entity<DragPreview> {
                cx.new(|_| DragPreview(dragged.clone()))
            },
        )
}

/// A row's text, or the field it is edited in (the kit's field, so it
/// reads as one: the title's rename uses the same).
fn body(
    editing: Option<&QueueEdit>,
    text: String,
    tokens: &Tokens,
    narrow: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let neutral = tokens.neutral;
    match editing {
        Some(edit) => div()
            .flex_1()
            .min_w(px(0.0))
            .on_key_down(cx.listener(|shell, event: &KeyDownEvent, _window, cx| {
                if event.keystroke.key == "escape" {
                    shell.queue_edit_cancelled(cx);
                    cx.stop_propagation();
                }
            }))
            .child(edit.field.element(tokens, narrow, cx))
            .into_any_element(),
        None => div()
            .flex_1()
            .min_w(px(0.0))
            .type_(Type::Body)
            .text_size(px(14.0))
            .text_color(neutral.foreground)
            .line_clamp(2)
            .text_ellipsis()
            .child(text)
            .into_any_element(),
    }
}

/// A row's round buttons: edit (not while editing), steer (while a turn
/// runs, for an agent that takes it), send now (interrupting a running
/// turn) and remove.
fn actions(
    prompt: &QueuedPrompt,
    can_edit: bool,
    running: bool,
    steers: bool,
    side: f32,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let entry = prompt.id;
    let edit_prompt = prompt.clone();
    let mut actions = div().flex().flex_none().items_center().gap(px(8.0));
    if can_edit {
        actions = actions.child(
            round("queue-edit", entry, Glyph::Pencil, false, side, tokens).on_click(cx.listener(
                move |shell, _event, window, cx| {
                    shell.queue_edit(&edit_prompt, window, cx);
                },
            )),
        );
    }
    if steers {
        actions = actions.child(
            round("queue-steer", entry, Glyph::Steer, false, side, tokens).on_click(cx.listener(
                move |shell, _event, _window, _cx| {
                    shell.queue_change(QueueChange::SendNow {
                        entry,
                        delivery: Delivery::Steer,
                    });
                },
            )),
        );
    }
    actions = actions
        .child(
            round("queue-send", entry, Glyph::ArrowUp, true, side, tokens).on_click(cx.listener(
                move |shell, _event, _window, _cx| {
                    shell.queue_change(QueueChange::SendNow {
                        entry,
                        delivery: if running {
                            Delivery::Interrupt
                        } else {
                            Delivery::Start
                        },
                    });
                },
            )),
        )
        .child(
            round("queue-remove", entry, Glyph::Remove, false, side, tokens).on_click(cx.listener(
                move |shell, _event, _window, _cx| {
                    shell.queue_change(QueueChange::Remove { entry });
                },
            )),
        );
    actions.into_any_element()
}

/// A row's round button: surface2, or the accent for send.
fn round(
    name: &str,
    entry: QueueEntryId,
    mark: Glyph,
    accent: bool,
    side: f32,
    tokens: &Tokens,
) -> Stateful<Div> {
    let neutral = tokens.neutral;
    let (fill, ink) = if accent {
        (neutral.accent, neutral.accent_foreground)
    } else {
        (neutral.surface2, neutral.foreground)
    };
    let hover = popup::Surfaces::of(tokens).surface2;
    div()
        .id(SharedString::from(format!("{name}-{entry}")))
        .size(px(side))
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(fill)
        .cursor_pointer()
        .when(!accent, |button| button.hover(move |style| style.bg(hover)))
        .child(glyph(mark, 14.0, ink))
}

/// A small text button in the queue's heading.
fn text_button(id: &'static str, label: &'static str, tokens: &Tokens) -> Stateful<Div> {
    let neutral = tokens.neutral;
    div()
        .id(id)
        .px(px(8.0))
        .py(px(2.0))
        .rounded(px(6.0))
        .cursor_pointer()
        .text_color(neutral.foreground_muted)
        .hover(move |style| style.bg(neutral.surface2).text_color(neutral.foreground))
        .child(label)
}
