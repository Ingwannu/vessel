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
  workspace" with a ghost small "Show all" pulled 8 into the gap, or
  "Sessions on {host}"); the
  refresh icon before the close button (16 `RotateCw` muted in a 16 slot,
  padding 8, radius 8, pressed `surface2`, a 16 spinner and disabled while
  loading, tooltip and accessible name "Refresh sessions"); the search row
  under the title (the kit's header search seam), no autofocus.
- Provider filter (more than one provider): a pill (leading icon 14, label 14
  medium, chevron 14; padding 6 by 12, radius 6, `surface1`, `border`
  hairline, hover `surface2`, pressed `surface3`), 8 above the list, opening a
  non-searchable list ("All", then each provider sorted by id, every row
  leading with a 14 muted icon).
- Rows: a pressable row (gap 8, padding 8, radius 8, 4 between rows; hover
  `surface1`, pressed `surface2`): the agent mark (muted, in a 16 slot, 2
  down), then a column:
  the title (14 foreground, one line) with the activity time on its right
  ("just now", "5m ago", 12 muted; "Importing..." while this row imports),
  and the folder (12 muted, one line, ellipsized) only when unscoped, named
  as Paseo's `resolveDirectoryLabel`: inside a known project of the host
  "{project} · {relative}", the project's root its name, else the full path
  (`history::folder_label`, the deepest root wins, paths compared by
  component).
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
- `Leading::SmallAgentMark`: the agent mark at 14 in the muted ink, in the
  small glyphs' 14 column, so the filter list's "All" (`Layers`) and its
  providers start their words at one edge (Paseo's `ProviderIcon size={14}`).
- Glyph roles `Inbox` (the empty state) and `Layers` (the filter's "All"),
  Lucide's children at the provider's 1.5, appended to the inventory
  (`assets::sheet_role_tests`).

## View

[Decision Log]
- Purpose: the rulings need every provider of a host at once, merged, with
  per-provider limits, failures and Retry, and one import per press.
- Before: `History`, one scope (machine, agent, account, folder) with
  multi-select rows and batch receipts.
- Alternatives: run one `History` per provider in the app (the merge, the
  limits and the empty states would live in the app, untested in view).
- Choice: `history::Sheet`: one feed per provider, each with its tickets,
  cursor and failure. A feed reads page after page until it holds its limit
  of not-yet-imported sessions (the node has no limit, only cursor pages);
  `answer` returns the follow-up reads. `empty_state` is Paseo's
  `computeEmptyState` (only once every provider answered and not all failed).
- Read slots: the node lists at most two at once (`service::history::AT_ONCE`)
  and answers more `unavailable`, which would show as a failed provider. The
  sheet sends at most two reads; the rest wait (`FeedState::Queued`) and go
  as answers come. A refresh while reads are outstanding can still meet the
  node's limit, as the earlier reads still count there until they finish.
- Impact: `History`, `Loading` and `Selection` are removed; their boundary
  tests (native identity, cursor bound, receipt scope, node change, capacity)
  moved to `history::sheet::tests`. Link is unchanged; its delivery test now
  drives a `Sheet`.

## App

[Decision Log]
- Host: Paseo's new-workspace screen calls `useImportSession({ serverId })`
  with no folder, so the new-session page passes its machine and no folder:
  the list covers the whole host ("Sessions on {host}") and every row shows
  its folder. The workspace form ("This workspace", "Show all") stays in the
  model and the app (`open_import_for(host, cwd)`) for a workspace caller;
  no Vessel caller passes a folder yet (none does in Paseo at the pin).
- No host in context: the first page is Paseo's host chooser inside the same
  sheet ("Import from host", "Search hosts...", rows with the status dot, the
  name over its node id and the `Server` mark, "No matching hosts"). One
  connected host skips it, as `useHostChooser` does.
- A row press imports that session. On the receipt the sheet closes and the
  session opens on the next frame with a window (`settle_import`, beside
  `settle_sessions`), bound to the node the import went to, never a later
  lookup of the machine's name. A receipt after the sheet was closed opens
  nothing (the session is imported and listed as usual).
- Rows are drawn in the dialog body, not a virtual list: the body is the
  compact sheet's scroller (its drag hands off to it), and the per-provider
  limits bound the rows.
- Stale controls: every control captures the opening's visit; closing,
  choosing a host, "Show all", a fleet change of the drawn hosts' controls
  and an exact retirement advance it (`history_fleet`, `history_retired`).

## Differences kept

- Korean: Paseo shows `chooseHostTitle`, "Filter by provider", "Search
  hosts..." and "No matching hosts" in English in every language; Vessel
  translates them ("호스트에서 가져오기", "프로바이더로 필터", "호스트 검색...",
  "일치하는 호스트가 없습니다").
- The host chooser lists connected machines only and has no arrow-key
  selection (Paseo's chooser lists every host with its state and takes
  ArrowUp/ArrowDown/Enter).

## Tests

- View: `history::sheet::tests` (16), `history::folder::tests` (4).
- Kit: `dialog::header_subtitle_tests`, `assets::sheet_role_tests`,
  `picker::rows::small_mark_tests`.
- App: `areas::import::tests` (one host skips the chooser and reads every
  agent; several hosts open "Import from host"; a row press imports, closes
  and opens on the import's node; a receipt after closing opens nothing; the
  filter narrows on the client; a failed provider's Retry reads it alone;
  already imported says so; stale controls do nothing) and
  `areas::import::tests::geometry` (560 on a desktop with rows 8 past the
  inset; the full width on a phone).
- Link: `history::delivery_tests` drives a `Sheet`.

## Status

Gates on 72b62fc12 (package base 14.28 merged), build VM, ordinary lane:

- strict (fonts, fmt, quality, clippy) on 71198de52: 0; 72b62fc12 changes
  one test's expected width only.
- whole app suite: 292 passed, 0 failed, 2 ignored.
- whole view crate: 1289 lib tests and every integration suite passed.
- kit 332 and link 183 passed.
- wasm32 check of link and app: rc 0, no warnings.

After the capture pass (76e3928e3: the 14 muted provider marks in one
column, the 16 slot for row marks): strict 0; kit and app suites rerun on
the final head.

Captures (export of 76e3928e3 on the base14.19 shared fleet, fake agents):
en-dark and ko-light at 1280 and 390: the merged list with the provider pill
and a failed provider's banner, the filter list (a titled sheet on a phone),
one provider (no pill), Codex only, no matches, "No Codex sessions found.",
and an imported session opened. Sent to the coordinator.
