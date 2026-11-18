//! A one-line field drawn inside a menu row's box (`menu::form_rows`,
//! Paseo's `MenuTextField`): the field supplies only its line; the box is
//! the menu's, so the text sits on the rows' rail.
use gpui::prelude::*;
use gpui::{AnyElement, App, div, px};
use gpui_base::input::Input;

use super::styles::project;
use super::{Kind, TextField};
use crate::fonts::{self, Type};
use crate::theme::Tokens;
use crate::widgets::TypeExt as _;

impl TextField {
    /// Its line in a menu row's box: the row's type on the row's `line`
    /// height, muted while empty. A field of several lines draws nothing
    /// here (a menu has no use for one).
    pub(crate) fn menu_line(&self, tokens: &Tokens, line: f32, cx: &mut App) -> AnyElement {
        match &self.kind {
            Kind::Line(state) => {
                project(state, tokens, tokens.neutral.foreground_muted, cx);
                let text = div()
                    .flex_1()
                    .min_w(px(0.0))
                    .type_(Type::Body)
                    .line_height(px(fonts::ui(line)))
                    .text_color(tokens.neutral.foreground);
                self.form_text
                    .apply(text)
                    .child(Input::new(state))
                    .into_any_element()
            }
            Kind::Lines(_, _) => div().into_any_element(),
        }
    }

    /// Whether it holds the keys now: its box's border turns the accent.
    pub(crate) fn holds_keys(&self) -> bool {
        self.focused.get()
    }
}
