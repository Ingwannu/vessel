# Milestone 12: inbox and push

Track N1. ADRs 0040 and 0041. Scope: node, wire, store, CLI, client,
native link/view/inbox. The board waiting lane and sidebar count use this
view model but are the shell track's; the session's permission and question
controls are the conversation track's.

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
   key/subscription without replay; 404/410 prunes only the matching subscription.
4. Private, loopback, mixed-DNS and redirected endpoints are refused; logs
   expose no endpoint token, key or content. File mode/ownership is checked.
5. A fake push receiver proves encrypted wire and decrypts in the test;
   no real push account, preview node or paid agent is used.

## Contract

Protocol version 12 (minimum unchanged), schema's next appended migration.
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

The following native contract is reserved for the next slice; it is not
installed or gated in this node checkpoint.

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

## Progress and evidence

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
