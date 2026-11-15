# Existing ActivityIndicator as a minimal0.19 producer

[Decision Log]
- Purpose: Models can consume the existing shared Small20 loader without the
  unrelated schedules, menu, button and glyph dependency chain.
- Constraint: start from the lead0.19 tree, keep its asset split and all other
  callers/variants. Preserve legacy Spinner vector/API/800ms and one rotator.
- Alternatives: hand-edit consumers or cherry-pick the older pre-split asset
  commits. Both would duplicate integration work or pull unrelated controls.
- Choice: re-express the same V1-passed control as one additive producer over
  the lead. Only its three activity vectors, registration, module, legacy
  delegation and size record accompany the unchanged loading implementation.
- Why and cost: one consumable hash with no area override, dependency change
  or new control design. One additional SVG draw is the existing alpha fix.

Parent d30804b2. loading.rs and slider.rs are byte-identical to e2827533;
three registered vectors and existing combined-asset tests are copied exactly.
All pre-existing lead vectors stay byte-identical. The slider record only drops
719 to the measured711; no size/token/boundary allowance is added.

API: kit::loading::activity_indicator(id, Size::Small, colour) -> AnyElement.
Small20 / Large36 / Numeric(Points::TEN or SIXTEEN or TryFrom<Pixels>).
Reference: Paseo components/ui/loading-spinner.tsx and installed RNWeb
ActivityIndicator. The e282 source/gate and limited Linux interior-alpha,
visible phase and explicit App-flag results passed V1. That prior pass is not
an exact whole-gate result for this newly based producer. New gate and source
review remain pending; product first-use, OS motion preference, four scales,
other backends and optical period are not claimed.

This producer is prepared with an alternate Git index over the immutable lead
parent; it does not alter the active schedules working tree or its build.
No native gate or real-agent/fleet action was performed by its construction.
