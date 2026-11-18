# One tab menu builder (TRACKER 797)

Visual lane, 2026-11-18. Built on TRACKER 795's branch (head 65c10b67b, with
789 and package base 14.36). Reference: pinned Paseo 0.15.4 (`1c00b365c`):
`screens/workspace/workspace-tab-menu.ts` (`buildWorkspaceTabMenuEntries`),
`screens/workspace/workspace-screen.tsx` 2110-2190 (copy resume command,
reload agent), `utils/provider-command-templates.ts`,
`screens/workspace/workspace-tab-trailing-accessory.tsx`.

Status: built; gates under "Status".

## Paseo

One builder, `buildWorkspaceTabMenuEntries({ surface })`, for the desktop
tab context menu and the phone switcher row menu. Per tab kind, in order:

- Agent: Copy resume command (copy), Copy agent id (copy, hint = the id's
  first 7 characters).
- Terminal: Copy terminal id (copy, hint = first 7).
- File: Copy file path (copy), then "Open with…" when the file has several
  panels.
- Agent and terminal: Rename (pencil), then a rule.
- Every tab: Close tabs to the left / above (arrow-left-to-line, disabled on
  the first tab), to the right / below (arrow-right-to-line, disabled on the
  last), Close other tabs (copy-x, disabled when alone).
- Agent: Reload agent (rotate-cw, tooltip "Reload agent to update skills,
  MCPs or login status.").
- Every tab: Close (x).

Labels per surface: desktop "Close to the left/right", mobile "Close tabs
above/below".

Copy resume command: the provider's own session id (`runtimeInfo.sessionId`
or `persistence.sessionId`) in the provider's template (claude
`claude --resume {sessionId}`, codex `codex resume {sessionId}`, opencode
`opencode --session {sessionId}`, …); toasts resumeIdUnavailable when there
is no id, resumeCommandUnavailable when the provider has no template.

Reload agent: `client.refreshAgent(agentId)` (the agent's process starts
again on the same conversation, picking up skills, MCPs and login), then a
timeline refetch; toasts reloadingAgent, reloadedAgent, failedToReloadAgent.

## Vessel before

Two builders that drift: `layout/menu.rs` (the wide strip's context menu:
Copy ID or Copy path, Rename for workspace layouts only, the closes when
the tab closes locally) and `layout/compact/row_menu.rs` (789: Paseo's
mobile items, then the session's own actions).

## Rulings (coordinator, 2026-11-18)

- Reload agent is not park and wake: Paseo's refresh unarchives, interrupts
  a running turn, reloads the agent from persistence and re-reads the
  provider's history (server `handleRefreshAgentRequest`); a client-side
  park, cancel and wake would race. It becomes TRACKER 803 with a node call
  `session.reload` (ADR 0110). 797 ships without it, with its slot after
  Close other tabs.
- The kit menu's row tooltip comes with 803, its first consumer.
- The app reviewer's P3 from 795 folds in: the session "…" has role button
  and a name but no click listener, so assistive tech cannot press it.

## What was built

[Decision Log]
- Purpose and intent: one tab menu for both widths, as Paseo's, so the strip
  and the switcher cannot drift; Paseo's copies.
- Existing implementation and constraints: `layout/menu.rs` built the
  strip's rows and `layout/compact/row_menu.rs` the switcher's; the strip's
  had no glyphs or hints, Rename only on workspace layouts, and showed no
  closes when the tab itself did not close locally. Cards carried no
  harness session id, so only the open session could name its own.
- Alternatives considered: keep two builders sharing helpers (the drift
  Paseo's comment warns of); resume commands built by the node (no such
  call; the coordinator ruled client templates).
- Chosen approach: `layout::tab_menu::entries(shell, tab, surface)`; the
  surface picks only the directional close labels. One press path per row
  (`menu::chose`). Copy resume command reads the open session's
  description, else its card (cards now keep `harness_session`), through
  `vessel_app_view::session::resume_command`, Paseo's table.
- Why: Paseo's structure; the card field lets every agent tab copy its
  command, not only the open one.
- Pros, cons and impact: the strip's context menu gains Paseo's rows,
  glyphs, hints, 220 width and the session's extras on the open agent's
  tab; its closes now show disabled instead of being hidden.

- `layout/tab_menu.rs`: the builder, the copies and their toasts; the
  switcher's old `compact/row_menu.rs` is gone.
- `layout/menu.rs`: `Action` (Copy, CopyResume, Rename, Close, Session),
  one overlay for both surfaces, 220 wide (`DROPDOWN_WIDTH`), the switcher's
  above its sheet.
- Rename: one action. On a workspace layout it opens the layout's rename
  (it carries the original node's authority and waits for the answer),
  elsewhere the session's rename dialog (789), as the header's title does.
  Shown on the open session's agent tab when the node describes it as
  renamable.
- Toasts as Paseo's `toast.copied`: "Copied Agent ID", "Copied Terminal
  ID", "Copied File path", "Copied resume command"; "Resume ID not
  available" and "Resume command not available" as advisory toasts (Paseo
  shows them as errors; Vessel's error toasts stay until dismissed).
- Words (en, ko): Copy resume command, Copy terminal id, Copy file path,
  the copied labels and notices; the strip's closes are Paseo's desktop
  words "Close to the left / right" (ko unchanged, Paseo's own).
- View: `session::resume_command` (Paseo's table: Claude Code, Codex,
  OpenCode; Gemini and added agents have none); `CardDetail::harness_session`
  (serde default, left out when absent).
- Session "…" (P3): opens on a click instead of a press, so GPUI offers
  assistive tech its Click action; a pointer and a screen reader take one
  path and it opens once.

## Differences kept

- Rename shows on the open session's agent tab only (both rename dialogs
  read the open session); a terminal has no rename yet.
- A file tab has no "Open with…" (one panel per file).
- The closes stay disabled where they would close a tab that does not close
  locally (an agent archives, a terminal confirms); Paseo disables them by
  position only.
- No Reload agent until TRACKER 803.

## Tests

- view `tests::resume`: each CLI's command; the id notice before the
  command notice; added agents and Gemini have none.
- app `layout::tab_menu::tests`: the strip and the switcher list the same
  actions, only the directional labels differ; an agent tab leads with Copy
  resume command and Copy agent id (7-character hint) then a rule; a
  terminal's Copy terminal id has its hint; Copy resume command copies
  Claude Code's command from the open session and Codex's from another
  tab's card, and says why not without an id or for Gemini; the id copies
  put the id on the clipboard with Paseo's toast.
- app `layout::compact::tests`: the switcher's agent row as Paseo's, then
  the session's extras.
- app `layout::workspace_menu::tests`: the session "…" offers Click in
  GPUI's accessibility tree, and a click opens its menu once.

## Status

Built; gates pending.

## For TRACKER 800

Two rename dialogs exist: the layout's (workspace layouts: original-node
authority, waits for the answer, keeps the text on refusal) and the
session's (789: Paseo's frame, sends and closes). 800 could make one: the
789 frame with the layout's send-and-wait.
