//! The prompts waiting their turn (protocol 10's queue, ADR 0024), above
//! the composer: Paseo's queue track (`composer/index.tsx`,
//! `QueuedMessageRow`) with what GOAT adds (`QueuedRunsControl.tsx`).
//!
//! [Decision Log]
//! - Purpose: a message written while the agent works waits its turn and
//!   can be changed there: edited, moved, taken out, sent into the
//!   running turn, or sent now in place of it.
//! - Constraint: the queue is the node's (every client sees one queue);
//!   the composer's card is the lead's and is not touched: the queue is
//!   drawn in the slot above it.
//! - The references: Paseo shows each waiting message as a row (surface1,
//!   a hairline, radius 8, 12 by 8 inside, the text on two lines) with
//!   two round buttons, edit (surface2) and send now (the accent); its
//!   edit moves the text back into the composer, losing its place. GOAT
//!   adds a grip to reorder by dragging, a steer for agents that take one
//!   during a turn, and remove; its edit keeps the place.
//! - Choice: Paseo's row and buttons, GOAT's actions as more round
//!   buttons of the same kind: edit in the row itself, keeping the
//!   place (the node's `edit`, checked against the revision it was made
//!   from); steer while a turn runs, for an agent that takes it; send now,
//!   which interrupts the running turn when one runs (its own choice, not
//!   steer's); remove; a grip to drag a row before another (the node's
//!   `move`, named by entry). A held queue says why and offers resume and
//!   clear.
//! - Not yet: the queue's own keys (GOAT's arrow keys on the grip, its
//!   steer shortcut); attachments of a queued message are not shown.

use gpui::AppContext as _;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::picker::glyph;
use vessel_app_kit::ui::{
    AnyElement, App, Context, Entity, FluentBuilder as _, InteractiveElement as _, IntoElement,
    KeyDownEvent, ParentElement as _, Render, SharedString, Stateful,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{TextField, Tokens, Type, TypeExt as _, breakpoint, popup};
use vessel_app_link::Act;
use vessel_app_view::timeline::block_text;
use vessel_proto::{ContentBlock, Delivery, QueueChange, QueueEntryId, QueuedPrompt};

use crate::shell::Shell;

/// A row's round button (Paseo's 32).
const BUTTON: f32 = 32.0;
const BUTTON_NARROW: f32 = 40.0;

/// A queued message being edited in its row.
pub(crate) struct QueueEdit {
    entry: QueueEntryId,
    revision: u32,
    field: TextField,
}

impl std::fmt::Debug for QueueEdit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueueEdit")
            .field("entry", &self.entry)
            .finish_non_exhaustive()
    }
}

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

/// A queued message's text, as its row shows it.
fn text_of(prompt: &QueuedPrompt) -> String {
    prompt
        .content
        .iter()
        .map(block_text)
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

impl Shell {
    /// Refresh the retained edit's hint without touching its text or keys.
    pub(crate) fn refresh_queue_words(&self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if let Some(edit) = &self.areas.conversation.queue_edit {
            edit.field
                .set_placeholder(self.t.lang.conversation().queue_edit(), window, cx);
        }
    }

    /// Ask the node to change the queue of the session shown.
    fn queue_change(&self, change: QueueChange) {
        self.act(Act::Queue(change));
    }

    /// Edit `prompt` in its row.
    fn queue_edit(
        &mut self,
        prompt: &QueuedPrompt,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let words = self.t.lang.conversation();
        let field = TextField::new(words.queue_edit(), Self::queue_edit_entered, window, cx);
        field.set_text(&text_of(prompt), window, cx);
        field.focus(window, cx);
        self.areas.conversation.queue_edit = Some(QueueEdit {
            entry: prompt.id,
            revision: prompt.revision,
            field,
        });
        cx.notify();
    }

    fn queue_edit_entered(
        &mut self,
        text: String,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(edit) = self.areas.conversation.queue_edit.take() else {
            return;
        };
        if !text.trim().is_empty() {
            self.queue_change(QueueChange::Edit {
                entry: edit.entry,
                revision: edit.revision,
                content: vec![ContentBlock::Text { text }],
            });
        }
        cx.notify();
    }

    fn queue_edit_cancelled(&mut self, cx: &mut Context<'_, Self>) {
        if self.areas.conversation.queue_edit.take().is_some() {
            cx.notify();
        }
    }
}

/// The queue above the composer, when something waits.
pub(super) fn queue(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let area = &shell.areas.conversation;
    let queue = area.timeline.facts().queue.as_ref()?;
    if queue.entries.is_empty() {
        return None;
    }
    let words = shell.t.lang.conversation();
    let narrow = breakpoint::phone(window);
    let neutral = tokens.neutral;
    let running = shell.areas.session.conversation.turn_running();
    let steers = running
        && area
            .timeline
            .facts()
            .abilities
            .as_ref()
            .is_some_and(|abilities| abilities.steer);
    let mut track = div().flex().flex_col().gap(px(8.0)).child(
        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(4.0))
            .type_(Type::Meta)
            .text_color(neutral.foreground_muted)
            .child(match queue.held {
                Some(why) => words.queued_held(queue.entries.len(), why),
                None => words.queued(queue.entries.len()),
            })
            .child(div().flex_1())
            .when(queue.held.is_some(), |head| {
                head.child(
                    text_button("queue-resume", words.queue_resume(), tokens).on_click(
                        cx.listener(|shell, _event, _window, _cx| {
                            shell.queue_change(QueueChange::Resume);
                        }),
                    ),
                )
            })
            .child(
                text_button("queue-clear", words.queue_clear(), tokens).on_click(cx.listener(
                    |shell, _event, _window, _cx| {
                        shell.queue_change(QueueChange::Clear);
                    },
                )),
            ),
    );
    for prompt in &queue.entries {
        track = track.child(row(shell, prompt, running, steers, tokens, narrow, cx));
    }
    // The slot puts it in the composer's column, at the card's width.
    Some(track.pb(px(8.0)).into_any_element())
}

/// One waiting message: Paseo's row, GOAT's actions.
fn row(
    shell: &Shell,
    prompt: &QueuedPrompt,
    running: bool,
    steers: bool,
    tokens: &Tokens,
    narrow: bool,
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
    let dragged = Dragged {
        entry,
        text: text.clone().into(),
        tokens: *tokens,
    };
    let grip = div()
        .id(SharedString::from(format!("queue-grip-{entry}")))
        .flex_none()
        .cursor_grab()
        .child(glyph(Glyph::Grip, 14.0, neutral.foreground_extra_muted))
        .on_drag(
            dragged,
            |dragged, _offset, _window, cx: &mut App| -> Entity<DragPreview> {
                cx.new(|_| DragPreview(dragged.clone()))
            },
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
) -> Stateful<vessel_app_kit::ui::Div> {
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
fn text_button(
    id: &'static str,
    label: &'static str,
    tokens: &Tokens,
) -> Stateful<vessel_app_kit::ui::Div> {
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

#[cfg(test)]
mod language_tests;
