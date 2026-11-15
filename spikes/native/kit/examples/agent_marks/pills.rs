//! Actual `pill_parts` snapshots, including the icon-only touch target.
use gpui::{Div, ParentElement as _, Styled as _, div, px};
use vessel_app_kit::agent_mark::{self, Artwork};
use vessel_app_kit::{PillParts, Tokens, Type, TypeExt as _, assets::Glyph, pill_parts};

pub(super) fn scene(tokens: &Tokens) -> Div {
    let mut body = div()
        .size_full()
        .p(px(16.0))
        .flex()
        .flex_col()
        .gap(px(8.0))
        .bg(tokens.neutral.surface0)
        .text_color(tokens.neutral.foreground)
        .type_(Type::Body)
        .child("Actual pill_parts · labelled / icon-only · 실제 모델 필");
    for (name, art) in [
        ("Claude Code", Some(agent_mark::CLAUDE)),
        ("OpenCode", Some(agent_mark::OPENCODE)),
        ("Gemini CLI", Some(agent_mark::GEMINI)),
        ("Generic / 기존 Bot", None),
    ] {
        let mut section = div().flex().flex_col().gap(px(4.0)).child(name);
        for touch in [false, true] {
            let label = if touch {
                "Touch 44 · 긴 모델 이름과 복구 흐름"
            } else {
                "Pointer 28 · authentication recovery model"
            };
            section = section.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(pill(name, art, Some(label), touch, tokens).w(px(252.0)))
                    .child(pill(name, art, None, touch, tokens)),
            );
        }
        body = body.child(section);
    }
    body.child(
        div()
            .flex()
            .items_center()
            .gap(px(12.0))
            .child("Row fallback / Folder / Server")
            .child(agent_mark::agent_mark(
                None,
                agent_mark::Size::Fourteen,
                agent_mark::Space::Row,
                tokens,
            ))
            .child(vessel_app_kit::glyph(
                Glyph::Folder,
                16.0,
                tokens.neutral.foreground_muted,
            ))
            .child(vessel_app_kit::glyph(
                Glyph::Server,
                16.0,
                tokens.neutral.foreground_muted,
            )),
    )
}
fn pill(
    name: &'static str,
    art: Option<Artwork>,
    label: Option<&'static str>,
    touch: bool,
    tokens: &Tokens,
) -> gpui::Stateful<Div> {
    pill_parts(
        format!("{name}-{touch}-{}", label.is_some()),
        PillParts {
            leading: Some(Glyph::Bot),
            agent_mark: art,
            label: label.map(Into::into),
            caret: label.is_some(),
            ..PillParts::default()
        },
        false,
        touch,
        tokens,
    )
}
