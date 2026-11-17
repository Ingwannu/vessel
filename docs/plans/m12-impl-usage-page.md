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
card mounts (cached only; hovering never contacts a vendor) and draws compact cards (Refresh per card, no
pins; the freshness follows Paseo's `showsFreshnessInline`: in the Refresh tooltip where there is hover, on the
card at phone width or when nothing can be refreshed). The figures are Paseo's three lines: the title, "{pct}%
used" in foreground, then the tokens and the session's cost, muted. Nothing on machines below protocol 26, with lookup off, or when the session has no
account; a failed read shows Paseo's one line ("Unable to load usage: …").
`features/usage/use-account-usage.ts` holds the reader and clock hooks both views share.

### The session's limit notice

The native app's dock notice (areas/usage/session.rs `limit_notice`; Paseo has none) over the page's composer,
first among the docks (`features/usage/limit-notice.tsx`): when the node's `UsageLimits.notice` for the session
says its saved login reports a full general window, the native English words ("The saved account reports a
usage limit. This does not prove why the turn stopped.", the latest reported reset while ahead, "Wait for
reset", "No prompt will be sent automatically. Continue when the account is ready.") and Switch model.

- Data: `usage.limits` (protocol 23) with `{ kind: "session" }`, cached only, read when the composer opens and
  again after each record after which the login may be another (`Timeline.binding`: status, profile or options
  change, parked, woken, rewound; native `binding_epoch`). A notice read before such a record is hidden until
  the new answer; a failed read shows nothing (an agent query's error hides its values, as on native). This is
  the one fact the report methods do not carry; `usage.limits` is not a second card path.
- Switch model presses the same composer's model trigger (`data-model-picker` in compose/agent-menu.tsx), as a
  person's click does; the kit Menu takes no outside open, and the composer and kit menu are at their budgets.
- The composer's memo compares `binding` too (composer-props.ts), so the read follows those records.
- The e2e fleet has no saved login, so the notice is covered by unit tests (`session-notice.test.ts`,
  `limit-notice.test.tsx`) and by a capture on the recorded usage fixture.

## P2: kept choices, pins, the sidebar summary and the modal (ADR 0107)

- Bridge (vessel-web): `wire/prefs/usage.rs` (UsageDisplay, UsagePin, UsagePins bounded at 32,
  UsagePreferences), `Preferences.usage`, `SidebarView.usage`, changes SidebarUsage / UsageDisplay /
  UsageHost / UsagePins; the keeper's `usage` section (keep/prefs_doc/sections.rs) and tests
  (keep/prefs_doc/usage_tests.rs: an older document reads as the defaults and is not rewritten for them;
  each change writes only its field; `[]` pins nothing, null follows the defaults; an unreadable field is
  kept as written). Generated `web/src/wire/bridge.ts` follows.
- Page: `data/prefs-change.ts` applies the four changes; `features/usage/use-usage-prefs.ts` reads them;
  `usage-options.tsx` is Paseo's options popover (340 px, no visible heading on wide screens; SettingsSwitch
  rows with label and hint left and the control right, hairlines between: the lookup with our consent hint,
  "Summary in sidebar" / "Pinned windows show in the sidebar footer", the "Percentages" row);
  `data/account-usage/pins.ts` is Paseo's pin rules (defaults: windows marked for the summary, else each
  source's first with a percent; the first edit keeps the shown defaults); window rows are pin toggles while
  the summary is on (`quota-card.tsx`).
- `sidebar-usage.tsx`: Paseo's sidebar item above the sidebar's foot (marks, meters/percents/short labels,
  the richest layout that fits, measured off screen with N2's quotaPinnedLayout); it follows the picked host,
  then the active machine, then the first that reports (N2's quotaSummaryHost). A click opens
  `usage-modal.tsx`: Paseo's "Usage" dialog, its title row holding the actions (the HostFilter pill when more
  than one machine is connected, `usage-host-filter.tsx`; Refresh all; the options cog) and its body the cards
  alone (no "Plan usage" heading or caveat, which are the Settings page's). Picking a host writes the host
  preference. Settings and the modal share `usePlanUsage`, `PlanControls` and `PlanContent`.
- Kit: `Popover` takes `showTitle={false}` (the title still names the popup) and the `settings` width (340);
  `Dialog` takes title-row `actions` (before Close).

## TRACKER 793: the default turn delivery is Steer (ADR 0108)

Not usage, but this track's next item (coordinator, 2026-11-17). The bridge's `TurnDelivery` default and the page's
`DEFAULT_PREFERENCES` move from Queue to Steer together; the sparse keeper means a document that never set the
delivery reads as Steer and one that chose Queue keeps it (no migration). The empty box's placeholder now says what
Enter will do (`nextDelivery` in data/delivery.ts), which it did not for Steer before. The conversation e2e sets Queue
in Settings for the queue cases and puts Steer back; the screens harness queues with the modifier. The page still
queues where the agent cannot steer (its stated fallback); native and Paseo replace the turn there, reported to the
coordinator as a separate parity item.

## TRACKER 794: Paseo's two delivery rules on the page

[Decision Log] (ADR-light: no wire change; both are deliveries the node already takes)
- Purpose: a message sent during a turn goes as on Paseo and the native app (view/src/session/delivery.rs
  `send_delivery`), not by the page's own rules.
- Constraint: the page queued a Steer the agent could not take, and disabled the box while a permission waited, so
  the Queue-behind-a-permission case could not arise.
- Alternatives: keep the queue fallback (gentler, but the person chose to reach the agent now); keep the box
  disabled during a permission (then a Queue cannot be stranded, but the person cannot redirect a turn parked on a
  request, which Paseo and native allow).
- Choice (data/delivery.ts `sendPlan`):
  - Steer the agent cannot take (or once refused) -> Interrupt, fallback `cannot-steer` ("cannot steer: this
    replaces the turn").
  - Queue while a permission waits -> Interrupt, fallback `permission-waits` ("A permission waits: this stops the
    turn instead of queueing"); Steer and Interrupt go as chosen; the other way stays Queue.
  - The box stays usable while a permission waits on nodes from protocol 10 (before 10 nothing can be sent during
    a turn, so it stays disabled with "Answer the permission to continue"); the placeholder says what Enter will
    do (`nextDelivery`).
- Costs: a Steer to an agent without steering now stops its turn where it used to wait (the button says so).
- Follow-up (coordinator, same day): the other way of Queue is interrupt-and-send, as Paseo's "Command/Ctrl+Enter
  submits" (`resolveAlternateSendAction`), where the page used to steer; Settings › General's Default send hint is
  Paseo's words (en.ts settings.general.defaultSend.descriptions / commandEnterDescriptions) by the send key.
- Tests: delivery.test.ts (both rules, nextDelivery), composer-words.test.ts (placeholders), conversation.spec
  (Queue while a permission waits: box enabled, placeholder, "Interrupt and send"), app.spec (box enabled).

## Next

- The native test's copy of the page fixture gains the bridge's new fields (app line, separate packet).
- Captures of P2/P3 (summary, modal, pins, session hover card) to the coordinator.
