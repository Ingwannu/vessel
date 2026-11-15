# Terminal retention setting producer

[Decision Log]
- Purpose: persisted counts and live changes use the existing validated
  ScrollbackLines type, including zero history, without losing an inspected row
  that still remains retained.
- Existing assets: the native terminal size/model/history/viewport types and
  ordered executor handovers. Reuse their eviction origin and opening identity;
  no second setting store, parser, renderer or resident map is introduced.
- Alternatives: raw integer preferences, clearing/replaying the terminal on every
  change, or applying only to the currently present model. Those admit invalid
  state, discard retained text or miss changes during an executor handover.
- Choice: validated serde on the existing count, a model capacity setter and one
  owner-held applied limit. Apply it before loading/output handovers are painted;
  new openings use the same retained value. Generation invalidates old rows.
- Why: retained physical addresses already distinguish genuine front eviction
  from new output. A larger limit cannot resurrect evicted rows.
- Effects: settings owns one preference/page and its canonical apply/rollback
  caller. Terminal owns model/viewport updates. The app hook is literally test
  staging until that real product caller lands in the same source union; no
  dead-code waiver or uncalled product seam is permitted.

Reference: Paseo hooks/use-settings/storage.ts54-56 defines the default and
range; screens/settings/terminal/terminal-section.ts uses digit-only draft,
blur/Enter commit and one ordinary form input. Terminal runtime applies the
scrollback setting to its local renderer, not an authority to rewrite every
node's global history. The new authoritative-frame consumer must likewise keep
this a per-view retention preference.

Scope: the current legacy native terminal model plus staged owner adapter.
No settings UI/persistence caller, all-resident cutover, node history mutation,
new RPC or current shared-engine/native parser retirement is claimed here.
The checked producer handoff must distinguish product value/model code from
the test-only app hook. Checks remain owed.

The value/model/history implementation and two view regressions are ported
from the retained prototype, with invalid persisted counts computed from the
single named bound. Separate GPUI owner tests cover changes while output and
history handovers own the model, plus a new opening inheriting the limit.
The literal cfg(test) on the app settings module must be removed ONLY together
with the canonical settings startup/apply/rollback caller and actual union
checks. No dead-code exemption or fake product caller is used. The owner may
remove that exact guard as part of the coordinated consumer cutover; the
producer alone is not a live settings UI or all-resident retention proof.

A further regression is written before its repair: shrinking to zero then
increasing while the executor owns the model must still discard the old history.
Keeping only the final desired capacity loses the intermediate trim intent.
The new test requires zero old history after return, then future output may grow
under the final capacity. It has not been executed on the old prototype yet.
