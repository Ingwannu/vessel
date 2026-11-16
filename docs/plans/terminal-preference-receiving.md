# Terminal preference: one validated scalar, one committed effect owner

[Decision Log]
- Purpose: provide the exact typed field/change requested by W19 for its sole
  canonical preference merge, before building a visible Terminal row.
- Existing assets: C2's checked but provisional cd3e9805b on App base13,
  the sole terminal::ScrollbackLines with validated serde/default/bounds,
  Preferences::changed and the existing lenient per-field JSON reader.
- Choice (modify/reuse): add terminal_scrollback:ScrollbackLines and
  PreferenceChange::TerminalScrollback. Reuse the same value and store; no
  numeric type, range, editing engine, writer or receipt is duplicated.
- Why: changing the limit can irreversibly discard local history. W19 alone
  owns exhaustive scalar merge and minimumSaved/finalSaved/revision delivery.
  Shell::prefer must not trim optimistically, and Failed/LaunchOnly never
  advance the applied limit. New owners use the final value returned by the
  same pre-owner-change drain, not another potentially newer global read.
- Costs: this bounded producer has no visible row or live effect yet. C2's
  literal cfg(test) module guard remains unchanged until W19's actual Saved-only
  caller and the complete receiving checks are in the SAME source union.

This isolated tree starts at C2 cd3e9805b (exact parent App base13 2ebeedf9),
under the author's explicit provisional receiving permission. It does not
modify the frozen frame tree or start another Cargo run. Source is UNCOMPILED;
old C2 tests/strict are prior-source evidence, not evidence for this new field.
The eventual consumer gate/review must cover serde, min trim, replay, stale/new
openings, coalescing, failure and LaunchOnly, plus actual UI/input behavior.

The native JSON field is terminal_scrollback under Link Prefs.settings. Missing
or invalid stored values use the existing per-field default semantics. A direct
PreferenceChange must deserialize a valid scalar; it never silently clamps.
No RPC, protocol/schema change, Root crate, new dependency or permission change.
L's approved Terminal wording stays reserved for the later actual row; none is
introduced here. Existing tests preserve the old page/bridge DEFAULT fixture and
explicitly derive the new native-only field using the canonical default.

## Prepared source checks

Direct rustfmt, native size708 files/13 existing shrinking exceptions, token
ratchet none new and whitespace checks pass. These are source checks only;
three authored preference cases and the adapted native-default golden are UNRUN.
The old bridge/page DEFAULT fixture itself is unchanged. C2's11 owned paths
(including its literal guard) are byte-identical to cd3e9805b. Root delta is empty.
No Cargo job was submitted: the sole existing frame gate remains queued/running.
W19 may use this as an explicitly UNCOMPILED receiving dependency and must compile
its exhaustive scalar merge plus actual ordered-effect caller in the union.

## Stop checkpoint (2026-11-16 03:41 UTC)

Exact producer9d1427e6d5dd593436d6b7903c0849f4a360311f remains SOURCE-ONLY and
UNCOMPILED. W19 accepted its exact dependency chain in f12989e6/00adcb9d and is
preparing a separate receiving tree; that is not actual effect/receipt approval.
No Cargo job was submitted here. Three new cases and the modified native golden,
all-target Clippy, live Saved-only canonical effects and row/input/UI proof remain
UNVERIFIED. This docs-only checkpoint preserves C2's guard and all11 source paths.
