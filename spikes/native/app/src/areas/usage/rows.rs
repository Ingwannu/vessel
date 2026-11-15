//! Usage facts keep full values inside the shared Settings row frame.
use vessel_app_kit::ui::{
    AnyElement, Div, IntoElement as _, ParentElement as _, SharedString, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, settings, theme::spacing};

/// Row order is explicit at every call site, rather than a positional boolean.
#[derive(Debug, Clone, Copy)]
pub(super) enum Position {
    First,
    Following,
}
/// Facts use a column on touch so large exact token counts remain readable.
#[derive(Debug, Clone, Copy)]
pub(super) enum Density {
    Compact,
    Wide,
}

/// Text-only values share the kit frame; this is an area's composition, not a new control.
pub(super) struct Fact {
    pub(super) label: SharedString,
    pub(super) value: SharedString,
    pub(super) hint: Option<SharedString>,
}
impl Fact {
    pub(super) fn draw(self, position: Position, density: Density, tokens: &Tokens) -> Div {
        let label = div()
            .min_w_0()
            .type_(Type::Body)
            .text_color(tokens.neutral.foreground)
            .child(self.label);
        let value = div()
            .min_w_0()
            .max_w_full()
            .type_(Type::Body)
            .text_color(tokens.neutral.foreground_muted)
            .child(self.value);
        let header = div().min_w_0().w_full().flex().gap(px(spacing::S2));
        let header = match density {
            Density::Compact => header.flex_col().child(label).child(value),
            Density::Wide => header
                .flex_wrap()
                .justify_between()
                .child(label)
                .child(value),
        };
        let content = div()
            .min_w_0()
            .flex_1()
            .flex()
            .flex_col()
            .gap(px(spacing::S1))
            .child(header)
            .children(self.hint.map(|hint| {
                div()
                    .type_(Type::Meta)
                    .text_color(tokens.neutral.foreground_muted)
                    .child(hint)
            }));
        settings::row_with_content(
            content.into_any_element(),
            None,
            matches!(position, Position::First),
            matches!(density, Density::Compact),
            tokens,
        )
    }
}

pub(super) fn message(text: impl Into<SharedString>, tokens: &Tokens) -> Div {
    div()
        .p(px(spacing::S4))
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground_muted)
        .child(text.into())
}

pub(super) fn notice(
    text: impl Into<SharedString>,
    variant: vessel_app_kit::alert::Variant,
    tokens: &Tokens,
) -> AnyElement {
    vessel_app_kit::alert::alert(
        vessel_app_kit::alert::Content {
            title: Some(text.into()),
            description: None,
            action: None,
        },
        variant,
        vessel_app_kit::alert::Size::Small,
        tokens,
    )
    .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Context, IntoElement, Render, TestAppContext, Window};
    use vessel_app_kit::Scheme;
    use vessel_app_kit::ui::InteractiveElement as _;

    struct Probe {
        density: Density,
        width: f32,
    }
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
            let tokens = Tokens::of(Scheme::Dark);
            let fact = Fact { label: "전체 토큰 / Total tokens".into(), value: "18,446,744,073,709,551,615".into(), hint: Some("전체 턴 중 보고된 표본만 포함합니다. Only reported samples are included; missing reports are never estimated.".into()) };
            div()
                .w(px(self.width))
                .child(
                    settings::card(&tokens)
                        .child(
                            fact.draw(Position::First, self.density, &tokens)
                                .debug_selector(|| "usage-tested-fact".to_owned()),
                        )
                        .child(
                            Fact {
                                label: "Observed".into(),
                                value: "Nov 15, 2026 at 10:30 AM +09:00".into(),
                                hint: None,
                            }
                            .draw(
                                Position::Following,
                                self.density,
                                &tokens,
                            ),
                        ),
                )
                .child(message("No accounts", &tokens))
                .child(notice(
                    "Could not read usage",
                    vessel_app_kit::alert::Variant::Error,
                    &tokens,
                ))
        }
    }
    #[gpui::test]
    fn exact_counts_and_long_korean_descriptions_stay_inside_compact_rows(cx: &mut TestAppContext) {
        let (_, cx) = cx.add_window_view(|_, _| Probe {
            density: Density::Compact,
            width: 358.0,
        });
        cx.run_until_parked();
        let bounds = cx.debug_bounds("usage-tested-fact").unwrap();
        assert!(f32::from(bounds.size.height) > spacing::S4);
        assert!(f32::from(bounds.size.width) <= 358.0);
    }
    #[gpui::test]
    fn wide_facts_reuse_the_same_settings_frame(cx: &mut TestAppContext) {
        let (_, cx) = cx.add_window_view(|_, _| Probe {
            density: Density::Wide,
            width: 688.0,
        });
        cx.run_until_parked();
        let bounds = cx.debug_bounds("usage-tested-fact").unwrap();
        assert!(f32::from(bounds.size.width) <= 688.0);
    }
}
