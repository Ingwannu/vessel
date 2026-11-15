# ADR 0063: Labels, background, reload, and the log's selection

Status: proposed, 2026-11-14 (milestone 12). The protocol number is the
one given at merge. This commit does not change `PROTOCOL_VERSION`.
New calls and fields are gated on one constant, `lifecycle_version::SINCE`, so
the integrator changes that one line. The plan is
`docs/plans/agent-lifecycle-parity.md`. This track adds no
`node.settings` key.

## Purpose and intent

Paseo lets a person mark an agent with `key=value` labels and filter the
list by them, hide a workspace (and therefore its agents) from the default
lists until asked, restart a live agent's process without a new id or a
new conversation (to pick up tools, skills or a login), and read that
agent's activity with a tail, a follow, a kind and a start time. Vessel
already renames a session and prints its log from a sequence number. It
has none of the other three, and its log has none of those selectors.
The intent is those four behaviours on the session the node already has,
with the same names, defaults and refusals, except where a difference is
written below.

## Existing implementation and constraints

- A session's current title is `session.rename` (protocol 9) and a column
  of its row (schema 6). Marks are columns (schema 7). `SessionSpec` is
  what was asked; `SessionInfo` is what is true now. New fields on either
  are optional and defaulted, or an old peer fails to decode, or an old
  node silently ignores a field and does the wrong thing (`Call::since`).
- `session.list` is a unit request and returns every session, newest
  first, including ended and archived. Its historical encoding is
  `params: null`. Changing the type must still decode that.
- `vessel log` prints from `--from` through `session.read`. The CLI's
  view already joins streamed reply chunks. Follow is a subscription;
  subscriptions are not forwarded (`session.read` with `wait` is).
- A live structured session is restarted today by a move to another
  profile and by a wake: hold input, stop the process through
  `Child::signal`, start it again with `session/load` on the same
  harness session id, drop the replay because the log already has it.
  On Unix that signal reaches the process group; on Windows there is
  no polite signal, so the same call terminates the child's job
  (`crates/vessel-proc/src/child.rs`, the comment on `signal`). Reload
  uses that stop. It does not add a signal, a service unit or a
  startup script. Parking, rewind and fork own their own uses of it.
  This track does not change those, and it does not change
  `vessel-agent` (the drivers are another track's).
- An ended session starts again only through `session.resume`. A parked
  session stays `running` and starts again only through a wake. A reload
  must not become a second implementation of either.
- Agents may call `session.rename` and `session.create`. They may not
  call a method until `workspace/policy.rs` names them. Every new method
  also has to decide `forwardable`, `Call::since`, and `Reply::for_protocol`.
- The store's migration list is authoritative. A migration is appended, never edited.
  `PROTOCOL_VERSION` stays that base. Minimum stays 2. `lifecycle_version::SINCE`
  is the only new version this track names, and its value is assigned
  at merge.
- No new crate edge and no new dependency.

## Alternatives considered

- **Labels as a free `HashMap<String, String>` on the session.** Rejected:
  a string key is not a type, two labels can be swapped with a title, and
  an unbounded map is a reply a client cannot trust.
- **One boolean `background`.** Rejected: `true` at the call site does not
  say whether the session is hidden or the list should include hidden
  ones. Paseo uses that boolean; Vessel uses two enums.
- **Hide every session of a project**, the way Paseo hides every agent of
  a workspace. Rejected: a Vessel project is a folder people share, not
  an agent workspace created for one run. Hiding it would hide work nobody
  asked to hide.
- **Make `session.reload` a forced `session.move_profile` to the same
  profile.** Rejected: a move's failure starts the old process again and
  publishes `ProfileChanged`, and `--force` kills a turn that did not
  end. Reload's failure and its turn are specified below, and a move's
  tests must not change.
- **Wipe the log and re-read the provider's history**, which is what
  Paseo's user-facing reload does to its in-memory timeline. Rejected:
  Vessel's log is the cursor every client holds. Deleting it shows a gap
  that cannot be filled. The provider's replay is dropped, as a move
  already drops it.
- **A new MCP tool for reload, and a filtered log tool.** Rejected: Paseo
  has neither. Reload is a person's command. The log's selectors are a
  presentation of `session.read`, not a second protocol.

## The choice

The number given at merge adds two calls, two events, and fields on the
session and on `session.list`, all gated on `lifecycle_version::SINCE`. The
next schema append (when the code lands) stores the current labels and
the listing. `PROTOCOL.md` is updated in the commit that the integrator
uses to publish the number, not in this one.

### Protocol assignment

`lifecycle_version::SINCE` is `Option<ProtocolVersion>` and remains `None`
until all documented client consumers can validate, show and edit metadata.
Unassigned fields are omitted for every peer, including the current version;
after assignment, only peers at or above the assigned version receive them.
Both current and creation maps remain in durable storage independently.

The shared `required_protocol()` gate refuses unassigned calls and fields
before sending and at dispatch. Existing session creation, saved-prompt
creation/update and schedule creation/update ask that accessor about their
embedded spec's labels; forwarding envelopes propagate the same requirement.
Empty labels retain the existing requirement. Once assigned, nonempty labels
require the greater of the existing requirement and the assigned threshold.
This extends the shared admission rule rather than inventing a second gate.

Session events use that same optional availability model through one public
`required_protocol()` accessor. The old numeric `SessionEvent::since()` is
removed: it cannot honestly represent an unassigned variant. Existing variants
retain their assigned thresholds; agent events retain their content-dependent
requirement. Event and page projections treat `None` as unreadable by every
peer and preserve the existing stand-in at the same sequence and time. This
foundation introduces no new event; each later variant must supply its optional
assignment and an older-peer projection together.
The node's earlier ingress checks remain as defense before work. A starting
import must still check its source metadata before export/placement, because
the caller does not carry those labels. No new method is registered by this
admission change. Refusal avoids accepting data that no client can see.

### Export classification before peer projection

An export must not silently lose its current labels or Background classification
on the way to a later import. After the existing ownership guard, the source
reads the original session info and checks the same lifecycle visibility rule
used by reply projection, before reading provider history or opening a transfer.
Default metadata needs no feature support. Nonempty current labels or a
Background listing require an assigned threshold readable by that connection;
otherwise export returns typed Unsupported. No metadata values enter the error.

Forward and Relay apply that same check to an ExportSession manifest before
returning it to their connection. A newer source and home must not let an older
importing node receive a silently projected classification. They use the returned
manifest, without another remote lookup or a new Origin field. Requests, shared
call admission and all reply projections remain unchanged.

A returned manifest already owns a source transfer. Before refusing that result,
the forwarding node schedules the existing CloseTransfer for that exact handle
on the retained destination and Origin, including Links' callback route. The
receiving guard still intersects the same scopes and checks the same holder;
cleanup grants no extra authority. The node tracks this task independently of
the caller, so cancelling the caller while close waits does not discard it.
The existing transfer idle lifetime bounds an uncertain close; failure or timeout
is logged without replacing the original typed metadata refusal. Normal shutdown
keeps its existing bounded forwarding-task grace, and expiry/restart remains the
fallback if the source cannot confirm closure. No export is replayed.


This deliberately also refuses a non-starting history export when its current
metadata cannot travel: the existing export request carries no import-start
intent, so the source cannot promise preservation only for one future consumer.
Paseo's provider-history import (`agent/import.ts:110-123`) supplies labels from
its caller and has no corresponding fleet transfer route. Vessel's refusal
protects the source classification instead of silently making the import Shown.
While the assignment is None it applies to every peer; afterwards it applies
only below the assigned threshold. Ordinary metadata-free transfers are kept.

### Stop failure before a restart

The existing profile-move stop helper is the candidate for reload reuse. Its
signal failure must be returned with the OS error as a typed source, before
starting any replacement. Logging and waiting for a later kill can turn a
refused first stop into an apparent success, so it does not satisfy that
boundary. Existing successful stop/drain behavior and grace budgets remain.

A per-actor, test-only next-stop fault injects `PermissionDenied` into that
signal boundary. It follows the existing private test control, has no global
state, and is absent from product builds. The real runtime and fake agent
then prove that a refusal cannot reach replacement startup. No test changes
process ownership or signals another account to manufacture an OS refusal.
Product configuration is checked separately: the constructor and exhaustive
control dispatch compile without the test-only field, control or branch.

### Labels

A label is a `LabelKey` and a `LabelValue`. A key is trimmed, then 1 to
`LabelKey::MAX_CHARS` Unicode scalars, containing neither `=` nor any character for which
`char::is_control` is true. A value is not trimmed, at most `LabelValue::MAX_CHARS` scalars,
and likewise contains no character for which `char::is_control` is true.
That is the same refusal `SessionTitle` already uses
(`crates/vessel-proto/src/title.rs`, `FromStr`): every control,
including the C1 controls U+0080 to U+009F. A
session holds at most `SessionLabels::MAX`, in a `SessionLabels` map ordered by key. A
key starting with `vessel.` is reserved and refused. Anything else is
`invalid_request` naming the key, before a session is created or a row
written.

The public patch only sets keys. Keys omitted from the patch stay.
There is no clear. An empty patch is `invalid_request` ("nothing to
update"), not a silent success. The same key twice in one request keeps
the last value.

Set at `session.create` (and so at `vessel run`, including `--resume`),
copied by `session.import` with the row, and replaced as a patch by
`session.labels` (`operate`, forwardable, since `lifecycle_version::SINCE`).
The reply is the
`SessionInfo`. The log gains `labels_changed` with the whole map, so the
latest record is the state. `SessionSpec` keeps what creation asked;
the column keeps the current map, as the title's column does.

Side-question creation is a shortcut that reuses a side or synthesizes its
spec from the parent. Non-empty creation label overrides on that shortcut
are refused before lookup, rather than silently discarded. A later explicit
`session.labels` call can update the opened side's metadata.

`session.list` gains `labels`. Every listed session must carry each
pair (logical and). Empty means no label filter.

### Background

`SessionListing` is `shown` or `background`, immutable after creation. For a
bound session, workspace visibility is the one truth (ADR 0082). Creation
orchestration resolves that visibility to the same typed listing, refuses a
conflicting explicit session override before work, and writes an immutable
mirror. Discovery, storage and source inheritance read that fixed mirror;
they never override the owning workspace. That bridge belongs to creation
orchestration and is not implemented by this legacy/unbound package.

For a legacy or unbound session, omitted means `shown`, except when the caller
is an agent: then omission inherits that agent's resolved session listing.
An explicit value wins, including a local schedule's run made as an agent.
A person's omission is `shown`. The forwarding rule below applies to unbound
creation and to a listing already fixed by the owning-workspace bridge.

For a forwarded creation the authenticated source resolves an omitted
Background inheritance after scope/forwardability checks and before sending.
It fills `Some(Background)` only for its own background agent; Shown keeps
the legacy omission, and explicit overrides remain untouched. The receiving
node treats a remote-agent omission as Shown. A filled Background field still
passes shared admission and is refused for a pending or older protocol. This
reuses the existing trusted source/origin boundary without another origin
field or a privileged remote-parent lookup. Paseo has no node forwarding;
Vessel needs this distinction to preserve both inheritance and old ordinary
creation. Saved templates keep omission until a local run; the scheduler's
existing remote-definer limit stays. Imports explicitly keep their source
session's effective listing instead of inheriting from the importer.

`session.list` gains `scope`: `shown` (default) or `all`. `shown` leaves
background sessions out. `all` includes them. It does not change which
archived sessions appear; those stay listed as they are today.

A peer that negotiated a protocol older than `lifecycle_version::SINCE` has no
field. The node includes background sessions in that peer's list and
rewrites each `SessionInfo` so the listing reads `shown` and the labels
are absent. A current client does not send `scope: all`, a label
filter, a background listing or any labels to an older node
(the shared `required_protocol()` accessor returns the assigned threshold,
or `None` while the feature is unassigned).
`params: null` remains the legacy full-list request. A present typed
selection instead requires an explicit wire scope and the feature's
assignment. An empty object or labels-only object is invalid; Rust callers
use named Shown/All constructors. The SDK's existing full-collection method
keeps the null path. CLI default discovery uses Shown when the assigned feature
is supported, and the legacy path before assignment or on an older connection.
This compatibility difference is necessary because an older peer cannot supply
listing metadata. Explicit background inclusion or label predicates never
fall back: unsupported or unassigned requests are refused. The exact nullable
request shape and compatibility table are in the plan. The protocol owner
accepted the required-scope form. Its optional field requirement uses the
integrated method-table head; storage and discovery wiring remain in progress.

`session.get`, attach, log, send, kill, rename, labels and reload do
not care about the listing. A background session is still there.

### Reload

`session.reload` (`operate`, forwardable, since `lifecycle_version::SINCE`)
restarts the live
process and keeps the session id, the log, the subscribers, the title,
the labels, the listing, the marks, the profile, the place, the queue
and the harness session id. The reply is the `SessionInfo`. The log
gains `reloaded` carrying that same harness session id, once the
replacement is ready; the replacement's `ready` is still recorded.

Before anything is stopped, the node refuses:

- no such session: `not_found`;
- a terminal session, or a harness that cannot `session/load`: `unsupported`;
- not live (ended, failed, interrupted, exited): `conflict`, and the
  message names `session.resume`;
- parked: `conflict`, and the message names `session.wake`;
- live but no harness session id yet: `conflict`.

No process is spawned on those refusals.

Otherwise the session runtime, not a driver, does what a move does up
to the point the behaviours differ:

1. New input is held. A kill during the reload wins and the session
   ends as any killed session does.
2. A running turn is cancelled and waited on for the grace a forced
   move already uses (10 seconds). If it does not end, the call is
   `conflict`, the process stays, and the turn stays. The reload does
   not kill a turn that failed to cancel.
3. The process is then stopped with the existing `Child::signal`. On
   Unix that is the group signal a move already uses. On Windows the
   same call terminates the job, because Windows has no polite signal
   for a windowless process. If that stop fails, the call is
   `unavailable` and nothing new is spawned.
4. The same launch as a wake starts the replacement with `session/load`
   on the same harness session id, so the current tool servers are the
   ones the launch already injects. Its replay is dropped.
5. If the replacement does not become ready, or becomes ready under a
   different harness session id, the new process is stopped, the
   session is `failed`, the log is kept, and `session.resume` is the
   retry. The old process is not started again.

Agents are refused `session.reload` (`forbidden`), with a reason that a
person reloads. There is no MCP tool for it.

### The log's selectors

`vessel log` keeps `--from`. It gains:

- `--tail N`, `N` a non-negative integer. `0` prints nothing. One-shot
  with no `--tail` prints every selected record. Follow with no `--tail`
  prints the last 10.
  Here a count is log records, including terminal output and status, not
  rendered lines or complete messages. Paseo counts projected timeline
  items after merging text chunks and tool lifecycles. Vessel keeps its
  existing `--from` sequence/retention unit for both terminal and structured
  logs; applying tail before rendering can therefore start within a message.
- `--filter tools|text|errors|permissions`. Anything else is a usage
  error, before connecting. `tools` is a tool call and its update;
  `text` is a user turn, a replayed user chunk, a reply chunk, a
  thought and a steered message; `errors` is a failed turn and a
  `failed` status; `permissions` is a request and its resolution.
  `HarnessLog` is not an error.
- `--since` an RFC 3339 timestamp. Unparseable is a usage error. A
  record's `at` must be greater than or equal to it. Paseo's history keeps
  whole projected items by their final merged fragment's time
  (`utils/timeline.ts:23-26`, `timeline-projection.ts:156,234,286`). Vessel
  selects raw records, the same unit as `--from`, `--tail` and retention,
  without CLI projection; output can start inside a message or tool lifecycle.
  This keeps one predicate for history and live records, whereas Paseo's live
  path already compares individual events (`logs.ts:385-386`).
- `-f` / `--follow`. After the selection, wait on the session stream
  for later matching records until the session's log ends (its final status
  record) or the person interrupts. No sleep.

Order: `--from`, then `--since`, then `--filter`, then `--tail`. Follow
continues after the last raw record examined, including records hidden by
a predicate or the tail. Keeping the raw cursor avoids replaying history at
the subscription boundary. Rendering stays the view
that already joins chunks. A terminal session has no agent events of
those four kinds, so a filter prints nothing of its bytes.

These flags are not a wire call. `read_events` is unchanged. Count parsing
consumes the whole integer rather than Paseo's numeric-prefix `parseInt`;
a trailing typo is refused. Both commands refuse a non-numeric or negative
count: Paseo's command checks the parser's `undefined` result in both modes
before applying a default. Vessel refuses before connecting; Paseo connects
and resolves the agent first (and fetches one-shot history before refusal).
Early refusal avoids credential, connection and history work for invalid input.
RFC 3339 replaces locale-sensitive date parsing.
The existing session stream ends at a run's final status, so follow ends
there or at Ctrl-C; a later resume needs a new invocation. Paseo's timeline
subscription waits for an explicit stop. Preserving the existing stream EOF
avoids a new wire mode solely for this presentation command.

An empty selection emits no records or activity banner. Paseo's one-shot
curator emits "No activity to display." Vessel preserves its existing log
capture contract: stdout is selected session output, stderr is selected
status and genuine retention notices, without a fabricated activity entry.

Paseo also writes a follow-start banner to stdout after history and before
live output (`logs.ts:362-363,402`). Vessel writes no boundary banner on
either stream: stdout remains session output and stderr remains status and
real stream notices, preserving the same capture contract for follow.

### MCP consumer contract (separately integrated)

The tools consumer owns decoding/schema/facade integration; this node package
exposes the typed spec and selection boundary without shipping another decoder.

MCP discovery follows the same negotiated default as the CLI. If an older fleet
hop refuses an implicit Shown read with Unsupported, that implicit default alone
may make one legacy null read. Explicit scope or labels never fall back, and
no other refusal or transport failure is retried. Paseo has no such forwarding
hop; this preserves the existing full-list compatibility contract. Invalid
metadata is decoded before even target-name lookup. Existing-session schedules
and heartbeats refuse creation-only labels/listing instead of discarding them.


On the session's own `vessel` server, as one protocol call each:

- `create_session` gains `labels` and `listing` (`shown` or
  `background`). Omission is the inheritance above. The description
  says to set `background` only when the person asked; the node does
  not try to decide that.
- `set_labels` is the patch.
- `list_sessions` gains `scope` and `labels`.

No reload tool and no log-filter tool.

### Older peers, both ways

| Direction | What happens |
| --- | --- |
| New client, old node | A call that uses a new field or a new method is not sent. The CLI says the node is too old. Plain `session.list`, `session.create` and `vessel log --from` stay as they are. |
| Old client, new node | `session.list` of `params: null` returns every session, background included, with listing and labels removed from each `SessionInfo`. `labels_changed` and `reloaded` arrive as a `HarnessLog` line (`vessel: labels: …`, `vessel: reloaded`) at the same sequence, so the log does not grow a gap. Unknown methods are the node's existing refusal. |
| New client, new node, through an old hop | `Call::since` includes the new fields, so the client does not hand a background listing or a label patch to a hop that would drop it and create or update the wrong session. |

### What reload is allowed to reach

Reload starts no listener, no browser and no script of its own. The
replacement is the session's existing harness, launched as that session
is already launched: the same profile, the same working directory, the
same agent credential already held in the session's `vessel` MCP server.
It gains no second credential. The refusals above happen before any
spawn. A test fake of a harness stays behind `cfg(test)` or a test-only
feature a release build cannot enable; this track adds no fake that
answers in a product build.

## Differences from Paseo

Each line below was read. The quote is the behaviour; the reason is why
Vessel does not copy it.

- Labels have no maximum in Paseo's schema (`messages.ts:915`,
  `labels: z.record(z.string(), z.string()).default({})`). Vessel caps a
  key at `LabelKey::MAX_CHARS`, a value at `LabelValue::MAX_CHARS`,
  and a session at `SessionLabels::MAX`, and refuses
  `=` and every character for which `char::is_control` is true, the same
  check `SessionTitle` uses, including the C1 controls. Reason: a pairing
  label already has its own scalar bound, and an unbounded map is a reply
  the other list calls refuse rather than cut. A control the title
  refuses must not reach the same list through a label.
- Paseo's wire map is raw strings (`messages.ts:915,993`), and run
  (`run.ts:478-501`) and list filters (`ls.ts:126-136`) keep key
  whitespace and allow an empty key. Vessel trims keys and refuses
  empty ones at every ingress. Reason: creation, updates and filters
  share one validated identity instead of disagreeing about a key.
- Paseo formats labels using `Object.entries` (`update.ts:144-149`).
  Vessel orders by key and uses comma-space separators in human summaries.
  Reason: storage and display stay deterministic regardless of the sender's
  map construction order, and summaries follow the existing CLI list style
  (`vessel-cli/src/devices.rs`, scope summaries). The wire map is unchanged.
- Paseo deletes one key with `null` (`applyLabelPatch`,
  `agent-manager.ts:631`). The CLI and MCP send only strings. Vessel's
  public patch only sets keys. Reason: the public surface has no
  delete, and a silent drop of an unknown key would hide a typo.
- A list filter without `=` is ignored by Paseo's CLI parser
  (`ls.ts:126`). Vessel refuses it. Reason: a typo must not look like
  an empty filter.
- An unknown `--filter` is a substring of the timeline type
  (`logs.ts:226`, the `default` arm). Vessel accepts only `tools`,
  `text`, `errors` and `permissions`. Reason: a typo must not print an
  empty log.
- Paseo's permission filter tests timeline item type names, whose declared
  union has no permission item (`agent-types.ts:375-389`); permissions are
  separate stream events (`:424-435`), and the log command selects timeline
  items (`logs.ts:288-295,384-393`). Vessel selects its typed, durable
  permission requests and resolutions. Reason: the existing log contains
  these approval records, and the named filter should expose them.
- Background is creation-only `workspace.background` in Paseo
  (`docs/agent-lifecycle.md:186-203`). Bound Vessel sessions derive their
  immutable listing mirror from canonical workspace visibility (ADR 0082).
  Only legacy/unbound sessions have an independent listing: they predate
  canonical workspace membership, and a shared project folder is not that
  membership. The creation bridge refuses conflicts instead of keeping two
  authorities. It remains outside this package until workspace creation lands.
- `run -d/--background` in Paseo is a hidden legacy alias for `--no-wait`,
  not workspace visibility (`run.ts:30-36,396-414`). The CLI retains that
  alias. Discovery uses the CLI's list filters, and bound visibility uses
  workspace creation's background choice; no run flag is repurposed here.
- `ls -a` includes archived and background (`ls.ts:149`,
  `options.background || options.all`). Vessel's `--background` only
  adds background sessions. Reason: Vessel already lists archived
  sessions; this track does not change that.
- User-facing reload wipes the in-memory timeline
  (`agent-manager.ts:1627`, "the timeline is wiped"). Vessel keeps the
  log and drops the replay. Reason: the log is the cursor; deleting it
  leaves a gap.
- A closed agent is loaded by refresh (`session.ts:4760`, the branch
  that calls `ensureAgentLoaded`). Vessel refuses a session that is not
  live and names `session.resume`. A parked session is refused and
  names `session.wake`. Reason: those two calls already own those
  starts, and this track must not grow a second one.
- After a successful close, a failed resume leaves the agent closed
  (`docs/agent-lifecycle.md:47`, "leaves the durable agent closed and
  retryable"). Vessel does the same: the session is `failed`, the log
  stays, `session.resume` retries. It does not start the old process
  again, which is what a failed move does. Reason: that sentence, not
  the move's recovery.
- If the turn's interrupt times out, reload fails and the turn stays
  (`docs/agent-lifecycle.md:116`, "keeps its active foreground turn").
  Vessel does the same, and does not then kill the process.
- There is no MCP reload tool (no `reload` registration was found in
  `paseo-tools.ts`; the RPC is `refreshAgent`, `reload.ts:66`). Vessel
  adds none and refuses the RPC to agents. Reason: the same absence.
- Paseo's follow writer joins fragments (`logs.ts`,
  `createFollowTranscriptWriter`). Vessel's log view already joins
  reply chunks, and follow uses that. Reason: one renderer.

Not read, so not asserted: the app's reload menu handler beyond the
tooltip string in `packages/app/src/i18n/resources/en.ts` ("Reload agent
to update skills, MCPs or login status."), and any label maximum outside
`messages.ts`, `update.ts`, `import.ts`, `ls.ts` and `paseo-tools.ts`.

## Why this and not the others

The session is already the thing Vessel lists, reads and restarts. Labels
and a listing fit the title's split between "what was asked" and "what
is true now". Reload fits the stop and `session/load` a move and a wake
already trust, without taking over their failure rules. The log's new
flags sit on the printer, so the protocol does not grow a second read.
The bounds are a floor Paseo does not have: a pairing label already has a scalar bound, and an unbounded map would be a reply past the budget the
other list calls refuse rather than truncate.

## Advantages, costs and effects

- A current client can hide a new session, mark it, filter it, restart
  its process, and follow a slice of its log. An older client keeps
  working, and still sees a hidden session because it cannot ask.
- `session.reload` does not edit parking, rewind, fork, move, or any
  driver. If the session runtime cannot restart without a new function
  in `vessel-agent`, the implementation stops and reports instead of
  editing that crate.
- The schema append and `lifecycle_version::SINCE` land with the first code
  slice. `PROTOCOL_VERSION` stays the base until the integrator sets
  the constant and, in that same merge, the published version.
  `docs/PROTOCOL.md` and the store sentence of `docs/ARCHITECTURE.md`
  change then. No crate edge is added, so ADR 0001 and the crate table
  do not change.
- The page learns the fields when `web/src/wire` is regenerated with
  the protocol change. This track does not draw a screen.
- Not verified here: no process was restarted and no list was filtered.
  The slices after this one implement the contract.


### Nonempty label changes at the input edge

[Decision Log]
- Purpose: a label mutation must contain a change before any actor or stored
  session is consulted. An empty label map remains valid as session state,
  creation input and a discovery predicate.
- Existing assets: `SessionLabels` already bounds and validates every map entry;
  its `apply` constructs an atomic result and checks the combined map's cap.
  Paseo's lifecycle command rejects an empty metadata update
  (`agent/lifecycle-command.ts:163-192`); its CLI validates before connecting
  (`agent/update.ts:150-201`).
- Alternatives: changing `SessionLabels` to reject empty maps would break valid
  state and reads. A boolean mode or a separate raw map would split validation.
- Choice: a `LabelPatch` wraps the existing validated map, without `Default` or
  mutable access. Construction and deserialization refuse `InvalidLabel::Empty`;
  deserialization first uses the map's bounded visitor. Serialization retains
  the same object shape, with the TypeScript derive kept for later call wiring.
- Why: the type carries the known input invariant across module boundaries while
  `SessionLabels::apply` retains the context-dependent combined-cap check.
- Effects: no new dependency or bound, method registration, event, storage write
  or protocol assignment is introduced by this value foundation. The later
  mutation package must prove actor/stored ordering and compatible projections.
