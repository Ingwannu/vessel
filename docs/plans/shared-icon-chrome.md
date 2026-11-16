# One source icon-button frame

## Source and ownership

Pinned Paseo 0.15.4 source is commit
`1c00b365c9e0b74c1580eba33d905d693125ae21`. Paths below are relative to
`packages/app/src`:

- `components/ui/pane-content-toolbar.tsx:81–160` and the existing Toolbar plan
  define the small Toolbar profile already in the native line.
- `components/left-sidebar.tsx:282–330,910–917` defines Footer's square hit area,
  accessibility label, glyph hover ink and existing tooltip decoration.
- `styles/density.ts:15,18–28,39–44` and `styles/theme.ts:595–602` define the
  layout pair: frame28/44 and md glyph16/20, with padding4.
- `components/sidebar/sidebar-footer-icon-highlight.tsx:1–25` explains why only
  the decorative pressed layer is round. Its fill is the existing interaction
  highlight; no Footer-specific colour token exists.

## Decision Log

- Purpose: let the existing Toolbar and the separately owned Footer wrapper use
  one frame renderer, without an area button copy or ordinary Button size change.
- Existing constraint: Toolbar already owns typed selected/open/disabled state
  and its canonical vector mapping. Footer has different source geometry and
  feedback. A single arbitrary size/radius/colour API would hide those rules.
- Alternatives: reusing ordinary Button would change source hit areas; copying
  Toolbar into Footer would duplicate the frame and handler policy. Both lose
  the shared-control boundary.
- Choice: `icon_chrome::Profile` is closed over `ToolbarSmall { config, icon }`
  and `SidebarFooter { glyph, sizing }`. `icon_chrome::render` returns the same
  element on which the owning wrapper installs its existing guarded callback
  and Tooltips. The Toolbar wrapper delegates to that same lower frame.
- Why: Toolbar keeps its existing config as the single geometry/state fact.
  Footer adds its own fixed source pair, glyph-only hover ink and a non-occluding
  inner pressed layer while leaving the outer hit area square.
- Consequences: the lower renderer creates no keyboard, menu, tooltip, owner
  registration or operation admission. Footer's actual wrapper and placement are
  still owned by the sidebar package. Its source icon-size adjustment escape
  hatch is deliberately absent until an exact checked caller needs it.

The shared icon token gains `theme::icon::MD` for the source's16-point desktop
glyph. Existing SM14/LG20 and all ordinary Button sizes stay unchanged. The
crate-private `toolbar_frame` also accepts the existing loading indicator so the
separate loading consumer can retain this same frame when composed.

## Verification boundary

The existing three Toolbar tests retain every assertion, with fill assertions
pointing at the extracted implementation. New source fixtures cover Footer's
literal frame/glyph/padding/square outer versus round inner style and actual GPUI
corner clicks for both layouts. They are authored but unexecuted in this leaf.
Local format, native size, tokens and diff checks pass. VM compilation/runtime
and source review remain required before a consumer handoff. No native Footer
capture, hover/pressed pixel parity, tooltip, focus or accessibility approval is
claimed by these model and hitbox fixtures.

The first exact receiving strict run,
`wp-layout-core-union-20261116T011730Z-4bce2bd5`, reached Kit library/test checking
and failed one documentation lint: `ActivityIndicator` needed backticks. The
correction changes that comment only; no tests ran and no renderer, handler or
assertion is changed. A successful corrected check is still owed.

## Receiving composition with Tooltip and loading

The standalone extraction completed exact Kit strict run
`wp-layout-core-union-20261116T012034Z-c2eba6ae` and same-head Kit all-target
runtime `wp-layout-core-union-20261116T012159Z-c2eba6ae`: fonts/cargo0 and245/0/0.
Both new Footer cases and all existing Toolbar cases passed explicitly. This
does not execute the later loading composition or prove native pressed pixels.

The next receiving source includes the reviewed Tooltip/loading followups. Both
SVG and existing ActivityIndicator construction still enter the same private
`ToolbarButton::with_content`, which now delegates to `icon_chrome::toolbar_frame`.
No loading, disabled guard or tooltip method body changes. Only the old frame
body moves; config keeps its new loading helper and widens the extracted frame's
accessors inside the Kit crate. The public Footer profile calls the same renderer
path, without taking over the sidebar wrapper or adding arbitrary style knobs.
This changed receiving source still owes its own strict/runtime checks.
