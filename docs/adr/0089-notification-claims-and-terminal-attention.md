# ADR 0089: One connected notification recipient and explicit terminal attention

Status: proposed.

## Purpose and intent

Complete the connected-client part of ADR 0041's notification policy: exactly
one recently active eligible connection may display an attention notification,
while a visible client focused on that target suppresses delivery. Add explicit
terminal activity and attention without guessing from arbitrary PTY output.
The durable session inbox remains ADR 0040's authority for requests and reads.

## Existing implementation and constraints

The node's push policy already computes Suppressed, Local(recipient) or Push
from connection presence, using the reference's 180-second presence threshold.
Only the push worker consumes that decision. No wire response tells a desktop
that it was selected, so several connected clients can independently notify.
The existing presence call targets a session, elsewhere or background; it has
no terminal focus target. TerminalInfo reports process state but has no
explicit activity/attention revision. A plain shell's output cannot establish
that the person needs to answer anything.

Paseo reference (0.15.4): server `agent-attention-policy.ts:3-79`,
`websocket-server.ts:2838-3010`, and
`packages/server/src/terminal/activity/terminal-activity-tracker.ts:12-80`; client
`hooks/client-activity-tracker.ts:1-117`,
`notification-open/model.ts:16-84`. Read source before implementing the
corresponding service; this decision does not assert runtime parity yet.

## Alternatives considered

1. Keep per-window local watermark delivery: retains duplicate desktop alerts.
2. Add unsolicited delivery frames to every stream: requires another ordered
   fan-out and reconnect contract, duplicating the durable inbox projection.
3. A typed exact-source claim by the current eligible recipient, reusing node
   presence and store rechecks; explicit terminal attention revisions.
4. Infer terminal prompts or work from output text: incorrect for plain shells,
   arbitrary programs, localization and malicious escape sequences.

## Choice

Choose exact-source claims. A claim carries only an existing attention identity
and the reporting node's cursor/revision. The node checks current unread/live
attention, current effective Read authority, the connection's current presence,
the recipient decision and prior admission before returning permission to
notify. A reply has explicit outcomes for admitted, not recipient, focused
suppression, withdrawn/read, and already admitted. Unknown future outcomes
never authorize delivery. Claims do not resolve or mark attention read.

All identities are typed and scoped to the authenticated reporting connection;
a caller cannot nominate a different recipient. An uncertain claim is never
retried into a second delivery. Session admission uses one persistent table keyed by inbox cursor with an
ON DELETE CASCADE foreign key to the inbox row. Its population is bounded by
the existing inbox retention and live request limits. Atomic insert rechecks
current unread/eligible source and a paired device's current Read grant;
owner-origin claims have their existing owner authority. A post-insert presence
or disconnect recheck may suppress best-effort delivery but never reopen that
source for duplicate admission. Losing a notification opportunity leaves the
inbox unchanged. Terminal admission is runtime-only, keyed by terminal revision;
terminals themselves do not survive node restart.

Terminal attention has an explicit Unknown state for a plain shell. Known
activity is Idle or Working; attention is Finished or NeedsInput with an exact
non-reused revision. Only structured lifecycle/hook input owned by that
terminal may change activity. Working to idle yields finished; an explicit
needs-input signal yields needs-input; visible focus clears only that exact
revision. Raw PTY output is never a semantic input. Session and terminal
notification targets stay separate typed variants; no fabricated session id
stands in for an unattached terminal.

The terminal integration producer and the existing terminal actor are joined
through their typed boundary; a UI must not invent activity for a terminal
whose producer cannot report it yet. The native terminal owner receives the
contract before its controls are wired.

## Privacy, authority and older peers

ADR 0041's privacy floor remains: OS/push text is generic. Session titles,
paths, prompts, commands, questions and source identifiers never enter the OS
payload. The desktop keeps a bounded process-local mapping from an unrelated
random notification tag to the source. A stale tag after restart/eviction opens
the inbox without choosing another source. The existing UUID implementation
supplies the unrelated tag at the app boundary, not in the pure view model.

This extends ADR 0041's stated per-window recipient limitation. Focus handling
follows the reference's focused-target suppression rather than inventing
activity merely because a socket or window remains open. Web Push remains the
accepted account-free mobile delivery difference; no Expo/APNs/FCM credentials
or platform service are presumed to exist.

Agent and forwarded/delegated origins cannot claim a person's OS delivery.
Read-only devices can read/mark and receive notifications but cannot answer
permissions/questions. Revoke and disconnect invalidate connection presence
and pending admission. A typed node error contains no raw source content.

New calls/fields use one unassigned Option<ProtocolVersion> feature constant
and the shared required_protocol accessor. No provisional numeric version is
chosen here. Fields are hidden in every direct, list, nested and forwarded
older-peer reply; old clients receive no unsolicited new frame. Both directions
of unassigned/older-peer refusal are tested before Quality B registration.
No schema number is chosen by this document; the node integrator assigns it
once admission persistence is specified.

## Benefits, costs and effects

The store remains the lifecycle authority and one node owns recipient choice.
Clients cannot turn unread-count changes into repeated notifications. Explicit
terminal revisions preserve first-clear behavior and honest unknown state.
Costs are a bounded claim operation, admission retention, and platform-specific
terminal producers; those gaps remain visible until exercised against fakes.
No new network listener, shell helper or provider credential is introduced by
the claim boundary. Pure state/routing is cross-platform; platform delivery
and terminal producer support must be reported separately on Windows/macOS.

## Verification and resume

Required tests: two concurrent clients and equal-activity tie; focused target
suppression; absent/stale presence; read/resolve/revoke/disconnect before claim;
duplicate/uncertain claim; restart and bounded retention; session/terminal
identity isolation; explicit terminal transitions and stale revision clears;
all unassigned and older-peer paths; no source contents in OS payloads.

Resume: contract proposal only. No new call is registered. Value contracts and the pure terminal reducer are staged. The terminal
normal-client report carries the exact observed revision; supplied stale revisions conflict
instead of completing a newer run. Next are store admission tests, connection
policy integration, then pending Quality B rows and older-peer projections.

## Activity-only local producer capability

Paseo CLI hooks.ts uses per-terminal activity attribution and silently ignores
missing setup or failed reports. Vessel uses the existing local IPC endpoint,
not another HTTP listener. A TerminalActivityAuth frame authenticates a random
256-bit per-terminal secret; only its constant-time-verified hash is retained.
The capability is injected into a shell only after the protocol is assigned,
with an exact terminal id and socket/pipe path. Its credential dies at terminal
end, explicit removal or node shutdown. Inherited hook target/secret/socket are
scrubbed before another terminal receives its own capability. CommandSpec Debug
already hides environment values; the new auth/secret Debug is redacted too.

[Decision Log]
- Purpose: a CLI/provider hook may report its own terminal without acquiring
  owner, agent, session, inbox, filesystem or push authority.
- Existing constraints: normal terminal.activity requires Operate and the exact
  observed revision, but a hook must work without a general node credential.
- Alternatives: fabricate an owner/agent principal; add a public HTTP hook port;
  add a terminal role to every principal match and hope other methods refuse it.
- Choice: a separate ActivityProducer Handler generated from the same one method
  table. It holds only TerminalHandle, accepts only ReportTerminalActivity for
  that id, and rejects every other current or future method by construction.
- Why: no general principal is fabricated and no second method list or listener
  is introduced. Local capability authentication precedes the usual handshake.
- Impact: normal clients still require a revision; this activity-only handler may
  advance its own current revision when absent. Explicit supplied stale revisions
  still conflict. This never writes session turn state. CLI maps provider events
  to the semantic signal and retains its bounded, silent failure behavior.

The producer contract, environment injection, closure/denial tests and actual
CLI consumer must pass before the feature's pending protocol is assigned. A
wire test with the final assigned protocol remains required at integration.

A hook without an explicit revision is scoped to its terminal and follows the
reference producer's event ordering. It does not attest which provider command
generation produced an event. Revision-supplied reports and all attention-clear
callbacks retain exact revision checks; no stronger causal ordering is claimed.

## Desktop idle evidence and lease renewal

An optional push.presence idle_age is a nonnegative elapsed millisecond newtype,
not an absolute client timestamp. It uses the same pending feature constant as
terminal focus; older peers never receive it. The server computes its own now
minus the elapsed age, and never moves activity behind newer actual input.
Native samples retain their monotonic observation time so queueing and repeated
heartbeats age the evidence instead of keeping an idle window present forever.
Unknown platform evidence is absent; ordinary in-app input remains independent.

Paired heartbeat renews only its caller's existing active Web Push lease, after
half of the 48h lifetime. The committed update rechecks current Read/revocation
and never resurrects an expired or missing lease. Node-owner and agent/delegated
origins cannot borrow another device's subscription identity. A renewed expiry
also prevents a late old 410 from deleting that newer lease.

## Local error attention is independent of Web Push eligibility

Paseo websocket-server.ts:2899-2904 passes every attention reason to the local
recipient election, while agent-attention-policy.ts:77-79 excludes error ONLY
from push eligibility. Local admission therefore accepts an exact unread failed
Inbox source with the same device/source/read/once-only checks. Web Push
candidates and final push admission still exclude failed outcomes. This server
recipient/admission rule is not the client OS-delivery rule: Paseo
`packages/app/src/contexts/session-context.tsx:267-269` returns immediately for
error. The native opportunity and final projection gates therefore exclude
Failed while retaining the source in Inbox and failed badges. Generic OS text
and focus suppression do not change. A real-store regression checks the server
local once-only admission, shared read suppression and actual Web Push query;
separate native tests check error suppression. No new wire or scope is added.
