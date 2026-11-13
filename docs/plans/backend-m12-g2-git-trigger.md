# Shared git-header menu trigger profile

[Decision Log]
- Purpose: G1's git overflow and G2's schedule row share one vertical-dots
  trigger, with the geometry each Paseo surface actually supplies.
- Existing assets/constraint: the accepted-in-review schedule trigger is
  glyph 14/radius 4/padding 4. Reusing that geometry for git would resize
  Paseo's header; blindly sharing its hover callback would change git paint.
- Alternatives: another screen-local trigger or changing all callers.
- Choice: typed `TriggerProfile::{Schedule, GitHeader}` at `trigger_with`;
  the existing `trigger` delegates to the enabled Schedule profile.
- Why: one shared entry owns the registered glyph and reference geometry,
  while legacy callers remain unchanged.
- Cost: GPUI has no Pressable hitSlop, so an outset interactive wrapper and
  negative margins preserve the visual layout footprint. Captured bounds
  belong to the inner visual frame, not the expanded clickable region.

Reference read whole: Paseo `git/actions-split-button.tsx`, with
`components/ui/menu/menu-root.tsx` and `control-geometry.ts`. Git's overflow
has MoreVertical 16, radius 6, square HEADER_CONTROL_HEIGHT 26 (pointer) / 32
(touch), and hitSlop 8 on each side: actual hit region 42 / 48. Its styles
are static: no schedule hover/pressed/open ink or fill override. The node's
busy state does not invent a trigger paint. Disabled callers attach no
handler or pointer cursor; the reference has no disabled opacity style.

The caller's separate marginRight -8 belongs on an enclosing wrapper so
it does not overwrite the hit-region compensation margins. `open_end`
receives the recorded visual bounds; it shares the same menu renderer and
4-point offset policy as other placement seams. The accepted `menu_right`
seam on the newer lead base must survive integration with this older base.

No app caller, screen fixture or route changes here. Exact geometry tests
cover both densities and retain the schedule values. G1 supplies the first
use ready/open/disabled captures in both themes and sizes; source/value
tests alone do not certify those interactions.
