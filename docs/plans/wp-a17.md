# A5: durable Inbox, attention and desktop delivery

Status: implemented/staged source, with changed-source checks in progress.
No final whole gate, platform capture, assigned-wire integration or final V4
approval is claimed. Remaining owner integrations are listed explicitly below.
Historical commands/results and continuation details are retained in the private
A5 evidence packet.

## Reference behavior

Paths are relative to the Paseo 0.15.4 reference checkout.

- `packages/app/src/contexts/session-context.tsx:260-310`: skip error alerts,
  suppress the actively viewed source and deduplicate attention transitions.
- `packages/app/src/utils/agent-attention.ts:18-90`: permission attention survives
  reading/focus; reading is not answering.
- `packages/app/src/hooks/use-agent-attention-clear.ts:112-150`: focus entry clears
  existing attention; attention arriving in place waits for input/send/blur.
- `packages/app/src/utils/os-notifications.ts:144-181` and
  `packages/desktop/src/features/notifications.ts:80-130`: desktop display,
  independent sound preference, restoration/open routing.
- `packages/app/src/desktop/settings/desktop-settings.ts:33-35,87-107`: OS alerts
  and sound default on; failed persistence restores the preceding preference.
- `packages/app/src/components/toast-host.tsx:66,80-118,127`: one replacement toast,
  default 2200 ms, error helper 3200 ms, hover pause, explicit sticky, blank ignored.
- `packages/server/src/server/agent-attention-policy.ts:3-79`,
  `server/websocket-server.ts:2838-2888`, and
  `terminal/activity/terminal-activity-tracker.ts:12-80` (the latter two under
  `packages/server/src/`): 180 s activity, focused-target suppression, most-recent
  recipient, explicit terminal working/idle/needs-input/clear/interrupt.
- `packages/app/src/subagents/workspace-root-policy.ts:6-16`: no current parent OR
  known different workspace qualifies a workspace-relative root. Unknown parent
  ownership is not a root.
- `packages/app/src/utils/workspace-agent-activity.ts:22-48`: use the latest
  eligible non-archived root by attentionTimestamp/updatedAt, not every historical
  root. The workspace owner combines this one root with terminal activity.
- `packages/server/src/server/session.ts:7655-7810`: canonical workspace ownership
  for attention clearing; unread selects a newest finished/read eligible root.
  `packages/app/src/workspace/mark-unread.ts:11-25` exits pane focus first and
  restores it after failure.

## Decision Log: reuse and ownership

- Purpose: durable cross-machine attention, exact request answers, shared reads
  surviving reload, and local delivery from the same authoritative facts.
- Existing assets: node Inbox list/get/mark, push/presence and shared marks;
  conversation request cards/validation through Place/Seat; persisted preferences;
  G4 connection admission; GPUI notification/bell APIs; shared kit controls.
- Alternatives: replay session logs to reconstruct an Inbox, persist a second
  read engine, allocate feature-private connection epochs, add a notification
  library or duplicate terminal/parser/pane ownership.
- Choice: modify/reuse those assets. Keep view policy pure, use typed SDK requests
  through G4 SelectedConnection, retain only node snapshots and bounded local
  delivery opportunities, and embed existing conversation cards.
- Why: node transactions remain lifecycle/read authority across devices. One
  shared admission fences effects; a separate renderer/read engine would diverge.
- Cost: native host APIs and other owners' unready capabilities remain explicit;
  a source-only or narrow result does not establish runtime/platform parity.

## Modules and data flow

| Layer | Responsibility |
| --- | --- |
| `view/src/inbox/` | Source/Key/tickets, snapshots/detail, cursor windows, notices, presence/read policies, status buckets, terminal metadata and pure badge projection. No I/O or GPUI. |
| `link/src/inbox.rs` and children | Exhaustive Ask/Answer, bounded typed requests, safe failures, exact current form validation and committed-event confirmation. |
| `app/src/areas/inbox/` | One window store, existing permission/question cards, actions, settings, presence/idle hooks, local notification boundary. |
| `view/locales/{en,ko}/inbox.ftl` | Whole translated labels and honest unsupported/unknown/error states. |
| node/store/proto | ADR 0040/0041/0089 persistence, routing, claims, revisions, narrow passive read and rollups; see `wp-a5-node.md`. |

Node snapshots -> exact Source/ticket acceptance -> Inbox/terminal rows and
status -> bounded claim opportunities -> node election/current-source recheck ->
opaque local routing + generic OS display and selected system bell. Request
answers go through the existing input protocol once, then authoritative durable
withdrawal/confirmation; acknowledgments alone are not resolution.

## Connection and request safety

The exact G4 producer `074ade16..95c256bce`, with result documentation `72f081c35`,
was adopted, then the approved app line through package base 6 `18eedad4`.
Source retains canonical ConnectionContext; no Inbox UUID allocator remains.
The private transport facade exposes only original node/protocol metadata and
SelectedConnection::call, never a raw client. Presence, get/list/mark, answer,
long-poll confirmation, claims and terminal actions all use that fence.

Every result retains its original admission and a non-repeating ticket where an
operation awaits settlement. Retirement rejects queued callbacks/grants before a
replacement fleet read. Same-key drafts survive reconnect, but controls remain
unconfirmed until a fresh full get. Passive session reads pin the explicit open's
full context; a departing deferred read is settled before replacing the binding.
Retirement removes the binding rather than replaying through a same-node reconnect.

Several aliases may authenticate as one node. New reads retain one exact current
admission while available, avoiding duplicate counts/hydration resets. This read
selection never retargets an already-issued write or claim. A replaced admission
invalidates its pending ticket.

## Inbox and read semantics

- Persistence, current request identity and counts come from existing node
  `inbox.list/get/mark`. Sixteen loaded entries never stand in for whole-node
  counts or waiting keys.
- Cursor windows fetch newest plus explicitly selected older window: at most
  two pages/32 rows and two list calls, independent of history depth. Newest
  reset retains notification watermarks. Cursor/order/duplicate/foreign keys
  are validated.
- Full live-request recovery precedes answer controls. No shortened choices or
  manufactured permission/question ids. Compact oversized requests keep an
  explicit source-session reference.
- One send, no offline replay. Changed/current request and connection are
  checked again. Committed session events confirm withdrawal; lost/uncertain
  replies retain typed failure rather than assuming success.
- Whole-projection session rollups are separate from log marks; unsupported
  peers expose a labelled legacy fallback. Failed count is independent of Seen.
- Focus entry/input/send/blur policy retains exact observed exclusive watermark.
  Pending requests veto passive clears. Older Operate peers reuse existing
  conversation pending check + Mark::Read; old read-only peers remain unsupported.
- Node-staged `explicit_unread` separately projects committed marked_unread,
  including empty/pruned Inbox. Exact-entry Seen/answer/removal do not clear it;
  whole-session Read does. Native source consumes the reviewed c2dcebea values;
  assigned runtime/consumer checks remain owed. Counts are never OR-ed with
  historical attention_seq/read_seq.

## Presence and delivery

Real key/pointer/scroll input drives activity, throttled at 5 s; heartbeat is
15 s. Focus-only/timer reports do not refresh activity. Node timestamps and the
180 s policy remain authoritative. Presence workers coalesce real input and are
bounded by transport lifetime and the common request deadline.

X11 idle uses existing x11rb ScreenSaver, one process-wide worker, one queued
sample and 500 ms consumer deadline. Timed-out workers are not auto-replaced.
Samples keep aging through queues/heartbeats. Unsupported backends return unknown.
Optional idle age/Terminal focus are omitted unless the original peer supports
the assigned contract, preserving ordinary legacy presence.

First hydration/reconnect stays silent. Accepted new entries enqueue at most
128 opportunities. Only a node-admitted exact source can display. Lost grants
are spent, never retried. Overflow leaves a translated in-app coalescing notice;
there is no unclaimed summary OS notification.

OS payloads contain generic app text and an unrelated random tag; source ids,
paths, titles and request text stay in the bounded process-local route map.
Stale/evicted/restarted tags fall back to Inbox. Final UI checks reject obsolete,
read, withdrawn or newly visible sources before display.

## Terminal attention

Existing terminal.list metadata is projected into the same Inbox snapshot.
Plain uninstrumented shells have no indicator; missing/future evidence remains
Unknown. Neither infers a job from PTY bytes. Explicit working, idle,
finished and needs-input states retain exact non-reused node revisions. Claims
recheck current state after admission; explicit row clear carries the observed
revision. Old/unassigned peers retain unsupported terminal attention.

No parser, pane manager or fake session id is introduced. C2 reports no checked
TerminalId resident/focus producer yet. Therefore no focused-terminal capability
is advertised: passive terminal clearing/direct pane reveal remain unwired.
Notification activation can select the exact Inbox terminal row meanwhile.

## Settings, sounds, toasts and badges

- Missing/new alert preference defaults to SystemAndSound, per approved A8 ruling.
  Stored explicit off/system/sound modes remain unchanged; an Off regression is
  retained. Display and sound switches update one existing persisted preference.
- GPUI returns no permission/delivery result. Approved first-version copy says
  OS-managed/cannot check and requested/check appeared. A test never claims
  delivered success.
- Approved local Linux GPUI patch adds only SuppressSound(true) to the builder.
  Sound requests use Window::play_system_bell, subject to OS settings. Releases
  must rebuild the patched GPUI dependency. Actual private D-Bus hint proof is
  still owed; source presence alone is not delivery evidence.
- Shared toasts replace the previous toast, use 2200 ms default/3200 ms error
  helper, pause every variant on hover, ignore whitespace, and require explicit
  sticky. Stale-id and hover-race tests were retained/extended. **All areas'
  toast timing changes; the lead must receive this note at package handoff.**
- Aggregate urgency is needs-input > failed > working > review > done.
  Sidebar display/shortcut groups put review before working, following the
  separate reference order. Terminal untracked/unknown rows retain no dot.
  Pure workspace projection takes all canonical agent contributions plus terminal facts; unknown
  ownership/attention never becomes zero/Done. W11 owns canonical selection.
- Dock/taskbar badge is an approved native difference: GPUI has no API. W19b owns
  the later small host seam. Pure typed workspace count/global icon projection
  is retained. Native favicon is absent as in Paseo native; current title count
  remains available through the existing preference.

## Remaining integrations, not completion claims

1. V3 bounded correction and actual changed-source evidence, then integrator
   protocol/schema assignment and B0 app-base wiring. Five canonical pending
   methods remain unassigned; no provisional wire number is used.
2. W11 registry + W2 workspace-relative roles for atomic workspace read/unread
   and canonical workspace status/time sorting. No cwd/Creator/client-loop
   substitute. Pending/archived/internal/unknown visibility must remain exact.
3. C2 TerminalId resident/focus + W12 single pane registry for passive clears
   and real terminal reveal. Layout selection alone is not resident focus.
4. W4 reviewed shared response registry/durable receipt seam; isolated Inbox
   Place override stays until that handoff. W21 checked full address/follow
   opener is not yet a portable reviewed producer; no full stream safety claim.
5. Whole root/native gates, assigned-wire tests, private fleet captures,
   actual D-Bus SuppressSound proof, final V4 and lead handoff.

The coordinator assigned adjacent tracker618 to orchestration: W18 owns finish
subscriptions and restart recovery through existing ADR0033 automation::notices
and the notify watcher; W10 owns timeline notification rendering. A5 adds no
second watcher or wire. The old restart assumption needs an ADR0033 amendment
in that owner package; it is not an OS notification implementation here.

## Stated native differences

Durable Inbox follows Vessel ADR 0040; it is not a second copy of Paseo flags.
Generic OS text/local opaque routing follows approved privacy policy. Web Push is
ADR 0041 account-free node VAPID/service-worker delivery, not Expo/APNs/FCM/mobile
UI parity. Native desktop does not register mobile tokens or claim F-Droid proof.
OS idle is unknown on Wayland/Xwayland, macOS, Windows and web. GPUI permission,
delivery status and dock badge limitations have explicit coordinator rulings.
Unready owner integrations above remain outstanding work, not approved parity.

## Verification and operations

All new Cargo/rustc/build/test/capture builds run on the build VM through
`vessel-remote-gate`; no new local Cargo. Static size/token
checks read source only and have passed on current app source. Exact commands,
SHAs and raw failed/queued results are retained in the private A5 evidence packet.
VM-restart rc143 logs remain infrastructure interruption, not compiler PASS.

Latest node generation evidence: run wp-inbox-ts-20261115T134300Z-4d7ac8d6
passed three proto tests and the web declaration test. Its additional primitive
alias test failed only on the coordinator-classified base LimitPercent defect,
fixed centrally in the pending gate49 batch. This run counts with that named
defect; no head expectation is edited or unchanged TS run repeated. The exact
generated explicit_unread/Untracked declarations are retained on the node tree.
This does not establish current native/whole/runtime or assigned-wire behavior.

Final gate commands use --only native / the prescribed root gate once source is
complete, then freeze through captures/review. Export the app via --export-app
with approved shared-root binaries, or full export for newly assigned methods.
Capture 1280x900, 390x844 and relevant 800x600 states, dark/light and EN/KO; open
every PNG. Use only fake agents, fresh private homes, authenticated private Xvfb
and a private D-Bus daemon. Never live preview ports or a pairing link in UI.

Execution account verified ubuntu UID/GID 1000. New proof roots/cookies use
0700/0600; shared root binaries are read-only consumers. No production ownership,
permissions, service configuration or infrastructure changed. Stop only verified
owned PIDs after matching their command line and private data path.

Background read admission rotates source order independently of presentation.
With the link's bounded concurrency, a fixed first batch could otherwise starve
later machines on every refresh. The pure regression proves every candidate
reaches the head even if only the first admission succeeds; no effect is replayed
or retargeted and the visible machine order stays unchanged.

Notification activation explicitly redraws generic Inbox fallback even when its
opaque route was consumed/evicted and the window was already active. OS/window
activation alone is not a state-change notification. Exact source lookup and
opaque routing remain unchanged; platform capture verification is still owed.

Shared mark controls and the action entry point now require current successful
node-read readiness as well as a confirmed exact detail. Read-only people keep
mark access, while retained failed/offline snapshots cannot launch it. Answers
recheck the existing may_answer policy at the action boundary too; node scope
and original G4 admission remain final authorities. A regression retains the
stale entry while verifying read-only/failed/recovered/disconnected transitions.

## Explicit untracked terminal projection

The row retains whether canonical TerminalKind is a known Shell. Its own
indicator remains None for explicit Untracked or Unknown. Workspace TerminalState separates
Untracked (only that known plain-shell case), Known(Bucket), and Unknown
(missing/unavailable/managed-or-future kind/future reason). Untracked is neutral
only in aggregation; it never means a terminal completed or may be archived.
Known root Working/NeedsInput survives an Untracked shell; Done+Unknown stays
Unknown. Confirmed Absent root + only Untracked uses the source's explicit empty
workspace Done rule, not an inferred terminal lifecycle: Paseo server
session.ts:5714,5811 seeds Done; workspace-directory.ts:424-429 skips null/no-bucket
terminal activity and :478-493 defines empty Done; its test :525-530 explicitly
covers no agent plus an uninstrumented terminal with null activity => Done. Unknown membership/root remains
Unknown. This policy is agreed with W11; checked/reviewed handoff is still owed.

The portable policy producer is ed95028410d28dfb31a8001a9dfd4cf724ffcc3e,
based exactly on approved line dd13ce378. It adds only view::attention and its
plan/module registration, with no root/Inbox/proto/registry dependency. This
branch merges that exact producer and reexports its Bucket/workspace/badge
policy; there is one implementation. Inbox-specific Card classification and
terminal metadata projection remain A5 adapters. Portable successor911438d2
changes only comments/plan to distinguish aggregate urgency from sidebar order.
Its exact VM view all-target Clippy passed (run
wp-inbox-projection-check-20261115T135741Z-911438d2, rc0/62s). Executable
predecessor ed950 passed three focused tests (run
wp-inbox-projection-20261115T131303Z-ed950284, 3/0/0, rc0/151s).
Bounded source review returned two findings; 911 is held and is not adoptable.
Successor246f1ba28 separates latest-root DISPLAY from the complete aggregate of
all workspace-relative roots and working same-workspace descendants. Independent
typed icon activity/attention replaces the lossy Bucket input. Missing timestamps
block display/order facts, not an aggregate whose coverage is otherwise complete.
Exact246 portable tests passed3/0/0 (67s) and View all-target Clippy passed
with -D warnings (50s). V4 row234 passed bounded checked source adoption;
both source findings are closed. W11 received this exact three-file producer.
The lead merged its identical source as76674f1b; the line gate is still pending.
Consumer/whole/host evidence remains owed. W11 owns role/membership coverage;
this is neither full A5 nor a registry admission/runtime approval.

The pre-adoption app DTO conflated explicit unknown and serde(other), so the Row
adapter stayed conservative until c2dcebea's V3/integrator handoff below. Current
source uses canonical explicit Untracked, while lost/missing/future Unknown
remains non-neutral. The standalone projection API accepts only proven facts.

Exact native strict run wp-inbox-adapter-20261115T124704Z-24178c54 reached the
view build script and failed on an own duplicate inbox-combined locale key.
Both EN/KO now retain only the later in-app coalescing message. A static key
check found unique identical key sets in those two files; this does not replace
the Rust Fluent parser/build check. The failed immutable evidence is retained.

Sidebar group display order is distinct from aggregate urgency. Actual Paseo
app utils/sidebar-agent-state.ts:36-42 puts review before running, consumed by
hooks/sidebar-status-view-model.ts:43-50; aggregate priority still ranks running
before review (protocol agent-state-bucket.ts:13-19). Inbox's groups now use
explicit GROUP_ORDER input/failed/review/working/done, and drawing plus numbered
shortcuts share it. The own added complete-group assertion names that explicit
reference order. This changes no shared-head assertion or badge aggregation.

OS idle probes cache evidence independently of heartbeat scheduling. The earlier
probe path sent every five seconds and reset the regular fifteen-second timer;
it now updates only the cached sample. Ordinary heartbeat/input/focus remains
the sole report scheduler, and the original sample continues aging. A clock-
injected regression samples twice before the due time, checks no early report,
and then checks one unchanged-activity report exactly at the original deadline.

## Reviewed values adoption

V3 row162 supplemental SOURCE PASS accepted canonical c2dcebea; the node
integrator landed it as a5311bc2 with its next combined gate still owed. Under
the explicit values-only exception, the app applies that one commit's diff as
127689a9d, preserving its app base rather than merging node-head ancestry.
Both DTO module blobs match the reviewed source exactly. Add/add conflicts in
notifications.rs/inbox_attention.rs use those canonical blobs; proto/lib.rs
retains the existing notifications_tests module and one declaration of each
public module. No node runtime, new row/number or assignment is imported.

Native adapters now consume explicit Untracked only for known Shell kind; real
future wire states still decode Unknown and cannot become neutral. Both retain
no terminal dot. Session read/group badges use explicit_unread OR current unseen
Inbox count, preserving pending/failure/work priority. A regression covers an
empty Inbox deliberately marked unread, then work, pending request and actual
clear. This is source wiring; changed-source native checks and actual assigned
node/runtime behavior remain owed. The node tree owns runtime integration;
this app-only values adoption is not a whole-root gate claim.

## Safe manual-open boundary

W21 confirmed that legacy Open::of(Card), even after a UI G4 context check,
does not retain the original admission through queued attach/follow after a
rebind. Inbox removes that legacy route rather than inventing another resolver
or stream owner. The Open control is explicitly unavailable with Sessions
fallback copy until W21 supplies a reviewed bound opener. OS activation already
keeps exact Inbox selection. This is an outstanding integration, not navigation
parity or an approved permanent difference; request answers/shared marks remain
available under their existing selected-call fence.

Native strict wp-inbox-adapter-20261115T132516Z-5171bbca returned three own View
lints: doc path backticks, elidable groups lifetime and indexed route replacement.
They are corrected; route selection uses iter_mut/find with unchanged same-node
admission semantics. The immutable failure remains retained. No current native
PASS is inferred from source corrections.

Approved app package base8 91407173 is merged before the next remote check,
per the current worker brief. It carries the shared FieldId/menu/Delivery seams
and B0 node82ae7c91; this is an approved app-line merge, not a node-head import.
The automatic merge had no conflicts. Existing shared behavior and Inbox's
registrations/pending rows remain present; size/token ratchets are not raised.
No final whole gate was running, so no frozen source was changed. Union checks
remain owed; previous narrow results retain their original source scope.

Exact native unit run wp-inbox-tests-20261115T133806Z-f21e7293 compiled View then
returned three Link type errors: existing GetConversation/MarkSession live in
proto::conversation (not root exports), and selected-call presence returns Ack
rather than unit. The canonical method table confirms the unchanged wire calls;
paths and the Ack-to-unit projection are fixed. No new call or scope is invented.
The current app source includes approved base8; a new union check is required.

Run wp-inbox-tests-20261115T142343Z-4ed6d447 executed View/Link libraries:
1089 passed, one failed (About dependency inventory). The A5 X11 idle probe's
existing x11rb runtime dependency was missing from BUNDLED. Its entry is added
in manifest order using the local crate manifest's MIT OR Apache-2.0 licence;
the assertion stays unchanged. This is an own omission, not a base defect.
The newer aggregate/icon correction was absent from that immutable source;
changed-source union checks remain required.

The same4ed strict run reached Link and returned two equivalent boolean-gate
lints and the dispatch function's 101/100-line budget. The pending/older-peer
tests use is_none_or with the same refusal semantics. SavePrefs persistence is
extracted unchanged into a private dispatch helper; no raised size record,
call list, retry, error behavior or scope change. A new union narrow is owed.

The app tree still carries earlier A5 root runtime source from before the
separate node-tree rule. Values adoption therefore also requires a root union
check before any line handoff: native compilation does not compile vessel-node.
The existing terminal reducer explicitly accepts canonical Untracked in its
non-attention arms, and the existing store rollup projects marked_unread from
the same read transaction as the counts. These small carrier adaptations match
the node tree; no whole node branch/runtime merge, wire registration, protocol
number, recipient capability or new authority is introduced. Root build/tests
on this app tree remain owed and cannot be inferred from native checks.

Passive read observation also preserves explicit_unread independently of Inbox
counts, matching the sidebar. An empty/pruned Inbox with a deliberate unread
mark must still enter the existing deferred read policy and whole-session Read
transaction; exact-entry Seen never clears that flag. This fixes the remaining
app observation that checked only the count. Native union checks remain owed.

Root union preflight found the retained activity capability fixture still had
the already-corrected node-tree length/default lints. Only the test section is
aligned after checking the entire product prefix is byte-identical; the narrow
read method's existing doc reference is backticked. This imports no node runtime
behavior or branch ancestry. The app root strict predecessor is obsolete while
queued; its record must be kept if superseded under the standing rule.
