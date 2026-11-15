# WP-A5a: side question

Status: proposed; Step 0 integration is pending. Worktree `vessel-wp-side`.

## Reference and contract

Paseo 0.15.4 app/server/protocol sources were searched for side question,
side chat and side conversation; no corresponding implementation was found.
This package therefore follows the plan's fallback, GOAT:

- `apps/web/src/components/sideChat/SideChatSurface.tsx:43-59`: separate
  conversation beside its parent; each question sees recent parent state;
  no side message enters the parent's history.
- `SideChatSurface.tsx:75-125`: find/reuse or start a side; inherit model,
  runtime mode, project and worktree; one creation in flight.
- `SideChatSurface.tsx:128-159`: send a pending question once; start over
  deletes the old side and creates another; errors remain visible.
- `SideChatSurface.tsx:165-245`: independent embedded ChatView, context
  explanation, reset, full-thread action and failure/retry state.
- `sideChat.logic.ts:16-40`: newest nonarchived side, scoped by machine and
  parent; a side cannot itself open another side.

Node contract, read on `242076b5` (not yet integrated):
`crates/vessel-proto/src/side_questions.rs` and
`crates/vessel-node/src/service/side_questions.rs`. Open and reset return
SessionInfo. Prompt takes the side id plus validated PromptText and returns
Delivered. Reset takes the parent id and the expected side id. Normal
session prompt ingress must never be used for a side. The node owns context
refresh and refuses slash commands. No new wire calls or versions.

## Decision Log

- Purpose and intent: an independent composer and conversation, without
  replacing the main session's subscription or writing to its timeline.
- Existing implementation and constraints: `ConversationArea`, `Seat`,
  `Place`, `timeline::rows` and `dock::requests` already support embedded
  conversations. Link's ordinary Open owns a single main subscription;
  reusing that command would disconnect the main timeline.
- Alternatives: duplicate timeline/permissions rendering; switch the main
  session to the side; introduce another UI/dependency. These either drift
  or violate simultaneous/history-isolated behaviour.
- Choice: modify/reuse the conversation renderer and kit composer; add a
  pure side lifecycle and a typed link module with its own subscription.
  Standard Rust enums and existing protocol/value types suffice; no new
  external dependency.
- Why: the shared renderer keeps tools, thoughts, permissions, questions,
  scrolling and Markdown identical while a separate seat owns state.
- Advantages, costs and effects: small registration seams in shared files;
  the side carries the normal ConversationArea's unused menu/find state.
  Typed tickets reject late replies after parent changes and close/reopen.
  Node/transport errors map to bounded translated failures, never raw text.

## Structure and data flow

- `view/src/side_question/`: target, validated question, operation tickets,
  lifecycle, stale-response rules and tests; no I/O or GPUI.
- `link/src/side_question/`: typed requests/events, node calls once Step 0
  lands, independent stream with cancellation; no UI or main Open command.
- `app/src/areas/side_question/`: own composer and ConversationArea, shared
  timeline/requests seat, responsive panel and header action.
- `view/locales/{en,ko}/side-question.ftl`: complete messages.
- Input → validated Question → typed Ask → existing node call → tagged
  result/records → side state and side ConversationArea only.

## Stated differences

1. Node-owned bounded parent snapshot, rather than GOAT's native fork when
   available (existing ADR 0045 contract); no pretend full-context claim.
2. Reset uses the expected side id and serialized node operation; an
   uncertain failure is never automatically retried. Recovery explicitly
   opens/finds the current side because reset may already have deleted it.
3. Side prompt slash commands are refused as the node requires; they must
   not route through the parent's composer command processing.
4. Narrow windows show the panel over the conversation using the kit's
   existing sheet/layout conventions; desktop shows it alongside.

## Validation and completion

Narrow view/link/app checks per commit through cargo-slot short lane.
One whole native remote gate at package end, then V4 review. Own fake fleet,
private authenticated Xvfb; capture/open dark/light EN/KO at 1280x900,
390x844 and 800x600. Verify main timeline isolation, follow independence,
reset, close/reopen, late results, offline and older-node refusal. Real
agents, live preview and owner-machine behaviour remain unverified.
