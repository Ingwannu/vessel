# The mark

Vessel's logo, given by the owner on 2026-11-13 and given again the
same day with an error corrected: a V drawn as nested loops with a
four-pointed star between its arms, in one colour. The corrected file is
the one kept here; the first is in the history.

| File | What it is |
|---|---|
| `vessel-logo-source.svg` | The owner's corrected file, byte for byte (an Illustrator export on a 2000 x 2000 canvas, the artwork centred and about three quarters of its width). Never edited. |
| `vessel-mark.svg` | The same two shapes in the brand colour, on a square view box cropped to the artwork (`170 170 1660 1660`). What every client draws and every icon is made from. |
| `vessel-mark-mono.svg` | The same with `fill="currentColor"`, for places that take the surrounding text's colour. |

Brand colour: `#1a5868`.

## What differs from the owner's file, and why

- **The margin is cropped.** The view box is a square around the
  artwork with about 5% of air on the wide side, so whoever draws the
  mark decides its padding (a tile, a title bar, a launcher's safe
  zone) and not the canvas.

Nothing else is touched: the paths are the owner's, not redrawn. (The
first file carried two stray placeholder text elements, which the mark
left out; the corrected file has none.)

## Using it

Seen at 256, 64, 32 and 16 px on white, on the app's dark surface and
on the brand colour:

- **On light surfaces**: the mark in the brand colour.
- **On dark surfaces**: the brand colour on near-black is too faint to
  read. Use the mono file in the surface's text colour, or white.
- **App and launcher icons, the favicon**: the mark in white on a tile
  of the brand colour (rounded as the platform rounds its icons). At 32
  px and below the nested lines merge into one stroke; on the tile the
  V and the star still read, on white they turn pale.
- **In the interface** (the title bar, the sidebar's head): the mono
  file at the size of the text beside it.
- Below about 24 px the lines cannot be told apart on any surface. A
  version drawn for small sizes (the V as one solid stroke) would be
  the owner's to give; none is made up here.
- Do not recolour the interface with the brand colour because of the
  mark: the interface's colours are the design system's
  (`docs/design/web-ui/DESIGN.md`).

Icons are generated from `vessel-mark.svg` by each client's build
(Windows `.ico`, the installer, Linux, Android's adaptive icon, the
page's favicon); generated files are not kept beside the source.

## Multiline rules

The owner's words for the brand are "Multiline Lettering, Multiline
Logo": the mark is lines, several in parallel. Anything drawn from it on
screen (the large drawing behind the new session and around an open
session, the agent's avatar, the window frame and controls) is derived
from these measurements, not from the mark's pixels (ADR 0059). The
mark's paths themselves are never edited, recoloured in parts, outlined,
stretched or redrawn.

Measured from `vessel-mark-mono.svg` rendered at one view-box unit per
pixel, by row, column and cross-section scans perpendicular to the arms
and by its enclosed holes:

| | Measured (units) | Rule |
|---|---|---|
| Line | 19.0–19.5 | 0.428 of the pitch |
| Gap | 25.0–25.5 | 0.572 of the pitch: 1.33 lines |
| Pitch | 44.4 | line + gap; the only thing that changes with size |
| Band | 4 lines | an arm is a capsule of 4 nested loops, 8 lines across |
| Turns | centre-line radii 22, 67, 111, 155 | (k + ½) pitches: every turn concentric, the innermost hole one gap |
| Line end | a ring: hole 25, line 19, outside 63 | the k = 0 loop around a point (hole one gap, outside pitch + line); neighbours keep one gap |
| Arms | 60° from the horizontal | the path's steps are 1 : √3 |
| V | 1483 × 1059 | 1.40 : 1 |
| Star | four equal arms, 490 tip to tip; straight flanks at 13.2° for 0.44 of an arm, then concave | a third of the V's width, its centre 23 % down the V |

So, for every derived drawing:

- Lines and gaps stay 19 : 25.4 at every size. The pitch is chosen for
  the place: on screen a hundred and forty-fourth of the content width,
  6 to 8 points (lines 2.6 to 3.4, gaps 3.4 to 4.6), so they stay
  separate lines at scale 1 and never merge into a grey band.
- Turns are concentric on the pitch; nothing turns tighter than half a
  pitch. A line ends by turning back into a loop, or in the mark's ring.
  Where one stroke passes in front of another, the back one breaks for
  one gap, as the mark's front capsules cover the back ones.
- One colour: on dark surfaces the foreground, on light ones the brand
  colour, at a quiet strength behind content (7 % and 10 % on screen).
- No glow, no gradient, no moving current, no decoration that is not a
  line, a loop end or a ring.
- Never under text: a drawing is placed to clear every text box by 12
  points, and steps down (fewer loops, a part left out) where it cannot.
- Circuit form (the window frame, the window controls): the same lines as
  traces with 45° turns; a trace's free ends fan out to twice the pitch
  into rings. At one-pixel sizes the ratio is rounded to whole device
  pixels (line 1, gap 2 at scale 1).

Small sizes: multiline lettering does not read at title-bar size on a
scale-1 screen (the lines merge); three parallel lines need about 32 px
of cap height at scale 1 (16 at scale 2), the mark's loops about 44 to
48. The title bar keeps the plain wordmark and the mark until the owner
decides otherwise.
