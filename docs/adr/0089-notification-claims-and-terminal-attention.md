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
`terminal/activity/terminal-activity-tracker.ts:12-53`; client
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
retried into a second delivery. Admission storage/retention, cancellation and
revocation are specified with the implementation before call registration.

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

Resume: contract proposal only. No new call is registered. Terminal producer,
admission persistence and exact field/call shape are completed before Quality B.
