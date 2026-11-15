# Native Models: reported routes and configured use

## Resume point

The Models screen is next, before registry/Add. Existing `view::models`
rules (`d359c072`) are reused: route users come only from explicit profile
references; one machine page rejects stale/duplicate replies and retains
successful facts during refresh/failure. No general agent compatibility
matrix is reported by the node. Named launch profiles are a separate node
contract and are not implemented by this work.

## Reference and decision before implementation

Paseo 0.15.4 `components/provider-diagnostic-sheet.tsx` is the reference:
`DiscoveredModelRow` (51–75), `ProviderModalBody` (488–570), its search and
provider-target header (575–749), and `sheetStyles.modelRow` (787–808).
`provider-diagnostic-models.ts` and its four tests specify retaining a
matching provider's cached models during loading without crossing hosts.
The existing Vessel page state already enforces the equivalent machine
identity boundary; those rules are extended, not forked.

[Decision Log]
- Purpose and intent: list this machine's configured models and the
  profiles explicitly using each, with honest empty/loading/failed states.
- Existing assets and constraints: `model.list` (protocol 7), `profile.list`
  (5), `agent.list` (11); existing pure Models rules; the machine chooser,
  settings card/custom row/section, field and typed matcher. No wire change
  or new external dependency is needed.
- Alternatives considered: infer agent compatibility from agent names;
  show missing methods as successful empty lists; create another settings
  row, loader or search implementation.
- Choice: extend the existing snapshot/state, use bounded read tasks with
  one named deadline, keep errors typed and words bilingual. Search uses
  the existing matcher and preserves node order when no query is supplied.
  All controls are existing shared controls.
- Why: the node's actual routes and configured accounts are sufficient for
  a useful read-only screen, without inventing unsupported capabilities.
- Advantages, costs and effects: a new registered Models page in the
  existing machine group, no second chooser or provider settings entry.
  Snapshot/profile labels are from one connection. An unsupported list
  remains distinct from empty. Machine switches clear facts and retain a
  never-reused reply sequence.

## Deliberate differences and verification still owed

Paseo presents agent-discovered models in a provider sheet. Vessel currently
reports configured model routes; this standalone page uses the existing
native Settings frame. Rows preserve the model name/identifier/provider
order and add only node-reported small-model/API/configured-profile facts
using shared settings furniture. “Used by profiles” never means “compatible
with every agent”; absent profiles make no capability claim. Custom-model
add/remove and provider diagnostic buttons are omitted because this screen
has no corresponding operation. No dead controls are drawn.

The existing shared ActivityIndicator Small20 is owned by its original
control track. Its current integrated handoff is pending; no area loader
will be created. Implementation/gate/captures are not yet complete. Required
evidence is A13 on the exact commit and personally inspected EN/KO,
dark/light, 1280/390, long/many/empty/loading/failed/search shapes.

## Dispatcher extraction before Models

The assigned lead baseline is `d30804b2`, merged without conflicts as
`8dbf3ccd`. Its root link dispatcher is exactly the lead's 988-line blob;
the earlier 1082-line baseline is not copied over it. Existing removal
rules and the earlier screen evidence were already on the lead's line.
The later removal call/test correction remains on this branch, with its
own exact-source gate and V1 review packet.

[Decision Log]
- Purpose: make room for Models registration without increasing the root
  dispatcher's size record or changing another owner's error-type block.
- Existing asset: its current `State::options` already owns one coherent
  agent-option read and answer-cache sequence, with a real root caller.
- Alternatives: grow the record, duplicate that path, or take the other
  control track's separate error extraction. None is needed.
- Choice: move that existing method into native-only `link/src/options.rs`;
  change only its visibility to `pub(super)` for its existing root caller.
- Why: the same client lookup, option request, cache task, typed options
  event and order remain in one place. There is no new RPC or dependency.
- Costs and effects: the move-only commit is source-only, with no module
  registration or isolated compile/quality pass claimed. Its immediate
  follow-up registers the module, lowers the record to the measured size,
  and is the exact head run through A13. This explicit review split must
  not be mistaken for passing the intermediate tree's gate.

The original method block is retained byte-for-byte after removing only
the parent visibility change. No assertion or test case changes. Models
implementation is still an uncompiled preparation outside the repository;
this extraction does not register a screen or create a new control.

The immediate registration adds only the native module declaration and
records the measured 952 lines instead of 988. The root actor's existing
`Command::Options` arm is unchanged; the moved method is now reachable
through that same caller. A13 on this registered head is pending. No Models
screen, loader or registry behavior is included in this move/registration.
