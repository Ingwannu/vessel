//! Authored colour data and one role expansion; no theme preference or I/O.
use super::SyntaxColors;

pub(super) struct Roles([u32; 10]);
impl Roles {
    pub(super) fn expand(&self) -> SyntaxColors {
        let [
            base,
            keyword,
            comment,
            string,
            number,
            function,
            kind_type,
            tag,
            attribute,
            operator,
        ] = self.0;
        SyntaxColors([
            keyword, comment, string, number, number, function, function, kind_type, kind_type,
            tag, attribute, attribute, base, operator, base, string, number, comment, function,
            string,
        ])
    }
}

pub(super) const CATPPUCCIN_LATTE: Roles = Roles([
    0x004c_4f69,
    0x0088_39ef,
    0x008c_8fa1,
    0x0040_a02b,
    0x00fe_640b,
    0x001e_66f5,
    0x00df_8e1d,
    0x0088_39ef,
    0x00df_8e1d,
    0x0004_a5e5,
]);

pub(super) const CATPPUCCIN_MOCHA: Roles = Roles([
    0x00cd_d6f4,
    0x00cb_a6f7,
    0x0093_99b2,
    0x00a6_e3a1,
    0x00fa_b387,
    0x0089_b4fa,
    0x00f9_e2af,
    0x00cb_a6f7,
    0x00f9_e2af,
    0x0089_dceb,
]);

pub(super) const DRACULA: Roles = Roles([
    0x00f8_f8f2,
    0x00ff_79c6,
    0x0062_72a4,
    0x00f1_fa8c,
    0x00bd_93f9,
    0x0050_fa7b,
    0x008b_e9fd,
    0x00ff_79c6,
    0x0050_fa7b,
    0x00ff_79c6,
]);

pub(super) const TOKYO_DAY: Roles = Roles([
    0x0037_60bf,
    0x0098_54f1,
    0x0084_8cb5,
    0x0058_7539,
    0x00b1_5c00,
    0x002e_7de9,
    0x0000_7197,
    0x00f5_2a65,
    0x008c_6c3e,
    0x0000_6a83,
]);

pub(super) const TOKYO_NIGHT: Roles = Roles([
    0x00c0_caf5,
    0x00bb_9af7,
    0x0056_5f89,
    0x009e_ce6a,
    0x00ff_9e64,
    0x007a_a2f7,
    0x002a_c3de,
    0x00f7_768e,
    0x00e0_af68,
    0x0089_ddff,
]);

pub(super) const ONE_LIGHT: Roles = Roles([
    0x0038_3a42,
    0x00a6_26a4,
    0x00a0_a1a7,
    0x0050_a14f,
    0x0098_6801,
    0x0040_78f2,
    0x00c1_8401,
    0x00e4_5649,
    0x0098_6801,
    0x0001_84bc,
]);

pub(super) const ONE_DARK: Roles = Roles([
    0x00ab_b2bf,
    0x00c6_78dd,
    0x005c_6370,
    0x0098_c379,
    0x00d1_9a66,
    0x0061_afef,
    0x00e5_c07b,
    0x00e0_6c75,
    0x00d1_9a66,
    0x0056_b6c2,
]);

pub(super) const NORD_LIGHT: Roles = Roles([
    0x002e_3440,
    0x005e_81ac,
    0x006b_7280,
    0x004f_6f3a,
    0x008f_5e91,
    0x002e_6f8e,
    0x003b_7f87,
    0x005e_81ac,
    0x008f_5e91,
    0x005e_81ac,
]);

pub(super) const NORD_DARK: Roles = Roles([
    0x00d8_dee9,
    0x0081_a1c1,
    0x0061_6e88,
    0x00a3_be8c,
    0x00b4_8ead,
    0x0088_c0d0,
    0x008f_bcbb,
    0x0081_a1c1,
    0x008f_bcbb,
    0x0081_a1c1,
]);

pub(super) const GRUVBOX_LIGHT: Roles = Roles([
    0x003c_3836,
    0x009d_0006,
    0x0092_8374,
    0x0079_740e,
    0x008f_3f71,
    0x0042_7b58,
    0x00b5_7614,
    0x0007_6678,
    0x00b5_7614,
    0x00af_3a03,
]);

pub(super) const GRUVBOX_DARK: Roles = Roles([
    0x00eb_dbb2,
    0x00fb_4934,
    0x0092_8374,
    0x00b8_bb26,
    0x00d3_869b,
    0x008e_c07c,
    0x00fa_bd2f,
    0x0083_a598,
    0x00fa_bd2f,
    0x00fe_8019,
]);

pub(super) const SOLARIZED_LIGHT: Roles = Roles([
    0x0065_7b83,
    0x0085_9900,
    0x0093_a1a1,
    0x002a_a198,
    0x00d3_3682,
    0x0026_8bd2,
    0x00b5_8900,
    0x0026_8bd2,
    0x00b5_8900,
    0x0085_9900,
]);

pub(super) const SOLARIZED_DARK: Roles = Roles([
    0x0083_9496,
    0x0085_9900,
    0x0058_6e75,
    0x002a_a198,
    0x00d3_3682,
    0x0026_8bd2,
    0x00b5_8900,
    0x0026_8bd2,
    0x00b5_8900,
    0x0085_9900,
]);

pub(super) const GITHUB_DARK: SyntaxColors = SyntaxColors([
    0x00ff_7b72,
    0x008b_949e,
    0x00a5_d6ff,
    0x0079_c0ff,
    0x0079_c0ff,
    0x00d2_a8ff,
    0x00d2_a8ff,
    0x00ff_a657,
    0x00ff_7b72,
    0x007e_e787,
    0x0079_c0ff,
    0x0079_c0ff,
    0x00c9_d1d9,
    0x0079_c0ff,
    0x00c9_d1d9,
    0x00a5_d6ff,
    0x0079_c0ff,
    0x008b_949e,
    0x0079_c0ff,
    0x00a5_d6ff,
]);

pub(super) const GITHUB_LIGHT: SyntaxColors = SyntaxColors([
    0x00cf_222e,
    0x006e_7781,
    0x000a_3069,
    0x0005_50ae,
    0x0005_50ae,
    0x0082_50df,
    0x0082_50df,
    0x0095_3800,
    0x00cf_222e,
    0x0011_6329,
    0x0005_50ae,
    0x0005_50ae,
    0x0024_292f,
    0x0005_50ae,
    0x0024_292f,
    0x000a_3069,
    0x0005_50ae,
    0x006e_7781,
    0x0005_50ae,
    0x000a_3069,
]);
