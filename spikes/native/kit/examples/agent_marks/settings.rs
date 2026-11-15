//! The exact `ProviderRow` furniture: shared p16/min56 frame, `Chevron14`,
//! mark16, gap8, title/status/count; no G3 application file is imported.
use gpui::prelude::FluentBuilder as _;
use gpui::{
    Div, InteractiveElement as _, IntoElement as _, ParentElement as _, Styled as _, div, px,
};
use vessel_app_kit::agent_mark::{self, Artwork, SettingsRowEdge, Size};
use vessel_app_kit::{Tokens, Type, TypeExt as _, assets::Glyph, glyph, settings};

pub(super) fn scene(tokens: &Tokens, errors: bool, compact: bool) -> Div {
    let mut card = settings::card(tokens);
    for (index, (name, art)) in inventory().into_iter().enumerate() {
        let edge = if index == 0 {
            SettingsRowEdge::First
        } else {
            SettingsRowEdge::Following
        };
        let indicator = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(6.0))
            .type_(Type::Body)
            .text_color(tokens.neutral.foreground_muted)
            .child(vessel_app_kit::dot(tokens.status().success.foreground).size(px(8.0)))
            .when(!compact, |indicator| {
                indicator.child("Available · 24 models")
            });
        let title = div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(div().min_w_0().truncate().type_(Type::Body).child(name))
            .when(!compact, |title| {
                title.child(
                    div()
                        .type_(Type::Body)
                        .text_color(tokens.neutral.foreground_muted)
                        .child("·"),
                )
            })
            .child(indicator);
        let mut column = div().flex_1().min_w_0().flex().flex_col().child(title);
        if errors && !compact && art == Some(agent_mark::OPENCODE) {
            column = column.child(
                div()
                    .mt(px(4.0))
                    .type_(Type::Meta)
                    .text_color(tokens.status().red_300)
                    .line_clamp(3)
                    .child("The agent could not be checked. 에이전트 상태를 확인할 수 없습니다."),
            );
        }
        let content = div()
            .flex_1()
            .min_w_0()
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(glyph(
                Glyph::ChevronRight,
                14.0,
                tokens.neutral.foreground_muted,
            ))
            .child(agent_mark::settings_mark(art, Size::Sixteen, edge, tokens))
            .child(column);
        let hover = tokens.neutral.surface2;
        card = card.child(
            div()
                .id(format!("fit-row-{index}"))
                .hover(move |style| style.bg(hover))
                .child(
                    settings::row_with_content(
                        content.into_any_element(),
                        None,
                        index == 0,
                        compact,
                        tokens,
                    )
                    .min_h(px(56.0)),
                ),
        );
    }
    div()
        .size_full()
        .bg(tokens.neutral.surface0)
        .text_color(tokens.neutral.foreground)
        .type_(Type::Body)
        .child(
            div()
                .w_full()
                .max_w(px(720.0))
                .p(px(16.0))
                .child(card)
                .child(
                    div()
                        .mt(px(12.0))
                        .child("Shared settings frame · first / following / last · 56px rows"),
                ),
        )
}
fn inventory() -> [(&'static str, Option<Artwork>); 6] {
    [
        ("OpenCode", Some(agent_mark::OPENCODE)),
        (
            "Claude Code · authentication lifecycle recovery across reconnects",
            Some(agent_mark::CLAUDE),
        ),
        (
            "Codex · 저장하지 않은 긴 한글 편집 내용 복구",
            Some(agent_mark::CODEX),
        ),
        ("Gemini CLI", Some(agent_mark::GEMINI)),
        (
            "사용자 명령 · a custom agent with a long bilingual name",
            None,
        ),
        ("OpenCode", Some(agent_mark::OPENCODE)),
    ]
}
