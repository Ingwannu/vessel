# A5: existing node inbox and push boundary

## Ownership and scope

The integrated node inbox and account-free Web Push are the existing N1 base.
The attention/notifications app package owns its new node gaps, including
recipient claims and terminal attention, as well as its app implementation.
This audit must not start a second implementation of those gaps. The app
owner controls the A5 tracker rows; this node audit does not mark them passed.

References: the attention sections of `parts/A-lifecycle.md`, notification
routing/presence/push/attention sections of `.area6-parts/B.md`, terminal
activity in `area-4-terminals-scripts-browser.md`, and their notification and
workspace connections. The detailed original source comparisons and deliberate
differences remain in ADRs 0040/0041 and `backend-m12-inbox.md`.

## Available integrated client contract

The integrated calls are protocol 22; consume the node line's assigned numbers,
not the original track's provisional number or its whole staged-diagnostics
branch. Existing `NodeClient` methods are:

- `inbox(ListInbox)` returns a bounded descending page and transactional
  whole-node counts, with an exclusive cursor for older pages.
- `mark_inbox(key, read)` changes shared read attention for one exact source;
  it is not an answer or a bulk consume of future entries.
- `inbox_entry(key)` cheaply confirms the exact current source after a lost
  reply or page movement; absence is answered/withdrawn, not duplicate success.
- `inbox_request(key)` recovers one actual live form independently of retained
  history. Over-limit forms stay explicit compact references; choices are not
  silently shortened.
- `push_key`, `subscribe_push`, `unsubscribe_push` manage the caller device's
  browser Web Push capability and lease, never another device supplied by id.
- `report_push_presence` reports a real activity/focus observation for this
  connection; the node stamps its time. It does not return an exclusive OS
  notification recipient claim.

Answers reuse the existing operate-scoped session permission/question calls.
Recheck current entry, connection and authority at submission; keep submitting
until the live projection confirms resolution. Two devices do not both gain
successful answer ownership. Read-only devices may read/mark shared attention
but cannot answer. Agent origins, including delegation, cannot mutate personal
inbox read state or register push capabilities.

## Semantics the app must preserve

Node-local cursors cannot be globally compared. Fold machines with their node
identity and entry key, and use authoritative counts rather than inferring
all waiting sessions from the first page. Stored archive/read marks are
inherited inside delayed append transactions; answered/withdrawn sources and
archived sessions do not recreate stale push attention.

Push eligibility follows target presence, not socket existence: recently
active visible target suppression, otherwise local delivery when a client is
present, and push only when none is present. Error outcomes do not push.
Registered endpoints and lease identity are rechecked at send admission;
late gone responses remove only the attempted lease. Scope loss/revocation,
read/answered state and renewed leases are not inferred from registration time.

## Deliberate differences and remaining package gaps

- Push is node-owned VAPID Web Push with encrypted generic payloads. There is
  no owner-provided Expo/FCM/APNs project, and no phone subscription UI or real
  push-service delivery proof is implied by the API.
- No title, body preview, session id, path, prompt or file content crosses OS
  or push notification text. Clients reconnect to obtain authorized details.
- The subscription lease is renewed by registration. Paseo heartbeat renewal
  differs; the app must not assume reporting presence alone renews this API.
- The existing session presence API is not a terminal attention API. The app
  package owns that extension and the single-recipient claim. New methods or
  request variants use the canonical metadata/version contract after its head
  is announced; no second policy/transport or dummy version is introduced.
- Desktop routing, sound, settings, badges, toasts, navigation, deferred read
  clearing and platform capability states remain the app package's acceptance
  work. Earlier node evidence is not a pass for those surfaces.

## Evidence and workflow

The original inbox/push node checkpoint `dc9c6e8d` received final node review
and was integrated with final protocol/schema assignments. Its evidence and
known limits are preserved in the inbox plans. Later integrated node gates
cover the same base; this audit changes no source and needs no unchanged
whole-gate rerun.

Use one package gate/review when the actual package is complete, with security
changes retaining their per-commit review. Never prompt a real agent, use the
preview node or claim live OS/phone delivery from fake-node tests. The current
node gaps must be coordinated with the app package's owner, not duplicated.

## A5 worker continuation (separate runtime; not base evidence)

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


## Native original-recipient routing continuation

[Decision Log]
- Purpose: GPUI has one process notification callback; registering once per
  Shell overwrites an earlier window's recipient and fails after it closes.
- Existing assets: the existing opaque DeliveryTag, canonical ConnectionContext,
  GPUI AnyWindowHandle and WeakEntity, and the kit root ownership boundary.
- Alternatives: one independent window list, embedding source identity in the
  OS payload, or alias lookup after activation. Each loses privacy or authority.
- Choice: one App Global installs the callback once and stores at most the
  existing MAX_PENDING_NOTICES recipient records for the whole process. Every
  record retains original window, weak Shell, exact Source and destination.
- Why: entering that handle and checking current root content prevents an old
  retained Shell, closed-window successor or newer same-alias connection from
  receiving a source action. A retired context opens only its original Inbox.
- Impact: generic activation after unknown/consumed/evicted/dead recipients is
  explicitly GenericUnavailable until W19's reviewed sole host registry API is
  adopted. No second window manager or full open-route success is claimed.

Four GPUI consumer regressions exercise the actual registered platform callback
with kit-wrapped real windows, second-window registration, once-only consumption,
close/reopen, retained old Shell with replaced root, exact context replacement,
process-wide bounds and explicit unavailable outcomes. Execution remains owed.
The helper is a separate bounded commit for W19; it alone grants no source or
host-selection authority. Actual D-Bus delivery/silent-hint evidence remains owed.

Before this continuation, exact6cb55fcb passed four-crate all-target native
Clippy (run wp-inbox-adapter-20261115T161638Z-6cb55fcb, rc0), the prior Inbox
App test (one case, not the new routing tests), and actual root vesseld build
plus260 node library tests (wp-inbox-app-root-20261115T161638Z-6cb55fcb).
Those checks do not verify the new callback source or a whole A5 gate.

The old unused view NoticeRoutes container and its implementation-shaped test
are removed after moving the actual recipient map into the App Global. Opaque
tag and destination value types remain in view. There is one active route map;
its replacement consumer tests exercise actual platform callback ownership.

Exact1432bb58 native routing checks completed: four native crates all-target
strict rc0 (wp-inbox-app-root-20261115T164920Z-1432bb58), plus Inbox40/0/0
(wp-inbox-routing-20261115T164918Z-1432bb58): App5 including four real registered
platform callback/window cases and View35. These exercise original wrapped
recipient, later window, consumed tag, closed/reopened handle, retained old
Shell with root replacement, exact connection retirement and process cap.
They do not prove W19 host fallback, real OS delivery, audible bell or full A5.
The standalone portable helper993 Kit all-target strict also passes; its
sequential GPUI test and V1 source review remain pending.

## Actual Linux silent visual-notification probe

The VM exported app build wp-inbox-20261115T165338Z-1432bb58 completed the
fonts/build steps (build217s), source1432bb5854c61b110e2cdcb5a664e92905c3d188,
app SHA1d2692671f91185af453a65df9dcb8ac8374e7dd926e6f339eb8f164920845ea.
A fresh empty config/home, authenticated private X11 display and private D-Bus
notification service exercised the real Settings test button. With sound Off,
preferences saved alerts=system; with sound On, system_and_sound. Both actual
Notify requests contained suppress-sound=true. Captured screens were opened:
permission is OS-managed/uncheckable, and the test stays Requested/check that
it appeared. No fake permission or delivery success was installed.

The probe ran as UID/GID1000; its preferences were0600 in private0700 folders.
The test's own app, notification service, runner and X server were verified by
PID/command before shutdown; its display socket/lock and authority were removed.
No real node, provider, browser, user config or production permissions changed.
This is adapter-wire/settings evidence only, not a real desktop notification,
audibility, recipient claim, paired-device flow or final capture/whole gate.
The one-line local GPUI patch remains c7d29c2c; R must rebuild it for releases.

## Package base 9b merge

Canonical node9c and package a7aab9d6 contracts, registry/detach writer paths,
tests and version assignments are preserved. The worker notification-claims
migration is appended after registry, with the existing rollback helpers
removing it first. Five A5 pending rows remain unassigned. Current merged
root/native narrow checks are owed; earlier1432 evidence remains prior-source.

## Shared host fallback receiving union

[Decision Log]
- Purpose: notification activation must remain useful after its original window closes.
- Existing assets: V1 row273 bounded shared host slice d8b73397/docs89e2 includes the sole weak window registry and root helper993; it is provisionally adopted for this receiving union, not a full E3 branch.
- Alternatives: retaining old Shells, an Inbox-owned window list, or forwarding the old destination to a successor would break lifetime or source authority.
- Choice: keep exact delivery in the original handle/root/weak/context checks; otherwise ask the shared host owner to show generic Inbox. If no host exists, call its optional desktop factory, then resolve a current host again.
- Why: fallback receives no Source, key, session or terminal destination. A replacement never gains the old recipient's action even if its connection happens to match.
- Impact: dispatcher/reexport/factory field are promoted with this real caller; set_factory remains individually cfg(test) until E3 startup. A missing factory remains typed unavailable. There is no second window manager.

The original callback/window tests now include actual sole-registry registration,
closed-window generic fallback and consumed/evicted tags. Additional GPUI cases
cover a registered replacement root with a matching source and all windows
closed followed by an actual test factory. Own union strict/tests remain owed;
V1 host/helper verdicts do not grant notification authority or OS delivery proof.
