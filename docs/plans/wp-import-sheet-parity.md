# Import sheet parity (TRACKER 777)

Visual lane, 2026-11-17, on package base 14.25. Reference: pinned Paseo 0.15.4
(`1c00b365c`): `components/import-session-sheet.tsx`,
`components/import-session-sheet-view-model.ts`, `components/adaptive-modal-sheet.tsx`,
i18n `importSession.*` (en.ts 401-438, ko.ts 405-442).

## Rulings (coordinator, 2026-11-17)

Paseo parity is the floor, so the behaviour follows Paseo, not the earlier W2
design.

- (A) Selection: a row press imports that one session, closes the sheet and
  opens the agent. The pressed row shows "Importing..." while pending; every
  row is disabled meanwhile. No checkbox rows, no Import selected / Cancel
  footer. Already-imported sessions are not listed; when nothing is left the
  empty state says "All recent sessions are already imported."
- (B) No stacked pickers. The host comes from the caller (the workspace's host,
  or the host the sheet was opened for); with no host in context the sheet's
  first page is Paseo's "Import from host" (`chooseHostTitle`). Every enabled
  provider is fetched in parallel and merged; with more than one provider an
  "All" + per-provider pill filters on the client. Paging per provider: 15,
  then 45 / 90 / 200 behind "Load more" (Paseo's `PAGE_LIMITS`). No account
  picker: when the node lists sessions per account, each saved account is
  fetched and merged; the sheet never asks.
- (C) Each row leads with its agent's mark. The prompt preview waits for the
  node to send it (filed for the node line); no "No prompt preview" line.
- (D) Search filters the loaded rows on the client, with Paseo's placeholder
  "Search sessions...". Server-side search (200 ms debounce, feature-gated)
  is filed with the node item.

## Presentation (Paseo values)

- Card: 560 wide on desktop (`desktopMaxWidth`), content-sized up to the 85 %
  cap; compact: a draggable sheet with snap points 70 % / 92 %.
- Header: title; the scope line under it (12 muted, one line: "This
  workspace" with a ghost small "Show all", or "Sessions on {host}"); the
  refresh icon before the close button (16 `RotateCw` muted in a 16 slot,
  padding 8, radius 8, pressed `surface2`, a 16 spinner and disabled while
  loading, tooltip and accessible name "Refresh sessions"); the search row
  under the title (the kit's header search seam), no autofocus.
- Provider filter (more than one provider): a pill (leading icon 14, label 14
  medium, chevron 14; padding 6 by 12, radius 6, `surface1`, `border`
  hairline, hover `surface2`, pressed `surface3`), 8 above the list, opening a
  non-searchable list ("All", then each provider with its icon).
- Rows: a pressable row (gap 8, padding 8, radius 8, 4 between rows; hover
  `surface1`, pressed `surface2`): the agent mark (16, 2 down), then a column:
  the title (14 foreground, one line) with the activity time on its right
  ("just now", "5m ago", 12 muted; "Importing..." while this row imports),
  and the folder (12 muted, one line, ellipsized) only when unscoped.
- Status: loading is a spinner row ("Loading recent sessions..."); a provider
  that failed gets a banner row ("Could not load {provider} sessions" in red
  12, a ghost small "Retry" that refetches that provider only); an import
  failure says "Could not import selected session."
- Empty: Paseo's Inbox icon (20, muted, 0.6) over a centred title, padding
  32 by 16: "No sessions match your search.", "No {provider} sessions
  found.", "All recent sessions are already imported.", "No recent sessions
  to import."
- "Load more": a ghost button after the list, while a provider may have more.

## Kit

- `DialogInput::with_header_subtitle`: Paseo's `SheetHeader.subtitle` under the
  title in the title group (gap 4). The title truncates when a subtitle is
  present, as it does beside header actions.

## Status

In progress. Gates and captures are recorded here when they pass.
