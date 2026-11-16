//! Settings opt-in and attachment options for the same tooltip controller.
//! No second timer, shown store, surface renderer or focus owner lives here.
use super::{Configuration, Content, Place, Tip, Tooltips, pending};
use crate::picker::anchor_probe;
use gpui::prelude::*;
use gpui::{Div, MouseButton, SharedString, Stateful};
use std::rc::Rc;

/// Explicit Settings explanation behavior, preserving legacy touch suppression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfoInteraction {
    /// Pointer hover opens; pressing closes.
    Hover,
    /// Press opens; an outside press dismisses this explanation only.
    Press,
}
#[derive(Debug, Clone, Copy)]
pub(super) enum Alignment {
    Center,
    Start,
}
#[derive(Debug, Clone, Copy)]
enum Probe {
    Trigger,
    Retained,
}
#[derive(Debug, Clone, Copy)]
pub(super) struct Options {
    pub(super) alignment: Alignment,
    pub(super) interaction: InfoInteraction,
    probe: Probe,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            alignment: Alignment::Center,
            interaction: InfoInteraction::Hover,
            probe: Probe::Trigger,
        }
    }
}
impl Options {
    pub(super) fn placement(
        self,
        configuration: Configuration,
        bounds: gpui::Bounds<gpui::Pixels>,
    ) -> (
        gpui::Point<gpui::Pixels>,
        gpui::Anchor,
        gpui::Point<gpui::Pixels>,
    ) {
        match self.alignment {
            Alignment::Center => configuration.placement(bounds),
            Alignment::Start => configuration.placement_start(bounds),
        }
    }
}
impl Tooltips {
    /// Clear both the shown explanation and the existing pending hover task.
    pub fn clear(&self) {
        self.state.borrow_mut().dismiss();
    }
    /// Settings' top-start8, zero-delay opt-in, including compact press opening.
    #[must_use]
    pub fn on_info(
        &self,
        trigger: Stateful<Div>,
        place: &Place,
        text: impl Into<SharedString>,
        interaction: InfoInteraction,
    ) -> Stateful<Div> {
        self.attach_with(
            trigger,
            place,
            Content::Text(text.into()),
            Configuration::default(),
            Options {
                alignment: Alignment::Start,
                interaction,
                probe: Probe::Trigger,
            },
        )
    }
    /// Use the measured visual bounds, rather than probing its larger hit area.
    pub(crate) fn info_at(
        &self,
        trigger: Stateful<Div>,
        place: &Place,
        text: SharedString,
        interaction: InfoInteraction,
    ) -> Stateful<Div> {
        self.attach_with(
            trigger,
            place,
            Content::Text(text),
            Configuration::default(),
            Options {
                alignment: Alignment::Start,
                interaction,
                probe: Probe::Retained,
            },
        )
    }
    pub(super) fn attach_with(
        &self,
        trigger: Stateful<Div>,
        place: &Place,
        content: Content,
        configuration: Configuration,
        options: Options,
    ) -> Stateful<Div> {
        let lifetime = Rc::new(());
        self.state
            .borrow_mut()
            .refresh_trigger(place, &lifetime, &content, configuration, options);
        let click_content = content.clone();
        let click_place = Rc::clone(place);
        let click_lifetime = Rc::clone(&lifetime);
        let click_state = Rc::clone(&self.state);
        let hover_state = Rc::clone(&self.state);
        let hover_place = Rc::clone(place);
        let press_state = Rc::clone(&self.state);
        let trigger = trigger
            .when(matches!(options.probe, Probe::Trigger), |trigger| {
                trigger.child(anchor_probe(place))
            })
            .on_hover(move |hovered, window, cx| {
                if *hovered {
                    pending::entered(
                        &hover_state,
                        captured_tip(
                            content.clone(),
                            &hover_place,
                            configuration,
                            options,
                            &lifetime,
                        ),
                        window,
                        cx,
                    );
                } else {
                    hover_state.borrow_mut().leave(&hover_place);
                    window.refresh();
                }
            })
            .on_mouse_down(MouseButton::Left, move |_, window, _| {
                press_state.borrow_mut().dismiss();
                window.refresh();
            });
        match options.interaction {
            InfoInteraction::Hover => trigger,
            InfoInteraction::Press => trigger.on_click(move |_, window, cx| {
                pending::entered(
                    &click_state,
                    captured_tip(
                        click_content.clone(),
                        &click_place,
                        configuration,
                        options,
                        &click_lifetime,
                    ),
                    window,
                    cx,
                );
            }),
        }
    }

    pub(super) fn dismiss_on_outside(&self, card: Stateful<Div>, tip: &Tip) -> Stateful<Div> {
        if tip.options.interaction == InfoInteraction::Hover {
            return card;
        }
        let state = Rc::clone(&self.state);
        let place = Rc::clone(&tip.place);
        let trigger = tip.trigger.clone();
        card.on_mouse_down_out(move |_, window, _| {
            let mut state = state.borrow_mut();
            if state.shown.as_ref().is_some_and(|current| {
                Rc::ptr_eq(&current.place, &place)
                    && std::rc::Weak::ptr_eq(&current.trigger, &trigger)
            }) {
                state.leave(&place);
                window.refresh();
            }
        })
    }
}

fn captured_tip(
    content: Content,
    place: &Place,
    configuration: Configuration,
    options: Options,
    lifetime: &Rc<()>,
) -> Tip {
    Tip {
        content,
        place: Rc::clone(place),
        configuration,
        options,
        trigger: Rc::downgrade(lifetime),
    }
}
