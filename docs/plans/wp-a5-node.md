# A5 node: authoritative attention and recipient admission

Status: staged source, pending changed-source checks and V3 acceptance. The one
ATTENTION_NOTIFICATIONS_PROTOCOL constant remains None. No assigned-wire,
complete package gate, native delivery or platform parity is claimed.

## Reference and scope

Paseo 0.15.4 reference paths:

- `packages/server/src/server/agent-attention-policy.ts:3-79` and
  `websocket-server.ts:2838-2888`: 180 s activity, focused-target suppression,
  most-recent eligible local recipient, otherwise push.
- `packages/server/src/terminal/activity/terminal-activity-tracker.ts:12-80`:
  semantic working/idle/needs-input/clear/interrupt; unknown plain shells have
  no inferred state.
- `packages/server/src/server/push/token-store.ts:34-42,95-141`: 48 h lease, renewal
  at or after half life, revoke/prune/persist. Vessel retains the ADR 0041 account-free
  Web Push difference, not Expo/APNs/FCM or mobile permission/UI parity.
- `packages/app/src/utils/agent-attention.ts:18-90` and
  `hooks/use-agent-attention-clear.ts:112-150`: reading never answers permission;
  attention arriving in place defers until real input/send/blur.
- `packages/app/src/subagents/workspace-root-policy.ts:6-16` and
  `utils/workspace-agent-activity.ts:22-48`: canonical workspace-relative roots
  and the latest eligible non-archived root. Workspace mutation integration is
  still owed; retained registry membership alone does not establish eligibility.

## Decision Log

- Purpose: one source of durable unread/waiting facts and one node decision about
  local delivery or push, including terminals without fabricating session ids.
- Existing assets/constraints: Inbox projection and marking writer, push service,
  terminal registry/actor, generated method table, existing IPC transport and
  authenticated connection principal already own these responsibilities.
- Alternatives: client election, replayed logs, another read engine, PTY text
  heuristics, a public hook endpoint or fabricated general-purpose principal.
- Choice: modify/reuse those assets; keep values in proto, transactions in store,
  connection admission in node and semantic terminal state in session runtime.
- Why: existing transactions, lifetime fences and generated policy remain the
  authorities. Standard Rust/existing dependencies suffice; no library is added.
- Impact: new calls remain unavailable until the integrator assigns the shared
  protocol; each future consumer must retain exact source/revision/context.

## Contract and modules

Five pending rows live only at the end of `vessel-proto/src/rpc/table.rs`:
notification.claim, terminal.attention_clear, terminal.activity, inbox.attention,
and inbox.read. Generated guards/envelopes/SDK share that table. The first,
fourth and fifth are direct-person/nonforwardable; terminal operations retain
existing forwarding/scope and target ownership checks. Agent/delegated contexts
cannot consume a person's read or recipient grant.

Optional terminal focus/idle age request fields use Call::since field rules.
TerminalInfo attention is omitted for unsupported peers in direct/list/watch,
script and nested fleet replies. Pending methods never acquire a provisional
number. One durable claim migration follows the current store head; the node
integrator owns final protocol/schema numbering at integration.

| Module | Responsibility |
| --- | --- |
| `vessel-proto/{notifications,inbox_attention,push}` | Typed source/revision/activity/read values and pending capability |
| `vessel-store/inbox`, `push`, `writer` | Exact shared marks, rollups, committed recipient claims and lease admission |
| `vessel-node/service`, `sessions` | Authenticated admission, pending veto, cancellation-safe existing mark task |
| `vessel-node/push` | Existing recipient policy, one coalescing delivery worker, generic payload |
| `vessel-node/terminals` | Registry ownership, bounded metadata wakes and activity capability lifetime |
| `vessel-session/terminals` | Semantic reducer, non-reused revisions, exact clear and once-only claim |

## Local recipient and source authority

Presence retains stable transport identity and node-timestamped actual activity.
Focus-only/periodic reports do not refresh it. Optional OS idle evidence is aged
at receipt; a cached sample does not keep a person present forever. Election
rechecks current direct human/device Read authority and original connection.

A session grant commits once per exact unread eligible Inbox cursor in the
existing writer transaction. Durable rows cascade with their Inbox source.
Authority, read/resolution/source and recipient/focus are rechecked after
commit. A lost reply spends the opportunity; it never authorizes another display.
Terminal grants use exact non-reused revision under a runtime mutex. No synthetic
SessionId is used. Native OS delivery remains generic with a process-local
opaque source map (ADR 0041/0089).

Origin/side-question/archive eligibility is shared by candidate selection before
LIMIT and final admission. Existing Creator filtering is not a replacement for
W2 current parentage or W11 workspace-relative root ownership; that integration
remains outstanding.

## Shared read and badge projection

inbox.attention accepts at most ATTENTION_BATCH distinct session identities and
reads all their current Inbox rows in one read transaction, independent of the
loaded sixteen-entry head. Missing/deleted sessions are omitted. unread/waiting/
failed remain Inbox-derived counts; failed is independent of Seen.

SessionAttention.explicit_unread separately reads committed sessions.marked_unread
in that same transaction, including empty/pruned Inbox. Exact-entry Seen, answer
or removal must not clear the deliberate whole-session choice. Existing whole
Read does. Historical log attention/read watermarks are never OR-ed into counts.
The native values consumer awaits reviewed handoff; it must use explicit flag OR
unseen count with pending/failure/work priority intact.

inbox.read is narrow direct-human Read authority through Sessions' existing
marking lock and cancellation-safe task. Live pending requests and the writer's
atomic durable pending veto both apply. The same Mark::Read transaction updates
the actually observed exclusive watermark, including delayed appends, and wakes
other readers after commit. General MarkSession scope is not widened.

Drafted fixtures cover empty Inbox + Unread, exact Seen preserving the flag,
whole Read clearing it, delayed append/pending veto, and two concurrent store
readers racing writer-committed Read/Unread. The latter is not paired UI proof.

## Terminal producer and lifetime

Explicit semantic reports and known script/service lifecycle share the existing
TerminalHandle info watch. Arbitrary PTY bytes cannot imply a job. Exact observed
revisions reject stale clear/finish; revision exhaustion never wraps. End/removal/
shutdown close producer admission. Ended attention may be read/cleared, but a
producer cannot restart ended work.

ActivityProducer's generated Handler owns only one TerminalHandle and rejects
every other method, including future generated methods. Existing IPC uses a
random terminal-scoped 256-bit secret, constant-time stored hash, redacted Debug
and a closure token. Inherited activity variables are scrubbed before exact
injection. Generation/admission stay off while the feature constant is None.
No listener or Principal is invented. W17 owns the CLI grammar and bounded silent
producer invocation; checked DTO/API handoff is still owed.

## One push worker for both sources

Inbox cursor and terminal revision watermarks are independent in one worker.
Startup baselines exclude pre-existing attention. Metadata watchers retain only
watch receivers, never a TerminalHandle, and stop on closed sender/shutdown.
Unknown/cleared/failed/removed/foreign/regressed terminal metadata cannot push.
No output watch, durable terminal replay or second delivery engine is added.

Each lease/burst gets one generic encrypted request, coalesced across both
sources. Crypto::encrypt -> Delivery::prepare (including DNS) -> final admission
-> prepared.send is reused. There is no preparatory await after final admission.
Factored lease SQL checks exact device/subscription/expiry, unexpired current
Read and unrevoked/existing device. Inbox source+lease checks remain one SQL
snapshot. Terminal admission awaits that lease check, then synchronously
rechecks exact runtime revision and presence. No session identity is fabricated.
Gone responses expire only the attempted stored lease; an old 410 cannot remove
a newer renewal. Leases renew at or after half of PUSH_LEASE only while current;
expired/revoked/missing/readless rows cannot resurrect.

Existing real HTTP/encryption/preparation cancellation/restart fixtures remain.
Their completion observer now requires the canonical Session source before
asserting exact keys. The terminal fake-node fixture propagates its outer timeout.
While the feature is None it proves only SDK refusal/no delivery. Assigned
positive terminal delivery and restart/runtime proof remain explicit obligations.
A preparation-barrier fixture now covers exact clear, newer Working, removal,
focused recipient, revocation and exact lease renewal at both Crypto and
Transport. It waits for attempt completion before shutdown, so shutdown cannot
make an absent admission check pass. While pending, its explicit capability
guard does not establish any positive cancellation proof.

## Verification state

V3 row162 inspected ba45a6e3..a032bfe65 and returned one A7 bound-duplication issue;
no other inspected security defect was asserted. Named-bound corrections are
committed. This was not a PASS or a review of the later terminal-push/explicit
unread deltas. The next review packet must include actual changed-source checks.

Actual immutable results retained in the private A5 evidence packet:

- `wp-inbox-node-20261115T113109Z-15805629`: strict check failed only the own
  writer apply function's 109/100 lines; helper extraction now reduces it.
- `wp-inbox-unit-20261115T113109Z-15805629`: proto 301 passed/1 failed on own
  forwardable-method expectation order; the two pending terminal names now match
  their table-end position. Fail-fast prevented subsequent crate execution.
- `wp-inbox-ts-20261115T112026Z-294d9cca`: 3 proto + 1 web generator tests passed;
  rc1 records the generated tracked proto.ts diff. Its commit/status/patch and
  hashes were verified before applying that exact artifact. Later explicit_unread
  still requires regeneration; generated whitespace is retained.
- Current strict/unit runs `wp-inbox-node/unit-20261115T123346Z-c2a75fda` are
  queued, not compiler results. Later fixture-only source also needs checking.

All new Cargo/rustc/build/test/generation runs use the build VM through
vessel-remote-gate. Static size checks pass without increasing any ratchet.
VM restart rc143 evidence remains an infrastructure interruption, not PASS.
No whole root gate, V3 acceptance, final native gate/capture or V4 exists yet.
Tests use fake agents/private temporary data; no production credential/service,
file ownership/permission or infrastructure changes have been made.

## Half-life boundary correction

Rereading the actual v0.15.4 token-store.ts:34-42 exposed an own implementation
off-by-one: the reference skips renewal only while remaining life is greater
than half. The SQL now includes exact half life. The own new fixture checks no
rewrite before half, renewal exactly at half, no second rewrite a step later,
and late old 410 preservation. Device/read/revocation/expiry predicates and
existing tests are retained. This intentionally changes only the own half-life
expectation and is separate from V3's earlier bound-duplication correction.
Changed-source execution remains owed.

## Unknown cannot establish untracked activity

The canonical pending DTO now has explicit Untracked. Known Shell construction
uses it; Unknown remains default/serde(other) for missing/lost/future evidence.
The runtime reducer accepts Untracked as a no-attention starting state, but only
explicit Working followed by Idle creates Finished. Clear loses tracking and
returns Unknown. This fixes an information-loss bug in the earlier native
adapter proposal. No extra wire call/number/scope/schema is introduced; reviewed
values adoption and regenerated TypeScript are still owed.

Run wp-inbox-ts-20261115T125329Z-b4989aa4 completed actual 3 proto + 1 web
generator tests, all passed. Its rc1 is only the generated tracked proto.ts diff.
Commit/status/all artifact hashes were checked, then its exact patch was applied
without hand-editing generated bytes. This adds explicit_unread from that source.
Later explicit Untracked changes the enum and still requires fresh generation;
this predecessor artifact is not claimed current for that variant.

Latest-root ordering remains a projection gap: current rollups contain counts
and deliberate flag, not canonical attentionTimestamp/updatedAt. Legacy activity
or sixteen head entries must not silently substitute for that missing evidence.
W11 keeps selection Unknown until authoritative ordering facts exist. The
same-transaction membership lookup for workspace Read/Unread is also still a
planned W11 adapter, not an available runtime API.

Actual scoped Store integration: wp-inbox-store-20261115T125350Z-b4989aa4
completed rc0, 2 passed/0 failed/0 ignored at 13:42:57 UTC. It executed pending
veto/delayed append watermark and two read-pool snapshots racing writer-committed
whole-session Read/Unread. This is neither paired-device UI nor a whole runtime
gate. The later enum/fixture changes were not present in that immutable source.

Exact wp-inbox-node-20261115T123346Z-c2a75fda strict result: rc101, old completion
fixtures still accessed NotificationSource as a struct (already corrected in
510a6be1a), plus three own lints: similar watch binding name, needless borrowed
Store argument and missing doc backticks. Those three are now corrected without
changing deadlines/admission. The queued corrected30e9 run predates these lint
fixes; its immutable result must be retained rather than relabelled current.

Corrected30e9 strict then reached two additional own test lints: the activity
capability test was 106/100 lines and used an ambiguous Default constructor.
Its unchanged unpolled/nonexistent-command handle fixture is extracted and uses
Leftover::default explicitly. Assertions, denial boundaries and deadlines stay
unchanged. The previously submitted dbf strict predates this test-only fix;
its immutable evidence must not be called current.

TS run wp-inbox-ts-20261115T134300Z-4d7ac8d6 returned rc101: all 3 proto TS tests
and the web declaration test passed, but the newly selected web primitive-alias
classification test failed on pre-existing LimitPercent. It remains declared as
FiniteNumber; the stale-entry test considers only primitive aliases. The test,
alias entry and usage type are unchanged by A5, including the approved82ae7c91
source. This is reported for baseline-owner disposition, not fixed by deleting
a head expectation. The step is NOT green.

The generator's exact Untracked enum patch is applied after checking source
commit/status/all artifact hashes. Proto/web generator inputs are byte-unchanged
between4d7 and the current node source. This records generated-byte provenance
and the passing declaration test, not a whole TS/strict/runtime PASS.

The coordinator classified the sole TS failure as the known base LimitPercent
alias defect, fixed centrally by 9dfdf0a1 (with ee43fee2) in the pending gate49
batch. This run counts with that named defect; no head assertion is edited and
no unchanged TS rerun is requested. Adopt the announced green node head later.

Exact acf19 strict/unit runs both stopped before executing tests because the
extracted fixture named Leftover in vessel-proc instead of its existing owner,
vessel_session::terminals. The path is corrected; no product/admission behavior
or assertion changes. Replacement narrow checks remain owed.

Executed changed-source strict evidence: exact00a47d41094eb512152db0bf55a1e149ab356302,
run wp-inbox-node-20261115T144650Z-00a47d41, five crates (proto/store/session/node/
client), all targets, Clippy -D warnings: rc0/36s at15:07:16 UTC. Cargo log SHA256
a22e3e5e221ff2e65fb0f2176b7ff444a90d3771fe37d065728b43410fa4f11a matches
the recorded step. This settles compilation/lints only; same-source unit run
is executing, and V3 source closure, whole root, assigned wire and native/CLI
producer proof remain owed. No protocol number or runtime adoption follows.

Same-source unit run wp-inbox-unit-20261115T144650Z-00a47d41 executed725 passed,
one failed: the new no-resurrection fixture assumed a revoked subscription row
remains. Existing devices::revoke deletes it transactionally, so QueryReturnedNoRows
was correct product behavior. The fixture now asserts exact absence after revoke
and unchanged expiry after scope loss/expiry; all three still assert no admitted
lease. No product code, boundary, deadline or security expectation is weakened.
The corrected test and changed-source strict checks remain owed.

V3 row162 at15:31 UTC closed ONLY the original A7 source return after reading
the complete7c926b41d correction and independently verifying exact00a strict
spec/step/log hash. The725/1 failure remains recorded; V3 also read the test-only
revoked-row correction against the canonical DELETE, without claiming execution.
Later terminal-push, explicit-unread, Untracked, exact-half renewal and conditional
cancellation additions remain outside source adoption and need separate complete
reading/checks. No owning whole, native/CLI or assigned-wire approval follows.

Source follow-up: the new lease test incorrectly treated internal list metadata
as delivery authority after scope loss. Listing may retain the unrenewed row;
current Read is rechecked at final admission. The node test now checks that
actual predicate; revoke still requires exact absence and expired/readless
leases retain no extended expiry. An explicit redundant trait import is removed
after actual strict diagnostics. Prior failed test/strict runs remain red.

Local OS failure attention was incorrectly sharing Web Push reason eligibility.
Pinned websocket-server.ts:2899-2904 elects a local recipient for errors, while
agent-attention-policy.ts:77-79 suppresses only error PUSH. The local store claim
and native opportunity/current-form gates now accept failed sources, preserving
exact identity, unread, authority, once-only and focus checks. Web Push filters
remain unchanged. New real-store and native opportunity regressions are owed
execution; no source/runtime PASS is inferred from this correction.

Before the owning whole gate, the announced common node head82ae7c91 is merged
as041c429cf. The merge is conflict-free; five pending attention rows and None
remain unchanged, and the Store product/tests are byte-identical to8829d9856.
Older queued Store checks retain their original immutable scope. A current
five-crate union narrow is required after they finish, before a whole/gate or
adoption claim. The known alias classification base defect still awaits the
announced gate49 head; it is not repaired by editing A5 head expectations.
