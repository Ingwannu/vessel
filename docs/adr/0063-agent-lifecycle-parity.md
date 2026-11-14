# ADR 0063: Labels, background, reload, and the log's selection

Status: proposed, 2026-11-14 (milestone 12). The protocol number is the
one given at merge. This commit does not change `PROTOCOL_VERSION`.
New calls and fields are gated on one constant, `lifecycle::SINCE`, so
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
- The store's schema is 14. A migration is appended, never edited.
  `PROTOCOL_VERSION` stays that base. Minimum stays 2. `lifecycle::SINCE`
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
session and on `session.list`, all gated on `lifecycle::SINCE`. The
next schema append (when the code lands) stores the current labels and
the listing. `PROTOCOL.md` is updated in the commit that the integrator
uses to publish the number, not in this one.

### Labels

A label is a `LabelKey` and a `LabelValue`. A key is trimmed, then 1 to
64 Unicode scalars, with no ASCII control and no `=`. A value is not
trimmed, 0 to 256 scalars, with no ASCII control. A session holds at
most 32, in a `SessionLabels` map ordered by key. A key starting with
`vessel.` is reserved and refused. Anything else is `invalid_request`
naming the key, before a session is created or a row written.

The public patch only sets keys. Keys omitted from the patch stay.
There is no clear. An empty patch is `invalid_request` ("nothing to
update"), not a silent success. The same key twice in one request keeps
the last value.

Set at `session.create` (and so at `vessel run`, including `--resume`),
copied by `session.import` with the row, and replaced as a patch by
`session.labels` (`operate`, forwardable, since `lifecycle::SINCE`).
The reply is the
`SessionInfo`. The log gains `labels_changed` with the whole map, so the
latest record is the state. `SessionSpec` keeps what creation asked;
the column keeps the current map, as the title's column does.

`session.list` gains `labels`. Every listed session must carry each
pair (logical and). Empty means no label filter.

### Background

`SessionListing` is `shown` or `background`, chosen at creation, never
updated. Omitted means `shown`, except when the caller is an agent: then
omission inherits that agent's own session listing. An explicit value
always wins, including a schedule's run made as an agent. A person's or
a schedule-definer's omission is `shown`.

`session.list` gains `scope`: `shown` (default) or `all`. `shown` leaves
background sessions out. `all` includes them. It does not change which
archived sessions appear; those stay listed as they are today.

A peer that negotiated a protocol older than `lifecycle::SINCE` has no
field. The node includes background sessions in that peer's list and
rewrites each `SessionInfo` so the listing reads `shown` and the labels
are absent. A current client does not send `scope: all`, a label
filter, a background listing or any labels to an older node
(`Call::since` is `lifecycle::SINCE` when any of those is present).
`params: null` keeps decoding as the default request.

`session.get`, attach, log, send, kill, rename, labels and reload do
not care about the listing. A background session is still there.

### Reload

`session.reload` (`operate`, forwardable, since `lifecycle::SINCE`)
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
- `--filter tools|text|errors|permissions`. Anything else is a usage
  error, before connecting. `tools` is a tool call and its update;
  `text` is a user turn, a replayed user chunk, a reply chunk, a
  thought and a steered message; `errors` is a failed turn and a
  `failed` status; `permissions` is a request and its resolution.
  `HarnessLog` is not an error.
- `--since` an RFC 3339 timestamp. Unparseable is a usage error. A
  record's `at` must be greater than or equal to it.
- `-f` / `--follow`. After the selection, wait on the session stream
  for later matching records until interrupted. No sleep.

Order: `--from`, then `--since`, then `--filter`, then `--tail`. Follow
continues after the last record it printed. Rendering stays the view
that already joins chunks. A terminal session has no agent events of
those four kinds, so a filter prints nothing of its bytes.

These flags are not a wire call. `read_events` is unchanged.

### MCP

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
  key at 64 scalars, a value at 256, and a session at 32, and refuses a
  control character or `=`. Reason: a pairing label is already bounded
  at 64 scalars, and an unbounded map is a reply the other list calls
  refuse rather than cut.
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
- Background is `workspace.background`, creation-only
  (`docs/agent-lifecycle.md:186`, "controls discovery only"). Vessel
  puts `SessionListing` on the session. Reason: a project is a shared
  folder, not an agent workspace; hiding the folder would hide sessions
  nobody asked to hide. The flag stays creation-only, as Paseo's does.
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
The bounds are a floor Paseo does not have: a pairing label is already
64 scalars, and an unbounded map would be a reply past the budget the
other list calls refuse rather than truncate.

## Advantages, costs and effects

- A current client can hide a new session, mark it, filter it, restart
  its process, and follow a slice of its log. An older client keeps
  working, and still sees a hidden session because it cannot ask.
- `session.reload` does not edit parking, rewind, fork, move, or any
  driver. If the session runtime cannot restart without a new function
  in `vessel-agent`, the implementation stops and reports instead of
  editing that crate.
- The schema append and `lifecycle::SINCE` land with the first code
  slice. `PROTOCOL_VERSION` stays the base until the integrator sets
  the constant and, in that same merge, the published version.
  `docs/PROTOCOL.md` and the store sentence of `docs/ARCHITECTURE.md`
  change then. No crate edge is added, so ADR 0001 and the crate table
  do not change.
- The page learns the fields when `web/src/wire` is regenerated with
  the protocol change. This track does not draw a screen.
- Not verified here: no process was restarted and no list was filtered.
  The slices after this one implement the contract.
