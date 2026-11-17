//! The import sheet's provider filter: "All" (a 14 glyph) and each provider
//! (a 14 muted mark, known or the bot) keep one 14 column, as Paseo's list.
use super::rows::leading_column;
use super::{ComboRow, Leading};
use crate::assets::Glyph;

#[test]
fn small_marks_and_small_glyphs_share_one_14_column() {
    let row = |label: &str, leading: Leading| {
        let mut row = ComboRow::new(label.to_owned());
        row.leading = Some(leading);
        row
    };
    let rows = [
        row("All", Leading::SmallGlyph(Glyph::Layers)),
        row(
            "Codex",
            Leading::SmallAgentMark(Some(crate::agent_mark::CODEX)),
        ),
        row(
            "OpenCode",
            Leading::SmallAgentMark(Some(crate::agent_mark::OPENCODE)),
        ),
        row("added", Leading::SmallAgentMark(None)),
    ];
    assert_eq!(
        leading_column(&rows).map(f32::to_bits),
        Some(14.0_f32.to_bits())
    );
}
