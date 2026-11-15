# Connected notification routing and terminal attention

Status: staged pending contracts, ADR 0089; narrow verification in progress.
No assigned protocol, whole gate or V3 approval yet.

Existing assets: the node's push presence and policy, transactional inbox
projection, terminal actor and typed terminal registry are reused. Recipient
nomination is currently computed only for push suppression; terminal activity
is absent from the wire. The native app's node boundary must not guess either.

Decision: modify existing policy/registry and add a typed exact-source claim.
Rejected: a second inbox, arbitrary PTY prompt detection, client-side election,
and a new public hook listener. Standard Rust state and existing dependencies
suffice for contract values; no new external dependency is proposed here.

Next: read the reference terminal producer and existing terminal actor, specify
admission persistence/retention and the connection-bound claim outcomes, then
submit pending rows through the node integrator's Quality B. Pure types and
staged service tests precede registration. Every new wire field gets all-path
older-peer tests. No production node or real agent is used.

## Wiring checkpoint after Quality B

Initial checkpoint merged green node29eebb8b under A8. Its first three pending rows used only method_table:
notification.claim (Read, direct person connection only), terminal.attention_clear
(Read, person only), terminal.activity (Operate plus existing terminal ownership).
The one protocol constant remains None; no production peer can receive or send
these calls or the added terminal focus until the integrator assigns it.
TerminalInfo attention is optional and rewritten on direct/list/subscription,
script and nested fleet replies for all unsupported peers. Existing push policy
is reused with stable connection ordering to break equal-activity ties.
Session admission commits once per inbox cursor; terminal admission is once per
current revision in the terminal's bounded lifetime. Source/authority/focus are
rechecked after preparation. Lost local grants never authorize a second display.

[Decision Log]
- Purpose: exact terminal clears and semantic attention without a second parser.
- Existing constraints: raw PTY output has no reliable shell lifecycle meaning.
- Alternatives: output heuristics, polling guessed prompt text, separate public hooks.
- Choice: reuse terminal info watch and explicit revision-checked producer reports;
  known script/service lifecycle feeds the same reducer. Plain shell stays unknown
  until explicit activity. Clear/interrupt never fabricate finished attention.
- Why: stale hooks and view callbacks cannot consume newer work, and no new listener
  or credential is introduced. Person read marks remain separate from agent work.
- Impact: native D1 consumes the snapshot through C2 and the single pane contract;
  a real shell integration producer is still an explicitly outstanding parity row.

Validation checkpoint: first narrow proto check found serde(other) positioned
before other variants, corrected. Subsequent contract clippy/tests are pending.
At that earlier checkpoint the size gate was blocked solely by shared rpc.rs
503/500 with the first three rows. The authorized W17 extraction and gate40
merge removed that blocker; five pending rows now live at the end of table.rs.
No budget was raised.
No whole gate, V3 security review or app consumer handover has occurred.

## Producer extension

Activity-only local authentication now reuses the existing IPC transport with
TerminalActivityAuth and a generated, deliberately restricted Handler holding
only TerminalHandle. No new Principal variant, public listener or second method
table is introduced. Normal clients still need Operate/ownership/exact revision;
the hook capability can advance only its own terminal. W17 owns CLI hook grammar
and bounded silent invocation, waiting for the reviewed DTO/hash before wiring.
Capability/env injection and denial tests are drafted, not yet compile verified.

Coordinator authorized immediate A8 merge of V3-passed W17 seams1c8d0b93 and
ed3a3323; merged asd493254f/304d29d8. LocalGateway assembly now lives in IPC's
constructor so node.rs carries one registration line and no credential policy.
Measured node.rs688 versus recorded691; records are not increased. rpc.rs is
below500 after the shared contract extraction. Auth handler test runs on an
explicit current-thread runtime with an unpolled actor and nonexistent command;
no provider or real process is launched by the capability checks.

## Badge/read seam

N1 confirmed exact-entry inbox reads never changed SessionInfo log marks. The
existing SQL projection is reused by inbox.attention (256 distinct sessions,
all pages, one read transaction). New inbox.read is a narrow Read-scoped direct
human ingress through Sessions' existing marking lock and cancellation-safe
task, live pending-request veto and the writer's atomic durable pending veto.
It calls the same marks::mark transaction so delayed appends inherit its actual
observed watermark. No general MarkSession scope is widened. Mark/read publishes
post-commit inbox wakeups for other devices. ADR0040 clarifies the distinction.
Pending rows now total five; same unassigned feature constant, no new schema.

OS idle age is optional and version-gated. An aged cached sample cannot renew
activity; actual input is node-timestamped. Existing paired push leases renew
only after half of their 48-hour life; expired/revoked/readless leases never
resurrect, and an old 410 cannot delete a newer lease. New tests are drafted;
these additions are not yet compile/runtime verified.

## V3 security source return and bounded correction

Row162 source review of ba45a6e3..a032bfe65 returned one A7 issue: duplicated
bounds in a service error and boundary fixtures. The service message now formats
ATTENTION_BATCH. Lease half/full/after-bound tests derive from PUSH_LEASE and a
named one-hour step, preserving every assertion. Wire-only presence gating uses
arbitrary non-boundary elapsed samples. The exact terminal tracker reference
path is corrected. TypeScript quantities classify elapsed idle and revision;
regeneration and exact changed-source checks are still owed. V3 asserted no
other inspected security defect; this is not a source PASS or runtime evidence.

The next strict check found the two new reply variants missing from the
pre-existing agent-field older projection's exhaustive no-op arm. They are
added there; no field or authority semantics change. Session rollups now also
carry current failed outcomes, independently of unread, so native five-bucket
status cannot confuse a read failure with a finished result. A regression marks
a failure read and verifies failure persists until a new turn removes it.

The pre-existing local queued check finished under the operator's existing-job
exception. It was a non-strict check in a changing worktree, so it is diagnostic
only, not immutable-source evidence. It exposed two test warnings after the
ProcessLifetime base merge: unnecessary TimeDelta qualification and an ignored
TaskTracker::close boolean. Both are corrected without changing runtime logic.
All new compilation remains VM-only; immutable strict checks are still pending.

## Generated TypeScript evidence

Remote named TS run `wp-inbox-ts-20261115T112026Z-294d9cca` executed four selected
Rust generator tests successfully (3 proto + 1 web). The step returned rc1 only
because generated `web/src/wire/proto.ts` differed from the tracked baseline.
Its retained source commit, status, tracked patch and every SHA256SUMS entry were
verified locally; that exact generated patch is applied without hand edits.
Proto/web generator sources are unchanged between 294d9cca3 and 158056295.
The generated file retains the generator's whitespace. This establishes the
artifact's origin/content, not a whole root gate or assigned-wire behavior.

## Deliberate unread with an empty/pruned inbox

N1 confirmed the separate `explicit_unread` rollup flag from committed
`sessions.marked_unread` is required: it is read in the same transaction as
inbox unread/waiting/failed counts, without OR-ing historical attention/read
watermarks into those counts. Exact-entry Seen, answer and removal leave this
whole-session choice intact; the existing whole-session Read clears it.
The pending method's DTO carries this distinct flag; no new method, number,
scope, migration or read engine is introduced. UI Review can use flag OR unseen
entries, with pending/failure/work priorities unchanged. Drafted fixtures cover
empty inbox, appended unread result, exact-entry Seen and actual session Read.

A real-store integration fixture also races two read snapshots against repeated
writer-committed whole-session unread/read toggles on one retained outcome. Each
snapshot must return a matching deliberate flag and inbox count; this is a
transaction/read-pool regression, not proof of paired-device UI synchronization.
