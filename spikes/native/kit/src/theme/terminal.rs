//! The terminal's palette (ADR 0073): Paseo's, from
//! `packages/app/src/styles/theme.ts` (`lightTerminalAnsi` 244–259 and
//! the light theme's `terminal` 309–319 with its tint 323–345;
//! `darkTerminalAnsi` 373–388 and the dark theme's `terminal` 441–451
//! with the default Paseo tint 460–477, its foreground `#fafafa` at 391).
//!
//! [Decision Log]
//! - Purpose: a program's colours look as they do in Paseo's terminal,
//!   and the colours the grid answers OSC 10, 11 and 12 with are the ones
//!   drawn.
//! - Choice: the values as Paseo writes them, as `0xRRGGBB` words: the
//!   model's answers need the exact bytes, and the grid draws them
//!   through `gpui::rgb`. Selection keeps Paseo's alpha (`0xRRGGBBAA`).
//!   The default background and foreground are the theme's `surface0` and
//!   foreground, as Paseo's are, so the terminal sits on the page's own
//!   surface.
//! - Cost: Paseo's other dark tints (zinc, midnight and the rest) change
//!   only black and bright black and the surface; the app draws Paseo's
//!   default tint until the themes track gives the app a tint to choose.

/// Black, red, green, yellow, blue, magenta, cyan, white, then their
/// bright forms: the order of SGR 30–37 and 90–97.
pub type Ansi = [u32; 16];

/// One scheme's terminal colours, as `0xRRGGBB` (selection `0xRRGGBBAA`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalColors {
    /// The 16 ANSI colours.
    pub ansi: Ansi,
    /// The default foreground.
    pub foreground: u32,
    /// The default background (the theme's `surface0`).
    pub background: u32,
    /// The cursor.
    pub cursor: u32,
    /// What the cursor's cell's glyph is drawn in under a block cursor
    /// (Paseo's `cursorAccent`).
    pub cursor_accent: u32,
    /// The selection, with its alpha.
    pub selection: u32,
}

impl TerminalColors {
    /// The light scheme's.
    #[must_use]
    pub const fn light() -> Self {
        Self {
            ansi: [
                0x001a_1a1e,
                0x00dc_2626,
                0x0016_a34a,
                0x00ca_8a04,
                0x0025_63eb,
                0x0093_33ea,
                0x0008_91b2,
                0x00ff_ffff,
                0x003f_3f46,
                0x00ef_4444,
                0x0022_c55e,
                0x00f5_9e0b,
                0x003b_82f6,
                0x00a8_55f7,
                0x0006_b6d4,
                0x00fa_fafa,
            ],
            foreground: 0x001a_1a1e,
            background: 0x00ff_ffff,
            cursor: 0x001a_1a1e,
            cursor_accent: 0x00ff_ffff,
            // rgba(0, 0, 0, 0.15)
            selection: 0x0000_0026,
        }
    }

    /// The dark scheme's (Paseo's default tint).
    #[must_use]
    pub const fn dark() -> Self {
        Self {
            ansi: [
                0x0014_1716,
                0x00e0_7070,
                0x005d_ba80,
                0x00d4_a44a,
                0x006a_9de0,
                0x00b0_7ad0,
                0x004a_abb8,
                0x00d4_d4d8,
                0x0043_4645,
                0x00e8_9090,
                0x007e_cf9a,
                0x00e0_be6e,
                0x008a_b4e8,
                0x00c4_9ae0,
                0x006e_c2cc,
                0x00f0_f0f2,
            ],
            foreground: 0x00fa_fafa,
            background: 0x0018_1b1a,
            cursor: 0x00fa_fafa,
            cursor_accent: 0x0018_1b1a,
            // rgba(255, 255, 255, 0.2)
            selection: 0xffff_ff33,
        }
    }
}
