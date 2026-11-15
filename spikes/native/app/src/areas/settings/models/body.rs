//! `ProviderModalBody`'s mutually exclusive bodies; all choices come from view.
use super::*;
use vessel_app_kit::{assets::Glyph, glyph, loading};

const ERROR_GLYPH: f32 = 16.0;
pub(super) fn render(
    content: models::Content<'_>,
    refresh: models::Refresh,
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words.models();
    match content {
        models::Content::NoMachine => empty(tokens).child(text(words.no_machine(), tokens)),
        models::Content::Loading => empty(tokens)
            .child(loading::activity_indicator(
                "models-loading",
                loading::Size::Small,
                tokens.neutral.foreground_muted,
            ))
            .child(text(words.loading(), tokens)),
        models::Content::Failed(issue) => empty(tokens)
            .child(glyph(
                Glyph::Alert,
                ERROR_GLYPH,
                tokens.neutral.foreground_muted,
            ))
            .child(text(words.issue(issue), tokens))
            .child(actions::retry(refresh, shell, tokens, compact, cx)),
        models::Content::Unsupported => {
            empty(tokens).child(text(words.problem(models::Problem::Unsupported), tokens))
        }
        models::Content::Empty => empty(tokens).child(text(words.empty(), tokens)),
        models::Content::NoMatches => empty(tokens).child(text(words.no_matches(), tokens)),
        models::Content::Rows {
            snapshot,
            first,
            rest,
        } => rows::card(
            std::iter::once(first).chain(rest),
            snapshot,
            shell,
            tokens,
            compact,
            window,
            cx,
        ),
    }
}
