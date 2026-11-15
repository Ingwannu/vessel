# Attention inbox and desktop delivery

Status: existing inbox calls and native state/drawing are implemented; narrow
checks and the additional node contracts are in progress. No whole gate, visual
proof or final package approval yet.

## Reference and reuse

Paseo reference paths below are relative to its repository, version 0.15.4.

- `packages/app/src/contexts/session-context.tsx:260-310`: attention delivery
  skips errors, suppresses the actively visible source, and deduplicates by
  attention timestamp. Notification routing carries the server and agent.
- `packages/app/src/utils/agent-attention.ts:18-66`: pending permissions precede
  errors and finished turns, oldest first within a reason. Lines 68-90 keep
  permission attention when a session is focused: reading is not answering.
- `packages/app/src/hooks/use-agent-attention-clear.ts:112-150`: visibility and
  focus changes clear outcome attention, with deferred clearing when new
  attention arrives while the source is already viewed.
- `packages/app/src/utils/os-notifications.ts:144-181`: desktop sender first,
  web permission fallback; native mobile delivery is remote push only.
- `packages/app/src/desktop/settings/desktop-settings.ts:31-40,87-107`:
  sound defaults on and failed writes restore the preceding settings.
- `packages/desktop/src/features/notifications.ts:80-130`: OS delivery uses the
  sound preference and clicking restores the window and opens the source.

Existing assets: `view::notices` defines transition notices; `view::prefs`
retains alert preferences; `link::Prefs` already has a private atomic writer.
`areas::conversation::{ConversationArea,Place,Seat}` and `dock::requests`
explicitly support inbox embedding. Reuse their permission and question cards,
including their existing question checker; never synthesize timeline records.
`link::devices` provides the architectural pattern for bounded request tasks,
request identity and safe error projection. New state belongs in `view::inbox`,
RPC plumbing in `link::inbox`, and drawing in `app::areas::inbox`.

## Decision Log

- Purpose and intent: one durable cross-machine attention list, direct answers,
  shared read state, and desktop delivery from the same authoritative source.
- Existing implementation and constraints: the app has fleet transition policy
  but no caller or durable inbox. Its conversation already supplies reusable
  request cards. The starting app's protocol predates inbox; integration is
  centralized before transport wiring.
- Alternatives considered: infer durable items from session cards; maintain a
  second local persisted inbox; use a new notification dependency.
- Choice: modify existing conversation and preference infrastructure; add a
  narrow inbox state machine and typed transport boundary over existing calls.
  Use GPUI's existing platform notification API when available, without another
  dependency or subprocess. Tests use an in-memory fake at the boundary.
- Why: the node owns request lifecycle and read state across reloads/devices;
  reconstructing either locally would duplicate validation and lose history.
- Benefits, costs and effects: a reconnect hydrates from the node without
  replaying old notifications. Offline and unsupported machines stay explicit.
  Exact request identity includes the reporting node and source event, so a
  late result cannot answer or mark a newer request with the same session.

## Stated differences

- The durable list is Vessel's node inbox contract (ADR 0040), rather than a
  second copy of Paseo's session attention flags. Reading never resolves a live
  permission/question; only the actor's existing input protocol can do that.
- The existing four-mode alert preference remains compatible with stored app
  preferences. Sound-only is retained; OS sound capability must be reported
  honestly where the current platform adapter cannot express that mode.
- First hydration/reconnect is silent, following the existing native notice
  policy; only newly observed attention on an uninterrupted connection delivers.
- Unknown future item kinds remain readable and openable, never actionable.
- The exact UI and platform limitations are recorded with the package gate.

## Verification contract

Pure tests cover node identity separation, stale responses, unread counts from
whole-node counts rather than a page, first hydration/reconnect suppression,
selected-source suppression, pagination deduplication, and a mark or answer
failure retaining its request. Link tests cover deadlines, queue rejection,
unsupported nodes and safe error projection. The real request cards are reused.
Capture the actual fake fleet at desktop and phone sizes in both themes and
languages. Inspect captures. One whole native gate and one review at package
end; narrow changed-crate checks during implementation.

## Resume point

Research read; worktree isolated; no wire invented. Next: pure model and fake
boundary tests, area skeleton; then merge the centrally supplied baseline and
wire inbox.list/get/mark plus existing session input.


## Platform patch verification

The local GPUI Linux notification builder adds `SuppressSound(true)`;
releases rebuild this local platform patch. Sound ON uses the existing
`Window::play_system_bell()`. Windows already requests silent visual delivery;
macOS leaves the visual notification's sound unset. Verification planned:
private D-Bus capture of the real app's `Notify` message, asserting the hint
is true. Not yet run. OS delivery/actual audibility on the owner's desktop
remain separate from this transport-level proof.

## Expanded attention package

The full feature survey expands this package beyond the inbox: agent attention
and unread lifecycle, client presence, notification recipient routing, OS
settings and opening, push adapters, terminal attention, badges and toast
parity. Specification: lifecycle attention sections, notifications half of the
notifications/usage survey, and the terminal attention survey. Existing toast,
sidebar, read-mark and platform facilities are extended rather than duplicated.

Pending node contracts (checked against the reviewed node line):

- In-app recipient nomination: the node's push policy selects a local recipient
  but the inbox contract does not tell a desktop which connection won. Local
  duplicate suppression cannot prove exactly-one-client delivery. Node-owned
  routing must arrive before claiming that part.
- Terminal attention and focus: the current wire has no terminal activity or
  attention state, and push focus names only sessions, elsewhere or background.
  Plain terminal output is not proof that a terminal requires human input.
- Native mobile registration: the accepted node contract is device-scoped Web
  Push, not an Expo token service. A mobile push provider/registration adapter
  is a platform dependency, not a reason to fabricate an RPC method.

The existing presence RPC can be wired after the central baseline. Its activity
must come from real keyboard/pointer/touch/focus events, never periodic polling
that falsely reports the person active. Heartbeats and activity coalescing use
Paseo's named 15-second and five-second intervals. Desktop OS idle polling needs
an actual platform API; no presence is inferred from an open socket.

The default alert preference currently remains off, preserving this app's
existing explicit opt-in behavior. Paseo's desktop sound defaults on; this is
a stated compatibility difference until the common preference owner migrates
that default. A requested test notification is independent of visual opt-in.

Resume: the skeleton is drafted; model tests cover the first eleven cases.
Expanded page, authority and queue tests and narrow Clippy remain pending.
Actual inbox calls await the centrally announced baseline. No whole gate,
visual capture or platform delivery evidence has been claimed.

### Opaque route dependency

The app and link directly reuse the already resolved `uuid` crate (Apache-2.0
OR MIT; unpacked source about 456 KiB in the checked toolchain cache). The
normal dependency is already linked through vessel-proto; no additional
provider SDK, download or OS helper is introduced. Rust std has no random UUID
facility. The app uses random tags unrelated to source ids for the bounded
local OS-open map; link uses them for transport incarnations so a reconnect to
the same node invalidates stale reads. These generated bytes enter pure view
state only as typed opaque identities. The OS sees no session/node/event id.


## Central baseline integration

The package checkpoint merges the centrally reviewed native baseline. Conflicts
were limited to `app/src/shell.rs` and `link/src/lib.rs`: the baseline's extracted
bodies win in full. Inbox's module, command/event, dispatch, refresh and window
registrations are reapplied in their new owning modules. No size record is
raised. Node contracts are consumed from this baseline, not merged separately
into the app. The checkpoint remains work in progress pending narrow checks;
it is not a delivery claim.

## Existing inbox transport (2026-11-15)

The app adapter now uses device.self and the existing inbox list/get/mark and
session.send/read methods from the centrally merged app base. It validates
node + transport incarnation before dispatch and rejects cross-node keys.
Eight permits bound calls including completed results awaiting UI delivery;
a fifteen-second outer deadline bounds full-request recovery and confirmation.
Answers fetch the current full form, validate the exact request and choices,
send once, then wait on committed session events until the inbox projection
withdraws. An acknowledgement alone never releases the submitting state.
A lost reply remains an explicit failure with no automatic answer retry.
Whole-node counters stay independent of the loaded bounded prefix. Malformed
cursor ordering, duplicate keys and foreign detail replies fail closed.
Omitted tool bodies get an explicit source-history explanation.

Transport tests use the real NodeClient over bounded framed memory I/O rather
than a second inbox implementation. The pending validation checkpoint is not
a delivered package; the whole native gate, actual captures, private D-Bus
sound-hint proof and V4 review are still required.

## A5 shared toast behavior (coordinator A8 approval, 2026-11-15)

Paseo `app/src/components/toast-host.tsx:66,80-118,127` defines one current
card, 2200ms ordinary show/copy and 3200ms error-helper duration. All variants
pause remaining time on hover; whitespace is ignored and sticky is explicit.
The old native stack, Ok4000ms, Error-forever and Info3200ms default assertions
are replaced with these expectations by the coordinator's explicit A8 ruling.
Existing stale-id and hover-replacement tests remain and cover every tone.
Git/forge guidance uses an explicit 3200ms duration rather than changing the
shared Info default. Existing Ok is the success semantic tone for callers.
The kit uses the five reference looks; all areas share this timing change.
The package report must tell the native lead about the shared timing change.

## Native notification status difference (coordinator accepted for version one)

The current GPUI App::show_system_notification returns (), with no capability,
permission or delivery result (gpui app.rs:1958). Backend refusal is logged below
that boundary. Settings therefore says permission is managed by the OS and
cannot be checked here. A test says requested/sent, check that it appeared; it
never shows a success result inferred from the void call. Typed status/permission
support is the later desktop-shell owner W19b's item after .19, per coordinator.
The approved Linux GPUI SuppressSound patch is still exactly one product line.

## Attention clear and native settings checkpoint

Paseo use-agent-attention-clear.ts:94-138 and utils/agent-attention.ts:63-83
require focus-entry, input-focus, prompt-send and pane-blur triggers. Newly
arriving attention while already viewed is deferred until input/send/blur.
The new pure Reader carries exact node/session and observed exclusive sequence;
pending permission/question state vetoes passive clear. The link rechecks live
requests and uses the existing Mark::Read watermark, never a fabricated call.
Explicit unread holds the current pane, including its own outgoing blur, and
can clear on a later return. The old repaint timer yields to this controller.

Coordinator A8 approved defaults parity with Paseo desktop-settings.ts:33-35:
new/missing Alerts defaults SystemAndSound, while explicit off/system/sound
remain unchanged. Only that field of the baseline default JSON is updated and
an explicit-off preservation regression is added.

Narrow pure view test log /tmp/work/wp-inbox/view-feature-tests-1.log:
881 passed, zero failed, 17.09s execution. One unnecessary qualification warning
in the new prefs test was corrected afterward. App/link integration, root node
and runtime/visual evidence are still pending; this checkpoint is not delivery.

A8 merged package base6036eeb8 asb76c97e1. Passive clear pins an explicit
session open to the shared Machine.node welcome identity; a reused route name
cannot retarget it. Missing/changed identity suppresses passive writes instead
of falling back to the old name-only repaint timer. The policy's pane focus,
read state and request state are enums, replacing four independent bools after
the narrow clippy check identified excessive flags.

## A5 recipient and OS-idle checkpoint

Merged package base4 3cd0d3fc under A8. Shared Connected is now Clone, so the
presence worker closes when its last authenticated transport clone is dropped.
New OS delivery opportunities pass a bounded exact-source/transport/ticket gate.
Only an admitted node reply may reach GPUI, and read/source/cursor/visibility
are rechecked immediately before display. Lost claims are not automatically
retried. The typed app boundary explicitly refuses claim/idle extensions until
the reviewed pending node contract is integrated; direct local delivery no
longer bypasses the single-recipient policy. A window update is scheduled from
the reply, independent of whether a minimized window is painting.

The OS idle probe reuses x11rb0.13.2, already linked by GPUI, with its ScreenSaver
feature (MIT/Apache-2.0). It replaces no shared GPUI host API: W19b confirmed none
exists. One dedicated worker per process has a one-request queue, a 500ms
consumer deadline and no automatic replacement after failure. Five-second
samples carry elapsed idle only and keep aging in queues. Wayland/Xwayland,
macOS, Windows, browser and unavailable X11 extension/display explicitly remain
unknown; actual app input still reports through the compatible presence shape.
This avoids calling Xwayland-only activity whole-desktop activity. No real
OS-idle/private-display runtime proof has been captured yet.

N1 confirmed inbox.seen is separate from SessionInfo log marks. ADR0089 adds
pending inbox.attention (bounded 256-session authoritative SQL rollup, including
off-page entries) and inbox.read (Read-scoped direct person, existing mark order
and transaction, live/durable pending veto). No whole-session read is substituted
for an exact entry mark. Native badge/reader consumption of this contract is
pending; SessionInfo marks are not silently redefined.

## Bounded older-entry navigation

The initial prefix expansion could require 256 sequential calls and then stop
at 4096 rows. It is replaced by a cursor window: every refresh reads the newest
page (authoritative counts/new opportunities) and at most one selected older
page. Strict descending cursors allow arbitrarily old retained attention with
at most 32 entries retained per node and at most two page reads. A Newest action
resets the window without resetting notification watermarks. Overlap after
projection removal is deduplicated by exact key, never by title. This uses only
existing inbox.list cursors. The new regression traverses 9999 cursor windows
without increasing memory/read bounds. Runtime results are pending.

## Combined staged SDK consumer

Own node source7c926b41d (including announced green gate40 ba45a6e3) is merged
into this app worktree under A8. V3 returned one A7 bound duplication finding;
that correction is committed, and exact changed-source checks remain queued.
This is local package assembly, not producer handoff or source PASS.

Native claim now uses the actual NodeClient method, with exact projection
checks before and after the admitted reply. Terminal focus and aged idle use
the real optional DTOs; passive reads use the narrow inbox.read SDK method.
The single feature constant is still None, so the SDK explicitly refuses these
operations until the integrator assigns it. No invented call or fake success
is used. Authoritative per-session rollups feed board/sidebar/read policy when
supported; older peers explicitly show a legacy badge caption. The existing
SessionInfo marks retain their log-watermark meaning. Session focus uses the
identity pinned on follow, covering a newly started session as well as a card
open; unstructured terminals never masquerade as structured session focus.

G4's canonical ConnectionContext/MachineKey/NodeId/ConnectionEpoch producer is
not consumer-ready yet. Its owner will send a bounded reviewed range. Replace
our temporary private incarnation with that value and selected.call boundary
then; do not import the unreviewed G4 branch. W12's bounded shared source1720 is
announced; C2 D1 focused-target/renderer handoff and W21 reviewed open routing
remain dependencies. No pane manager or resident parser is duplicated here.

## Five status buckets and identifying metadata

The sidebar now uses a pure typed priority model under view::inbox::buckets:
NeedsInput, Failed, Working, Review, Done. It reuses sidebar::listed filters,
computes each status once, then bounds each displayed group. The existing
keyboard first-nine and next/previous actions consume those same groups; no
parallel shortcut registration or pane tree is added. Dot colours come from
kit semantic warning/error/running/success tokens. Current failed outcome counts
are independent of read state, so a seen failure cannot look like a finished
result. Pending requests still win after they are read. Existing SessionMark
compatibility APIs and their tests are retained; new regressions cover the
five priorities and bounded-group overflow.

Status sort currently uses the node's existing Card.detail.active_at followed
by machine/title for stable ties. Exact workspace statusEnteredAt/project sort
awaits W11's reviewed registry/context; no cwd-derived project or workspace id
is fabricated. Inbox rows now retain the node's project_name and use the shared
conversation local-time formatter, rather than dropping their source metadata.

For old peers, passive read retains the existing Operate-scoped Mark::Read path
only after explicit device.self Operate and a live conversation request veto.
The observed exclusive watermark still excludes new attention. Read-only peers
remain unsupported for this fallback; the new Read-scoped ingress is used only
when actually assigned and supported. No offline write is replayed.

## Shared connection adoption

Reuse decision: modify the existing Inbox adapter to consume G4's exact reviewed
producer `074ade16..95c256bce` (result documentation `72f081c35`), rather than
maintaining an Inbox-specific incarnation allocator. `Source` now retains the
canonical `view::connection::ConnectionContext`; the actor selects that exact
admission and every typed RPC passes `SelectedConnection::call`, including
presence, detail loads, marks, answers, long-poll confirmations and claims.
No raw client is exposed by the private Inbox facade. Original context and local
ticket accompany every response; retirement invalidates grants and pending detail
responses before a replacement fleet snapshot. Reconfirming the same exact item
keeps its draft but disables actions until the new admission confirms its form.

Identical list snapshots and presence results request no window frame. Pure
regressions cover repeated successful/failed polls and retained-but-unconfirmed
forms after retirement. G4's own evidence remains bounded source/Link evidence;
this adoption still needs its own exact changed-source native checks and final
package gate. The private follow staging remains unused; no live-stream adoption
or whole-package approval is implied.

## Terminal attention consumer

Reuse decision: project the existing `terminal.list` metadata into the same node
Inbox snapshot and call the canonical ADR 0089 claim/clear DTOs through G4's
selected admission. No terminal parser, pane manager, HTTP ingress or session id
is invented. Unassigned/old peers retain `terminals: None`; raw shell output does
not imply Working or Finished. New exact semantic revisions announce once after
initial hydration; reconnect is silent. Clear retains the observed revision and
never clears a successor. OS text and tags follow the same generic/opaque policy
as session entries. Notification activation reveals the exact terminal row in
Inbox until the checked terminal/layout owner can reveal a real resident.

C2 explicitly reports no ready TerminalId resident/focus producer yet. Therefore
no terminal focus capability is advertised and no SessionId TUI is converted to a
TerminalId. Passive visible-terminal clearing and direct pane reveal remain
unverified/unwired owner integration work; explicit row clearing is staged here.
Workspace bulk marking still requires W11's authoritative membership and W2's
capability-gated root relationships; retained membership alone is insufficient.

Presence compatibility: optional OS idle age and Terminal focus are included
only when the feature constant has been assigned and the original selected
peer supports it. Otherwise basic session/elsewhere/background presence still
uses its existing call; an unassigned optional field must not disable the
ordinary heartbeat. The older-peer projection regression checks both fields.

Coordinator ruling (11:2x): GPUI 0.3.10 has no dock/taskbar badge API. This remains
a stated native difference; the small fork host seam belongs to W19b after core
packages, alongside permission/delivery status. `view::inbox::badges` retains the
reference workspace aggregation, distinct global icon priority and per-workspace
badge count for that future adapter. It consumes eligible workspace facts rather
than guessing membership from a path or counting tabs. Native favicon is absent
as in Paseo native; the existing window title remains the current count surface.

Several saved aliases can authenticate as the same node. Inbox chooses one read
admission per node and retains that exact context while available, so aliases do
not double-count shared rows or reset hydration on every poll. This selection is
only for new reads/presence. An already-issued mark, answer, claim or clear keeps
its original admission and never retries through the replacement. A regression
covers duplicate aliases and rejection of a ticket from the retired read route.
