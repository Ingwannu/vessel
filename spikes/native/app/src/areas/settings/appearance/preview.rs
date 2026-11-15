//! Prepare the fixed source example once with the existing workspace lexer.
//! Local draft sizes/families resolve outside paint, never touching globals.
use super::State;
use gpui::App;
use vessel_app_kit::{
    code_preview::{PreviewError, PreviewRow, PreviewTypography},
    fonts::{FontSize, Typeface},
    output::Tone,
    syntax::CodeRun,
};
use vessel_app_view::{
    prefs::Appearance,
    workspace::syntax::{LineState, language_of, tokenize},
};

const SOURCE: [(Tone, &str); 7] = [
    (Tone::Plain, "// Format a price for display"),
    (
        Tone::Removed,
        "export function formatPrice(cents: number) {",
    ),
    (
        Tone::Added,
        "export function formatPrice(cents: number): string {",
    ),
    (Tone::Plain, "  const amount = cents / 100;"),
    (Tone::Removed, "  return \"$\" + amount;"),
    (Tone::Added, "  return `$${amount.toFixed(2)}`;"),
    (Tone::Plain, "}"),
];
pub(super) fn rows() -> Result<Vec<PreviewRow>, PreviewError> {
    let language = language_of("preview.ts");
    SOURCE
        .into_iter()
        .enumerate()
        .map(|(index, (tone, text))| {
            let (tokens, _) = tokenize(text, language, LineState::Code);
            let runs = tokens
                .into_iter()
                .filter_map(|token| {
                    token.kind.map(|kind| CodeRun {
                        range: token.range,
                        style: crate::syntax_kind::of(kind),
                    })
                })
                .collect();
            PreviewRow::new(
                gpui::ElementId::Name(format!("appearance-preview-{index}").into()),
                text,
                tone,
                runs,
            )
        })
        .collect()
}
impl State {
    pub(super) fn refresh_preview(&mut self, saved: &Appearance, cx: &mut App) {
        let values = self.drafts.values(saved);
        let (Some(content_size), Some(code_size)) = (
            FontSize::new(values.content.get()),
            FontSize::new(values.code.get()),
        ) else {
            tracing::error!("validated preview has a zero text size");
            return;
        };
        let available = cx.text_system().all_font_names();
        self.preview = Some(PreviewTypography {
            content: super::super::typography::stack(
                &saved.interface_family,
                Typeface::Interface,
                &available,
            ),
            code: super::super::typography::stack(values.mono, Typeface::Code, &available),
            content_size,
            code_size,
        });
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_example_uses_existing_classifier_and_valid_ranges() {
        let rows = rows().unwrap();
        assert_eq!(rows.len(), SOURCE.len());
    }
}
