# Reload agent in the native tab menu (TRACKER 803)

Visual lane, 2026-11-18. Built on package base 14.54 (`d96f0baef`), which
carries the node op (`session.reload`, protocol 31, ADR 0110, impl U's row
332). Reference: pinned Paseo 0.15.4 (`1c00b365c`):
`screens/workspace/workspace-tab-menu.ts` (`reload-agent`),
`screens/workspace/workspace-screen.tsx` (`handleReloadAgent`),
`components/ui/menu/menu-item.tsx` (the row's `tooltip`),
`components/ui/tooltip.tsx`, `i18n/resources/{en,ko}.ts`
(`workspace.tabs.menu.reloadAgent`, `reloadAgentTooltip`,
`workspace.tabs.toasts.reloadingAgent`, `reloadedAgent`,
`failedToReloadAgent`). The page's own row: `web/src/data/reload.ts`.

Status: plan approved by the coordinator (2026-11-18) with one ruling
(below); built and gated; frames with the coordinator.

## Paseo

- An agent tab's menu has "Reload agent" (rotate-cw) after Close other
  tabs and before Close, on every agent tab, desktop and mobile.
- The row has a tooltip, "Reload agent to update skills, MCPs or login
  status.": a menu row's `Tooltip` with `delayDuration 250`, desktop only,
  `side="right"`, `align="center"`, `offset 10`, its text 14 in the
  foreground on the kit tooltip card (max 280 wide).
- Pressing it: "Reloading agent..." stays up (`durationMs: null`) until the
  answer; then "Reloaded agent" (success), or the error's message, else
  "Failed to reload agent". While the host is disconnected it says so
  instead.

## Vessel before

The tab menu builder (TRACKER 797, `layout/tab_menu.rs`) marks the slot
("TRACKER 803: Reload agent goes here"). The link has no reload act; the
kit menu row has no tooltip; a toast always times out.

## Plan

[Decision Log]
- Purpose and intent: Paseo's Reload agent row in the one tab menu, over
  the node's `session.reload`.
- Existing implementation and constraints: the builder serves the strip
  and the switcher; session calls go through `vessel_app_link::Act` and
  come back as `Event::Acted` with an `ActKind`; the kit tooltip
  (`Tooltips`) is screen-held, shows above or below its trigger; the toast
  host shows one toast at a time and every tone has a life.
- Alternatives considered: a tooltip kept by the screen and threaded into
  the menu (every menu caller would hold one); a toast tone with a very
  long life for "Reloading agent..." (it would still time out on a slow
  reload, or stay after a lost answer).
- Chosen approach:
  - link: `Act::Reload` (`ReloadSession`, answered with the session's
    info), `ActKind::Reload`.
  - view: `CardDetail::cannot_load` (its agent said it cannot load a
    conversation) and `session::can_reload(protocol, …)`, the page's
    `canReload`: shown only on a node speaking `RELOAD_PROTOCOL`, for an
    agent that can load its conversation. Words in en and ko from Paseo.
    A held toast (`Toasts::hold`): no clock, replaced by the answer's.
  - kit: `MenuItem::tooltip`; the menu keeps its own `Tooltips` and a
    place per row, and draws the tip in its own layer; a tooltip
    `Side::Right` with Paseo's 10 offset and 250 ms delay; none on a phone
    (Paseo's `enabledOnMobile={false}`).
  - app: the row in `tab_menu::entries` after Close other tabs on an
    agent tab; `Action::Reload`; the act sent to the tab's session on its
    machine, the toasts on its answer.
- Why: the menu's tooltip stays the kit's business (no caller threads a
  controller), and the reload toast lasts exactly as long as Paseo's.
- Pros, cons and impact: one more act; the row is hidden on a connected
  node older than 31 (Paseo has no such node).

Ruling (coordinator, 2026-11-18): copy Paseo for the disconnected case:
the row shows on a disconnected node and a press toasts Paseo's
`workspace.terminal.hostDisconnected` ("Host is not connected", en and ko)
as an error. Hidden only on a connected node older than protocol 31 (and,
as the page, for an agent that cannot load its conversation).

## Tests

- kit: a row with a tooltip shows it right of the row after its delay,
  vertically centred, and not on a phone; a row without one shows none.
- view: `can_reload` by protocol and by the agent's ability; the held
  toast has no clock and is replaced by the next toast.
- app `layout::tab_menu::tests`: an agent tab lists Reload agent after
  Close other tabs with Paseo's tooltip, on the strip and the switcher; a
  node older than 31 lists none; a terminal tab lists none; choosing it
  sends `Act::Reload` for the session and shows "Reloading agent...";
  the answer replaces it with "Reloaded agent", a failure with the node's
  words, else "Failed to reload agent".

## Frames

1280 dark English: the strip's tab menu with Reload agent and its tooltip;
"Reloading agent..." and "Reloaded agent"; a failure. 390: the switcher's
row menu with Reload agent. Light Korean, the same.

## As built

- link: `Act::Reload` → `ReloadSession`, answered `Acted::Session`;
  `ActKind::Reload`.
- view: `session::reload_offer(protocol, cannot_load)` → `ReloadOffer`
  (`Hidden`, `Disconnected`, `Ready`); `CardDetail::cannot_load` (serde
  default false, left out when false); `ToastTone::Plain` (Paseo's default
  variant: words alone, 2.2 s) and `Toasts::hold` (no clock: stays until the
  next toast replaces it). Words: `layout-reload-agent`,
  `-reload-agent-tooltip`, `-reloading-agent`, `-reloaded-agent`,
  `-failed-to-reload-agent`, `-host-disconnected`, en and ko from Paseo.
- kit: `MenuItem::tooltip` / `with_tooltip`; `menu::tip` (the menu state's
  own `Tooltips` and a place per row; the tip drawn after the surface in
  the menu's layer, its holder out of the flow); tooltip `Side::Right` and
  `Configuration::menu_row()` (250 ms; 10 from the row's edge, which is 11
  from the row's place, measured inside its 1-point border);
  `Tooltips::render_in_layer` and `dismiss` (an opening takes down the last
  tip); the card has a `tooltip-card` selector.
- app: `layout/tab_menu/reload.rs`: the row after Close other tabs on an
  agent tab (its card's machine, else the open session's), the press, and
  `tab_menu_reload_acted` in the event chain before the conversation: a
  success puts the session in its card and toasts "Reloaded agent", then
  goes on to the conversation; a failure toasts the node's words, else
  "Failed to reload agent", and stops there.

## Differences

- The failure toast shows the node's words as they come (English in a
  Korean window), as Paseo shows an error's message; one line, cut with an
  ellipsis like every Vessel toast. A turn that ignores its cancel gives
  "the session's agent cannot be reloaded: the running turn did not end
  after its cancel; it goes on: …" with the reason said twice by the node.
- Paseo refetches the timeline after the reload; Vessel's log is live, and
  the `reloaded` record arrives on it (the conversation reads it but does
  not draw it yet, package base 14.54).
- No tooltip on a phone's sheet (Paseo's `enabledOnMobile={false}`).
