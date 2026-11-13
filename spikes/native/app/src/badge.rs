//! The one adapter between a view's semantic status and the kit's badge.
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
