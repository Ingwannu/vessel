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

The existing shared ActivityIndicator Small20 is consumed from the isolated
source-reviewed producer `434933b3`; no area loader is created. The Models
consumer gate and first-use captures are not yet complete. Required
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
  and receives narrow checks before the final package A13 gate. This explicit review split must
  not be mistaken for passing the intermediate tree's gate.

The original method block is retained byte-for-byte after removing only
the parent visibility change. No assertion or test case changes. That extraction itself did not register a screen or create a new control.
The Models implementation is now installed in the feature worktree and is
under narrow compilation; its final package evidence is recorded below.

The immediate registration adds only the native module declaration and
records the measured 952 lines instead of 988. The root actor's existing
`Command::Options` arm is unchanged; the moved method is now reachable
through that same caller. Its options-only queued A13 was cancelled under A14; the final Models
package receives the required whole gate. No Models
screen, loader or registry behavior is included in this move/registration.

## Earlier package boundary and remaining dispatcher headroom

Models was first prepared as a separate user-visible slice. It now belongs
to the coordinator-assigned A9 package, whose final gate and review are
combined with Providers and the registry under A14. The extra queued
options-only whole gate was cancelled before acquiring a slot (exit 143,
not a pass). The registered move did pass local narrow link Clippy and all
65 link tests. Final Models source, one A13 gate and first-use captures are
submitted together; there is no further whole gate for a helper commit.
The earlier removal correction independently has V1 source/gate pass.

The reviewed minimal shared loader was consumed as `df4ee7ae` (upstream
`434933b3`) plus its results-only note `d35508fc` (`533b21ff`). The assigned
shell-event extraction is `8317ee05` (`31495e4d`), with one unchanged event
dispatcher, creating room for a single Models forwarding arm. No menu or
newer vendor-mark chain was taken.

The coordinator also delegated the unchanged `State::keep_commands` method
to `options.rs`, beside the same retained-agent-list cache path. Its real
root actor caller stays in place and visibility becomes `pub(super)`. This
move-only commit keeps the current 952 record unchanged; the following
Models registration measures and lowers the final record. This is necessary
because adding registration lines must never raise a shrunken allowance.
The function body, its client lookup, cache data, timestamp and task order
remain unchanged. No new RPC, assertion or dependency belongs to the move.

New Models words use `locales/en/models.ftl` and `locales/ko/models.ftl`
through the generated, cached Fluent scope already on this lead baseline.
The translation owner confirmed that the new domain is not otherwise owned.
Existing Agents tables and translation-engine code are not changed.

## Models feature implementation (verification in progress)

The data flow is the existing machine chooser → pure Models page/ticket →
`Command::Models` → native read worker → `Event::Models` → the same page.
Only `model.list`, `profile.list` and `agent.list` are read. Each request is
checked against its generated method's protocol introduction. A missing
method is typed `OlderNode`, never a successful empty list. One snapshot
uses one cloned connection and retains original typed RPC failures.

The accepted configured-route page deliberately retains successful rows
after a failed refresh and puts the failure above them. Paseo's
`provider-diagnostic-models.ts:29–45` reuses its discovered catalogue only
while refreshing; after an empty/error result it returns no rows. Keeping
Vessel's already accepted snapshot rule avoids throwing away known
configuration when this separate read fails, while the alert makes the
failed refresh visible. This is a difference, not a claimed clone of that
failure-cache branch.

The native worker allows four dispatched reads and gives each one complete
snapshot a named 15-second deadline. Event-channel backpressure retains the
permit instead of spawning unbounded pending deliveries. This deadline
starts at dispatch. This is a deliberate transport difference: Paseo
`packages/client/src/daemon-client.ts:1125,5545–5567,5745–5760` uses
60 seconds for its cached snapshot and 120 seconds for provider refresh.
Vessel here only reads three existing configuration lists, without an agent
discovery/probe operation; the bounded native read uses 15 seconds rather
than retaining a worker for those longer discovery budgets. It is not
claimed as the same discovery operation or deadline. The existing shared actor's startup/periodic session
reads can delay command dispatch independently. That inherited actor wait
was reported to the shared owner; it is not claimed as a bounded end-to-end
refresh. No node or wire change belongs to this package.

`view::models::presentation` owns initial versus cached loading/failure,
true empty versus unsupported, query results and refresh availability.
Repeated gestures retain the original active ticket; answers from a prior
machine or request cannot replace the current snapshot. Search uses the
existing matcher over route name, model identifier and provider; empty
query and equal scores preserve node order. `usage` distinguishes an older
profile method from an actual unused route and preserves separate accounts
of the same agent. Rows without an explicit profile reference have no
additional negative sentence or empty profile heading, following the
reference row's optional-description shape. Whole sentences and action labels come from the new
English/Korean Fluent domain, using the existing generated cached scope.

The missing general agent/model compatibility fact remains a node contract
gap: `crates/vessel-harness/src/profile/parse.rs:149` names only two routed
harnesses in `NotRoutable`; lines 269–277 consult an internal descriptor.
The screen cannot use those internals as a general compatibility list.
Only an explicit profile's route reference is shown as “Used by profiles”.
Paseo's fixture also hides discovered entries marked `isSelectable: false`;
Vessel's configured `ModelRoute` has no such field, so the screen lists all
reported route definitions and does not invent an equivalent filter.

Additional necessary platform differences: native shared labels currently
have no selectable-text control, so whole model identifiers have the kit's
copy button. Long identifiers wrap rather than disappear in a one-line
ellipsis. API names and smaller-model identifiers are reported route facts;
they do not imply agent capabilities. No selectable/custom-model/add/delete
or diagnostic operation is drawn without its node contract. The normal
Refresh control uses the existing shared Ghost button; initial failure uses
the existing primary Retry, following the reference footer/body placement.

The fleet fixture now checks 24 routes on one machine and a genuinely empty
route list on the other. It includes two provider APIs, a long Korean model
identifier, an unbroken identifier and three profile references including
two accounts of the same agent. Both provider endpoints point at an unused
loopback port; listing models does not send a prompt or contact a provider.
Fixture configuration files are private (0600). No real account, preview
node or installation is used.

Shared-file seams: Models section/state registration in the Settings area;
one `Event::Models` forwarding arm in the shared shell event child; one
unsent-command refusal arm; native/web Command/Event registrations; native
worker initialization and forwarding; the small root module/hub declarations after the assigned dispatcher split.
The root remains off the size list. The delegated unchanged `keep_commands`
body is retained once in the lead's `refresh.rs`. The machine-actor test fixture also receives the new Hub field so existing
actor tests construct the same production State. Existing
Connections, menu, LinkError and other owners' screen bodies are preserved.
No new dependency or control is introduced.


## Current package assignment

The coordinator's full parity plan now assigns this work to package A9
(providers/models, `parts/B-providers.md` and area-5 section 55). The current
configured-route screen is a prerequisite, not proof of the entire provider
catalogue/diagnostics/mode/effort survey. It will be fixed by narrow checks
and committed first. A14's one whole native gate and one review/capture
packet apply to the completed A9 package; no extra whole gate is started
for this partial Models checkpoint. Agent profiles remain A10 and wait for
N3's amended ADR 0076 and a callable, numbered handoff. No proposed call is
implemented. The unannounced app-base branch is read only until assigned.


## Assigned package-base merge

Merged the coordinator's `8d5513e4` package base before restoring the Models
checkpoint. The link root is now 387 lines and has no size-list entry.
Registrations belong to `command.rs`, `event.rs`, `dispatch.rs` and the
root module/hub declarations. The shared shell event child retains the
base's fork-header updates; its root has one event-module declaration.
`State::keep_commands` is already in the lead's cohesive `refresh.rs`, with
exactly the same body as the separately delegated move. The merge keeps
that one canonical implementation instead of duplicating it in `options`.
The options reader is also already on the lead's line.

The uncommitted Models checkpoint was preserved and checked as a patch
before the merge. Its local runtime test never started: it was cancelled
while queued (143), as the coordinator's new native-narrow instruction
allows. Earlier App Clippy completed with an independently observed zero
exit status; later wording/test additions are not covered by that old
result. No Models runtime/UI/A9 pass or whole gate is claimed. Source is
restored onto the new registration seams next, then exact committed native
narrow checks run remotely. Disposable fixture nodes and the authenticated
virtual display used for preparation were stopped by verified PID/UID/exe;
no app capture run was started. Generated wire whitespace inherited from
the package base is kept unchanged.


## First package-base narrow result and compatibility repair

Exact candidate `3792ceba` reached remote native fonts (0), then Clippy
returned 101 because the assigned RPC error enum has a new local
`UnassignedProtocol` refusal. Both exhaustive adapters (Models and removal)
intentionally failed to compile until its meaning was handled. This remains
red evidence; no test or UI result is inferred from it. The first submission
was stopped before registration during the build tool's approved recovery;
only the same unregistered commit was resubmitted once, as instructed.

The new refusal is unsupported and definitely not sent, matching the
assigned base's other error adapters. Models preserves its typed source and
uses the existing unsupported meaning. Removal also preserves the source,
uses its existing unsupported category, and does not invalidate definitions
as if a mutation might have occurred. RPC admission in
`crates/vessel-rpc/src/client/protocol.rs` rejects it before request-id or
writer admission. The 30-second removal deadline, physical identity guard,
acknowledgement checks and no-resend policy are unchanged. Direct category
regressions cover the new source, including removal's non-uncertain verdict.
No pending method, dummy version or new RPC call is introduced.

## Actual-connection cache invalidation

The isolated connection-ready producer is now consumed. Its existing
`hand_kept` happens first; Models registers one typed notice in the same
bounded fan-out, before sessions/Fleet publication. No second hook or
generation counter is introduced. This timing alone proves neither that
the client will be retained nor that the device can read Models.

The current pure page accepts only its selected book route's notice,
records the handshake NodeId, and clears cached, pending and failed facts
on every real connection. An equal NodeId is still a reconnect. Ticket
allocation is not reset, so responses from the previous connection cannot
overwrite the replacement page. Unrelated machine notices are ignored.
The area clears a stale start refusal on that same accepted notice; normal
read results still decide Offline/Denied/Busy. Tests cover the shared
fan-out identity, equal-id reconnect, replaced node, stale responses,
unrelated notices and navigation. Changed-source narrow/runtime checks
remain pending; these rules are not a reconnect UI approval.


## Shared search field consumption

Models now uses the checked shared SearchField (V1 bounded source/requested
Kit narrow on13c46693). This replaces the temporary ordinary form surface, with
no area-owned Search/X drawing or second query state. Its horizontal full-width
rail preserves the reference max420 field; locale placeholder and selected-machine
reset keep the existing engine/selection lifetime. App compilation and actual
consumer captures remain separate from the control's two focused test results.
