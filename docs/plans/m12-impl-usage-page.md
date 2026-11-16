# Page account usage (TRACKER 771, usage half), impl U, 2026-11-16

Takes over the usage half of N2 (backend-m12-web-usage-resume.md) after the account methods were assigned
protocol 26 (node head d077aa94: usage.reports, usage.inspect, usage.source_lookup; ADR 0042). Built on N2's
inactive values (merged from aeee445d4: data/account-usage clock, duration, format, values, hosts,
pinned-layout; kit QuotaMeter f876a23a), copying pinned Paseo (packages/app/src/usage).

## P1 (this packet): Settings > Usage, Plan usage

- `data/account-usage/calls.ts`: the four calls; `servesAccountReports(status)` reads the connected protocol.
- `data/account-usage/cards.ts`: report list → card values (Paseo card.tsx, window-bar.tsx, balance-bar.tsx
  `resolveBalance`, format.ts `formatAmount`); pure, `now` and the language supplied.
- `data/account-usage/consent.ts`: consent rows, the native app's rules (no row for an unsupported source or one
  that inherits the machine-wide switch).
- `data/account-usage/reader.ts`: one machine's list; cached read on open; Refresh all / one; the switch and
  consent each followed by one cached read; answers from an older generation dropped.
- `features/usage/plan-usage.tsx`, `quota-card.tsx`, `quota-card.css`: the section above the ledger.

### Decisions

[Decision Log]
- Purpose: Paseo's plan usage cards on the page from the node's own reports, as on the native app.
- Constraint: the methods exist from protocol 26; an older machine refuses them. The page drew no account cards
  before, and its words are English.
- Alternatives: (a) call and fall back on `not_in_protocol`; (b) wire `usage.limits` (23) for older machines;
  (c) read the connected protocol first.
- Choice: (c). An older machine is never asked and its card says it needs an update (Paseo
  `hostUpgradeRequired`). (b) would add a second, legacy card path the page never had.
- Display (Used/Remaining) is page state in P1, default Used. The bridge's preferences (ADR 0028) have no usage
  field; persisting it with pins and the sidebar summary needs a bridge preference change and comes with P2.
- Source marks: the native kit's approved artwork (copied byte for byte into ui/kit/provider-marks), light and
  dark forms by scheme; sources without an approved mark keep the kit glyph, as on the native app.
- Costs: English words are copied from the native usage words (view/locales/en/usage.ftl); numbers and dates
  follow the browser language through N2's Intl helpers (so tokens are compact here, unlike the native
  interim formatter, which waits for ADR 0101).

## P3: the session's account in the context hover card

Paseo's context window hover card (components/context-window-details.tsx, usage/agent-usage.tsx) shows the
session's plan usage below the context figures. The page's context ring turns from a tooltip into a kit
`HoverCard` (Base UI PreviewCard, the popover surface; same 400 ms delay): the figures as before, then
`features/usage/session-usage.tsx`, which opens the same reader with `{ kind: "session", session }` when the
card mounts (cached only; hovering never contacts a vendor) and draws compact cards (freshness on the card,
Refresh per card, no pins). Nothing on machines below protocol 26, with lookup off, or when the session has no
account; a failed read shows Paseo's one line ("Unable to load usage: …").
`features/usage/use-account-usage.ts` holds the reader and clock hooks both views share.

## P2: kept choices, pins, the sidebar summary and the modal (ADR 0107)

- Bridge (vessel-web): `wire/prefs/usage.rs` (UsageDisplay, UsagePin, UsagePins bounded at 32,
  UsagePreferences), `Preferences.usage`, `SidebarView.usage`, changes SidebarUsage / UsageDisplay /
  UsageHost / UsagePins; the keeper's `usage` section (keep/prefs_doc/sections.rs) and tests
  (keep/prefs_doc/usage_tests.rs: an older document reads as the defaults and is not rewritten for them;
  each change writes only its field; `[]` pins nothing, null follows the defaults; an unreadable field is
  kept as written). Generated `web/src/wire/bridge.ts` follows.
- Page: `data/prefs-change.ts` applies the four changes; `features/usage/use-usage-prefs.ts` reads them;
  `usage-options.tsx` is Paseo's options popover (lookup, Show in sidebar, Display as);
  `data/account-usage/pins.ts` is Paseo's pin rules (defaults: windows marked for the summary, else each
  source's first with a percent; the first edit keeps the shown defaults); window rows are pin toggles while
  the summary is on (`quota-card.tsx`).
- `sidebar-usage.tsx`: Paseo's sidebar item above the sidebar's foot (marks, meters/percents/short labels,
  the richest layout that fits, measured off screen with N2's quotaPinnedLayout); it follows the picked host,
  then the active machine, then the first that reports (N2's quotaSummaryHost). A click opens
  `usage-modal.tsx`: Paseo's "Usage" dialog with a host picker when more than one machine is connected
  (picking writes the host preference) and the Plan usage section for that machine.

## Next

- The native test's copy of the page fixture gains the bridge's new fields (app line, separate packet).
- Captures of P2/P3 (summary, modal, pins, session hover card) to the coordinator.
