//! Named sidebar footer wrapper over the ONE shared icon chrome.
//! The owner retains effect admission; GPUI owns focus and keyboard activation.
#[cfg(test)]
mod tests;
use crate::tooltip::{Configuration, DisplayChords, KeyCaption, Place, Side};
use crate::{ControlSizing, Tokens, Tooltips, assets::Glyph, icon_chrome};
use gpui::prelude::*;
use gpui::{App, ClickEvent, Div, ElementId, FocusHandle, SharedString, Stateful, Window};

/// Source square28/44 footer action; no area size/color override is exposed.
#[must_use]
pub struct FooterIconButton {
    element: Stateful<Div>,
    label: SharedString,
    sizing: ControlSizing,
}
impl FooterIconButton {
    /// Keep the accessible name and canonical glyph on the same square target.
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        glyph: Glyph,
        sizing: ControlSizing,
        tokens: &Tokens,
    ) -> Self {
        let label = label.into();
        let element = icon_chrome::render(
            id,
            label.clone(),
            icon_chrome::Profile::SidebarFooter { glyph, sizing },
            tokens,
        )
        .focusable()
        .tab_stop(true);
        Self {
            element,
            label,
            sizing,
        }
    }
    /// Native pointer/Enter/Space converge on this one callback. The owner
    /// still validates its original captured target before any effect.
    pub fn on_click(
        mut self,
        listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.element = self.element.on_click(listener);
        self
    }
    /// Reuse an owner's existing focus/reveal checkpoint, without another
    /// focus owner or key listener. Otherwise GPUI retains it by element id.
    pub fn track_focus(mut self, focus: &FocusHandle) -> Self {
        self.element = self.element.track_focus(focus);
        self
    }
    /// Source top/center8 after300ms; compact tooltips remain suppressed.
    pub fn tooltip(self, tips: &Tooltips, place: &Place) -> Self {
        self.tooltip_with_keys(tips, place, DisplayChords::default())
    }
    /// Effective caller-prepared display chords, with no copied shortcut
    /// defaults or registry. Empty/unavailable keys leave no phantom gap.
    pub fn tooltip_with_keys(
        mut self,
        tips: &Tooltips,
        place: &Place,
        keys: DisplayChords,
    ) -> Self {
        self.element = tips.on_caption(
            self.element,
            place,
            KeyCaption::new(self.label.clone(), keys),
            Configuration::toolbar().side(Side::Top),
            self.sizing,
        );
        self
    }
}
impl IntoElement for FooterIconButton {
    type Element = <Stateful<Div> as IntoElement>::Element;
    fn into_element(self) -> Self::Element {
        self.element.into_element()
    }
}
impl std::fmt::Debug for FooterIconButton {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FooterIconButton")
            .field("sizing", &self.sizing)
            .finish_non_exhaustive()
    }
}
