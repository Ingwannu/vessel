//! Authored sRGB bytes from Paseo's dark tint definitions. They stay separate
//! from Hsla conversion so contrast is measured on the source bytes.

/// The dark builder's ring, independent of a tint (theme.ts:392).
pub(super) const DARK_RING: u32 = 0x00d4_d4d8;

#[derive(Debug, Clone, Copy)]
pub(super) struct Tint {
    pub surface0: u32,
    pub surface1: u32,
    pub surface2: u32,
    pub surface3: u32,
    pub surface4: u32,
    pub sidebar: u32,
    pub muted: u32,
    pub extra_muted: u32,
    pub border: u32,
    pub border_accent: u32,
    pub accent: u32,
    pub accent_foreground: Option<u32>,
    pub destructive: u32,
}

// theme.ts:480-498. Only Zinc overrides the dark builder's white accent text.
pub(super) const ZINC: Tint = Tint {
    surface0: 0x0018_181b,
    surface1: 0x001f_1f22,
    surface2: 0x0027_272a,
    surface3: 0x003f_3f46,
    surface4: 0x0052_525b,
    sidebar: 0x0013_1316,
    muted: 0x00a1_a1aa,
    extra_muted: 0x0071_717a,
    border: 0x0027_272a,
    border_accent: 0x0030_3036,
    accent: 0x00e4_e4e7,
    accent_foreground: Some(0x0018_181b),
    destructive: 0x00c4_4a4a,
};
// theme.ts:501-518.
pub(super) const MIDNIGHT: Tint = Tint {
    surface0: 0x0016_1820,
    surface1: 0x001c_1e27,
    surface2: 0x0025_2731,
    surface3: 0x003c_3e4c,
    surface4: 0x0053_5564,
    sidebar: 0x0012_1420,
    muted: 0x009a_9db0,
    extra_muted: 0x006b_6e82,
    border: 0x0024_2636,
    border_accent: 0x002e_3040,
    accent: 0x003b_6fcf,
    accent_foreground: None,
    destructive: 0x00c4_4a52,
};
// theme.ts:521-538.
pub(super) const CLAUDE: Tint = Tint {
    surface0: 0x001f_1f1e,
    surface1: 0x0026_2523,
    surface2: 0x002f_2d2b,
    surface3: 0x004a_4745,
    surface4: 0x0060_5d5b,
    sidebar: 0x001a_1918,
    muted: 0x00ad_a9a5,
    extra_muted: 0x0078_746f,
    border: 0x002c_2a27,
    border_accent: 0x0036_332f,
    accent: 0x00d9_7757,
    accent_foreground: None,
    destructive: 0x00cf_513e,
};
// theme.ts:541-558.
pub(super) const GHOSTTY: Tint = Tint {
    surface0: 0x0028_2c34,
    surface1: 0x002f_333d,
    surface2: 0x0038_3c48,
    surface3: 0x004a_4f5e,
    surface4: 0x005b_6175,
    sidebar: 0x0021_252d,
    muted: 0x00c8_ccd8,
    extra_muted: 0x00a0_a4b2,
    border: 0x0035_3a47,
    border_accent: 0x003f_4454,
    accent: 0x0035_70e0,
    accent_foreground: None,
    destructive: 0x00c4_4a55,
};
// theme.ts:723-740. The shared Paseo-green substitution is ADR 0070's,
// using the one existing brand constant rather than a second colour literal.
pub(super) const PURE_BLACK: Tint = Tint {
    surface0: 0x0000_0000,
    surface1: 0x000a_0a0a,
    surface2: 0x0011_1111,
    surface3: 0x0020_2020,
    surface4: 0x002d_2d2d,
    sidebar: 0x0000_0000,
    muted: 0x00a1_a1aa,
    extra_muted: 0x0071_717a,
    border: 0x001c_1c1c,
    border_accent: 0x0024_2424,
    accent: crate::assets::BRAND,
    accent_foreground: None,
    destructive: 0x00c4_4a4a,
};
