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
