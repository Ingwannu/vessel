# ADR 0124: Inline review comments on the diff, and a prompt's context, as typed blocks

Status: accepted (implementer G, 2026-11-21; the coordinator's ruling of the same day accepted items 1 to 4 as
drafted and added item 5). The protocol is drafted as 45 (`PROMPT_CONTEXT_PROTOCOL`); the integrator assigns it at
the node-line merge. Packets: P6a node (this wire), P6b native, P6c page.

## Context

Paseo 0.15.4 lets a person comment on lines of the working diff and send the comments to an agent
(survey `p3-forge-review.md`, "Review: inline diff comments sent to the agent"; CATALOGUE F-E45-52,
F-E82-85; all "none" in Vessel today):

- Every reviewable line of the Changes diff has a gutter "+" (`review/surface.tsx:83`). Wide: an inline
  editor under the line (auto-focus, Escape cancels, Cmd/Ctrl+Enter saves, Save disabled while the trimmed
  body is empty) and the line's thread of saved comments, each with edit and delete. Compact: a bottom
  sheet "Review comment", subtitle `<file> · <-|+><line>`. Geometry: comment 72, editor 132, gap 6,
  padding 8.
- Drafts are kept on the client (`review/store.ts`, persisted, version 2, strict schema; invalid data is
  dropped). One list per key `review:server:workspace:mode=<uncommitted|base|all>:base:ignoreWhitespace`;
  a comment is `{id, filePath, side old|new, lineNumber, body, createdAt, updatedAt}`.
- A floating "Send feedback (N)" (`review/feedback/surface.tsx`) sends the comments to an agent of the
  workspace: one agent at once, several through "Choose an agent". The message is the prompt "Please
  address this code review." plus a `review` attachment: per comment the file, side, line, body, the
  hunk header and up to 3 lines either side within the hunk (`buildReviewAttachmentSnapshot`); comments
  whose line left the diff are dropped, and none left is "Saved comments no longer match this diff."
  Delivery follows the send setting: steer into the running turn, otherwise interrupt. Success removes
  only the versions sent (an edit made meanwhile stays) and toasts "Feedback sent to <agent>". Disabled
  reasons: no agents, disconnected, no context; failure "Failed to send feedback. Try again."
- While the diff is open the review is also a workspace attachment, a "Review · N comments" pill in every
  composer of the workspace, sent with the next message (and cleared once sent).
- The server renders the attachment for the agent (`server/agent/prompt-attachments.ts:91-117`):
  `Paseo review attachment (<mode>)`, `CWD:`, `Base:`, then per comment `Comment i: path:side:line`, the
  body, the hunk header and the context lines `<"> "|"  "><old ln> <new ln> <+|-| ><content>`. The
  timeline shows the person's message with the review as a pill, not as the rendered text.

Vessel today: a prompt is `ContentBlock`s (text, resource link, opaque, attachment). The native app's
composer already has `ContextKind::Review` in its tray, unused; the page gained context pills in
TRACKER 833. Both send a pill as one more `Text` block, so the timeline shows its whole text in the
person's message. The page's browser storage does not survive a launch (ADR 0028: the bridge keeps the
page's drafts).

## Decision

1. Agent text (F-E50, F-E85): a typed prompt block, rendered by the node.
   - New `ContentBlock::Review { root, mode, base, comments }` in what a client sends; each comment
     `{ path (RelPath), side, line, body, hunk_header, target, lines[] }` with Paseo's bounds plus ours:
     at most 100 comments, body 16 KiB, 7 context lines, 64 KiB per block in all.
   - `mode` is the Changes pane's scope the comments were made on (Vessel's `DiffScope`: unstaged,
     staged, uncommitted, base, commit; Paseo's uncommitted | base | all), `base` its base when it has one.
   - The node checks the block, renders Paseo's text (heading "Vessel review attachment (<mode>)", the
     same lines after it) and gives the agent that text block, where it already turns an `Attachment`
     block into what the agent takes (`attachments/load.rs`); the session's timeline keeps the typed
     block, so clients draw it as Paseo's pill ("Review", "N comments"), with the text a press away.
   - Older nodes (before the protocol): clients render the same text themselves and send `Text`; the
     timeline then shows the text, as today.
2. Drafts (F-E47, F-E82): kept by each client, as Paseo's. Native: in its drafts document beside the
   text drafts (`view/composer/drafts.rs`). Page: in the bridge beside the text drafts (ADR 0028,
   `keep/drafts_doc.rs`), one record per key, bounded (200 comments per key, 64 keys). Key: machine, workspace root, mode, base, whitespace (Paseo's parts, the
   workspace named by Vessel's root). Not on the node: Paseo shares nothing between clients either, and a
   node store would add a schema and a write path for unsent text.
3. Sending (F-E49, F-E84): a normal prompt (`conversation::PromptSession`) with the words "Please
   address this code review." and the review block; recipients are the workspace's sessions that take prompts; delivery
   steer when the setting says steer and the agent can, otherwise interrupt (Paseo's); partial clear by
   id, `updated_at` and body. No new call.
4. The review as a composer pill (F-E83): while the diff is shown, each composer of the workspace offers
   the review as a pill (the native `ContextKind::Review`, the page's tray kind `review-draft`), removable
   per composer, cleared from the drafts once a prompt carrying it went.
5. A prompt's other context (the coordinator's ruling, same bump): `ContentBlock::Context { kind, title,
   subtitle?, text, url? }`, Paseo's titled text attachment, for a pull request's comment, review and check
   logs, a workspace file, a browser element, earlier conversation, a forge's issue or pull request and a
   plugin's resource. Bounds: title and subtitle one line of at most 200 characters, text at most 64 KiB,
   url `https` or `http`. The agent reads its text; the timeline keeps the block. Clients send `Context`
   when the node speaks the protocol and `Text` otherwise (P6b switches the native sender, P6c the page's
   833 tray).
6. Timeline draw, both blocks: a pill (Paseo's FileText mark for a context, its review mark for a review)
   with the title and the kind's word or "N comments", the text a press away; never the text inline.
   A peer before the protocol reads each block as a `text` block of the agent's text (the turn's prompt,
   a steer, a user message chunk, and the queue of `session.conversation` and `queue_changed`), and
   `Call::since` makes a prompt, a send or a queue edit holding one need the protocol.

## Alternatives

- Client-rendered text only (no wire change): no node work, but the timeline shows the whole rendered
  review in the person's message, unlike Paseo, and every client keeps its own copy of the renderer.
- Drafts on the node (shared between clients): goes past Paseo; needs a store table, a write path for
  unsent text and conflict rules between clients.
- Pills as text blocks only (TRACKER 833 as first built): no wire change, but the timeline shows each
  pill's whole text in the person's message, where Paseo shows the pill.

## Consequences

- One protocol bump (node line) for both blocks, then native and page packets; older nodes and older
  clients keep working through the text fallback.
- The node gains a renderer and its bounds tests; clients gain the gutter, editor, threads, sheet, drafts,
  feedback action and pill.
- Packets: P6a node (block, render, bounds, e2e with a fake agent that echoes the prompt), P6b native
  (with the two owed app-reviewer P3 follow-ups), P6c page.
