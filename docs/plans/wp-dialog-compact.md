# Compact-only dialog configuration

## Decision Log

- Purpose: let callers match Paseo's adaptive form sheet without copying the
  canonical desktop width, height cap or body spacing.
- Existing asset: `DialogSize` already resolves one `DialogLayout`; its configured
  compact variant already uses the shared sheet renderer and snap engine.
- Constraint: existing Normal/Wide fitting geometry and the snap engine's 60/90
  default must remain unchanged for callers that have not opted in.
- Alternatives: repeat 520/85 in every form, expose private profile fields, or
  replace the legacy default globally. Repeated values drift; the other choices
  widen the API or change unrelated callers.
- Choice: additive `DialogSize::with_compact(CompactLayout) -> Self` resolves the
  existing profile and replaces only its compact setting. It returns Configured,
  preserving width, desktop height and body spacing even for a configured input.
- Tradeoff: the caller still owns its validated compact values and all domain
  action/dismissal admission. This builder adds no renderer, focus state, gesture
  physics, authority, or new default.

## Source and use

Pinned Paseo `1c00b365c9e0b74c1580eba33d905d693125ae21`:
`packages/app/src/components/schedules/schedule-form-sheet.tsx:438-458` supplies no
snap override. `packages/app/src/components/adaptive-modal-sheet.tsx:639-663`
resolves the absent snap values to 65% and 90%. A form uses validated
`SnapPoints::new(65, 90)`, wraps them in `CompactLayout::Draggable`, and calls
`DialogSize::Normal.with_compact(...)`. Normal's shared 520px/AtMost85% desktop
geometry and inset body remain canonical; Wide and authored profiles also retain
their own desktop geometry.

The dialog module comment now matches the existing full-width compact renderer;
no width implementation changes. Tests pin source values, preserve both legacy
desktop profiles and configured exact-height/caller-spacing profiles, support
replacement of a prior override, and keep legacy fitting90 and snap60/90 intact.

## Validation status

First ordinary selected run `wp-desktop-dialog-compact-20261116T010904Z-2eab2ef8`
passed fonts, formatting and quality, then stopped at Clippy's panic-in-result-fn
rule in all three new tests. No tests ran. The correction uses ordinary unit
test returns and the workspace's existing permitted unwrap-in-tests policy;
all product logic and assertions stay unchanged, with no new allowance. The
receiving source also includes announced base13.1. Corrected ordinary selected run
`wp-desktop-dialog-compact-20261116T021420Z-e1dba79c` passed all five selected
steps (fonts, fmt, quality, all-target Clippy, tests) on exact source
`e1dba79c5e7972a1e55fa081a0c93669e467710e`. The actual test totals were
1620 passed, 0 failed, 2 ignored; Kit was 248/0/0 and all three new compact
cases passed. The raw test log SHA256 is
`bd4c6b49811bb9967412e8a9f8edcf9c963ec25bb13425e74a904454923d6d8b`.
This records selected source evidence; V1 review and lead adoption are still
required before consumer use. F1 must own
its actual sheet captures and source comparison; this builder does not qualify
consumer focus, native OS behavior or visual parity.

V1 subsequently confirmed bounded SOURCE / requested selected NARROW PASS on
exact `ad14f5f6..e1dba79c` plus docs-only `d3b27f014`. The five paths, source
values, unchanged legacy profiles, all log hashes and three pure cases were
independently checked. The selected packet has been sent to the lead; G2 waits
for an announced adopted green line. There is no painted 65/90 sheet, consumer,
OS, IME, accessibility or full-package approval. At the quota stop this producer
is finished; no unchanged rerun is requested.
