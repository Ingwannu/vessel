//! The shared adapters between pure semantic signals and kit controls.
//! The public module is the shared import point for independently landing
//! areas; the kit must never depend on the view crate.

/// Map a pure decision to its rendering signal. Adding a variant to
/// either side must make this match fail until its meaning is chosen.
#[must_use]
pub const fn variant(variant: vessel_app_view::badge::Variant) -> vessel_app_kit::badge::Variant {
    use vessel_app_kit::badge::Variant as Draw;
    use vessel_app_view::badge::Variant as Meaning;
    match variant {
        Meaning::Success => Draw::Success,
        Meaning::Warning => Draw::Warning,
        Meaning::Error => Draw::Error,
        Meaning::Muted => Draw::Muted,
    }
}

/// Map the separate, five-variant notice vocabulary exhaustively.
#[must_use]
pub const fn alert_variant(
    variant: vessel_app_view::alert::Variant,
) -> vessel_app_kit::alert::Variant {
    use vessel_app_kit::alert::Variant as Draw;
    use vessel_app_view::alert::Variant as Meaning;
    match variant {
        Meaning::Default => Draw::Default,
        Meaning::Info => Draw::Info,
        Meaning::Success => Draw::Success,
        Meaning::Warning => Draw::Warning,
        Meaning::Error => Draw::Error,
    }
}
