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

Merged green node29eebb8b under A8. Three pending rows use only method_table:
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
Size gate is blocked solely by shared rpc.rs 503/500 with the three added rows;
requested W17's approved contract extraction from the integrator. No budget raised.
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
