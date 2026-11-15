//! A permissions mode's mark in the kit's glyphs (Paseo's `MODE_ICONS`):
//! the rule is `vessel_app_view::composer::modes`'; this only names the
//! glyph each mark is drawn with.

use vessel_app_kit::assets::Glyph;
use vessel_app_view::composer::modes::{ModeMark, PillMark, mark_of, pill_mark_of};
use vessel_proto::{HarnessKind, OptionValueId};

const fn glyph_of(mark: ModeMark) -> Glyph {
    match mark {
        ModeMark::Shield => Glyph::Shield,
        ModeMark::ShieldPlus => Glyph::ShieldPlus,
        ModeMark::ShieldCheck => Glyph::ShieldCheck,
        ModeMark::ShieldOff => Glyph::ShieldOff,
        ModeMark::ShieldEllipsis => Glyph::ShieldEllipsis,
        ModeMark::ShieldAlert => Glyph::ShieldAlert,
    }
}

/// A mode row's mark, or none (Paseo's `getAgentModeOptionIcon`).
pub(crate) fn row_glyph(kind: Option<HarnessKind>, mode: &OptionValueId) -> Option<Glyph> {
    mark_of(kind, mode).map(glyph_of)
}

/// The closed permissions control's mark (Paseo's `getAgentModeIcon`: the
/// agent's `Bot` for a mode with none).
pub(crate) fn pill_glyph(kind: Option<HarnessKind>, mode: &OptionValueId) -> Glyph {
    match pill_mark_of(kind, mode) {
        PillMark::Mode(mark) => glyph_of(mark),
        PillMark::Agent => Glyph::Bot,
    }
}
