//! What the composer's row measures and points at, and an agent's other
//! options: since the owner's effort control (2026-11-13), the row is the
//! model-and-effort control and the mode control; every option the effort
//! control does not drive (Claude Code's agent, Codex's collaboration
//! mode, anything an agent sends) is a row at the top of the model's list
//! (`pickers::model_rows`), an on/off option a switch there, a choice its
//! value, opening its own list from there.
//!
//! [Decision Log]
//! - Purpose: 0.0.0.0.7 drew a control per option with its name; the owner
//!   asked for one control for the effort, fast mode and the agent's top
//!   step, and did not know what "Agent" was: the rest belongs with the
//!   model (the reference has its one switch at the top of its model
//!   list).
//! - Choice: the row's width measured each frame for Paseo's ladder
//!   (`density.rs`); Paseo's tooltip on the row's controls; one list for
//!   an option's values (the draft's `features` combobox), opened at the
//!   model control with the option's name above it.
//! - The features sheet (Paseo's `AdaptiveModalSheet`) and the named
//!   controls are gone with the row they belonged to.

use std::cell::Cell;
use std::rc::Rc;

use vessel_app_kit::ui::{
    AnyElement, Bounds, Context, IntoElement as _, Pixels, Styled as _, Window, canvas, px,
};
use vessel_app_kit::{Type, fonts};
use vessel_app_view::Controls;
use vessel_proto::OptionId;

use super::density::{self, Density, Geometry, Presence};
use super::pickers::{Picker, list_of};
use crate::shell::Shell;

/// Where a trigger was last drawn.
type Anchor = Rc<Cell<Option<Bounds<Pixels>>>>;

/// The row's state, kept by the draft.
pub(crate) struct FeatureState {
    /// The width the controls had last frame.
    toolbar: Rc<Cell<Pixels>>,
    /// The step of Paseo's ladder the controls are at.
    density: Cell<Density>,
    /// The composer's tooltips (one shown at most).
    pub(crate) tips: vessel_app_kit::Tooltips,
    /// Where the model-and-effort and the mode controls are drawn, for
    /// their tooltips and what opens from them.
    pub(crate) places: [Anchor; 2],
}

/// The words the model control shows, as the ladder needs them.
pub(crate) struct ModelWords<'a> {
    pub(crate) model: &'a str,
    pub(crate) agent: Option<&'a str>,
    pub(crate) effort: Option<&'a str>,
}

impl FeatureState {
    pub(crate) fn new() -> Self {
        Self {
            toolbar: Rc::new(Cell::new(px(0.0))),
            density: Cell::new(Density::Full),
            tips: vessel_app_kit::Tooltips::new(),
            places: [Rc::new(Cell::new(None)), Rc::new(Cell::new(None))],
        }
    }

    /// The step for the row's words at the width measured last frame (Full
    /// until the row has been drawn once).
    pub(crate) fn density(
        &self,
        controls: &Controls,
        words: &ModelWords<'_>,
        touch: bool,
        window: &Window,
    ) -> Density {
        let width = f32::from(self.toolbar.get());
        if width <= 0.0 {
            return self.density.get();
        }
        let measure = |text: &str| fonts::measure(window, text, Type::Body);
        let presence = Presence {
            model: Some(measure(words.model)),
            agent: words.agent.map(measure),
            effort: words.effort.map(measure),
            mode: controls
                .permissions
                .as_ref()
                .map(|mode| measure(mode.current_name())),
        };
        let density = density::resolve(width, self.density.get(), &presence, Geometry::of(touch));
        self.density.set(density);
        density
    }

    /// Records the row's width every frame, and draws again when it
    /// changed (the step may change with it).
    pub(crate) fn measure(&self) -> AnyElement {
        let cell = Rc::clone(&self.toolbar);
        canvas(
            move |bounds, window, _cx| {
                if cell.get() != bounds.size.width {
                    cell.set(bounds.size.width);
                    window.refresh();
                }
            },
            |_, (), _, _| {},
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .into_any_element()
    }
}

/// A feature's name, its id when the agent gave it none.
pub(crate) fn feature_name<'a>(name: &'a str, id: &'a OptionId) -> &'a str {
    if name.trim().is_empty() {
        id.as_str()
    } else {
        name
    }
}

impl Shell {
    /// Open `id`'s list at the model control (from its row at the top of
    /// the model's list), its name above it.
    pub(crate) fn open_feature(
        &mut self,
        id: OptionId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        for which in super::pickers::ALL {
            list_of(self, which).close();
        }
        let draft = &mut self.areas.draft;
        let anchor = draft.features.places[0].get();
        draft.lists.features.set_anchor(anchor);
        draft.feature = Some(id);
        list_of(self, Picker::Features).open(None, window, cx);
        cx.notify();
    }
}
