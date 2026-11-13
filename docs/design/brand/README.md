# The mark

Vessel's logo, given by the owner on 2026-11-13: a V drawn as nested
loops with a four-pointed star between its arms, in one colour.

| File | What it is |
|---|---|
| `vessel-logo-source.svg` | The owner's file, byte for byte (an Illustrator export on a 2000 x 2000 canvas). Never edited. |
| `vessel-mark.svg` | The same two shapes in the brand colour, on a square view box cropped to the artwork (`649 650 700 700`). What every client draws and every icon is made from. |
| `vessel-mark-mono.svg` | The same with `fill="currentColor"`, for places that take the surrounding text's colour. |

Brand colour: `#1a5868`.

## What differs from the owner's file, and why

- **Two stray text elements are left out.** The export carried two
  "Lorem ipsum" placeholders at 12 px inside the artwork (in a font
  that is not embedded). They draw as specks. They are not part of the
  mark.
- **The margin is cropped.** The artwork fills about a third of the
  2000 x 2000 canvas; an icon made from the whole canvas would be a
  third of its size. The view box is a square around the artwork with
  about 5% of air on the wide side.

Nothing else is touched: the paths are the owner's, not redrawn.

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
