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
- Effects: settings owns one preference/page and its committed-effect
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
with the canonical settings committed startup/effect caller and actual union
checks. No dead-code exemption or fake product caller is used. The owner may
remove that exact guard as part of the coordinated consumer cutover; the
producer alone is not a live settings UI or all-resident retention proof.

A further regression is written before its repair: shrinking to zero then
increasing while the executor owns the model must still discard the old history.
Keeping only the final desired capacity loses the intermediate trim intent.
The new test requires zero old history after return, then future output may grow
under the final capacity. It has not been executed on the old prototype yet.

The owner now retains the minimum intermediate trim separately from the final
desired capacity during a handover. A matching completion applies that trim
first, then the final capacity; increasing cannot revive old rows. Closing or
replacing an opening discards only its pending trim, retaining the preference.
A second regression checks new-opening history and stale completion rejection.
These changes are staged on top of the original immutable producer checks;
neither the new regressions nor this repair has a runtime result yet.

Shrinking retention is irreversible for this model: increasing or restoring the
preference does not restore evicted history. The consumer must therefore attach
the live trim to the existing canonical COMMITTED preference receipt, never an
optimistic preview or a failed-save rollback effect. New openings use the last
committed applied limit. No new writer/receipt or mutable snapshot restoration
is introduced, and history rollback after a failed save is not promised.

The existing canonical owner must deliver the minimum committed limit followed
by the final committed limit when its latest-value watch coalesces several
saves. A model currently outside the UI preserves that minimum in its existing
opening-owned pending trim; a returned model applies minimum then final.
New openings inherit only the last applied committed limit and do not replay
another opening's predecessor effects. The existing Coordinator/App owner owns
this ordered effect aggregation, not the terminal area or another receipt store.

Only an actual Saved canonical outcome admits the irreversible effect. LaunchOnly
and Failed keep both current live retention and new-opening applied limit
unchanged. There is no memory-only trim alternative. A later ordinary retry that
actually reaches Saved may apply it; a past failure does not permanently lock
the setting. No product caller for these outcomes exists in this producer yet.
