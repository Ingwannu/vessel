# The menu split on the lead0.19 base

[Decision Log]
- Purpose: establish the lead's single renderer before typed extensions return.
- Constraint: preserve every lead-defined API, input path, placement and motion;
  keep the existing tests and avoid a second renderer or a size allowance.
- Alternatives: retain an old extended renderer through merge conflicts or
  repeat source by hand. The lead's menu is identical to the original published
  file; its previously reviewed isolated split is therefore reusable exactly.
- Choice: move item values and drawing from the lead root module into its two
  child modules, preserving root re-exports and caller-owned state/callbacks.
- Why and cost: one render path fits the source budget. Two additional modules,
  with no control behaviour or external caller change.

Base merge `270010cd`, lead `d30804b2`. The lead menu SHA256 is
`b9543b0656a158a9e8aa4cc08af36a09a2555409c9bca3d7dde6040ab034a2f5`,
identical to56a. All three resulting Rust files are byte-identical to the
V1-passed pure split at `f6674d0e`; a per-file parity manifest is retained.
The earlier source comparison of each item/state/draw body therefore still
applies. This is a source-parity fact, not a merged-tree gate or UI pass.

Legacy menu/menu_right, TopRight placement, priority1, QUICK arrival, fill_in,
keyboard/outside handling and callbacks remain unchanged. The known after-close
root-key observation is not fixed here. Typed feedback, leading sizes, measured
end placement and dismissal remain a separate following restoration commit.
Whole checking on the0.19 integration is still owed.
