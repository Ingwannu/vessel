# The reference's ActivityIndicator profile

## Decision Log

- Purpose: schedules' large loading state, settings' numeric10 refresh and menu pending numeric16 use the same source control.
- Constraint: the existing24-grid Spinner arc/800ms is not equivalent to Paseo LoadingSpinner. Existing API/vector/callers must remain available without a silent global repaint.
- Alternatives: an area-owned indicator, or replacing every legacy spinner.
- Choice: additive `loading::activity_indicator(id, Size, colour)` using the SVG pipeline and one extracted rotation helper. Legacy `slider::spinner` delegates motion with its original vector/800ms; its callers do not change in this commit.
- Why: Paseo `components/ui/loading-spinner.tsx` delegates to ActivityIndicator; its installed RNWeb renderer uses small20, large36 or numeric, a32-unit ring with radius14/stroke4, base alpha0.2, foreground dash80/offset60, and750ms linear rotation.
- Impact: `Size::{Small, Large, Numeric(Points)}` carries the choice. Points validates positive finite dimensions once, preserves fractions, and has typed TEN/SIXTEEN literals for the two existing source consumers. No new dependency or app flag is introduced.

The drawing is original geometry, not a copied asset: the dash pattern leaves20 units of a radius14 circumference visible. A short arc starts at(30,16) and ends at(16+14*cos(20/14),16+14*sin(20/14)). Its32-unit grid is a deliberate ActivityIndicator profile exception; the legacy Spinner24 vector remains unchanged. SVG default butt ends match the source dash ends.

The source wrapper's current consumers expose colour and size, always animating. The shared GPUI AnimationExt pipeline also obeys the existing App reduce_motion flag, rendering repeating motion's initial phase without scheduling frames. The new entry adds no pause/hide API or fixture switch to an installable app.

Tests inspect the actual vector's size/ink, named/numeric/fractional sizes, invalid numeric inputs, linear quarter phases and both750/800ms motion periods. Asset tests rasterize actual registration at10/16/20/36 and verify background alpha51, foreground255 and clear centre/corners at four quarter rotations. Native captures must still prove actual pipeline alpha/geometry, reduced-motion start phase and animation, both themes, widths and languages. The list uses Large36 without a caption; it has no area-local spinner.
