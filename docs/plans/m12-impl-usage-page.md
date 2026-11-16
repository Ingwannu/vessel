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

## Next

- P2 (ADR 0107, coordinator approved): the bridge preference document gains a `usage` section shaped like the
  native view/src/prefs/usage.rs (display, host, pins) plus PreferenceChange UsageDisplay/UsageHost/UsagePins
  and the sidebar visibility flag; old documents load with Used, no host, no pins. Then the sidebar summary
  (pins, N2's quotaPinnedLayout) and the usage modal on the page.
