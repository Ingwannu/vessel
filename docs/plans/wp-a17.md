# Attention inbox and desktop delivery

Status: proposed; implementation begins with the pure state boundary while the
central app/node integration supplies the existing inbox protocol (ADR 0040).

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
