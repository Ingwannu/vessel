# Shared status badge

The Devices, Agents and Schedules rows need the same short status label.
`kit/badge.rs` follows Paseo's `components/ui/status-badge.tsx`: success,
warning, error and muted; small and extra-small paddings; optional leading
content; no ellipsis, focus or click behaviour. The Decision Log at the
module's top records the layout values and consequences.

`theme/status.rs` carries the semantic text colours and their tints from
Paseo's current `styles/theme.ts`. These are deliberately separate from
the board's lane and status-dot colours. Both themes map every badge
variant to these signals; muted maps to the existing neutral tokens.

The cases in Paseo's `status-badge.browser.test.tsx` are retained as Rust
checks: the muted shell, the same hue for a semantic fill and text, a
translucent fill and the semantic text token. That browser test uses a
fixture theme; exact golden checks here use the reference's current theme
values, in light and dark. Both padding sizes are checked too.

Pure area logic returns `view::badge::Variant`. The kit owns its own enum
and depends on no view type. `app::badge::variant` is the one exhaustive
adapter, imported by every area; adding a meaning requires updating it.
The public app module is the deliberate shared import seam for areas
landing independently, before every consumer has landed.

No dependency or screen is added by this shared-control slice.

Validation: the pinned native workspace passed `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings` and `cargo test` as a whole.
The badge-only patch was independently checked against the starting
commit; it adds no dependency on the Devices screen or QR work.
