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
