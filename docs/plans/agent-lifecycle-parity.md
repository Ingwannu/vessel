# Agent lifecycle parity

Labels, a background listing, a live reload, and selectors on `vessel log`.
The decision is ADR 0063. The protocol number is the one given at merge.
Calls and fields use `lifecycle_version::SINCE: Option<ProtocolVersion>`,
which remains `None` until integration; this branch does not
change `PROTOCOL_VERSION`. This track adds no `node.settings` key, and
it does not touch parking, rewind, fork, or `vessel-agent`.

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
- Vessel has no workspace visibility. The flag is `SessionListing` on
  the session (ADR 0063). Archived sessions stay listed as they are;
  `--background` does not mean `-a`.

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
   `--background` on `run` and on `sessions`. MCP `listing` and `scope`.
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

## Resume point

Current label-mutation preparation: the nonempty `LabelPatch` contract is written;
implementation and narrow checks follow. This is a value foundation only, with
no mutation method/event, actor/store write or feature assignment. Earlier notes
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
