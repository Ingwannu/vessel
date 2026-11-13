//! An agent's features in the composer, as Paseo's
//! (`composer/agent-controls/index.tsx`: `DesktopFeatureItem`,
//! `SheetFeatureItem`, the features control and its `AdaptiveModalSheet`):
//! with room, a control per feature (an on/off feature a square that
//! switches it, a choice a control with its value that opens its list);
//! with less, one square that opens the features sheet (a card on a wide
//! window, a sheet from the bottom on a phone), a row per feature with its
//! name and value, each opening its list beside it.
//!
//! [Decision Log]
//! - Purpose: 0.0.0.0.6 always folded the features into one list; Paseo
//!   shows them one by one when the composer is wide (the owner's rule:
//!   Paseo's composer).
//! - Choice: one list for every feature (the draft's `features`
//!   combobox), opened at the trigger that asked for it: each feature's
//!   control and each row of the sheet records where it is drawn, and the
//!   list takes that place when it opens. From a control the list has
//!   Paseo's search row and no title; from the sheet it has the feature's
//!   name above and no search (Paseo's `header`).
//! - Differences: every control carries its feature's name (an on/off
//!   feature its name, a choice its name quieter, then its value): Paseo
//!   tells features apart by their own icons and their colour when on,
//!   and ACP sends no icon, so each control here would be the same glyph
//!   reading "Off" (the coordinator's capture of 0.0.0.0.7's first
//!   build). An on/off feature that is on takes Paseo's colour for the
//!   features it knows (fast mode, auto-accept, plan) and the foreground's
//!   for others. Paseo's tooltip is on every control ("name: value" for a
//!   choice, the feature's description or name for an on/off one).

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::dialog::{DialogContent, DialogSize, DialogState, dialog};
use vessel_app_kit::ui::{
    AnyElement, Bounds, Context, Div, InteractiveElement as _, IntoElement as _,
    ParentElement as _, Pixels, StatefulInteractiveElement as _, Styled as _, Window, canvas, div,
    px,
};
use vessel_app_kit::{PillParts, Tokens, Type, TypeExt as _, anchor_probe, glyph, pill_parts};

use vessel_app_view::Controls;
use vessel_proto::OptionId;

use super::density::{self, Density, Geometry, Presence};
use super::pickers::{Picker, list_of};
use crate::shell::Shell;

/// Where a trigger was last drawn.
type Anchor = Rc<Cell<Option<Bounds<Pixels>>>>;

/// The features' state, kept by the draft.
pub(crate) struct FeatureState {
    /// The features sheet is open.
    pub(crate) sheet: bool,
    dialog: DialogState,
    /// Where each feature's control or row was last drawn.
    anchors: RefCell<BTreeMap<OptionId, Anchor>>,
    /// The width the controls had last frame.
    toolbar: Rc<Cell<Pixels>>,
    /// The step of Paseo's ladder the controls are at.
    density: Cell<Density>,
    /// The composer's tooltips (one shown at most).
    pub(crate) tips: vessel_app_kit::Tooltips,
    /// Where the model, thinking and mode controls are drawn, for their
    /// tooltips.
    pub(crate) places: [Anchor; 3],
}

impl FeatureState {
    pub(crate) fn new(cx: &mut Context<'_, Shell>) -> Self {
        Self {
            sheet: false,
            dialog: DialogState::new(cx),
            anchors: RefCell::new(BTreeMap::new()),
            toolbar: Rc::new(Cell::new(px(0.0))),
            density: Cell::new(Density::Full),
            tips: vessel_app_kit::Tooltips::new(),
            places: [
                Rc::new(Cell::new(None)),
                Rc::new(Cell::new(None)),
                Rc::new(Cell::new(None)),
            ],
        }
    }

    fn anchor(&self, id: &OptionId) -> Anchor {
        Rc::clone(
            self.anchors
                .borrow_mut()
                .entry(id.clone())
                .or_insert_with(|| Rc::new(Cell::new(None))),
        )
    }

    /// The step for `controls` at the width measured last frame (Full
    /// until the controls have been drawn once).
    pub(crate) fn density(
        &self,
        controls: &Controls,
        model: f32,
        touch: bool,
        window: &Window,
    ) -> Density {
        let width = f32::from(self.toolbar.get());
        if width <= 0.0 {
            return self.density.get();
        }
        let density = density::resolve(
            width,
            self.density.get(),
            &presence(controls, model, window),
            Geometry::of(touch),
        );
        self.density.set(density);
        density
    }

    /// Records the controls' width every frame, and draws again when it
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

/// An on/off feature's colour when on: Paseo's for the features it knows
/// (`getFeatureHighlightColor`: fast mode and speed yellow, auto-accept
/// green, plan blue, each its palette's 400), the foreground's for any
/// other (Paseo leaves those muted; with no value shown, a named control
/// has to say it is on).
fn active_colour(id: &OptionId, otherwise: vessel_app_kit::ui::Hsla) -> vessel_app_kit::ui::Hsla {
    match id.as_str() {
        "fast_mode" | "fast" | "speed" => vessel_app_kit::ui::rgb(0x00fa_cc15).into(),
        "auto_accept" => vessel_app_kit::ui::rgb(0x004a_de80).into(),
        "plan_mode" | "plan" => vessel_app_kit::ui::rgb(0x0060_a5fa).into(),
        _ => otherwise,
    }
}

/// What the controls have to show, their words measured as drawn;
/// `model` the model control's words' width.
fn presence(controls: &Controls, model: f32, window: &Window) -> Presence {
    let width = |text: &str| vessel_app_kit::fonts::measure(window, text, Type::Body);
    let mut features: Vec<density::Feature> = controls
        .toggles
        .iter()
        .map(|toggle| density::Feature::Toggle(width(feature_name(&toggle.name, &toggle.id))))
        .collect();
    features.extend(
        controls
            .selects
            .iter()
            .map(|select| density::Feature::Select {
                name: width(feature_name(&select.name, &select.id)),
                value: width(select.current_name()),
            }),
    );
    Presence {
        model: Some(model),
        thinking: controls
            .effort
            .as_ref()
            .map(|effort| width(effort.current_name())),
        mode: controls
            .permissions
            .as_ref()
            .map(|mode| width(mode.current_name())),
        features,
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

/// The features' controls in the composer at `density`.
pub(crate) fn controls(
    shell: &Shell,
    controls: &Controls,
    density: Density,
    touch: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Vec<AnyElement> {
    let state = &shell.areas.draft.features;
    let list_open = shell.areas.draft.lists.features.is_open();
    let showing = shell.areas.draft.feature.as_ref();
    if controls.toggles.is_empty() && controls.selects.is_empty() {
        return Vec::new();
    }
    if density.aggregate_features() {
        // Paseo's features control: the default glyph alone.
        let open = pill_parts(
            "pick-features",
            PillParts {
                leading: Some(Glyph::Sliders),
                ..PillParts::default()
            },
            state.sheet,
            touch,
            tokens,
        )
        .on_click(cx.listener(|this, _event, window, cx| this.open_features_sheet(window, cx)));
        return vec![open.into_any_element()];
    }
    // Each control carries its feature's name (Vessel's, a named
    // difference: Paseo tells its features apart by their own icons and
    // their colour when on, and ACP sends no icon, so every control here
    // would be the same glyph and an anonymous "Off"). Paseo's tooltip on
    // each: a feature's own words for an on/off one, "name: value" for a
    // choice.
    let neutral = tokens.neutral;
    let mut pills = Vec::new();
    for toggle in &controls.toggles {
        // On/off: its name, coloured when on, one click switches it.
        let choice = toggle.choose(!toggle.on);
        let name = feature_name(&toggle.name, &toggle.id).to_owned();
        let tip = toggle.description.clone().unwrap_or_else(|| name.clone());
        let place = state.anchor(&toggle.id);
        let pill = pill_parts(
            vessel_app_kit::ui::ElementId::Name(format!("feature-{}", toggle.id).into()),
            PillParts {
                leading: Some(Glyph::Sliders),
                label: Some(name.into()),
                tint: toggle
                    .on
                    .then(|| active_colour(&toggle.id, neutral.foreground)),
                ..PillParts::default()
            },
            false,
            touch,
            tokens,
        )
        .relative()
        .on_click(cx.listener(move |this, _event, _window, cx| {
            this.choose_option(choice.clone());
            cx.notify();
        }));
        pills.push(state.tips.on(pill, &place, tip, touch).into_any_element());
    }
    for select in &controls.selects {
        let id = select.id.clone();
        let anchor = state.anchor(&id);
        let open = list_open && showing == Some(&id);
        let name = feature_name(&select.name, &select.id).to_owned();
        let value = select.current_name().to_owned();
        let tip = format!("{name}: {value}");
        let pill = pill_parts(
            vessel_app_kit::ui::ElementId::Name(format!("feature-{id}").into()),
            PillParts {
                leading: Some(Glyph::Sliders),
                prefix: Some(name.into()),
                label: Some(value.into()),
                ..PillParts::default()
            },
            open,
            touch,
            tokens,
        )
        .relative()
        .on_click(cx.listener(move |this, _event, window, cx| {
            this.open_feature(id.clone(), window, cx);
        }));
        pills.push(state.tips.on(pill, &anchor, tip, touch).into_any_element());
    }
    pills
}

/// The features sheet, when it is open: a row per feature.
pub(crate) fn sheet(
    shell: &Shell,
    shown: Option<&Controls>,
    tokens: &Tokens,
    window: &Window,
    narrow: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.draft.features;
    let controls = shown?;
    if !state.sheet {
        return None;
    }
    let words = shell.t.lang.options();
    let list_open = shell.areas.draft.lists.features.is_open();
    let showing = shell.areas.draft.feature.as_ref();
    let mut rows = div().flex().flex_col().gap(px(16.0));
    let toggles = controls.toggles.iter().map(|toggle| {
        (
            toggle.id.clone(),
            feature_name(&toggle.name, &toggle.id).to_owned(),
            if toggle.on {
                words.setting_on
            } else {
                words.setting_off
            }
            .to_owned(),
        )
    });
    let selects = controls.selects.iter().map(|select| {
        (
            select.id.clone(),
            feature_name(&select.name, &select.id).to_owned(),
            select.current_name().to_owned(),
        )
    });
    for (id, name, value) in toggles.chain(selects) {
        let open = list_open && showing == Some(&id);
        rows = rows.child(sheet_row(
            &state.anchor(&id),
            id,
            name,
            value,
            open,
            tokens,
            cx,
        ));
    }
    Some(dialog(
        &state.dialog,
        DialogContent {
            title: words.features.into(),
            body: rows.into_any_element(),
            footer: Vec::new(),
            size: DialogSize::Normal,
        },
        tokens,
        window,
        narrow,
        |shell: &mut Shell, _window, _cx| shell.close_features_sheet(),
        cx,
    ))
}

/// A row of the sheet (Paseo's `AgentControlTrigger` on its `sheet`
/// surface): 44 high, 16 inside, round 16, on `surface1` (`surface2` on
/// hover and while its list is open); the glyph in a box of 20, the name,
/// the value muted and at most 45% of the row, each one line.
fn sheet_row(
    anchor: &Anchor,
    id: OptionId,
    name: String,
    value: String,
    open: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let neutral = tokens.neutral;
    div().child(
        div()
            .id(vessel_app_kit::ui::ElementId::Name(
                format!("feature-row-{id}").into(),
            ))
            .relative()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(8.0))
            .min_h(px(44.0))
            .mx(px(-4.0))
            .px(px(16.0))
            .rounded(px(16.0))
            .bg(if open {
                neutral.surface2
            } else {
                neutral.surface1
            })
            .hover(move |style| style.bg(neutral.surface2))
            .cursor_pointer()
            .type_(Type::Body)
            .child(
                div()
                    .size(px(20.0))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(glyph(Glyph::Sliders, 16.0, neutral.foreground_muted)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_color(neutral.foreground)
                    .child(name),
            )
            .child(
                div()
                    .flex_shrink(1.0)
                    .min_w_0()
                    .max_w(vessel_app_kit::ui::relative(0.45))
                    .truncate()
                    .text_color(neutral.foreground_muted)
                    .child(value),
            )
            .child(anchor_probe(anchor))
            .on_click(cx.listener(move |this, _event, window, cx| {
                this.open_feature(id.clone(), window, cx);
            })),
    )
}

impl Shell {
    /// Open the features sheet.
    fn open_features_sheet(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        for which in super::pickers::ALL {
            list_of(self, which).close();
        }
        let features = &mut self.areas.draft.features;
        features.sheet = true;
        features.dialog.focus(window, cx);
        cx.notify();
    }

    fn close_features_sheet(&mut self) {
        self.areas.draft.features.sheet = false;
        list_of(self, Picker::Features).close();
    }

    /// Open `id`'s list at its control or its row (closing it when it is
    /// the one open).
    fn open_feature(&mut self, id: OptionId, window: &mut Window, cx: &mut Context<'_, Self>) {
        let again = self.areas.draft.lists.features.is_open()
            && self.areas.draft.feature.as_ref() == Some(&id);
        for which in super::pickers::ALL {
            list_of(self, which).close();
        }
        if again {
            cx.notify();
            return;
        }
        let draft = &mut self.areas.draft;
        let anchor = draft.features.anchor(&id).get();
        draft.lists.features.set_anchor(anchor);
        // From a control, Paseo's search row; from the sheet, the name
        // above and no search.
        let search = (!draft.features.sheet).then_some(self.t.lang.options().search);
        draft.feature = Some(id);
        list_of(self, Picker::Features).open(search, window, cx);
        cx.notify();
    }
}
