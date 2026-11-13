//! The model-and-effort control and its card, as the owner chose
//! (2026-11-13, captures of another product's composer, described in
//! `docs/plans/backend-m12-spike.md`): one control in the composer's row
//! reads the model, then the effort's word (in the level's colour at the
//! top step), led by a lightning bolt when fast mode is on. Pressing it
//! opens a small card above it: the bolt at the left switches fast mode;
//! the step's word in the middle, the model's name under it opening the
//! model's list; a reset at the right (back to the agent's default); under
//! them the slider of the agent's levels, its last step the agent's own
//! "ultra" switch (`view::effort`).
//!
//! [Decision Log]
//! - Purpose: the owner, on 0.0.0.0.7's row of a control per option: put
//!   Ultracode or a Max into the effort, make the effort and fast mode one
//!   control with the slider as its default, give the slider a faster feel
//!   when fast is on, and let the model's name open the model's list.
//! - Values, measured on the captures: the card 256 wide, 12 in, round 14;
//!   the word 14 at weight 600 above the model's name at 12 with a small
//!   chevron (a hover background under it); the bolt and the reset 16 in
//!   28 squares; the slider (`kit::slider`) under them.
//! - Colours: two points are visible in the captures, "High" (a muted
//!   warm fill, the reference's muted text colour) and the top step (a
//!   gradient from blue to violet with sparkles, its word violet). The
//!   ramp between them is inferred: the lower half of the steps below the
//!   top is the muted text colour; above it the fill moves toward the
//!   violet, up to two thirds of the way just below the top; the last
//!   step (an agent's "ultra" switch, or its own highest level, as
//!   Codex's Ultra in the captures) is the gradient. Fast mode makes the fill move (a sheen crossing it, the
//!   sparkles quicker); with it off only the top step's sparkles drift.
//! - Differences: when fast mode is off the control leads with Paseo's
//!   model glyph (the captures show only the bolt, fast on); an agent with
//!   no levels has no card: its control opens the model's list directly.

use std::rc::Rc;

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::slider::{self, Fill, Look, Motion, OnStep, Position, SliderState};
use vessel_app_kit::ui::{
    AnyElement, App, Context, Div, Hsla, InteractiveElement as _, IntoElement as _,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window, div, point, px, rgb,
};
use vessel_app_kit::{PillParts, Scheme, Tokens, Type, TypeExt as _, glyph, popup};
use vessel_app_view::Controls;
use vessel_app_view::effort::{Effort, Step};

use super::Shown;
use super::pickers::Picker;
use crate::shell::Shell;

/// The card's width.
const WIDTH: f32 = 256.0;
/// Its padding.
const PAD: f32 = 12.0;
/// Its corners.
const ROUND: f32 = 14.0;
/// A square button's size (the bolt, the reset).
const SQUARE: f32 = 28.0;
/// The card's distance above the control.
const ABOVE: f32 = -6.0;

/// The top step's word, and the end of its gradient: the captures'
/// violet (`#924ff7` for the word).
const VIOLET: u32 = 0x0092_4ff7;
/// The gradient's start, the captures' blue.
const BLUE: u32 = 0x0024_49d8;
/// The gradient's end.
const VIOLET_END: u32 = 0x008b_5cf6;
/// How far toward the violet the highest level's fill goes.
const RAMP_REACH: f32 = 0.66;

/// The card's state, kept by the draft.
pub(crate) struct CardState {
    /// Whether it is open.
    pub(crate) open: bool,
    /// Its slider's state.
    pub(crate) slider: SliderState,
}

impl CardState {
    pub(crate) fn new(cx: &mut App) -> Self {
        Self {
            open: false,
            slider: SliderState::new(cx),
        }
    }
}

/// Whether `index` is the last step, drawn as the top: the agent's
/// "ultra" switch (Claude Code's Ultracode) or its own highest level
/// (Codex's Ultra, the reference's top step).
fn is_top(effort: &Effort, index: usize) -> bool {
    effort.steps.len().checked_sub(1) == Some(index)
}

/// The step's word, as the agent says it.
fn word(effort: &Effort, index: Option<usize>) -> Option<&str> {
    index
        .and_then(|index| effort.steps.get(index))
        .map(|step| match step {
            Step::Level { name, .. } | Step::Top { name } => name.as_str(),
        })
}

/// How far up the steps below the top `index` is, 0 to 1.
fn height(effort: &Effort, index: usize) -> f32 {
    let below_top = effort.steps.len().saturating_sub(1);
    let last = f32::from(
        u16::try_from(below_top.saturating_sub(1))
            .unwrap_or(1)
            .max(1),
    );
    let index = f32::from(u16::try_from(index).unwrap_or(0));
    index.mul_add(last.recip(), 0.0).clamp(0.0, 1.0)
}

/// The slider's look at `index`, and the word's colour.
fn look(effort: &Effort, index: Option<usize>, fast: bool, tokens: &Tokens) -> (Look, Hsla) {
    let neutral = tokens.neutral;
    let motion_fast = if fast { Motion::Fast } else { Motion::Still };
    match index {
        Some(index) if is_top(effort, index) => (
            Look {
                fill: Fill::Gradient(rgb(BLUE).into(), rgb(VIOLET_END).into()),
                sparkles: true,
                motion: if fast { Motion::Fast } else { Motion::Slow },
            },
            rgb(VIOLET).into(),
        ),
        Some(index) => {
            // The lower half muted; above it toward the violet.
            let up = height(effort, index).mul_add(2.0, -1.0).clamp(0.0, 1.0);
            let toward = up.mul_add(RAMP_REACH, 0.0);
            let violet: Hsla = rgb(VIOLET_END).into();
            let fill = neutral.foreground_muted.blend(violet.opacity(toward));
            (
                Look {
                    fill: Fill::Solid(fill),
                    sparkles: false,
                    motion: motion_fast,
                },
                neutral.foreground,
            )
        }
        None => (
            Look {
                fill: Fill::Solid(neutral.foreground_muted),
                sparkles: false,
                motion: motion_fast,
            },
            neutral.foreground,
        ),
    }
}

/// The model control's words: the model, the agent's name when shown, the
/// effort's word and its colour.
pub(crate) struct PillWords {
    pub(crate) model: String,
    pub(crate) agent: Option<String>,
    pub(crate) effort: Option<(String, Option<Hsla>)>,
    pub(crate) fast: bool,
}

/// What the control reads for `controls` (`model` and `agent` from the
/// model's list's words).
pub(crate) fn pill_words(
    controls: Option<&Controls>,
    model: String,
    agent: Option<String>,
    tokens: &Tokens,
) -> PillWords {
    let effort = controls.and_then(|controls| controls.slider.as_ref());
    let fast = effort
        .and_then(|effort| effort.fast.as_ref())
        .is_some_and(vessel_app_view::effort::Switch::on);
    let effort_word = effort.and_then(|effort| {
        let word = word(effort, effort.current)?;
        let colour = effort
            .current
            .filter(|index| is_top(effort, *index))
            .map(|_| look(effort, effort.current, fast, tokens).1);
        Some((word.to_owned(), colour))
    });
    PillWords {
        model,
        agent,
        effort: effort_word,
        fast,
    }
}

/// The control's parts: the bolt when fast mode is on (else Paseo's model
/// glyph), the model, the agent quieter, the effort's word.
pub(crate) fn pill_parts(
    words: &PillWords,
    caret: bool,
    show_agent: bool,
    show_effort: bool,
) -> PillParts {
    PillParts {
        leading: Some(if words.fast {
            Glyph::LightningFilled
        } else {
            Glyph::Bot
        }),
        label: Some(words.model.clone().into()),
        quiet: words.agent.clone().filter(|_| show_agent).map(Into::into),
        after: words
            .effort
            .clone()
            .filter(|_| show_effort)
            .map(|(word, colour)| (word.into(), colour)),
        caret,
        ..PillParts::default()
    }
}

impl Shell {
    /// Press the model control: its card when the agent has levels, else
    /// the model's list.
    pub(crate) fn press_model_control(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let has_levels = matches!(
            self.shown(),
            Some(Shown::Ready(controls)) if controls.slider.is_some()
        );
        if !has_levels {
            self.open_picker(Picker::Model, window, cx);
            return;
        }
        for which in super::pickers::ALL {
            super::pickers::list_of(self, which).close();
        }
        let card = &mut self.areas.draft.effort;
        card.open = !card.open;
        if card.open {
            card.slider.focus(window, cx);
        } else {
            self.areas.draft.composer.focus(window, cx);
        }
        cx.notify();
    }

    /// Close the card.
    pub(crate) fn close_effort_card(&mut self, cx: &mut Context<'_, Self>) {
        if self.areas.draft.effort.open {
            self.areas.draft.effort.open = false;
            cx.notify();
        }
    }

    /// Apply `choices` from the card to the draft.
    fn effort_choices(&mut self, pick: impl Fn(&Effort) -> Vec<vessel_proto::OptionChoice>) {
        let Some(Shown::Ready(controls)) = self.shown() else {
            return;
        };
        let Some(effort) = controls.slider.as_ref() else {
            return;
        };
        for choice in pick(effort) {
            self.choose_option(choice);
        }
    }
}

/// The card, when open, above the model control.
pub(crate) fn card(
    shell: &Shell,
    controls: Option<&Controls>,
    touch: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.draft.effort;
    if !state.open {
        return None;
    }
    let controls = controls?;
    let effort = controls.slider.as_ref()?;
    let place = shell.areas.draft.features.places[0].get()?;
    let neutral = tokens.neutral;
    let words = shell.t.lang.options();
    let fast_on = effort
        .fast
        .as_ref()
        .is_some_and(vessel_app_view::effort::Switch::on);
    let shown_step = state.slider.previewing().or(effort.current);
    let (look, word_colour) = look(effort, shown_step, fast_on, tokens);
    let surface = match tokens.scheme {
        Scheme::Light => neutral.surface0,
        Scheme::Dark => neutral.surface2,
    };
    let model_name = controls.model.as_ref().map_or_else(
        || words.default_model.to_owned(),
        |model| model.current_name().to_owned(),
    );
    let entity = cx.entity().downgrade();
    let on_step: OnStep = Rc::new(move |step, _window, cx| {
        let _updated = entity.update(cx, |shell, cx| {
            shell.effort_choices(|effort| effort.choose(step));
            cx.notify();
        });
    });
    let top = top_row(
        effort,
        shown_step,
        word_colour,
        &model_name,
        words.default_model,
        tokens,
        cx,
    );
    let slider = slider::slider(
        "effort-slider",
        &state.slider,
        Position {
            steps: effort.steps.len(),
            at: effort.current,
        },
        look,
        touch,
        tokens,
        &on_step,
    );
    let pill_place = place;
    let card = div()
        .id("effort-card")
        .w(px(WIDTH))
        .p(px(PAD))
        .flex()
        .flex_col()
        .gap(px(8.0))
        .rounded(px(ROUND))
        .bg(surface)
        .border_1()
        .border_color(neutral.border_accent)
        .shadow(popup::shadow_md(tokens))
        .child(top)
        .child(slider)
        .on_mouse_down_out(
            cx.listener(move |shell, event: &gpui::MouseDownEvent, _window, cx| {
                // A press on the control itself toggles the card there.
                if !pill_place.contains(&event.position) {
                    shell.close_effort_card(cx);
                }
            }),
        )
        .on_key_down(
            cx.listener(|shell, event: &gpui::KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    shell.areas.draft.effort.open = false;
                    shell.areas.draft.composer.focus(window, cx);
                    cx.notify();
                }
            }),
        );
    Some(
        gpui::deferred(
            gpui::anchored()
                .position(point(place.left(), place.top()))
                .anchor(gpui::Anchor::BottomLeft)
                .offset(point(px(0.0), px(ABOVE)))
                .snap_to_window_with_margin(px(8.0))
                .child(card),
        )
        .with_priority(3)
        .into_any_element(),
    )
}

/// The card's top: the bolt (fast mode), the step's word over the model's
/// name, the reset.
fn top_row(
    effort: &Effort,
    shown_step: Option<usize>,
    word_colour: Hsla,
    model_name: &str,
    default_word: &'static str,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let neutral = tokens.neutral;
    let fast_button = effort.fast.as_ref().map(|fast| {
        let choice = fast.choose(!fast.on());
        let (which, colour) = if fast.on() {
            (Glyph::LightningFilled, rgb(VIOLET).into())
        } else {
            (Glyph::Lightning, neutral.foreground_muted)
        };
        square("effort-fast", which, colour, tokens).on_click(cx.listener(
            move |shell, _event, _window, cx| {
                shell.choose_option(choice.clone());
                cx.notify();
            },
        ))
    });
    let reset = square(
        "effort-reset",
        Glyph::Reset,
        neutral.foreground_muted,
        tokens,
    )
    .on_click(cx.listener(|shell, _event, _window, cx| {
        shell.effort_choices(Effort::reset);
        cx.notify();
    }));
    let model_button = div()
        .id("effort-model")
        .flex()
        .flex_row()
        .items_center()
        .gap(px(2.0))
        .px(px(6.0))
        .py(px(1.0))
        .rounded(px(6.0))
        .cursor_pointer()
        .hover(move |style| style.bg(neutral.surface3))
        .type_(Type::Meta)
        .text_color(neutral.foreground_muted)
        .child(model_name.to_owned())
        .child(glyph(Glyph::ChevronRight, 12.0, neutral.foreground_muted))
        .on_click(cx.listener(|shell, _event, window, cx| {
            shell.areas.draft.effort.open = false;
            shell.open_picker(Picker::Model, window, cx);
        }));
    let centre = div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .items_center()
        .child(
            div()
                .type_(Type::Label)
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(word_colour)
                .child(word(effort, shown_step).unwrap_or(default_word).to_owned()),
        )
        .child(model_button);
    div()
        .flex()
        .flex_row()
        .items_center()
        .child(match fast_button {
            Some(button) => button.into_any_element(),
            None => div().size(px(SQUARE)).flex_shrink_0().into_any_element(),
        })
        .child(centre)
        .child(reset)
}

/// A square button of one glyph.
fn square(
    id: &'static str,
    which: Glyph,
    colour: Hsla,
    tokens: &Tokens,
) -> vessel_app_kit::ui::Stateful<Div> {
    let hover = tokens.neutral.surface3;
    div()
        .id(id)
        .flex_shrink_0()
        .size(px(SQUARE))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(8.0))
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .child(glyph(which, 16.0, colour))
}
