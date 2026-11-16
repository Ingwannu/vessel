# Agent lifecycle parity

Labels, a background listing, a live reload, and selectors on `vessel log`.
The decision is ADR 0063. The protocol number is the one given at merge.
Calls and fields use `lifecycle_version::SINCE: Option<ProtocolVersion>`,
which remains `None` until integration; this branch does not
change `PROTOCOL_VERSION`. This track adds no `node.settings` key, and
it does not touch parking, rewind, fork, or `vessel-agent`.

The [Resume point](#resume-point) states the current package boundary. Earlier
implementation and verification notes retain the checkpoints at which they were
written; their pending checks are historical unless restated in that section.

## What Paseo does

Read first, and reproduced except for the differences ADR 0063 lists.

### Labels

- `packages/protocol/src/agent-labels.ts:2-6` reserves
  `paseo.parent-agent-id`, `paseo.parent-server-id` and
  `paseo.open-agent-tab.<client>`. Those are parentage and open tabs, not
  a person's marks. Vessel already records the creator and a side
  question, so it does not copy the reserved keys. A key under `vessel.`
  is refused so the node can use that prefix later.
- The wire value is `Record<string, string>`
  (`packages/protocol/src/messages.ts:915` on a list item,
  `:989-996` on `update_agent_request`, and the import request's
  `labels` at `:1898`). No maximum length or count is declared.
- A patch merges. `null` deletes one key
  (`packages/server/src/server/agent/agent-manager.ts:631-644`,
  `writeLabels` at `:2296`). The CLI and the MCP tool only send
  strings, so a person cannot delete. An omitted key stays.
- CLI update (`packages/cli/src/commands/agent/update.ts:103-140`)
  splits each flag on commas, trims each whole segment, ignores empty
  segments, then splits at the first `=` and trims the non-empty key.
  The value is the remainder of that trimmed segment, including empty.
- CLI run (`run.ts:460-466,478-501`) uses one complete `key=value` per
  flag; neither side is trimmed, and it does not reject an empty key.
  Import (`import.ts:54-82`) also takes one pair, but trims and refuses
  an empty key. All three keep the last copy of a key.
- Vessel uses that one-pair run form and the comma-separated update
  form. Its validated key trims and refuses empty at every ingress,
  including wire/MCP and filters. The reason is one canonical key type;
  a label must not change identity between creation and discovery.
- Paseo displays `Object.entries(labels)` (`update.ts:144-149`). Vessel
  uses canonical key order so stored metadata and summaries have a
  deterministic order regardless of how a sender built the map. Human
  summaries use comma-space, following existing device-scope lists in
  `vessel-cli/src/devices.rs`; Paseo's formatter joins with a bare comma.
- Paseo's provider-history import (`import.ts:110-123`) maps to
  `vessel run --structured --resume ID --label key=value`. Vessel's
  separate `session.import` transfer path copies the source's current
  label map when it starts the imported session, as it copies the title.
- CLI update refuses an empty name, an empty `--thinking`, and combining
  `--thinking` with `--name` or `--label`
  (`update.ts:152-197`). Labels alone are a metadata update.
- List filter is logical and, and only when asked
  (`packages/cli/src/commands/agent/ls.ts:126-153` and `:228-241`).
  A filter without `=` is ignored by the CLI parser. Vessel refuses it
  instead of dropping it.
- MCP: `create_agent` takes `labels` (`paseo-tools.ts:1185`);
  `list_agents.includeBackground` defaults to false (`:2133`);
  `update_agent` takes `name`, `labels` and `settings`
  (`paseo-tools.ts:2266-2298`). Settings-only still calls the metadata
  command, which reports nothing to update, and the tool ignores that
  report. Vessel's `set_labels` has no settings, so an empty patch is a
  typed refusal.

### Background

- `docs/agent-lifecycle.md:186-203`. `workspace.background` is discovery
  only. It is chosen at creation and cannot be changed later. Omission
  inherits the caller workspace, or `false` when there is no caller.
  Default lists, including history, exclude background work.
  `filter.includeBackground` includes it. An exact id does not consult
  the filter. `ls -a` includes archived and background
  (`ls.ts:114-150`).
- MCP `create_workspace.background`
  (`paseo-tools.ts:1396-1401`) and `list_agents.includeBackground`
  (`:2133`, default false) and `list_workspaces`
  (`:1526`, default false).
- Bound Vessel sessions derive an immutable `SessionListing` mirror from
  canonical workspace visibility (ADR 0082); conflicting explicit session
  overrides are refused by creation orchestration before work. Only legacy or
  unbound sessions retain an independent listing. The bridge is owned by
  workspace/session creation and is not implemented here yet.
- Paseo `run.ts:30-36,396-414` makes `run -d/--background` a hidden legacy
  no-wait alias, not visibility. The CLI owner retains it; list filters consume
  discovery, while workspace creation owns bound visibility. Archived sessions
  remain listed as before; discovery inclusion does not change archival.

### Reload

- CLI `packages/cli/src/commands/agent/reload.ts:27` ("restarts the
  underlying process"), `:66` `refreshAgent`, result `reloaded` plus
  `timelineSize` (`:75`). The app's tooltip is skills, MCPs or login.
- Wire `refresh_agent_request` (`messages.ts:1903-1907`) answers
  `agent_refreshed` with `timelineSize` (`:3949-3952`).
- Handler `packages/server/src/server/session.ts:4760-4838`: a live
  agent is interrupted, then `reloadAgentSession` with
  `rehydrateFromDisk: true`. A closed agent is loaded
  (`ensureAgentLoaded`) and its provider history hydrated. A missing
  record, an unavailable provider or a record with no persistence is an
  error. The worktree is not recreated.
- `agent-manager.ts:1625-1638` starts the reload. Labels, workspace,
  owner, launch env, created and updated times, last usage, last error
  and attention are kept. A running turn is cancelled first. If the
  cancel fails or times out, the turn stays and reload fails
  (`docs/agent-lifecycle.md:116`). The old process is closed before the
  new one starts; a close timeout keeps the old runtime
  (`agent-manager.ts:1764-1783`, "Timed out closing previous session
  during refresh"). After a successful close, a failed resume leaves
  the agent closed and retryable (`docs/agent-lifecycle.md:46-47`).
  `rehydrateFromDisk` deletes the in-memory timeline so provider
  history is read again (`agent-manager.ts:1627`, applied at `:1715`).
- Vessel's difference, and why: the log is the cursor
  (`crates/vessel-proto/src/event.rs` decision log). It is not wiped.
  Replay is dropped, as a move already drops it. `next_seq` is the
  count Paseo returns as `timelineSize`. A closed agent is not loaded:
  that is `session.resume`. A parked agent is not woken: that is
  `session.wake`. Both refusals name the call. There is no MCP reload
  tool in Paseo (`paseo-tools.ts` registers none); Vessel adds none,
  and refuses the RPC to agents.

### Logs

- `packages/cli/src/commands/agent/logs.ts:17-20`: `-f` / `--follow`,
  `--tail`, `--filter`, `--since`.
- `--since` is `Date.parse`; a failure is `INVALID_TIMESTAMP` and the
  example form (`:211-224`). `--tail` is an integer `>= 0`; anything
  else is an error (`:201-209`). `0` prints nothing.
- Filter (`:226-249`): `tools` is `tool_call`; `text` is a user message,
  an assistant message or reasoning; `errors` is type `error`;
  `permissions` is a type containing `permission`; any other string is
  a substring of the type. Vessel accepts only the four names and
  refuses the rest, so a typo is not an empty log.
- The permission filter has a representation difference: `logs.ts:238-240`
  checks timeline item type names, but `packages/protocol/src/agent-types.ts:
  375-389` has no permission item, and request/resolution are separate stream
  events (`:424-435`). The live callback consumes timeline items only
  (`logs.ts:384-393`), as does one-shot history (`:288-295`). Vessel selects
  its durable typed permission request and resolution records instead, so
  the named filter can show the approval activity present in its log. This
  is source inspection, not an executed Paseo behavior claim.
- Paseo's count parser is `Number.parseInt` (`logs.ts:201-209`), so a
  numeric prefix such as `2x` is accepted. Vessel parses the whole
  non-negative count and refuses a suffix; this prevents a mistyped count
  silently selecting another window. A non-number or negative count makes
  the parser return `undefined`, but the command explicitly refuses that
  result in follow (`:273-278`) and one-shot (`:299-304`); it is not ignored
  or replaced by a default. Vessel makes this refusal before connecting.
  Paseo connects and resolves the agent first, and one-shot fetches history
  before checking the count. Early validation avoids that unnecessary work.
  Paseo uses `Date.parse`; Vessel uses
  RFC 3339 with an explicit offset so machine locale cannot alter selection.
- The `--since` unit also differs: CLI `utils/timeline.ts:16-26` fetches
  projected history, then compares each item's timestamp; under the server's
  `agent/timeline-projection.ts:156,234,286`, that is the last merged tool,
  reasoning or assistant fragment's time. Paseo keeps or drops the whole
  projected item. Vessel compares raw records, the same unit as `--from`,
  `--tail` and retention, so output can start inside a message or tool
  lifecycle without its earlier fragments. No CLI projection is added:
  history and live use the same predicate, while Paseo's live path already
  compares each event (`logs.ts:385-386`).
- Follow prints the tail of what exists (default 10 when `--tail` is
  omitted, `:334-335`) and then streams. One-shot with no `--tail` prints
  the whole selection. Follow joins assistant and reasoning fragments
  (`createFollowTranscriptWriter`). Vessel already joins reply chunks
  in the CLI view; follow uses that, and does not reprint Paseo's
  banners for a replaced timeline. The log has no epoch.
- Tail's unit differs: the callers `logs.ts:60-71,314,355` pass the
  limit to `curateAgentActivity`. Under `packages/server/src/server/agent/`,
  `activity-curator.ts:84-90,135-144,189-194` projects before taking the last
  items; `timeline-projection.ts:543-554` collapses tool lifecycles and merges
  assistant/reasoning chunks. Vessel counts raw log records, including
  terminal and status records, before rendering. This preserves the existing
  `--from` and retention cursor unit in both modes. Tail can begin inside a
  streamed message; it does not count visible lines or complete messages.
  The same raw-record predicate order applies to timestamps and kinds.
- `activity-curator.ts:200-207` returns "No activity to display." when
  curation has no entries; one-shot prints that result (`logs.ts:314-315`).
  Vessel's existing renderer emits only selected records and real retention
  notices. A filter with no match is quiet, preserving stdout capture rather
  than synthesizing an activity banner. A CLI case checks both streams.
- Paseo's follow-start banner is also omitted: its history/live boundary
  writes the tail label and Ctrl-C hint to stdout (`logs.ts:362-363,402`).
  Vessel emits no boundary banner on either stream, so stdout capture stays
  session output and stderr stays status and genuine stream notices.
- Paseo waits for an explicit stop even after timeline activity ends
  (`logs.ts:401-407`). Vessel follows through the current session run,
  ending at the stream's final status or Ctrl-C. This preserves the existing
  session stream's explicit EOF; a later `session.resume` is a new follow
  invocation from the saved sequence. No new wire mode changes that contract.

## Existing assets

| Asset | Where | Decision |
| --- | --- | --- |
| Title, separate from the spec | `session.rename`, schema 6 `title` | Modify: labels follow the same split. Do not fold them into the title. Keys and values reuse the title's control check (`char::is_control`, including C1). |
| Marks as columns | schema 7 | Modify: the next schema append adds `labels` (JSON text, null is empty) and `listing` (`shown` default). The number is whatever the migration list's length is at merge. |
| Unit `session.list` | `rpc/requests.rs` | Modify: a struct that still decodes `params: null`. |
| `Call::since`, `Reply::for_protocol`, `SessionInfo::for_protocol` | `rpc/since.rs`, `rpc/older.rs` | Modify: new fields need `lifecycle_version::SINCE`; older replies drop them; new events have a `HarnessLog` stand-in. |
| `Method::forwardable` and its pinned list | `forward.rs`, `rpc/tests.rs` | Modify: `session.labels` and `session.reload` are forwardable, as rename and wake are. |
| Agent refusal | `vessel-node/src/workspace/policy.rs` | Modify: `session.labels` allowed; `session.reload` refused with its own reason, not the file-and-git sentence. |
| Stop and `session/load` | `vessel-session/src/moving.rs`, parking's wake | Reuse from a new reload order. Do not change move, park, wake, rewind or fork. |
| `session.resume` | protocol 10 | Reuse only as the retry named when a replacement fails, and as the refusal for a session that is not live. |
| `vessel log`, the view, `session.read` | `vessel-cli` | Modify the command and the view's match for the two new events. No new read RPC. |
| MCP `create_session`, `list_sessions`, `read_events` | `vessel-mcp/src/tools/sessions.rs` | Modify the first two. Leave `read_events` as it is. |
| `ErrorCode` | `vessel-proto/src/error.rs` | Reuse. No new variant, so existing matches stay. |

No external crate. The standard library and the types above hold the
bounds, the map and the timestamps.

The legacy text-column decoder cleanup is a separate refactor, with its
own review and quality-inventory update. It is not part of label behavior.

The larger creation payload is boxed only inside the existing RPC driver's
command queue, matching its already boxed session subject. A small command
factory keeps cancellation/unsubscribe slots compact without changing wire
frames, request handling or the public client API.

## Slices

The labels work is divided for review: 2a validating types; 2b creation,
current storage and older-peer projection; 2c list filtering; 2d actor
patches and their RPC/event; 2e CLI/MCP ingress. Each is gated separately.

1. **This commit.** ADR 0063 and this plan. No code, so `PROTOCOL_VERSION`
   and the schema stay at the base.
2. **Labels.** `LabelKey`, `LabelValue`, `SessionLabels`. Create, import
   copy, `session.labels`, `labels_changed` and its stand-in, the list
   filter, the labels column on the next schema append, CLI `--label`, MCP `labels` and
   `set_labels`. Tests beside the parser, the patch, the filter and the
   old-peer rewrite. The parser refuses `char::is_control` on a key and
   on a value, and one test names the C1 character U+0085.
3. **Background.** `SessionListing`, creation inheritance, the `listing`
   column on the same schema append as the labels column when that
   append has not shipped yet, otherwise the append after it.
   `session.list`'s `scope`. CLI
   list inclusion filters are consumed by the CLI owner; the run no-wait alias
   keeps its legacy meaning. Bound creation visibility is the workspace owner's
   flag. MCP `listing` and `scope` cover legacy/unbound metadata.
   Tests: default hides, `all` shows, an exact get still finds it, an
   older peer is not filtered and does not see the field, an agent
   inherits, a person does not.
4. **Reload.** `session.reload` and `reloaded`, implemented in
   `vessel-session` beside move, using the existing stop and
   `session/load`. Refusals in ADR 0063, each with a test that no
   process was spawned or, for the cancel timeout, that the original
   process is still the session's. A failed replacement leaves the
   session `failed` and the log readable. Agents get `forbidden`.
   If this cannot be done without editing `vessel-agent`, stop and
   report; do not edit it.
5. **`vessel log` selectors.** Parse before connect. Order is from, then
   since, then filter, then tail. Follow waits on the stream. Tests use
   the testkit's fakes and an event, never a sleep and never a real
   agent.

The commit that first changes a wire type adds `lifecycle_version::SINCE` and
gates the new calls and fields on it. It does not change
`PROTOCOL_VERSION`. The integrator sets that one constant, and in the
merge updates `docs/PROTOCOL.md`. The same code commit regenerates
`web/src/wire` and extends the store sentence in `docs/ARCHITECTURE.md`.
No row of the crate table changes.

## Tests the contract requires

- A key at `LabelKey::MAX_CHARS` is kept; one scalar over it, empty, U+0000, the C1 character
  U+0085, `=`, and `vessel.` are `invalid_request` and write nothing.
- A value at `LabelValue::MAX_CHARS` is kept; one scalar over it, a newline, or U+0085 is refused. An empty
  value is kept.
- Adding beyond `SessionLabels::MAX` is refused and the previous map stays.
- A patch changes the keys it names and no other. `{}` is refused.
- `session.list` with two pairs returns only a session that has both.
  `params: null` still decodes.
- A background session is absent from a current default list, present
  with `scope: all`, and present in `session.get`. An older connection's
  list includes it and the encoded info has no listing field.
- `session.create` with `listing: background` sent toward a negotiated
  protocol older than `lifecycle_version::SINCE` is refused by the client before
  the socket write.
- Reload of a terminal, a parked session, an ended session, and a
  session with no harness session id does not spawn. A running turn
  that does not end within the grace does not spawn a replacement.
  A replacement that fails readiness leaves one `failed` session and
  the old records in place. Success keeps the harness session id and
  appends `reloaded` without duplicating the earlier conversation.
- `vessel log --tail 0` prints nothing. `--tail -1` and `--filter typo`
  and a bad `--since` fail before connect. Follow's first print is the
  last 10 matching records when `--tail` is omitted.

## Existing interrupt-loop repair

- The read-only script output path in `attach/mod.rs::print_output` and
  `terminals.rs::ports` recreate `ctrl_c()` within each select. As with the
  corrected log loop, the receiver disappears during an awaited write.
- Decision: modify each loop using the existing Tokio dependency: construct
  and pin one future before the loop, polling interruption first. Keep script
  detach, port revisions, rendering, reconnects and watched resources intact.
  A shared controller or a new dependency would add no needed behavior.
  Both loops now retain that future; interruption is polled before data.
- The existing owned-child cleanup and executable-checked SIGINT helper are
  first shared by the tests, with existing log assertions unchanged.
- This is a separate corrective slice from log follow. Tests use owned CLI
  children and real signals against temporary fakes/scripts. A claimed
  signal-during-write case must acknowledge that write deterministically;
  otherwise report the unforced interleaving explicitly, without adding a
  product-only test hook or using sleeps as synchronization. The Linux cases
  acknowledge real script output and a changed ports snapshot before SIGINT,
  then check successful exit and the still-live script/session/listeners.
  They cannot force a specific pending write through the public harness.
  The script executable is the existing fake in sleeper mode; only approved
  project configuration is written, with no generated executable script.

## Shared base defects and deferred wiring

- The existing move stop helper (`vessel-session/src/moving.rs::stop_agent`
  and its local `signal`) logs `SignalError` and continues. This is a base
  defect relative to the reload contract's refusal when stopping fails. A
  separate forcing-test-first correction is required before reload can reuse
  it; copying it would risk starting a replacement while the old process lives.
- Event availability has one public optional accessor, following call admission.
  Remove the numeric `SessionEvent::since()` when introducing
  `required_protocol()`: there is no truthful numeric value for an unassigned
  event. Existing event thresholds and content-dependent agent thresholds stay.
  Both the event projection and page precheck use the optional requirement;
  `None` is unreadable by every peer, with the existing gap-free stand-ins.
  Introduce this foundation separately, before `LabelsChanged` or `Reloaded`.
  Its tests cover every currently assigned session variant and retain the
  existing projection/cursor assertions. Actual pending-variant tests land
  with those variants; this foundation does not add an event or feature number.
- The latest merged node head now has the shared request accessor. Future
  call and field admission must use it and the one method table. Work held
  for the shared call-plumbing batch remains staged until that batch; event
  availability still waits for the protocol owner's named shape.

## Existing label request admission

[Decision Log]
- Purpose: close the temporary node-only admission boundary now that the shared
  optional protocol accessor is integrated.
- Existing assets: `Call::required_protocol`, its SDK-before-writer check and
  RPC-before-dispatch check already carry typed unassigned refusals. The five
  existing spec-bearing methods and three envelopes use those assets.
- Alternatives: a second SDK check or fabricated numeric threshold would split
  the protocol rule; neither is needed. No dependency is added.
- Choice: modify the shared accessor's field requirement, keeping `since()`'s
  signature and numeric compatibility calculation. Nonempty labels return
  the optional assignment combined with the existing requirement; empty or
  absent templates preserve it. Schedule session targets remain unchanged.
- Effects: the SDK can now refuse before a frame is sent, while raw callers
  are refused before dispatch. Starting import still checks stored labels
  at the node, because that metadata is not part of its request.
- Tests planned: each of the five direct requests, Forward/Relay/Deliver and
  nested combinations at the named current protocol, a synthetic assignment
  threshold, SDK no-frame admission and raw-server no-dispatch admission.
  The existing node case uses a raw websocket so SDK admission cannot hide
  the server boundary. Its reason is exact equality with the shared typed
  unassigned error; the server now refuses before the old node guard. Its
  Unsupported and no-work checks remain, and starting import is unchanged.
- Narrow verification passed: protocol/RPC clippy; all 15 label-selected
  unit tests, including the four new admission cases; node e2e clippy and
  both real-node label-ingress cases. The complete gate remains owed on
  the final admission commit. No failure was retried into green.
- Paseo contract remains the labels record and optional update at
  `messages.ts:915,989-996`, and reserved keys in `agent-labels.ts:2-6`.
  This is the already stated temporary Vessel difference: labels await their
  client consumers and protocol assignment instead of silently disappearing.

## Stop refusal prerequisite for reload

[Decision Log]
- Purpose: refuse replacement startup if the current agent cannot be signalled.
- Existing assets: reuse the real structured actor's stop, output drain and
  `try_move` error boundary. A reload must not duplicate this lifecycle.
- Constraint: the OS refusal cannot be forced through the fake ACP protocol;
  a vanished or reaped process is correctly treated as a successful stop.
- Choice: a per-actor `cfg(test)` next-stop fault, following the existing
  `Control::PrivateDir` test boundary, supplies one typed `PermissionDenied`.
  The actor, profile linking, fake process, log and restart remain real.
- Alternatives: another account's process, permission changes, global fault
  state, a real harness or a release-enabled fake are rejected. No dependency
  is needed; the session crate already owns the process signal error.
- Sequence: commit the forcing test against log-and-continue first and retain
  its expected red result; then return a typed move error with the source.
  Successful stop/drain behavior, deadlines and existing expectations stay.
- Product absence is a separate product-configuration check: the exhaustive
  control match and actor constructor have no test fault arm or field there.
  The test failure, corrective narrow checks and one package gate travel
  together. This is one prerequisite package, not a working reload command.
- Resume: the event accessor's corrective clippy and all 262 protocol units
  passed. Its first narrow compile failure (remaining old test accessor calls)
  is retained. The isolated stop-refusal regression failed against the old
  log-and-continue policy: after the unchanged kill grace it incorrectly
  completed the move. The corrective commit returns `MoveError::Signal`
  before replacement, preserving the requested signal and typed OS source.
  Both polite and forced stop delivery use that refusal boundary. The test
  now asserts the specific source/action, Internal code and original process,
  profile, conversation and responsiveness. Corrective clippy and all 61
  session library tests passed, including the previously failing regression.
  The event projection's profile/model fallback formatting is extracted in
  a separate behavior-preserving commit to keep its dispatch short; messages,
  thresholds and every existing test expectation remain unchanged.
- Final narrow checks passed: combined protocol/session clippy and all 323
  library tests, including the forcing regression after correction. The
  separate product library check passed with a non-test compiler artifact:
  its actor constructor and exhaustive control dispatch compile without the
  `cfg(test)` field, control and fault branch. The designated follow/display
  head is merged, and this package now owes its one complete gate.

## Typed label entry access for app consumers

Consumers read `SessionLabels::iter()` as an exact-size iterator of
`(&LabelKey, &LabelValue)`, in canonical key order. No generic session-label
editor is introduced by this accessor. Its `len()` is the count
of remaining entries. This extends the existing validated map, with no JSON
tree, summary parsing, mutable map access or new wire field. Values retain
spaces, commas and equals signs. This accessor does not add the pending
`session.labels` mutation method or workspace label catalogs.

## Proposed session discovery request contract

The protocol owner accepted this existing-wire-type change with required wire
scope and named Rust constructors. The method-table integration head is now merged. The request conversion and
its optional field rule are implemented, along with durable listing, source
inheritance, selected discovery and import preservation. Their package checks
are in progress; the separately integrated CLI/MCP consumers and canonical
bound-workspace bridge remain outside this implementation.

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct ListSessions(Option<SessionSelection>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SessionSelection {
    pub scope: ListingScope,
    #[serde(default)]
    pub labels: SessionLabels,
}
```

`ListSessions::default()` is `None`; its wire body remains `null`.
`ListSessions::selected(selection)` constructs `Some(selection)` and
`selection()` borrows it. `SessionSelection::shown()` and `all()` are
named Rust constructors with empty label filters; `with_labels(labels)` adds
the validated predicate. The selection has no Rust Default and its wire
`scope` is required: `{}` or a labels-only object is invalid. A valid present
selection requests the new feature. Missing or unknown scope and invalid
labels fail during decoding. No second request type, row or boolean scope is
introduced. On the method-table integration head the field rule belongs in
`Call::since() -> Option<ProtocolVersion>`: a present selection contributes the
feature constant, and an unassigned constant makes the whole call None.
`required_protocol()` delegates to since; it gets no second field check.

| Caller or body | Meaning | Admission and compatibility |
|---|---|---|
| Existing Rust unit-value caller | Complete only the value to `ListSessions::default()` | Existing assertions unchanged in the field-introducing commit |
| `params: null` / `ListSessions::default()` | Legacy full list, background included | Existing method requirement; works with old nodes and preserves the wire shape |
| `params: {}` / labels-only object | Invalid: required scope is missing | Decoding refuses before dispatch; no silent Shown choice |
| Present `scope: shown` | Shown sessions, no label predicate by default | Unassigned feature is refused; after assignment requires its threshold |
| Present `scope: all` | Both listing kinds | Same feature gate; explicit unsupported requests are never discarded |
| Present nonempty labels | Every pair must match current labels, after scope | Same feature gate; filtering preserves existing list order |
| Existing SDK `sessions()` | Keep the legacy full-collection API | Only its Rust value expression changes, not its behavior |
| CLI default list with supported assigned metadata | Send present Shown selection | Default discovery hides background work |
| CLI default list before assignment / on an older connection | Keep the legacy null path | Compatibility difference: that peer cannot supply listing metadata; no explicit filter is dropped |
| CLI explicit background inclusion or label filter | Always a present selection | Unsupported/unassigned is an error, with no silent fallback |

[Decision Log]
- Purpose: distinguish old full-list reads from new default discovery without
  breaking old `params: null` callers or treating omission as an explicit flag.
- Existing assets: extend the current `ListSessions` row and shared optional
  field admission, the validated label map and the existing list service.
- Choice: an optional typed selection in the same request; its required
  scope owns inclusion, and labels are an AND predicate. Named constructors
  keep Rust choices explicit. No dependency is needed.
- Alternatives: changing null to Shown would silently change old clients;
  sending an empty object to an old unit request would break decoding; a new
  method or second request alias would duplicate protocol knowledge.
- Consequence: defaults use the existing legacy behavior when metadata cannot
  be negotiated, while explicit new requests require the feature. Paseo has
  no negotiated Vessel legacy null request; this compatibility difference is
  deliberate. Its background discovery defaults still apply on supported peers.

## Background value staging

- Existing asset check: `DirListing` describes a directory, and `UsageScope`
  chooses usage aggregation. Neither models session discovery. The existing
  session listing merges live and stored information; that service and its
  ordering will be extended through the now-integrated shared admission accessor.
- Decision: new value types, using the existing serde and TypeScript derives,
  with no dependency. `SessionListing` separates shown/background discovery
  from run state; `ListingScope` separates default/inclusive reads. A generic
  boolean would lose that distinction at a module boundary.
- Paseo's source remains the reviewed background contract:
  `docs/agent-lifecycle.md:186-203`, CLI `ls.ts:138-151`, MCP
  `server/agent/tools/paseo-tools.ts:1396-1401,2133`, and
  `server/workspace-registry.ts:106,684-687` (durable default false).
  The stated difference remains session metadata rather than workspace
  visibility, because a shared project is not one agent workspace.
- These are values only. No request, reply, event, field, migration, filter
  service, caller inheritance or CLI/MCP input is added here. A future creation
  field must be optional; the value's default does not erase an omitted
  override. Exact-id operations will continue to bypass discovery filtering.

- The initial value-only branch passed TypeScript-enabled narrow clippy and
  three listing/default/serde tests. Its queued complete gate was cancelled
  before execution. Under the package workflow, these values now belong to
  the complete background feature and receive no separate value-only gate.

## Resume point

Current boundary: the canonical clock passed source review and its integrated
owning gate: eighteen stages passed, Rust 2862/0/12, web unit 465, browser e2e
80/80 and forge fixture 4, with fresh generated declarations. The clock's schema
is appended after listing. SESSION_MUTATIONS remains unassigned. This qualifies
the producer boundary, not label mutation, coverage, reload or consumer policy.

Next: attention-mark parity. Loaded Read/Unread preserve the clock; stored
attention/seen transitions tick, while repeated marks and watermark-only reads
preserve it. The existing Node residency branches decide under the same marking
lease as reopen. No SQL live/status inference or second clock is introduced.
The workspace selection consumer waits for this correction. Quiescence code
follows this package; its accepted contract alone is not a native proof.

The title-origin node head is merged before background fields. Its origin
projection remains authoritative; readable events use the existing origin mask,
and event pages detect actual projection changes. This also incorporates the
head's fix for the previously reported Replayed observation page inconsistency.
The legacy labels migration test already follows the table's latest version.

Current package boundary: legacy interrupt repair, label admission and the event/stop
prerequisite passed review; their integrated gate debt is also settled. The
background package implements validated listing/scope/selection values, the
nullable request, durable effective metadata, source-node inheritance, selected
current-label reads and source-preserving imports. The migration runner and
legacy SDK full-list behavior remain unchanged. Explicit discovery and metadata
are refused by the shared accessor while the assignment remains pending.

Generated declarations preserve unrelated integrated types. Metadata projection
covers infos, lists, subscriptions, transfers, templates and envelopes. The
strict check found fixture Arc/SQL-result and helper-reference defects, which
are corrected. The source correction for rejected-transfer cleanup passed review.
Scoped checks passed on their exact revisions: generated declarations at 58f9d7af
(four tests and no generated diff), node library tests at e292aa24 (267 passing),
and actual Forward/Relay cleanup wiring at f0578a20 (one passing regression with
four cases). These are not one current whole-package result. Admission of the
package requires a corrected changed-crate strict check and one complete root
gate; the handoff records their exact revisions and results. Integration and
consumer readiness require that complete evidence, rather than any scoped result.
The typed iterator belongs to this package.

The independent public call preflight passed its narrow, complete root gate and
review. Its approved legacy run anchor now checks the creation call after
connection but before placement. New CLI metadata/filter options and MCP
facades are separate consumer integrations. A real-node regression also covers
hidden durable listing and the dynamic import refusal before target-directory
work. That integration case belongs to the complete package gate, rather than
the earlier node-library checks.

Canonical workspace visibility is the authority for bound sessions; creation
orchestration still owes that mirror/override bridge. Independent listing and
source inheritance apply to legacy/unbound sessions or an already fixed mirror.
No label mutation call or LabelsChanged event is implemented yet. Actual reload
also remains unimplemented: a reviewed producer with proved transient-prompt
support/capture/restore precedes that common orchestration. No real-provider,
platform or end-user metadata workflow is claimed.

Current label-mutation preparation: `LabelPatch` and its five edge tests are
implemented. The corrected TypeScript-enabled prototype Clippy passed on its
exact pre-merge revision; runtime tests remain pending. The shared node base is
merged without conflict. A separate refactor wraps the existing rename queue as
one metadata queue, with its response and capacity unchanged. No label mutation
method/event, actor/store write or feature assignment exists yet. Earlier notes
below retain their historical checkpoints.


- Reviewed: slice 1 contract and slice 2a validating types. Bounds,
  diagnostics and boundary tests use each type's named constants.
- The named node head is merged without rewriting earlier commits. Its
  usage ledger, shared compatibility cases and document paragraphs remain. The
  latest announced head is merged at the next clean slice boundary as well;
  its protocol constant, migration order and assigned labels schema win.
  Protocol assertions use the named constant. The only manual conflict keeps
  the head's inbox argument module/export and adds the log module/export.
  The subsequent accessor head is merged before implementation begins. Its
  existing Log export stays, with the LogMode export appended; no head
  protocol, schema or expectation is changed.
  The later follow/display head is merged at another clean boundary. Its
  grouped Log/LogMode export wins the formatting-only argument conflict,
  and its workspace display source and expectations remain unchanged.
- Slice 2b keeps creation/current labels separately in durable storage.
  `lifecycle_version::SINCE` remains unassigned (`None`); both maps are
  hidden in direct replies, lists, subscriptions, saved/scheduled specs,
  transfers and forwarding. Event pages have no label record yet.
- Existing session/prompt/schedule inputs and starting imports refuse
  labelled metadata before work. Tests assert the unassigned-protocol
  reason and reopen the separate durable maps.
- The earlier corrective wire gate passed: Rust 1978 passed, none failed,
  12 ignored; TypeScript generation and diff, web checks, 417 unit tests
  and 62 browser tests passed.
- The store review correction appends labels to the migration table and
  leaves the migration runner byte for byte unchanged. `LABELS_SCHEMA`
  binds its provisional array length and the legacy-row/null/idempotence
  test. Integration assigns the number. Head rollback SQL is unchanged,
  preceded by the label-column rollback.
- At the merge onto the node line the labels took schema 17 (the inbox
  had 16), in the head's `schema/migrations.rs`, where the migrations'
  SQL lives since the store split.
- That correction's whole Rust gate reported 1978 passed, one failed and
  12 ignored. Its sole failure was the shared proxy deadline test's outer
  budget; the authorized single exact confirmation passed. Both results
  were submitted separately, preserving the initial failure. Other steps
  passed; the correction changes no wire or page surface.
- The shared `required_protocol()` accessor is now integrated. Its existing
  client/server checks will admit or refuse embedded labels using their
  optional assignment; the admission follow-up is being implemented. No new
  registered call or second client mechanism is introduced by that follow-up.
- Separate refactor: the three legacy Profile/Via/Title text-column adapters
  use a consuming serde string deserializer. The validating types and typed
  column/source errors remain; the quote/backslash, title trimming and
  corruption tests are unchanged. `QUALITY.md` records the removal without
  guessing a new global debt count. No dependency is added.
- The independent decoder refactor passed its narrow store checks and whole
  Rust gate: 1980 passed, none failed, 12 ignored; both prebuilds, format,
  size, boundaries and clippy passed. No waiver or rerun was needed. The wire
  and page were unchanged. Later label filtering, actor patches, CLI and MCP
  remain protected drafts until the shared accessor and call plumbing are
  ready; staged services are not a product build claim.
- Slice 5a tail passed its complete Rust gate on an independent build runner:
  1984 passed, none failed, 12 ignored. Format, size, boundaries, clippy,
  both prebuilds and dependency setup passed. Local narrow checks passed too.
  Zero, window eviction, raw chunk boundaries and the real CLI tail were
  covered. The original module-path lint was fixed before the whole run.
  The count is parsed before connection; retained records allocate on arrival.
  Tail uses the existing sequence/retention unit and can begin inside a message.
- Selection slice 5b's first narrow compile found two test-only errors: a
  failure fixture bypassed the existing status constructor, and a turn cursor
  was unused. Both are corrected without changing assertions; the narrow
  clippy, ten log units and two fake-node CLI cases passed. Its complete
  Rust gate also passed: 1989 passed, none failed, 12 ignored, plus the two
  TS freshness tests. Both prebuilds and all applicable checks passed; no
  waiver or rerun was needed. Forge checks were skipped because this base
  has no forge-fixture feature. The wire and page are unchanged.
- Review correction: the timestamp floor's raw-record unit, midpoint effect
  and history/live consistency are now explicit in the ADR, plan and help.
  This changes documentation only; the complete code gate remains valid.
- Slice 5b passed its complete gate: time/kind predicates precede
  the existing tail window. Sequence gaps separate message chunks, but only
  raw page gaps generate retention notices. Parser, predicate, render-boundary
  and fake-node selection tests passed in the narrow runs. Existing render
  expectations stay unchanged. A no-match CLI assertion also checks that no
  banner is fabricated.
  The inherited stream-end wait and the new turn-end wait share one named
  event budget; their expected records and output remain unchanged.
- Follow review found that recreating a Ctrl-C future for each iteration
  drops its receiver during output. The corrective slice first shares the
  test child/capture setup without changing any expectation; it will then
  retain one interrupt future across writes. The interrupt branch is polled
  first so its listener exists before the first live write. A Linux CLI case
  verifies the owned child executable before SIGINT, successful exit and the
  still-live fake session. It cannot force the signal-during-write interleaving.
  Corrective clippy and all four log CLI cases passed, including actual
  SIGINT, successful CLI exit and the still-live fake session. The new
  complete gate remains owed.
- Implementation slice 5c, verification pending: follow reuses these predicates,
  the tail window and the raw history cursor. Its omitted history count has
  one constant, also used by the CLI help. The CLI edge owns the execution
  mode; the selector module only owns predicates. Follow test turn cursors
  are explicitly ignored, without changing its output assertions. The ADR
  names the raw record cursor rather than the last visible match.
  EOF, interruption and stream notices finish an open text line; terminal
  bytes stay untouched. Follow passed narrow clippy, eleven log units and
  three fake-node CLI cases, including history-to-live output exactly once
  and EOF. The prior complete gate passed 1991/0/12, but the listener fix and
  signal case passed their narrow checks and need a new complete gate.
- The contract's follow bullet and EOF explanation now name the same end.
  Parser and command-level checks are documented together: an invalid tail
  is refused by both products, with earlier validation in Vessel.
- A source-preparation script continued after a failed conflict-resolution
  step and left conflict markers in an unverified follow commit. A new
  follow-up repaired both owned files with every prior scenario retained.
  That intermediate commit was never compiled or used as gate evidence.
- The corrected follow gate passed 1992/0/12 plus its two TS freshness tests,
  with no waiver or rerun, and was submitted for review. The next clean slice
  begins on the newly announced node head. The older attach/ports interrupt
  repair passed narrow clippy and all five CLI cases selected by interrupt,
  including both new script/ports signal cases. They check normal real signals
  and resource survival, with the exact write interleaving still unforced.
  Its separate complete gate remains owed.
- Next: finish the older interrupt-loop repair's complete gate and the
  existing label-input admission checks through the integrated accessor.
  New event availability and call plumbing held for the quality batch still
  wait for their shared owners. No new wire call or dependency is added by
  log options or the label admission correction. All slices include the caller-to-projection
  citations and retain the raw-chunk test; the selection render context changes
  without changing its expected fragment. Windows, macOS, native UI and real harnesses remain
  unverified.

### Reviewed prerequisite landing on the integrated method table

This merge carries only the reviewed label admission and reload prerequisite,
without discovery values, typed iterator or other background work. Readable
session events keep the head's title-origin mask, with the single optional
availability accessor and string-preserving formatters. Event pages retain
the head's projection-based precheck, including the Replayed usage mask fix.
The existing label requirement is folded once into `Call::since()` after its
assigned method/field requirement; `required_protocol()` and fleet recursion
remain the head's. The head's newly added title-origin event assertion uses
`Some` around the same assigned version under the single-accessor contract.
The owned event fixture fills its new origin field with Unknown; its previous
wire/sequence/value assertions are unchanged. A new exact-tree gate is owed.

The labelled-request invariant now asserts `since() == None` as well as
`required_protocol() == None` for every direct and nested call. Its previous
numeric-accessor comparison is obsolete on the integrated optional API; this
strengthens the pending check without changing any assigned expectation.


## Shared preflight for validation before work

Existing assets: the RPC client already checks the call's optional protocol
requirement before allocating a request id or writing. Expose that same check
through `Connection::check_call` and the SDK delegate so a CLI can refuse before
creating a worktree or other preparatory side effects. `start` uses the wrapper,
so there is still one condition and one error mapping. The wrapper allocates
no request id, enqueues nothing and grants no node authority; send-time checking
remains mandatory. A raw-memory peer test checks exact refusal and that the next
valid request is its first frame with id zero. No wire type, number, threshold,
message or dependency changes. This small seam lands independently of pending
background work so other callers can reuse it. Its strict crate lint and complete root gate passed. The first-frame
regression passed with the unchanged send path; platform-specific consumers
and actual preparatory-work callers remain separate integration.

Before adding the preflight delegate, the SDK's existing `protocol()` accessor
moves unchanged into `client::protocol`. This behavior-preserving extraction
keeps endpoint calls within their shrinking size budget; it changes no public
signature, value or assertion. The preflight wrapper follows as its own commit.

The following commit adds the thin wrappers and the exact first-frame test.
The shared checker, allocator and writer behavior are unchanged. The complete root gate passed on this implementation, including the first-frame
regression, TypeScript freshness, web checks and the forge fixture. This API
adds no CLI behavior until its separate consumer calls it.



### Optional call admission after the method-table merge

The label rule now contributes to `Call::since()` after the method and other
field requirements. A pending label assignment makes that one accessor return
`None`; `required_protocol()` delegates unchanged. The old numeric label
contributions are removed, so a label is considered once. Fleet envelopes keep
the head's recursive optional requirement, and all other thresholds stay as
written there. The labelled-template fixture asserts both accessors are pending
for every direct and nested request. The whole-method assigned-request invariant
keeps the head's optional expectations.

The typed label iterator is borrowed and exact-size; callers can count remaining
entries without parsing the display summary. Its first narrow check found a
missing must-use annotation and a newly added title-event test using the removed
numeric event accessor. Both are corrected; a new narrow check is owed.

### Session-template boundary naming

Before extending discovery metadata, the optional admission and older-peer
modules are named for their responsibility: session templates. This separate
refactor only renames modules and the internal synthetic-threshold projection
helper. It changes no value, expectation or wire representation. Label rules
and their tests remain byte-identical in the renamed files.

### Durable listing and template projection

The background append follows the head's newest migration through one named
constant; its table length is checked by the array type. The production runner
is unchanged. Existing rows default to Shown without rewriting their specs.
The codec reads the immutable effective listing independently of the requested
override; corruption keeps the session and column in its typed error.
The original approval/device/log/marks and inbox rollback cases gain only this
append's DROP before their existing SQL. The title-origin fixture first undoes
this later append and its version, then runs the head's relative rollback and
assertions unchanged. Reopening, idempotent migration, corruption and legacy
spec retention have new tests.

Metadata projection now hides listing together with labels on every existing
info/transfer/template path. Listing-only creation fixtures exercise shared
admission too, so an explicit Shown override cannot disappear at an older hop.
A synthetic assigned threshold retains the effective and requested listing for
a new-enough peer. These new checks still need execution; creation and discovery
service wiring remain next.

### Creation inheritance and scoped discovery service

The canonical session collection still includes both listing kinds. A selected
read filters a fresh copy by scope and then by every current label pair, keeping
its order. Exact-id operations and the existing SDK full collection are unchanged.
Real store/runtime tests include live and ended background sessions, current
labels different from creation labels, explicit Shown overriding inheritance,
unknown-parent refusal and direct access after a filtered read.

The authenticated source resolves an omitted Background before forwarding a
CreateSession, after the existing forwardability and scope checks. Shown stays
the legacy omission; explicit values are untouched. The destination treats
a remote-agent omission as Shown, without a remote-parent lookup or new Origin
field. This forwarding distinction has no Paseo counterpart and preserves old
ordinary creation while refusing a new Background field at pending/older hops.
A local scheduled run inherits then, not when its template is saved. The existing
scheduler limit on remote definers remains. Dynamic manifests refuse pending metadata before files are fetched. Import
spec assembly will first be extracted without a behavior change to make room
for explicitly retaining its source listing. Side-question overrides are refused before parent lookup instead
of being discarded. The source inheritance boundary is security-relevant and
needs its commit reviewed; no trust or scope is added. CLI/MCP wiring is next.

### Import specification assembly extraction

Before adding the immutable listing copy, import's existing spec assembly moves
to `transfers::import::session_spec`. This keeps the import transaction orchestration
within its size budget without raising a record. The extraction takes the same
validated harness, resume id, ordered choices, target cwd and profile; it copies
the same model/current title/current labels. No value or test expectation changes.
The existing import assertions remain the refactor's behavior contract.

The following metadata change copies the import source's effective listing
explicitly in the extracted spec builder. Both Shown and Background override
caller inheritance, including a source whose original spec omitted it. A focused
unit checks this with current labels and the same resume id. The dynamic starting
manifest guard remains before file fetching; no import deadline or existing
assertion changes.

### Canonical workspace authority and consumer ownership

Workspace visibility is the one authority for bound sessions (ADR 0082). The
creation owner resolves it into `SessionListing::{Shown, Background}`, refuses
a conflicting explicit `SessionSpec.listing` before any work, and writes the
immutable mirror read by this package. Its reviewed producer/adapter is still
pending: no bound bridge is claimed. The source-node inheritance rule applies
only to legacy/unbound creation or to an already fixed mirror. The registry and
workspace DTOs are not edited here.

The CLI owner consumes the discovery contract and owns new flags/creation flows.
Only the approved legacy run preflight line is added by this track. Paseo's
run background spelling is a legacy no-wait alias, as its command and caller
confirm; it is never repurposed as a visibility flag. The earlier contrary
plan promise is corrected. Native views, bound creation and CLI integration
remain separately owned consumer work before protocol assignment.

The first core narrow lint refused redundant SessionSpec qualifications in
the template rule and its test. They are removed without changing behavior
or expectations; a fresh corrected-source check remains owed.

### MCP consumer boundary

This node package supplies validated creation metadata and the nullable discovery
request. MCP decoding, schema and facade integration belong with the tools
consumer, so no second decoder or fallback implementation is shipped here.
The existing MCP full-list call only changes its Rust unit-value spelling to
`ListSessions::default()`, preserving null and its behavior. The consumer must
use the shared admission accessor after its integrated head, and explicit
selectors must never disappear. Its metadata ingress tests remain consumer work.

The discovery fixture gives stored rows distinct fixed timestamps. The existing
canonical sort promises newest-first, not an order among equal timestamps, so
this prevents a test from depending on a fresh hash map's tie order. Expected
selection contents and ordering requirements are unchanged. Key-cap atomicity remains covered beside the shared label map itself.

### Integrated admission and discovery merge

The integrated method table and request-contract modules are kept in their
new locations. The existing optional label rule remains one final field rule:
the session-template module name and its typed spec input also let listing
contribute there, without another accessor or numeric fallback. Both sets of
architecture and plan additions are retained. The integrated event projection,
title-origin masks and shared test expectations stay unchanged. Narrow checks
and fresh generated-wire bytes are still owed for the discovery package.

### Generated discovery declarations

The generator-produced declaration delta adds the nullable selection, scope,
listing and metadata fields. It applies on top of the integrated declarations
without replacing unrelated generated types. The source generator tests passed;
the original stale result remains a freshness failure until these bytes are
committed. A fresh integrated-head TS check and the package gate are still owed.
The MCP decoder draft is excluded; its consumer owns that separate integration.

### Dynamic import regression

A new real-node test seeds effective Background and explicit Shown metadata
while the wire assignment is pending. Legacy null discovery retains both ids,
replies hide both metadata fields, and starting imports receive the exact
listing refusal before creating their target directories. Reopening checks
that the effective classification and original omission/override stay durable.
The existing label import test and all its assertions and deadlines are kept
unchanged; this separate case covers the additional discovery boundary.

### Legacy run preparation preflight

The existing run path now calls the reviewed SDK checker immediately after
connecting and before resolving or creating its workspace placement. It checks
the parsed creation specification through the same typed CreateSession call;
the eventual send still checks again. No new option, parser or discovery
consumer is introduced here. The separate CLI consumer must prove its new
metadata flags cannot create a worktree before a pending/older refusal. That
end-user path is not claimed by the raw SDK regression alone.

### Discovery narrow-check corrections

The integrated strict check found two fixture defects and a private helper's
reference convention: append records are shared Arc values, the corruption
fixture must consume and assert its SQL row count, and the import spec builder
takes Option<&ProfileName>. These corrections retain imported profile values
and existing expectations; both insert and corruption fixtures now assert
that their SQL changed exactly the intended row. The failed
check is retained, and corrected-source checks and the complete package gate
are still owed.

The legacy schema fixture reads SQLite's user_version as its supported i64
representation and converts the migration length with TryFrom for comparison.
The earlier usize read did not compile; no schema value or expectation changes.

### Peer-safe export before metadata projection

Source review found a remote-import gap: an unreadable export reply hides
metadata before the destination's dynamic manifest check can identify it. The
source guard will reuse lifecycle visibility on its original current labels and
classification, after ownership and before provider history/transfer work.
Forward and Relay will apply the same guard to the returned session manifest
before their connection's projection, preserving the three-hop case with a new
source/home and old importer. No request, origin, shared accessor, schema or
reply projection changes. The ADR states the necessary non-starting export
refusal and the difference from Paseo's caller-labelled history import.
Tests precede the fix: real-node source refusal, default metadata unchanged,
synthetic assignment/older visibility and the three-hop return boundary.

The first source-export regression seeds durable labels and Background without
a provider conversation. The exact typed protocol refusal must precede the
otherwise-later missing-history error, and the stored metadata must remain
unchanged. This test is committed before the guard; its red run is still owed.

Before the export guard, the existing info/turn/terminal snapshot lookup moves
unchanged into a private exporter method. This separate behavior-preserving
extraction keeps export orchestration short and lets the peer guard validate
the same snapshot placed in the manifest, rather than read/check then re-query.
Existing fields, refusal ordering and test expectations are unchanged.

The corrected-source strict check also requires the discovery runtime tests to
consume testkit's deadline Result: a timeout must fail the test. They now unwrap
that result without changing the budget or any assertion. The snapshot method
also needs the SessionInfo import; its extraction was not used as compiled
evidence before that correction.

### Export preservation implementation

The original test commit fails on the real source node with Conflict instead
of the required Unsupported; it reaches the missing-conversation path before
metadata readiness. The fix shares lifecycle_version::visible with projection.
The source service supplies its connection protocol to the existing exporter,
whose single original-info snapshot is checked before turn/history work and
is then the same snapshot placed in the manifest. Local pulls supply no peer
and retain their existing original-info/manifest checks. This avoids a separate
check/re-fetch race. Forward and Relay check returned session manifests before
returning to their peer, without remote lookups or changes to broader replies.

The new peer tests exercise default metadata, pending and synthetic assigned
thresholds, plus serialized source/home/older-importer return boundaries. The
three-hop assignment is synthetic because the actual assignment remains None;
it is not a claim of an assigned feature running across a real fleet. Existing
ordinary-history transfer tests and their assertions remain unchanged. Corrected
strict checks, green source regression and the complete package gate are owed.

### Returned export checkpoint: test compilation

The first exact export-policy narrow checks stopped before tests because the new
fixture called a nonexistent service-error accessor and imported an unused trait.
The assertion now uses the existing wire mapping, keeping the same Unsupported
expectation. The returned-transfer cleanup correction and its cancellation proof
remain in progress; no package gate or completed background feature is claimed.

### Retaining a fleet route

A separate structural extraction moves the existing home/member transport choice
into envelope::route. It keeps Links' existing callback selection, destination,
origin, permission checks and response unchanged. The retained descriptor will
let rejected-transfer cleanup use the export's exact holder rather than rebuild
an origin after the original call has completed.

### Rejected returned-transfer ownership

[Decision Log]
- Purpose: an unreadable successful export must not strand a handle the caller
  never receives. Preserve the typed metadata refusal and its original authority.
- Existing assets: CloseTransfer, exact Origin, Links' member/callback routing,
  the node forwarding TaskTracker, and the transfer idle/expiry policy.
- Alternatives: awaiting close in the request loses cleanup on cancellation;
  replaying export or rebuilding an owner origin changes resources or authority.
- Choice: modify the envelope to retain its route; register an independently
  tracked close before awaiting settlement, bounded by the existing idle lifetime.
- Effect: confirmed closes release admission; uncertain closes retain the original
  Unsupported and rely on source expiry/restart. Shutdown grace is unchanged.
- Tests: exact transfer/Origin and holder, successful settlement, caller abort
  while close is held, timeout and transport refusal. Assignment remains synthetic.

The correction now retains the authorized route through the returned manifest,
registers close in the existing forwarding task tracker and keeps the original
typed metadata refusal even when close is uncertain. New tests exercise actual
transfer-store release, wrong-holder refusal, caller abort during held close,
virtual-time expiry, unchanged readable/default replies, and exact Home/Member
frames used by forwarding, relay and callback selection. The frame fixture is
not an assigned three-node fleet or a live callback transport proof; those remain
explicitly unverified until runtime coverage is added. No old assertion or budget
is changed. Narrow compilation and the complete package gate remain owed.

### Event-log boundary wording

The shared Status documentation now distinguishes a run's last runtime-produced
lifecycle record from checkpoint metadata appended after that run. This permits
a separately implemented ended-session file restore to report its checkpoint
without starting an agent. No event, accessor, stream implementation or assertion
changes here; log follow still has its previously reviewed run-EOF behavior.

New retained-cleanup tests use the existing testkit safety net for channel and
filesystem waits. The virtual-time case separately bounds its join after advancing
the production idle deadline; an absent timeout fails instead of hanging forever.
This changes no existing deadline or assertion.

A new private node-unit fixture also builds a temporary home/member fleet and a
loopback relay. The member advertises no own routes; the ordinary authenticated
grant establishes its callback as a handshake barrier. The fixture owns that
connection: it cancels the existing membership epoch before joining, avoiding
a second automatic grant producer without adding a product test control. Synthetic returned
metadata references real holder-bound transfers; cleanup runs through home
forwarding and member Relay back to callback Deliver, checking exact settlement
and wrong-holder refusal. This is actual callback transport with synthetic
metadata, not an assigned three-node export. No new product control or real
harness is introduced; execution remains owed.

The callback fixture setup is split by relay establishment and authenticated
member grant so each phase remains reviewable; its assertions, routes and safety
net are unchanged. The successful close acknowledgement is explicitly consumed.

The callback fixture uses distinct explicit node names rather than host-derived
names, and names its invitation lifetime once. This avoids a fixture name
collision and makes its temporary identity independent of the test machine.

### Deferred actor capture authority

The later creation/reload capture must bind the actual run's ProcessLifetime,
HarnessRef, resolved profile and immutable launch association in one actor
snapshot, with the existing native-options fence and same-run checks before and
after it. A provider identity is an explicitly resolved launch binding, never a
kind/family/account inference. Sign-in state is an attested observation bound to
that same run and provider association; absence of such observation is Unknown,
not inferred success from a credential or identifier. Provider binding types and
registry projection stay with the provider module. This records the authority
boundary only: no actor capture API or consumer-ready implementation exists yet.

The first cleanup increment's strict check stopped in a new fixture before tests:
a string converted with Into left the generic WireError constructor ambiguous.
The fixture now passes its string literal directly; its Forbidden result and
assertions are unchanged. The original compile failure remains recorded.

Retained route entry points document their authorization prerequisite: the
transport descriptor grants no permission, keeps the exact origin, and leaves
callback selection to Links. Test fixture visibility exists only to share the
synthetic manifest across policy and real-transport tests; it is absent from
product builds. These comments add no behavior or assertion change.

The route regression also compares the export frame directly with its input
origin, so identically rebuilding the wrong principal for both export and close
cannot satisfy the test. Existing expected values and product behavior remain
unchanged; this strengthens only the new authority regression.

### Guarded envelope regression boundary

The helper/physical-callback cases are complemented by tests of the actual
Forward and member-to-home Relay refusal arms. A raw in-memory fixture peer
intentionally returns unprojected metadata, without assigning a protocol. The
only shared test seam is a cfg(test)-only member connection cache insertion,
keyed by the negotiated NodeId; no installed node exposes it. The remaining
fixture lives beside export cleanup and uses existing guards, Origins,
CloseTransfer and actual transfer-store ownership. The raw fixture is a test
edge, not an authenticated production peer or an assigned fleet proof.

The wiring regression now drives both actual refusal arms through a raw peer,
checking exact Unsupported text and a separate retained cleanup task. It holds
CloseTransfer, aborts the caller, uses a normal request as a cancellation-order
barrier, releases close and verifies exact transfer settlement and original
Origin/scopes. The real callback fixture remains separate transport evidence.
The shared cache helper is cfg(test)-only and will be called out explicitly at
integration. The four-case regression passed at f0578a20; that scoped result
covers the guarded refusal arms, not a complete package or assigned fleet.

The approved connection-cache helper lives inside links.rs's existing cfg(test)
module. This keeps all helper documentation and spacing classified as test code
and preserves that file's product-line ratchet; no product code or size exception
is removed or increased. The helper's behavior and visibility stay unchanged.

The broad strict check found two new-test issues despite the library tests
passing: an expected error unnecessarily passed by value, and a large callback
fixture Future. Expectations now borrow the same error values; the fixture's
existing deadline Future is pinned on the heap so the test task does not retain
that state on its stack. No deadline, outcome or authority assertion changes.

### Package validation boundary

Source review includes retained exact-origin cleanup, actual transfer ownership,
caller cancellation, the independent authenticated callback route and deliberately
unprojected raw-peer Forward/Relay regression. The latter passed in 2.99 seconds;
the broader node library passed in 38.97 seconds on its earlier exact revision.
The generated declaration checks passed without a wire difference. Original
compile and strict failures remain recorded; the expected-error borrow and large
fixture Future fixes change neither deadlines nor assertions. Final package
validation pairs the corrected changed-crate strict result with one complete
root gate, including generated-wire freshness, forge and web checks. The earlier
scoped results are retained separately and cannot replace that final evidence.

No assigned three-node export, bound-workspace creation bridge, consumer UI,
new label mutation/event, actual reload, actor capture or real-provider/platform
behavior is inferred from these scopes. The future actor must preserve an explicit
actual-launch provider/profile association; today's normalized profile name and
quota-location association cannot supply that authority.

The new local-owner fixture helper uses its temporary roots, so their earlier
keep-alive-only underscore names are replaced with ordinary field names. This
resolves the strict used-underscore-binding check without changing ownership,
cleanup, deadlines or expected results. The original strict result is retained.

## Nonempty label patch foundation

- Modify existing validated map operations; introduce only the type distinction
  between an empty state/filter and a nonempty mutation. Existing `ServicePorts`
  demonstrates a constructor invariant enforced at the serde edge, but its port
  choices are not a label map and are not reused as a generic container.
- Use standard `TryFrom` and the existing serde/ts-rs dependencies. There is no
  external library or new dependency to justify; a second validator or generic
  JSON patch would add an unnecessary boundary.
- Follow Paseo's caller chain: CLI `agent/update.ts:150-201` checks for changes
  before connection; `agent/lifecycle-command.ts:163-192` refuses an empty metadata
  update; manager `agent-manager.ts:2529-2578` serializes live/stored metadata.
  This increment adds no runtime mutation and claims none of that execution.
- Planned tests: empty construction/serde refusal, the unchanged bounded map's
  refusal paths, valid duplicate-key/value behavior and a transparent object
  round trip. Combined-cap failure must still leave the original map unchanged.
- The mutation's later request registration, pending event projection and actual
  live/terminal/stored persistence remain separate work in the same package.


The value implementation uses a private transparent `SessionLabels` wrapper,
`TryFrom<SessionLabels>` and a deserializer that first invokes the existing
bounded map decoder. A borrowed accessor exposes no mutation. Five edge tests
cover empty input, valid canonical duplicates and preserved values, empty values,
invalid map shapes/entries and the original cap. Existing map-operation tests
remain the source for combined-cap atomicity; their assertions are unchanged.
No wire request, actor/store operation, protocol assignment or consumer readiness
is implied by this implementation. Compilation and execution remain unverified
until the remote narrow results arrive.


The first TypeScript-enabled narrow Clippy stopped at `doc_markdown`: the module
header named the map type without code markup. The header now links that existing
type. No input, serializer, test expectation or bound changed; the original
strict failure is retained and execution is still unverified.


## Metadata queue preparation (behavior-preserving refactor)

[Decision Log]
- Purpose: title and label mutations need one actor publication order instead
  of independently scheduled per-field queues.
- Existing asset: both session modes already receive renames through a bounded
  channel and publish through `Lifecycle`; title comparison and acknowledgement
  already happen there, including generated-title precedence.
- Alternatives: a second queue would not establish this shared order. A trait or
  dynamic dispatcher is unnecessary for a closed set of metadata changes.
- Choice: wrap the existing rename message in a private enum and retain the same
  channel under the metadata name. `METADATA_CAPACITY` retains the former bound.
  The only produced variant is Rename; both actors delegate to the unchanged
  rename implementation.
- Effects: no wire, dependency, new mutation, signal, response, deadline or test
  expectation changes. Existing structured/terminal rename and generated-title
  tests remain the behavior checks; no implementation-mirroring test is added.
- Follow-up: label mutation will add its own variant and tests in the behavior
  commit. The external native-options mailbox and later execution fence remain
  their existing owners' boundaries and are not edited by this refactor.


## Mutation clock and label coverage: design predecision

The durable session mutation clock and the current-label coverage boundary are
proposed in ADR 0063 before implementation. They have different purposes:
updated_at orders committed session mutations for selection, while a label
coverage watermark states which full-map events an Info snapshot includes.
Neither is an authority or execution fence. The timestamp is not a substitute
for the coverage watermark.

Source reading for this proposal:

- Paseo `server/agent/agent-manager.ts:1017-1031`: monotonic millisecond clock;
  `:2290-2328`: live/stored label mutation; `:2390-2435`: live and stored unread
  differ in whether they explicitly advance updatedAt.
- Paseo `server/session.ts:7770-7837`: canonical eligible root selection with
  updatedAt descending/id ascending. The predicate is not inferred from cwd,
  creation parent alone or an incomplete client list.
- Store `writer/apply.rs`: one savepoint per operation, followed by the writer's
  commit-before-acknowledgement. `log.rs` owns creation/append projections;
  `marks.rs`, `inbox.rs`, `again.rs` and `subagents.rs` own direct mutations.
  `recovery.rs` appends an interruption through the normal log path.
- Store profile/cost and search updates are projections inside append or
  maintenance; they do not justify a second clock or a duplicate tick.
- Session `publish.rs` and node `persist.rs` distinguish publication from a
  durable append. Node `sessions.rs` currently seeds storage from late
  `handle.info()` while create_session sets next_seq to zero. A frozen initial
  seed is a prerequisite for the proposed label coverage, not a claim about
  the already shipped creation behavior with immutable labels.

Proposed writer coverage to verify:

| Operation | Clock effect | Existing atomic boundary |
|---|---|---|
| Successful new row | Initialize from writer time, ignore supplied/imported ordering time | Create savepoint |
| Nonempty ordered append, including recovery and metadata events | Advance once per accepted batch | Append projection savepoint |
| Session mark or an existing inbox-entry mark | Advance with the accepted mark | Mark savepoint |
| Reopen | Advance only on success | Conditional reopen savepoint |
| First detach | Advance once; repeated receipt stays unchanged | Receipt savepoint |
| Future stored label mutation | Advance with its event/projection, once | Existing writer savepoint under reopen ordering |
| Read, failed write, empty append, migration, retention/index/backfill maintenance | Keep the clock | Existing behavior retained |

A private store helper must take the existing writer connection. Later workspace
attention mutations need no public setter or separate read: require_active,
owned members/roles, current attention/clock selection and marking stay in one
transaction. Legacy NULL remains Unknown and a selection requiring that fact
refuses. Widening session.mark permissions is outside this package.

Proposed protocol additions remain unassigned: optional SessionUpdatedAt and
LabelsWatermark fields and the future full-map LabelsChanged variant. The
mutation timestamp has its own pending feature; label coverage shares the
existing lifecycle metadata projection. No new feature constant, wire field,
event, RPC registration, schema append or clock writer is implemented by this
contract. Native timeline/terminal/current-metadata consumers remain separate;
no exhaustive-match repair is made in this node package.

Current evidence: the nonempty label patch's sixteen filtered map/patch tests
passed, and the metadata queue refactor passed TypeScript-enabled Clippy for
proto/session on their exact pre-merge revision. Existing options/generated-title
regressions are queued after the integrated discovery head merge. Those checks
do not prove timestamp production, label-event coverage or complete mutations.
The next step is design review of this contract, then implementation and the
forcing tests before one complete package gate.


### Creation boundary extraction (behavior-preserving)

Before adding a frozen initial seed, creation and its cancellation-safe startup
move unchanged from the session collection into sessions/creation.rs. The
collection still owns lookup, tracking and handover. The extracted methods keep
all existing ordering, cleanup, conditions and return values, including the
late handle snapshot that the following behavior correction will replace.
No test expectation or deadline changes in this extraction.


### Frozen creation seed prerequisite

The runtime now captures a boxed initial Info immediately before spawning its
actor and returns it with Created. Node creation stores that seed, while its
ordinary reply and subsequent publications still use the handle. Reopen keeps
its existing authoritative stored-row flow and ignores the extra initial seed.
This extends the existing start boundary, with one bounded snapshot clone per
run and no new queue, lock, dependency or wire field. The store still accepts
an explicitly supplied current map; it does not replace it with spec.labels.

A fake-actor regression holds the durable receiver, completes a rename and
checks that the returned empty-log seed still has the creation title and zero
cursor. This forces publication ahead of persistence without a sleep or a
new product hook. It does not yet prove future LabelsChanged behavior.
A node regression then stores that seed before draining the held receiver,
checks the zero-cursor creation title and verifies the changed title appears
only after the actual durable append. Existing Created destructuring in
resume, diagnostics and private-dir/detach fixtures gains only `..`; no
existing assertion or deadline is changed.
The mutation-clock and explicit label-coverage contracts remain proposed.


### Label mutation request value

Reuse the existing SessionId and validated nonempty LabelPatch in one SetLabels
payload, with required session/labels fields and the existing serde/TypeScript
derives. A nullable or optional patch would blur "nothing to update" with an
empty current state, so it is not used. No external package is needed. Payload
shape/refusal tests belong beside the RPC payload; registration, version checks,
authorization and mutation behavior remain the later wiring step.


The SetLabels value now retains both required fields, using LabelPatch's
existing validating deserializer. Payload tests check exact target/map round
trip and missing/empty/malformed target or patch refusal. No Request marker,
method row, handler, send path, event or available mutation is added yet.


### Mutation timestamp value foundation

SessionUpdatedAt reuses chrono and the existing signed-nanosecond store range,
while requiring whole milliseconds at the wire/value edge. Its pure advance
operation normalizes only the writer sample, chooses a later sample or the
previous value plus MIN_INCREMENT, and refuses exhaustion instead of saturation.
One named tick defines the increment. Tests cover serde/value precision refusal,
Unknown initialization, repeated/backwards time, a later sample and exhaustion.
This value adds no store column, writer, Info field, clock setter or protocol
assignment. It is not a producer attestation or a label coverage marker.


### Durable clock writer implementation, before public projection

The nullable column append now follows BACKGROUND_SCHEMA through one relative
MUTATION_TIME_SCHEMA and the migration table's checked length. The runner is
unchanged and legacy rows remain NULL. SQL lives with that append so the head's
migration file stays within budget. The separate mutation_time module reads and
advances only on the caller's existing connection, preserving typed range,
precision and exhaustion errors. It samples the writer's own wall time.

Creation, nonempty append batches, marks, exact inbox marks, reopen and first
detach now invoke that helper inside their existing savepoints. Append-derived
metadata is counted there once; a repeated receipt and failed/empty/read work
add no tick. No public Info field, caller timestamp setter or protocol number
is introduced in this step. Exact acknowledgement/live-info propagation remains
next, so this is not yet an external producer handoff.

New migration and real Store-writer tests cover legacy Unknown/idempotence,
strict per-batch/mark time, independence from event/client time, inbox/reopen/
detach and repeated receipts, failed operations and savepoint rollback on clock
exhaustion. Existing approvals/devices/log/marks/title-origin rollback fixtures
only gain DROP COLUMN updated_at immediately before their old SQL; existing
assertions and deadlines stay unchanged. Compile and execution remain pending.


### Acknowledgement preparation (behavior-preserving)

The private Store::write channel helper is generic over its existing typed
reply instead of fixing it to unit. Its send, Closed mapping and commit wait
are unchanged, and all current callers retain the same unit result. This lets
subsequent clock acknowledgements reuse that one channel path; no new reply,
public signature or mutation behavior is introduced by the refactor.


### Exact durable acknowledgement carriers

The writer now captures SessionCommit<T> (the operation's result and optional
mutation clock) inside that operation's savepoint, before applying another
queued write. Its existing outer commit and typed failure mapping still decide
whether a caller receives success. The additional public committed variants
serve runtime publication; the original create/append/mark/reopen methods keep
their original signatures and results by delegating and discarding only the
extra acknowledged time. No extra write queue or clock writer is introduced.

The paired mark regression verifies each operation retains its own kept marks
and distinct time even when queued together; final stored marks match the latest
acknowledged time. An empty append on a legacy-Unknown row preserves None until
an actual mark initializes it. Node live-clock application and public Info
projection are still next; no consumer readiness follows from these source tests.


### Pending mutation-time Info projection

SessionInfo now has optional updated_at, decoded with the writer's same cell
validator. Its constructor and old wire omission preserve Unknown. The separate
SESSION_MUTATIONS constant remains None, with assignment prerequisites stated
beside it; the normal Info projection composes the head's parentage/title/labels
rules before hiding this clock. New tests name every direct Info result, list,
subscription, detach, both session-shaped exports, starting import and mixed
Forward/Relay/Deliver nesting. A synthetic assigned threshold keeps a supported
value; no real number is assigned. Saved/scheduled specs contain no such clock.

The TS primitive is classified as a time quantity in the same change that first
references it on the wire; generated declaration recovery/freshness is still
owed. First detach now includes the exact newly committed row time in its Info,
while repeated detach keeps the existing time. Live acknowledgement publication
remains the next implementation step; this field is hidden from all peers now.


### Kept runtime metadata naming (behavior-preserving)

Before copying acknowledged clock data, marks.rs/Marks becomes
kept_metadata.rs/KeptMetadata. Its existing kept marks, detach receipt and
actor-derived attention already share that one overlay lock. Only private
module/type/field/accessor names move; public mark and receipt setters, reducer
logic, locking and test expectations remain unchanged. The following behavior
change can add durable time to this same bounded row-copy responsibility.


### Append projection extraction after the strict check

The first clock-storage strict check stopped because the existing append
function grew past the function budget. Its SQL/parameter block moves into
Projection::save, with the profile enum binding named separately. Record
validation/observation, exact SQL text and parameter order, indexing/retention,
clock timing and savepoint behavior are unchanged. No bound, assertion or
deadline is altered. This is a separate behavior-preserving correction;
the original strict failure remains in the validation history.


### Live acknowledgement publication

The existing kept-metadata lock now overlays the latest acknowledged mutation
time as well as kept marks/receipt. It takes the maximum known acknowledged
clock and never clears it with Unknown. Crucially, an append's newer time does
not authorize discarding an earlier mark that still needs publication: marks
retain the node's original serialized order, while only the scalar clock uses
a high-water rule. No extra actor queue, mutex, writer or authority is added.

Creation, persistence, live marking, reopen and detach consume their actual
Store acknowledgements. Persistence applies the clock before advancing its
durability notification. Reopen seeds/restores the canonical stored time before
opening its existing gate. The raw actor watch omits this separately kept time,
as it already omits kept marks; SessionHandle::info adds the current overlay,
including after actor EOF. Stored replies use the canonical row snapshot.

New unit tests force delayed/Unknown clock acknowledgements and verify the
older mark still applies without regressing time. Real fake-node tests exercise
create/append/mark/handover and live detach/repeat publication. Existing error,
cleanup, authority, cancellation, deadlines and assertions are unchanged. This
still establishes no label-map coverage, new label event or reload behavior.


### Exact inbox mark acknowledgement join

The entry-mark convenience API now delegates to a committed variant, preserving
its original unit result and Gone-entry semantics. The existing inbox handler
keeps its scope and exact agent refusal, then delegates to the session owner
instead of discarding the Store acknowledgement. That owner reuses the metadata
lock and tracked-task pattern so a caller leaving cannot abandon a committed
clock's live publication. No extra read path, timestamp source or permission is
introduced. Later passive attention joins use this same acknowledgement rule.

The fake-node regression waits for the exact TurnEnded record to be durable,
marks its entry and checks live time. A cancelled call while metadata is held
creates no retained task and leaves the entry unchanged. A separately admitted
call owns that same metadata lease before caller loss; normal shutdown drains
its task and the canonical mark/time remain committed. The shared inbox
handler change is only this delegation/error-boundary adjustment; its original
authority checks and messages are untouched.


### Owned metadata order preparation (behavior-preserving)

The existing metadata mutex is held in Arc so admission can transfer an owned
lease to a retained task. Existing lock calls dereference the same mutex; no
ordering, condition, operation, permission or test expectation changes here.
The following inbox cancellation correction obtains that lease before spawning,
instead of retaining arbitrarily many cancelled waiters. This is no new fence.


The inbox task acquires the owned metadata lease before spawning. This keeps
cancelled admission waiters within the existing RPC lifetime rather than
creating an unbounded detached wait queue. After admission, the single lease
travels with the already-authorized operation through commit/publication.
No deadline, numeric limit, permission, Origin or execution fence is added.


The inbox cancellation test setup is split into named turn/persistence,
before-admission and after-admission phases. Only test code moves; every input,
poll, assertion and deadline remains the same. This keeps the resource-admission
proof readable without weakening or lengthening its safety budget.


Additional clock regressions refuse a corrupt fractional stored value on both
read and mark, preserving the prior row on failure; supplied creation/import
clock data cannot choose the destination's local time. A fake-node reopen case
checks the actual writer clock is already visible when the existing publication
gate opens, the raw actor watch remains Unknown, and handover retains the final
acknowledged time. These are new cases, with existing assertions/deadlines kept.


The next changed-crate strict run stopped at a missing code span in the new
SessionUpdatedAt alias comment. That comment now uses code markup; the original
red result is retained. No validator, writer, projection, test expectation or
deadline changed. Latest-head strict and execution checks are still owed.


### Typed dispatcher answers after the next strict check

The dispatcher exceeded its function budget after adding clock acknowledgements.
Typed session/marks answer constructors now call the same savepoint/clock helper;
ordinary device/attachment/automation branches reuse the existing unit-answer
helper. Operation order, closures, errors, effects and commit-before-reply are
unchanged. This separate refactor shrinks dispatch without an exemption, numeric
budget change or modified test expectation. The original failure is retained.


### Clock fixture configuration after strict checking

The current strict check reached the new Store integration fixture and found
its three clock/corruption helpers outside the test-only module convention.
They now live in a cfg(test) module, and the shared fixture import records why
its unrelated paging/rollback helpers are unused here, as existing tests do.
No helper body, input, assertion, deadline or product behavior changes. The
original strict result remains red; the corrected strict check remains owed.


### Inbox rollback fixture after clock test execution

The real proto/store run passed the new clock cases but found the inbox replay
fixture still retaining the new clock column while rewinding its schema. It now
drops only this append's updated_at before the head's existing rollback SQL.
The production runner, migration ordering, inbox cases, assertions and deadlines
remain unchanged. Both original duplicate-column failures remain in the log.
An independent unchanged registry-watcher test also timed out under load; that
result remains separate from this fixture correction and has not been retried.


### Generated mutation-time declaration

The named TypeScript step passed its five generator/inventory tests and correctly
reported stale declarations. Its original generated proto.ts bytes are now
committed after checking every artifact hash and confirming the TS inputs did
not change since that generator commit. The only declaration change adds the
optional mutation-time field and its string type. Generated whitespace is kept
verbatim; source diff checking is separate from ts-rs formatting. Freshness on
the final package head is still required by the whole gate.


## Transferable marking-order prerequisite

[Decision Log]
- Purpose: an accepted metadata operation must keep the existing serialization
  lease through commit/publication after its caller leaves, without retaining
  tasks for callers still waiting to be admitted.
- Existing assets: Sessions already owns one Tokio marking mutex, shared by
  marks, reopen and detach. Arc and Tokio owned guards are existing dependencies.
- Alternatives: a second lock or queue would create competing metadata order;
  retaining tasks before admission would leave abandoned waiters unbounded.
- Choice: wrap that same mutex in Arc and expose a private owned-lease accessor.
  An existing stored-mark wait uses it at the same acquisition/drop points so
  the product accessor is exercised without widening visibility or suppressing
  dead code. Other existing lock users still dereference the same mutex.
- Effects: one cheap reference-counted guard replaces a borrowed guard for that
  caller; no operation, condition, permission, task admission, wire, number or
  test expectation changes. Consumers must acquire before spawning accepted work.
  This prerequisite itself adds no ReadAttention or clock implementation.
- Validation: existing mark/reopen/detach regressions and changed-node strict
  checking, followed by the bounded package gate. No new test mirrors the Arc
  representation; the consumer must force cancellation before/after admission.


The transferable-order prerequisite passed its exact-commit checks: changed-node
all-target Clippy, then all thirteen selected Rust gate steps. Workspace tests
were 2701 passed, none failed, twelve ignored; generated declarations were fresh
with five export/inventory tests, and the forge fixture passed four tests. Both
fixture prebuilds preceded the workspace tests. Existing mark/reopen/detach and
caller-loss cases ran unchanged. No web/native/platform or new consumer-runtime
acceptance follows from this internal representation-only change.


### Reuse of the isolated owned-order prerequisite

The independently validated marking prerequisite is merged without importing
clock code into that leaf. The clock's admitted inbox task now acquires its
existing owned guard through that same private accessor at the identical point
before spawn. Guard lifetime, task ownership, authority, SQL, acknowledgement
and cancellation-test expectations are unchanged. The announced node values and
membership-helper head is also merged; its shared entries and documents remain
authoritative. The final package gate still qualifies this combined clock tree.


The corrected fixture strict check reached the session watch documentation and
reported one missing code link around info(). That reference now links to the
existing method. No executable body, test expectation, wire declaration or
deadline changes; the original strict failure is retained. The current merged
head still needs a green strict result before its package gate.


The current runtime forcing run built its fixture binaries successfully but
found an unavailable convenience constructor in the new inbox test. The input
now uses ContentBlock's existing Text variant with the identical prompt bytes.
Only test construction changes; no assertion, product behavior or deadline is
altered. The actual frozen-seed integration test passed independently on the
same pre-fix executable source; Node/session runtime execution remains owed.


The new crate-private clock/acknowledgement boundaries and test-only fixture
functions now each state their reason locally: same-connection snapshot, exact
queued-operation result, projection order and retained publication. Only docs
change; no executable or test body, input, declaration or deadline changes.


## Clock package verification before the complete gate

The corrected changed-crate TS-enabled all-target Clippy passed on its exact
executable revision. The corrected Node/session library run passed 398 tests,
including all four clock publication/cancellation cases, the delayed/Unknown
acknowledgements and the held-persistence creation seed. The independent actual
creation integration passed its one frozen-seed case. Their inputs, original
assertions and safety budgets are unchanged by the fixture construction fixes.

The earlier proto/store run passed 641 tests and retained three failures in its
original log. The two inbox rollback failures were this append's missing DROP;
the corrected lifecycle fixture passed all five cases. The unchanged registry
watcher timeout had one explicitly permitted exact replay, which passed. It is
recorded as a named intermittent under load; its cause is not claimed solved.

The five generator/inventory tests passed before the expected stale-wire result;
all retrieved artifact hashes and original declaration bytes were verified. The
current package includes those bytes and the announced node head's additions.
The later merge preserves that head's launch admission and execution invalidator;
no stop/resume/archive caller joins or stronger process proof are added here.
The final complete gate must verify this combined head, including all wire/web
checks. None of the partial checks is represented as that complete result.


## Attention-mark activity parity

Reference read: agent-manager.ts 2389-2414 (loaded clear/read and unread do not
call touchUpdatedAt), 2420-2433 (stored unread advances nextStoredUpdatedAt), and
session.ts 7697-7712 (stored read only for attention requiring clearance). This
replaces the earlier deliberate difference that stamped every accepted mark.

Reuse decision: modify the existing Node residency branches, marking lease,
Store writer/savepoint, KeptMarks attention derivation and exact inbox seen state.
Two named loaded Store entry points carry a private writer residency enum; their
caller must hold the existing metadata order. Standard enums and the current
SQLite/chrono dependencies suffice. No external API or dependency is needed.

Forcing checks must cover loaded Read/Unread clock preservation, stored changed
attention ticks, repeated and watermark-only reads preserving time, exact inbox
seen transitions/no-ops, and the unchanged Pin/Archive tick rule. Existing caller
cancellation, read validation, inbox state and safety budgets remain. The former
loaded inbox clock-increase expectation changes to equality because that is the
explicit behavior correction; shutdown's later increase proves its final append,
not an inbox mark tick. Source review and narrow execution remain owed.


The existing loaded inbox regression now requires equal acknowledged time and
also checks ordinary loaded Read/Unread through Sessions::mark. Its task/lease,
inbox outcome and deadline assertions remain. The final shutdown increase is
explicitly attributed to the Exited append. This is the expected behavior change,
not a weaker general clock assertion. The test is committed before the fix.


The test-first actual Node run failed exactly at loaded inbox clock equality:
the accepted attention mark advanced by one millisecond. That red execution is
retained. The correction now carries private residency in the same writer queue;
no existing head test call needs a signature or expectation change. The Node's
ordinary loaded mark rechecks Found under the existing lease before selecting
the Store entry point, and inbox captures its existing runtime_handle there.
The savepoint compares actual before/after derived unread or exact seen state;
watermark-only propagation still commits. Four actual SQLite unit cases cover
loaded/stored transitions, repeats, Pin/Archive, and legacy Unknown. Current
changed-code Clippy and functional results remain unexecuted.


The first strict run found three unconsumed affected-row counts in the new
SQLite setup. Those fixture writes now assert exactly one row, strengthening
setup evidence without changing inputs, product code, existing expectations or
budgets. The original strict failure is retained; the functional run on that
immutable predecessor continues unchanged.
