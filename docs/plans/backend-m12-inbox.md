# Milestone 12: inbox and push

Track N1. ADRs 0040 and 0041. The resumed slice is node-only: wire,
store, node, client, CLI, testkit, generated declarations and documentation.
Native link/view/screens and desktop delivery are owned by the app workers.
The person-level acceptance below remains the overall contract; this slice
ends at the calls a screen consumes and does not claim those screens exist.

## Before writing code

- Existing assets: transactional session log and store writer, typed request
  forms, `SessionInput` and first-answer validation, CLI `conversation/answer`,
  node scopes/revocation, client protocol negotiation, app fleet and kit.
- Decision: modify/reuse those; add a durable attention projection and a
  UI-independent cross-node fold. Do not create a second answer validator,
  session timeline, permission card, transport or credential mechanism.
- External alternative: Web Push's encryption/VAPID builder, existing
  reqwest/rustls HTTP. The references use hosted push; no account exists here.

## Acceptance list (reference behaviour, step by step)

### Find everything waiting across machines

1. Connect two machines; ask for approval on each and a question on one.
2. Open Inbox: each request identifies machine, project, session and time;
   its title and tool detail/form are the agent's own, with its choices.
3. A partial, disconnected or failed machine says so; it never looks empty.
4. Long/Korean names and details wrap, many rows scroll, no items has a
   deliberate caught-up state. GOAT's `DecisionInbox.tsx` is the screen
   reference; Paseo has attention navigation/sidebar, no separate inbox page.

### Answer without opening a session

1. Choose the existing approval option or fill the existing question form.
2. Before sending, recheck the exact live item, connection and effective scope.
   Keep submitting disabled after ACK until the projection removes the exact
   source; failure stays beside it. Lost reply never triggers an automatic retry.
3. The actor confirms the answer; the request leaves on both paired devices.
4. A second answer to an already settled request is refused; a read-only
   device sees the same form and the refusal reason, without answer handlers.
5. Skip moves to the next request; opening goes to its session record.

### Keep read and unread across reloads and devices

1. Mark an entry read; reload and reconnect another paired device.
2. Both show read, and a waiting request remains answerable.
3. Mark unread; both agree. Revoking a device removes its access, not the
   already-shared read state. Reading a restricted device grants no operate.
4. Answered/withdrawn requests and previous outcomes after a new turn leave.

### See finished, failed and limited sessions

1. Finish one fake turn, fail another, end a third at a token/request limit.
2. Inbox kinds distinguish each; an ordinary cancelled turn is not success.
3. Open each at its event; starting a new turn removes its old outcome.

### Use the CLI

1. `vessel inbox list` lists the selected node's entries with stable ids,
   read state and the existing request's title and choices.
   `vessel inbox show ENTRY` expands one selected live form after its source
   has been pruned, without marking or answering it.
2. `vessel inbox answer ENTRY ...` resolves its session/request and reuses
   the existing typed question fill and permission selection code.
3. `vessel inbox read ENTRY` / `unread ENTRY` changes shared attention.
4. Unknown, read-only and already answered inputs fail clearly.
   Paseo's `packages/cli/src/commands/permit/{ls,allow,deny}.ts` is the
   behavioural reference; bulk implicit approval is deliberately not added.

### Notify when away, without leaking content

1. Initial snapshot/reconnect history makes no desktop notification or tone.
2. A new waiting/finished/failed/limit entry while the window lacks focus
   produces at most one notification/sound according to the preference.
3. Focus, read state and an already observed transition suppress repeats.
4. OS text is generic in English/Korean, with no agent content or path.
5. Click reconnects/opens the inbox; delivery failure is surfaced locally.
   Paseo attention-policy and GOAT notification-gate fixtures define the rules.

### Push to a disconnected paired device

1. Get the node's VAPID public key as a paired device; register a valid
   subscription. Agent, delegated and another-device writes are refused.
2. Disconnect that device; create attention. A bounded encrypted Web Push
   request is emitted with the node's VAPID identity and generic payload.
3. A recently active client selects local delivery rather than remote push;
   a window focused on the target
   within 180 seconds suppresses delivery. An idle open socket does not suppress
   push. Revoked/expired subscriptions are not sent. Restart preserves the
   key/subscription without replay; 404/410 prunes only the attempted lease.
   A same-subscription renewal survives a late response from the old attempt.
4. Private, loopback, mixed-DNS and redirected endpoints are refused; logs
   expose no endpoint token, key or content. File mode/ownership is checked.
5. A fake push receiver proves encrypted wire and decrypts in the test;
   no real push account, preview node or paid agent is used.

## Contract

Protocol number is assigned at merge; this checkpoint uses provisional 19
(minimum 2 unchanged), with inbox migration 14 after search's 13.
Inbox ids bind a session and event sequence, with a global per-node page
cursor. `inbox.list` is read scoped; `inbox.mark` is read scoped but refuses
agent callers. `push.key/subscribe/unsubscribe` require a local paired-device
principal with read scope; no caller-supplied device id. Answers keep the
existing `session.send` operate boundary and the session actor's validation.
New calls are rejected on an old negotiated connection before transmission;
old clients receive no new unsolicited event. The concrete types are `InboxCursor`, `InboxKey`, `InboxRead`, `InboxItem`,
`InboxPage` and `PushSubscription` in proto; exact source rechecks use
`inbox.get`. The page is 16 entries, complete item budget of 256 KiB and complete
pending pool of 32 MiB; the compact pending reference uses the actor's existing
64-per-kind-per-live-session bound. Outcomes: 2,048, tool snapshots: 32 KiB each /
128 per session / 512 globally, push subscriptions: 64 and 48-hour leases,
endpoint: 2,048 bytes before AND after URL normalization. HTTP 10s overall,
connect/DNS 5s, at most 16 DNS answers all public and pinned, no response body
consumed, HTTP/1 at most 32 headers (locked hyper buffer ceiling 417,792 bytes),
HTTP/2 header list 32 KiB. See ADRs for lifecycle and licences.

The following native proposal is historical, uninstalled and ungated in this
node checkpoint. App workers own the current model and screens; these draft
names must not be treated as merged APIs.

`link::inbox` bounds concurrent page/action calls to 4 and a 15-second deadline;
late results carry a `Ticket`. Unknown acknowledgement/disconnect never
repeats a write automatically. `view::inbox::Pages` keeps fresh heads for
shell totals and separate browsed pages. A disconnected fleet entry becomes
unavailable before a pending RPC timeout. `inbox_answer` returns only accepted
fresh heads for the shell notification hook; lookup/acted/older/late replies
never produce a notice. The OS payload has no content fields.

The shell owns OS delivery. Whole-focused-window silence deliberately follows
the task brief rather than Paseo's per-target-only local suppression. Node
push still follows Paseo's recent-recipient policy. A visible target A and
background target B with recent activity means Suppressed for A and Local for
B, not push for B; neither changes durable read state. The cursor scan tests
exercise both orderings, plus no-present/read candidate cases.

Admission is the final atomic SQLite read after cryptography and DNS:
current device, scopes, revocation, exact subscription/lease, source key/cursor
and unread kind are read together. Presence is checked synchronously after
that read. A change before admission cancels work; an already admitted
ciphertext packet cannot be recalled. Tests pause both crypto and transport
preparation and wait for the attempt's completion before asserting absence,
so shutdown cancellation cannot make a broken admission check pass.

Large forms use the live actor's existing pending projection through opt-in
`inbox.get` live-request mode (`NodeClient::inbox_request`). It recovers one
exact form rather than cloning the whole conversation; the reply cap is
15 MiB, with explicit compact references above it. Full pending forms survive
independently of pruned event records and tool caches. The fake `ask large` creates a valid oversized form,
emits 130 tool records and a marker; a newly connected client reads the full
current form and answers its final choice after source pruning. The inbox
never pretends an incomplete option set is the complete form.

The SQLite pool regression also fills the real pool to one byte below its
limit with two pending permissions for the same tool. Each updated row is
admitted in a separate statement step inside the append transaction: SQLite
may otherwise cache the old scalar total for every row. Omitted-content
markers participate in admission before encoding. Tests use the existing
`session.read` durability boundary before asserting a live stream's committed
inbox state; no timing sleeps stand in for the store commit.

## Historical checkpoint evidence (2026-11-13)

This is a node checkpoint, not a completed or review-approved slice.
Root formatting and whole clippy passed after the final test-expectation
correction. TypeScript generation (one proto and one bridge generating test)
passed before that test-only correction.
Web lint, typecheck, unit tests (55 files, 243 tests) and build passed. Web E2E
finished with 35 passed and one unresolved theme-paint assertion failure:
`page.spec.ts:34` expected `dark` and received `null`. No claim that it was
pre-existing or caused by this slice is made.

The exact `cargo test --workspace`, with `RUST_TEST_THREADS=3`, reached node
E2E: 144 passed and one large-form expectation failed. All 32 recovered field
payloads and choices were exactly equal; only numeric expected ordering
conflicted with ACP's lexical property-map ordering. The expectation now uses
that lexical order. Re-execution after that correction is still required;
the first run stopped before the later proto/session/store test binaries.
The node's new push crypto/policy/delivery/admission-race tests passed in that
run. No full-gate success is claimed. The resume note records final follow-up
results and the immutable checkpoint hash.

This checkpoint stops at the current node slice. Prepared native changes
remain outside the checkout; they are uncompiled and are not a finished API
for another track. The committed resume note records remaining contracts;
private working notes preserve exact draft and evidence locations.

Final checkpoint: the corrected whole test was stopped at the work-window
deadline, exit 143. It did not reach the corrected node E2E or store tests.
Whole clippy after the correction passed; web E2E's theme failure remains
unresolved. The slice stays WIP. All tracked test-process identities and the
web fleet were confirmed stopped. See `backend-m12-inbox-resume.md`.

## Node-line integration (2026-11-14)

The node line at `b981f5f2` is merged for validation before further work.
Inbox/push provisionally use protocol 19, minimum 2; their seven new calls
are `inbox.list`, `inbox.get`, `inbox.mark`, `push.key`, `push.subscribe`,
`push.unsubscribe` and `push.presence`. The appended store migration is 14,
after the existing search schema 13. Search indexing and inbox attention
share the writer's committed append rather than appending the records twice.
A rewind clears the previous branch's attention and cached tools; migration
backfill excludes outcomes before the latest turn start or rewind.

The previous theme-paint E2E failure is explained by upstream `59000e29`:
`preferThrough` displays the optimistic theme before its bridge op finishes,
so a single HTTP read can precede persistence. The merged test polls the
served index until the bridge confirms that state. The merged runtime checks below verify that correction.

The merged gate passed: root fmt, whole clippy with `-D warnings`, and
`cargo test --workspace` (1,814 passed, 10 default ignored tests,
`RUST_TEST_THREADS=3`); both TypeScript generating tests; web lint/typecheck,
81 unit files / 417 tests, build, and full Playwright E2E (62 passed).
The first merged run found the omitted inbox methods in the pinned
forwardable-set expectation; that expectation was corrected and the whole
root gate rerun. Product behavior and web sources did not change between
those two root runs. The 1,058-file source manifest stayed fixed throughout
the successful run. All recorded web-fleet processes and this track's fake
executables were confirmed stopped.

### What a screen calls

| Purpose | Typed client call | State carried |
| --- | --- | --- |
| Fold machines | `node_info`, `inbox(ListInbox { before })` | Node identity plus exact counts, bounded rows and next cursor; fold by identity, not a display name |
| Confirm one source | `inbox_entry(key)` | Current summary or absence, independently of a partial page |
| Draw a compact live form | `inbox_request(key)` | One exact permission/question, with explicit incomplete detail; never invent choices |
| Read/unread | `mark_inbox(key, read)` | Node-shared state; no answer, and agents are refused |
| Answer | Existing `send(session, SessionInput::AnswerPermission/AnswerQuestion)` | Operate scope and actor validation; hold submitting until exact source confirmation |
| Register Web Push | `push_key`, `subscribe_push`, `unsubscribe_push` | Only this paired Read device; public key and a 48-hour lease, no supplied device id |
| Presence | `report_push_presence` | Per connection: focused session/elsewhere/background, real input `Active`; focus-only `Unchanged` |

On the wire the empty push-key/unsubscribe requests are `null`. Every inbox
row identifies the session/source sequence, current session and project
labels, timestamp, shared read state and existing typed request/detail. The
machine comes from the authenticated client's node identity. Opening carries
the source sequence; when history is pruned, the exact live form is fetched
separately. Answers are never inferred from disappearing off a browsed page.

The next node acceptance work strengthens simultaneous-answer and CLI
question coverage; no native implementation is part of this merge.

### Node acceptance follow-up

- `inbox show ENTRY` explicitly recovers one pending live form through the
  existing `inbox.get` live-request mode. It leaves read state and pending
  counts unchanged. The CLI fixture prunes the source and ring, compares all
  32 fields and all 64 options in each field, then declines the request.
- The ordinary CLI question fixture rejects an invalid option without removing
  the request and verifies the exact accepted values echoed by the fake agent.
- Two concurrent answers have exactly one successful writer, one conflict and
  one resolved event. A tool update whose identifier and title no longer fit
  the snapshot budget invalidates its prior snapshot and compacts matching
  pending entries while retaining source, cursor and read state.
- These are node/CLI changes; no native model or page completion is claimed.

The focused acceptance checks passed: CLI inbox (3), CLI renderer (2),
node restart/push (1) and both TypeScript generators. The generated declarations
are byte-identical because this follow-up changes no wire shape. The restart
fixture reopens the same data directory, retains the VAPID identity and lease,
and proves that only fresh attention is admitted for encrypted HTTP delivery.
The whole root gate and fresh full web checks are the final acceptance gate.

The fresh acceptance web gate passed (81 files / 417 unit tests, all 62 E2E
cases). Root fmt and whole clippy passed; the full test stopped in the unchanged
fleet membership cancellation fixture (225 node unit passes / one failure).
The resume note records its duplicate-file-description probe and the separately
validated fleet prerequisite `ce228c10` on `9814cb0f`. That exact one-file fix is
now consumed here; the prior partial root run remains a failure. The new full
consuming-tree gate is required before reporting acceptance.

Build isolation follow-up: separate output directories are required even for
an isolated prerequisite checkout. Sharing one allowed a newer other-tree
fake to remain in the execution path despite the nested Cargo build. After
separating outputs and refreshing unchanged source mtimes, the canonical
large-form CLI test passed again. Final consuming evidence comes from the
subsequent whole gate and full E2E, with runtime fixture identity checked.

### Historical acceptance result before review correction

The corrected consuming tree passed root fmt, whole clippy (`-D warnings`)
and whole `cargo test --workspace`: **1,820/0/10**, with
`RUST_TEST_THREADS=3`, across 110 result sections. The default ignored cases
are three measurements, six LSP/toolchain prerequisites and one documented
proxy limit. The final full Playwright run passed **62/62**; web lint,
typecheck, build and 81 files / 417 unit tests passed, as did both TypeScript
generators. No test/deadline/exclusion was weakened.

The 1,636 source hashes stayed fixed through the successful run. Actual
original-tree compilation and the runtime fake's large-form scenario were
verified; the previous shared-output run is not used as final evidence.
The exact reservation correction from `ce228c10` is consumed, and its original
and new regression cases both passed. All recorded test/fleet processes were
confirmed gone. No native/phone/public-push-service/other-OS runtime claim is
made. V1 review of the complete inbox/push slice is pending.

Immutable acceptance implementation: `34986852e42a344bc10a949d6da0028ea6cbc861`.
The subsequent handoff note changes documentation only; compiled inputs remain
identical to the final successful consuming source manifest. Complete-slice
review is with V1 before node integration.

### Review correction checkpoint

The follow-up reuses recursive envelope version gating, stored marks at inbox
insertion, and exact attempted push-lease identity after HTTP. Focused rules
and real writer/HTTP barriers passed; whole fmt/clippy pass. The subsequent
whole test stopped at an unchanged CLI canary timeout (182 passed / one failed
across seven completed binaries), and fresh web E2E ended 60/2. The existing
model-call assertion rejects the production formatter's valid seconds unit;
the terminal-link case stopped before link handling at session creation. Their
owner decisions and full validation remain pending. The node-only resume note
records exact boundaries; no correction or UI approval is claimed.

### Common canary comparison

The owner Models correction and static canary diagnostics were consumed alone.
The requested isolated three-per-tree comparison completed: both the inbox tree
and frozen node-line HEAD pass two runs and hit the unchanged 60-second safety
net once. Median phase sums are 52.201 s and 54.145 s respectively. Both failed
runs exhaust the budget during initial setup/home/join, before shutdown. The
resume note contains every phase and before/after load records. No own-only
slow phase is established, and this does not waive the whole gate. Common-test
structure review belongs to the node integrator; the link comparison is next.

### SSD confirmation and remaining common-test evidence

Three own-tree canaries with private SSD runtime data produced phase sums
3.151 seconds (pass), 19.712 seconds (pass), and 59.955 seconds (60-second
timeout; final phase partial). Setup was 0.502/6.322/20.113 seconds and home
creation 0.151/3.742/21.146 seconds. The third execution followed a long FIFO
wait amid six real workspace builds. This is improved startup evidence, not
three passing runs or proof that storage alone explains every timeout.
Targets/logs remain on the data disk; runtime temporary data now uses SSD.
The corrected node's canonical SSD whole gate, six SSD Links comparisons and
full E2E remain pending. Shared-test timeouts/assertions/retry counts stay
unchanged. The resume document carries the exact comparison and limitations.

### Corrected node acceptance and wire handoff

The canonical root gate on `5342e173` passed fmt and full clippy, then
1,835 tests with zero failed and 10 existing ignored across 111 summaries
(`RUST_TEST_THREADS=3`). The seven changed crates actually recompiled.
All 62 full web E2E tests passed, including page 10, Models 3 and Links 1.
The six separate SSD Links comparisons passed: own `5342e173`, 3/3 in
19.6 seconds; baseline node head `9c7a45ca`, 3/3 in 18.6 seconds. Baseline
web sources and Cargo manifests are identical to the earlier canary baseline
`7f87d41c`; its node changes were built in a separate target.

The final source correction adds no wire fields, schema, scopes or dependencies.
These are the original seven new calls; their number is assigned at merge.

| Method | Request fields | Response |
|---|---|---|
| `inbox.list` | `before?: InboxCursor` | `InboxPage { counts, entries, next? }` |
| `inbox.get` | `key: InboxKey`, `fetch: Summary` (default) or `LiveRequest` | `InboxEntry` or absence |
| `inbox.mark` | `key: InboxKey`, `read: Read` or `Unread` | existing `Ack` |
| `push.key` | no fields | `PushKey { key }` |
| `push.subscribe` | `subscription: { endpoint, p256dh, auth }` | `PushLease { expires_at }` |
| `push.unsubscribe` | no fields | existing `Ack` |
| `push.presence` | `focus`, `activity` | existing `Ack` |

`InboxKey` has `session` and `seq`. Entries have `cursor`, `key`, `title`,
`project`, `project_name`, `at`, `read` and `item`; counts have `total`,
`waiting` and `unread`. Items reuse the existing permission request/tool or
question form, or an explicit `InSession` permission/question id; outcomes
are `Finished`, `Failed`, `Limit` or future `Unknown`. A permission's `detail`
is `Complete` or `InSession`. Push focus is `Session { session }`, `Elsewhere`
or `Background`, and activity is `Active` or `Unchanged`. Full typed DTOs
live in `vessel-proto::{inbox,push}`; capability values never belong in logs.

The client calls listed above remain the screen seam. Effective scope comes
from the merged node line's `device.self`, added by its own track; this slice
does not add that method. Unsupported or unavailable means unknown authority,
not a guessed grant. While authority is unknown, the app offers the existing
answer path and displays the node's refusal; known read-only access does not
get an answer handler. Answers still use the existing session Operate boundary.

The three source-review returns are closed: nested version admission,
stored-mark inheritance during delayed persistence, and exact attempted-lease
HTTP cleanup. The resume document records the immutable commits, all new
evidence, earlier failed runs and their limits. This node result does not
claim native adoption, OS delivery/sound, a real push service/phone, Windows
ACL runtime verification or combined-line integration approval.
