# Typed menu restoration on the lead0.19 line

[Decision Log]
- Purpose: retain the previously reviewed menu semantics on the current lead
  base without replacing any lead policy or adding a parallel renderer.
- Constraint: menu/menu_right, TopRight, QUICK arrival, fill_in, callback and
  keyboard/outside semantics belong to the lead. Control extensions are separate
  from the preceding behaviour-neutral split and from feature registrations.
- Alternatives: keep an old unsplit fork or rewrite the extensions by hand.
- Choice: reapply the exact source-reviewed typed extension files to the new
  split. The lead root was byte-identical to the original published root.
- Why and cost: preserves one renderer and exact earlier values; one newly
  added lead Git literal needs the same constructor completion as the two
  older literals. No action, label or pending policy changes for other areas.

All seven menu Rust files are byte-identical to `ef91b954` (V1 isolated source
and A13 gate pass); retained parity records list every size/hash. The new base
requires its own checking: this source identity is not a whole merged-tree pass.

The lead added `app/areas/git/menu.rs::merge_item`. Its label, tone, disabled
condition, starts_group and callback remain unchanged. Only completion with
`..MenuItem::new(label)` supplies legacy defaults for the new fields. Existing
`items` and `pull_item` constructor completions are preserved. No change to the
lead's PR merge behaviour or feedback mapping is included.

The0.19 hand-off must include the new split/restoration hashes and exact final
gate result. Known focus restoration and product first-use UI remain unapproved.
The e282 alpha correction and its exact pixel evidence remain separate scopes.
K's palette producer collision is a later coordinator/lead integration obligation,
not a reason to copy GREEN_500 or widen a public API here.

The combined0.19 source passed local App all-targets Clippy on the short lane
with actual View/Link/Kit/App checking (1m43s). It did not time out. Fast fmt,
size, boundaries and token checks passed. This is a narrow check, not the
required A13 whole integrated-tree gate; that result is still pending.
