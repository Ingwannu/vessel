# Shared screen header, after Paseo's

[Decision Log]
- Purpose: schedules, board and settings reuse one header renderer and the
  reference's title weights, rather than each area inventing a heading.
- Constraint: app chrome, window drag, sidebar state and route policy belong
  to the desktop lead. Settings supplies its own wired back control.
- Alternatives: separate area frames, or a kit coupled to Shell/navigation.
- Choice: `screen_header::frame(Content {title,leading,actions}, Density,
  tokens)` is the sole renderer. A title/actions wrapper and compact menu
  callback wrapper delegate to it. Content's actions/leading are optional
  AnyElement slots; convenience actions are Vec<AnyElement>.
- Why: G3/E2 back callbacks and the lead's sidebar callback remain at the app
  edge. S2 and schedules share geometry without sharing navigation policy.
- Cost: no subtitle API without a consumer; native drag/window controls are
  integrated by the lead. A physical touchscreen/screen reader is unverified.

References read whole: `components/headers/{menu-header,screen-header,
screen-title,compact-header,sidebar-menu-toggle,header-toggle-button}.tsx`,
`components/ui/icon-button-chrome.ts`, `constants/layout.ts`, and control
geometry/sizing. Desktop frame: 36 high, horizontal padding 12, leading/actions gap 8,
title 14/300, surface0 and bottom border. Compact: top 8, row 56, horizontal padding 12,
gap 8, one-line title 16/500, transparent frame and navigation/action islands.
The kit's existing face/fallback and text metrics remain shared. The
reference sets no bespoke header line-height; title size/weight are explicit.

The compact menu button is 44, round, muted, with interactionHighlight on
hover/pressed. Its original registered MenuCompact vector paints 16×12:
bars 16/16/8 by 1.5, top 0/5.25/10.5, radius 0.75. It has its own explicit SVG
ink. The two new navigation glyphs are a separate glyph-only commit; legacy
Menu and PanelRight are unchanged. No icon characters or reference assets.

The frame never changes sidebar state or starts window dragging. The compact
wrapper invokes only its supplied Window/App callback. Existing native
window-menu duplication is resolved by the lead during caller integration,
not by an area editing chrome. This commit has no app consumer changes.

Literal geometry/palette tests cover desktop/compact and both themes, plus
the actual SVG ink. Native first-use captures are required before visual
approval; unit checks alone do not certify a screen.


## Full-width desktop border correction

Paseo ScreenHeader's WindowChromeSafeArea puts horizontal padding inside
its bordered row. The native frame now returns that full-width 36-point
row directly, with padding 12 inside it, rather than bordering a child of
an outer padded wrapper. Compact keeps its separate top-padding wrapper.
A regression inspects the actual returned Div's full width, total height,
bottom border and padding; native edge-pixel captures must confirm both
ends.

## Integration with the current shared breakpoint

The canonical touch target now lives in `kit::breakpoint::TOUCH_TARGET` on
the desktop base. The menu control and its regression use that path rather
than the removed popup re-export. This changes no numeric value, geometry,
callback, appearance or motion profile; the target remains 44 points.
