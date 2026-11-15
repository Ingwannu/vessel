//! Paseo's footer overlay composed from the shared voice meter and Circle button.
//!
//! This area owns placement/telemetry adaptation only. G2 owns button painting,
//! loading/input guards and fixed tones; kit owns each microphone/filled glyph.

use vessel_app_kit::ui::{
    App, ClickEvent, Div, ElementId, InteractiveElement as _, IntoElement as _, ParentElement as _,
    Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonContent, ButtonIcon, ButtonState, CircleConfig, CircleTone, Tokens, assets::Glyph,
    button_with_design, theme::spacing, voice_meter as meter,
};
use vessel_app_view::{
    localization::Translator,
    voice::{Mute, Overlay, SpeechActivity, State, Telemetry},
};

// Pinned realtime-voice-overlay.tsx uses FOOTER_HEIGHT75 and (75-44)/2.
const FOOTER_HEIGHT: f32 = 75.0;
const VERTICAL_PADDING: f32 = 15.5;

#[derive(Debug)]
pub(super) struct Model {
    overlay: Overlay,
    meter: meter::Change,
    change_id: ElementId,
}
impl Model {
    pub(super) fn new(
        state: &State,
        before: Telemetry,
        after: Telemetry,
        change_id: ElementId,
    ) -> Result<Self, meter::InvalidLevel> {
        let overlay = state.overlay();
        let mute = match overlay {
            Overlay::Absent => Mute::Muted,
            Overlay::Switching(mute) | Overlay::Active(mute) => mute,
        };
        Ok(Self {
            overlay,
            change_id,
            meter: meter::Change {
                before: meter_state(before, before.mute)?,
                after: meter_state(after, mute)?,
            },
        })
    }
}
fn meter_state(telemetry: Telemetry, mute: Mute) -> Result<meter::State, meter::InvalidLevel> {
    if mute == Mute::Muted {
        return Ok(meter::State::Muted);
    }
    let level = meter::Level::new(telemetry.level.get())?;
    Ok(match telemetry.activity {
        SpeechActivity::Quiet => meter::State::Listening(level),
        SpeechActivity::Speaking => meter::State::Speaking(level),
    })
}

pub(super) struct Actions<M, S> {
    pub(super) mute: M,
    pub(super) stop: S,
}

/// Callbacks still recheck the live generation/authority at actual dispatch.
/// No missing RPC or physical device is invoked by this staged renderer.
pub(super) fn render<M, S>(
    model: &Model,
    words: &Translator,
    tokens: &Tokens,
    actions: Actions<M, S>,
) -> Option<Div>
where
    M: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    S: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
{
    let (mute, state) = match model.overlay {
        Overlay::Absent => return None,
        Overlay::Switching(mute) => (mute, ButtonState::Disabled),
        Overlay::Active(mute) => (mute, ButtonState::Enabled),
    };
    let muted = mute == Mute::Muted;
    let mute_button = button_with_design(
        "voice-mute",
        ButtonContent::Icon(ButtonIcon::new(if muted {
            Glyph::MicOff
        } else {
            Glyph::Mic
        })),
        CircleConfig::new(
            if muted {
                CircleTone::Danger
            } else {
                CircleTone::Surface
            },
            state,
        )
        .into(),
        tokens,
    )
    .aria_label(
        if muted {
            words.voice().unmute()
        } else {
            words.voice().mute()
        }
        .to_owned(),
    )
    .on_click(actions.mute)
    .into_element()
    .debug_selector(|| "voice-mute".to_owned());
    let config = CircleConfig::new(CircleTone::Danger, state);
    let config = if state == ButtonState::Disabled {
        config.loading()
    } else {
        config
    };
    let stop_button = button_with_design(
        "voice-stop",
        ButtonContent::Icon(ButtonIcon::new(Glyph::SquareFilled)),
        config.into(),
        tokens,
    )
    .aria_label(words.voice().stop().to_owned())
    .on_click(actions.stop)
    .into_element()
    .debug_selector(|| "voice-stop".to_owned());
    Some(
        frame(tokens)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .debug_selector(|| "voice-meter-slot".to_owned())
                    .child(meter::voice_meter(
                        model.change_id.clone(),
                        model.meter,
                        tokens,
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(spacing::S2))
                    .flex_shrink_0()
                    .child(mute_button)
                    .child(stop_button),
            ),
    )
}
fn frame(tokens: &Tokens) -> Div {
    div()
        .w_full()
        .h(px(FOOTER_HEIGHT))
        .flex_shrink_0()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .rounded_2xl()
        .px(px(spacing::S4))
        .py(px(VERTICAL_PADDING))
        .bg(tokens.neutral.surface1)
        .border_1()
        .border_color(tokens.neutral.border)
        .debug_selector(|| "voice-overlay".to_owned())
}

#[cfg(test)]
mod tests;
