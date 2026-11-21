# Page terminals answer live terminal queries (TRACKER 826)

The decision, its alternatives and its consequences are in ADR 0017, amendment "the page answers live terminal
queries". The rule it follows is ADR 0073's. This file records the work.

## Problem
- `web/src/data/terminal-input.ts` forwards only real keys, paste and IME (ADR 0017).
- xterm's replies to terminal queries were therefore dropped: replay replies, rightly, and live ones too.
- A program that asks at once waits or times out: nvim's device-attributes query, or a crossterm TUI's cursor report.
- Paseo 0.15.4 answers live queries in its daemon (`packages/server/src/terminal/terminal.ts` 883-922). The native
  app answers them in the viewer (ADR 0073).

## Pieces
- `web/src/data/terminal-queries.ts`: xterm parser handlers for exactly ADR 0073's five queries.
  - DA1, DSR 5, DSR 6, `CSI ? 6 n`, and OSC 10/11/12 with `?`.
  - The bytes are native's. The cursor's column is clamped to the last column while a wrap is pending.
  - `hasDuty` is `duty.rs`'s rule.
- `web/src/data/terminal-live.ts`: which followed records are live.
  - The `next_seq` of a read made as the follow starts, and again after every gap: a link close, or a stream
    reconnect.
  - A read that answers after a later gap began is ignored.
- `web/src/data/terminal-sender.ts`: `answer()` queues an answer with the keys. It drops the answer when more than
  `MAX_ANSWERS_PENDING` (64 KiB) would wait.
- `web/src/features/session/mount-terminal.ts`:
  - Every write is history or live. Answers are allowed while no history write is still being parsed.
  - Focus is the browser tab's together with the terminal's.
  - `origin` says whether this tab opened the terminal.
- The two views: `terminal-pane.tsx` reads `terminal.get` and `terminal-view.tsx` reads `session.get`, each through
  `liveBoundary`.
- `web/src/data/workspace/terminals.ts`: `openedInThisTab` lists the shells this tab's New terminal created.

## Tests
- vitest:
  - `terminal-queries.test.ts`
  - `terminal-live.test.ts`
  - `terminal-sender.test.ts`
- e2e `app.spec.ts`:
  - "a live device query is answered once, by the view that has focus and has typed"
  - "a query written while the tab's link was down is never answered late"
  - "terminal replay cannot answer device queries, while keys and paste still work" (unchanged)
- e2e `workspace-tabs.spec.ts`: "a shell this page opened answers a live cursor query after focus has left".

## Review
- Node reviewer round 1:
  - R1: a query written during a reconnect gap was answered late. Fixed by `terminal-live.ts`.
  - R2: the cursor column was one past the last while a wrap was pending. Fixed by `cursorCell`.
  - Notes, also taken:
    - the browser tab's focus;
    - the opener-plus-typer case, in the ADR;
    - the colours stated as the scheme's;
    - the answers' bound;
    - the test gaps.
