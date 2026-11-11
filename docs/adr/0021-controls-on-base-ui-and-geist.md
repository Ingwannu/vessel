# ADR 0021: Controls on a headless library, Geist for the type, the system's theme

Status: accepted, 2026-11-11 (the coordinator, on milestone 10's plan,
`docs/plans/backend-m10.md`, track W0). Amends ADR 0018 in three places,
named under Consequences.

## Context

The owner looked at milestone 9's page and said: the dropdowns and text
fields are the browser's own, and every control is to be built for this
design; there is no dark mode to be found; it looks poor and the font does
not look applied. A capture agreed: the new-session dialog's native
`<select>`s overflowed the dialog, Settings offered a two-option theme
switch, and the text was Pretendard's Latin, not the face the references
use.

The references the owner named draw every control themselves:

- [agent-orchestrator](https://github.com/OrchestratorInc/agent-orchestrator)
  (Apache-2.0) on Radix primitives with Tailwind classes; its type is
  Geist and Geist Mono, bundled from `@fontsource-variable/*`; its theme
  is system, light or dark, system by default.
- [GOAT](https://github.com/goat-si/goat) and
  [T3 Code](https://github.com/pingdotgg/t3code) (MIT) on
  [Base UI](https://base-ui.com) with Tailwind classes; system theme by
  default.

Vessel's page is React 19 and plain CSS on tokens: no Tailwind, no
CSS-in-JS (ADR 0018), served under a strict content security policy
(ADR 0017: scripts from the page's own origin only, no inline script).

What a dropdown has to get right is mostly not its look: focus moving
into the list and back to the trigger, typeahead, arrow keys and
Home/End, scrolling the highlighted row into view, placement that flips
and shifts at the window's edge, dismissal by Escape and by a click
outside, a portal so a dialog's `overflow` does not clip it, the ARIA
roles that tie a trigger to its list, and not stealing focus on touch.
The same holds for menus, comboboxes, tooltips and modal dialogs (focus
trap, scroll lock, inert background).

## Options

**The controls**

1. Keep the native controls and style them (`appearance: none`, a
   chevron background). A `<select>`'s open list is still the operating
   system's: unstyled, unable to hold a description or a group label the
   design needs, and different on each platform. Menus and comboboxes do
   not exist natively.
2. Write every control by hand. Each of the behaviours above is a small
   project with known failure modes, and the page would own all of them.
3. A headless (unstyled) library for behaviour, with the look in this
   page's own CSS:
   - Radix primitives, agent-orchestrator's choice: some twenty packages
     (or the `radix-ui` umbrella), no combobox.
   - Base UI (`@base-ui/react`, MIT): one package from the same authors as
     Radix and Floating UI, with every part this milestone needs,
     combobox and autocomplete, field, toggle group and tabs included.
     GOAT and T3 Code build on it, so their components are working
     examples of exactly this use.
   - React Aria: complete and careful, but its own composition model
     (render props and its own collections), further from both
     references, and the largest of the three.
4. A styled component library (shadcn/ui, MUI, Mantine). shadcn is
   Tailwind, which the stack excludes; the others bring a look of their
   own to fight.

**The type**

1. Keep Pretendard first. Its Latin is not the references' face, and the
   owner's complaint was about exactly that.
2. Geist and Geist Mono (SIL Open Font License 1.1), as agent-orchestrator
   bundles them, with Pretendard after them for the scripts Geist does
   not have (Hangul first of all).

**The theme**

1. Keep ADR 0018's rule: light on a first visit, whatever the system
   prefers.
2. System by default, as all three references do, with light and dark as
   explicit choices.

## Decision

**Controls: option 3, Base UI**, version 1.9.0, pinned exactly as
`package.json` pins everything.

- The kit lives in `web/src/ui/kit/`, one module per control, each typed
  strictly (variants are unions, matched exhaustively). Screens import
  controls from the kit, never from `@base-ui/react`.
- The look is plain CSS on tokens in `web/src/ui/styles/kit.css`. The kit
  sets no inline colour and no utility classes; Base UI's data attributes
  (`data-open`, `data-highlighted`, `data-checked`, `data-starting-style`)
  are the hooks the CSS uses.
- Values are agent-orchestrator's (control heights 24/28/32, radius 8 for
  controls and 10 for popups, its popup surface, rows and motion); where
  it has no such control, GOAT's; where neither has it, T3 Code's.
  `docs/design/web-ui/DESIGN.md` §6 records the values and the three
  deliberate differences.
- Nothing the browser draws by itself remains: `npm run lint` runs
  `scripts/check-controls.mjs`, which fails on a `<select>`, `<input>` or
  `<textarea>` element, or a `title` attribute (the browser's own
  tooltip), anywhere outside `src/ui/kit/`. The rest of that class is
  CSS: no disclosure triangle on `<details>`, popups' scrollbars and the
  text selection drawn from tokens, focus as the design's ring.

**Type: option 2.** `"Geist Variable"`, then Pretendard, then the system's
faces, then `sans-serif`; `"Geist Mono Variable"`, then the real monospace
faces, then Pretendard, then `monospace`. Bundled from
`@fontsource-variable/geist` and `@fontsource-variable/geist-mono` 5.3.0;
their licences are served beside Pretendard's under `/licenses/`.

**Theme: option 2.** System, light or dark; system by default.

- Every colour token is one `light-dark(<light>, <dark>)` value, resolved
  by the CSS `color-scheme` of the element that uses it. `:root` is
  `color-scheme: light dark`, so the page follows the system, live, with
  no script; `data-theme="light"` or `"dark"` on `<html>` pins one. The
  attribute is set from `localStorage` before React's first render.
- No inline script decides the theme: the CSP forbids one, and CSS alone
  already shows the system's theme on the first paint.

## Why

- **The look was never the hard part of a dropdown; the behaviour is.**
  Base UI is unstyled, so it adds behaviour and accessibility without a
  look to undo, and the page's CSS keeps being the one place the design
  lives. It is one dependency (with Floating UI underneath) instead of a
  family of packages, and the two references that use it show the same
  controls built on it.
- **Base UI over Radix** although agent-orchestrator is the first
  reference: what is taken from agent-orchestrator is values and
  behaviour, not its code (ADR 0018), and Base UI has the combobox and
  autocomplete Radix lacks, which a field with suggestions needs.
- **Geist** is the face the owner saw in the reference. Pretendard stays
  in every stack, ahead of any generic family (the owner's font rule,
  which `scripts/check-fonts.mjs` enforces), for Hangul and the other
  scripts Geist lacks.
- **`light-dark()`** writes each colour once, side by side, so a token
  cannot exist in one theme only and the dark values are not a second
  copy to keep in step. It is supported by the browsers ADR 0017 requires
  (Chrome and Edge 123, Firefox 120). A subtree can take a scheme of its
  own with `color-scheme`, which is how `/kit` shows both themes.

## Consequences

- ADR 0018 said "no component library" and "a first visit is light
  whatever the system prefers". The first now reads: no styled component
  library; behaviour comes from Base UI and the look is ours. The second
  is replaced by the system default above. ADR 0018's tokens
  (`:root` light, `:root[data-theme="dark"]`) are now `light-dark()`
  values on `:root`.
- `web/src/ui/app.css` is split into `web/src/ui/styles/*.css`, one per
  area, imported in a fixed order from `styles/index.css`, so two tracks
  can change different areas at once.
- Code that reads a colour token from script (xterm.js's theme) must read
  a resolved colour, not the custom property's text, since the property
  now holds `light-dark(…)`.
- The bundle grows by Base UI and the Geist files. The Latin Geist file is
  preloaded from `index.html`, so the first paint does not wait on
  discovering it through the stylesheet.
- Base UI writes positions as inline styles. The CSP already allows
  inline styles (`style-src 'self' 'unsafe-inline'`, for xterm.js); it
  allows no inline script, and Base UI needs none.
- A new control is added to the kit, with its states in the `/kit`
  gallery, before a screen uses it.
