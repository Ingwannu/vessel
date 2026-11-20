# Editor artwork: sources and rights

Retrieved 2026-11-20 for Open in editor (TRACKER 467, ADR 0123). These
trademarks belong to their respective owners; Vessel displays them to
identify the editor a path opens in, without endorsement. Vendor artwork is
excluded from Vessel's licence.

Paseo draws a PNG app icon for eight targets (Cursor, VS Code, Zed,
Antigravity, Android Studio, Xcode, WebStorm, Finder) and a symbol for the
rest. Vessel never copies or traces Paseo's PNGs. Each mark below comes
from an openly licensed collection or from its vendor, is drawn as a vector
at the display's own pixels (`kit::agent_mark`'s raster path, sharp at any
scale factor), and is sized as Paseo sizes its icons (16 points, edge to
edge in the slot). Antigravity and Finder have neither an openly licensed
vector nor a vendor download reachable here, so they keep Paseo's symbols
(`SquareTerminal`, `Folder`).

## Simple Icons marks (Cursor, Zed, Android Studio, Xcode, WebStorm)

- Collection: Simple Icons 16.38.0 (`simple-icons` on npm).
- Download: `https://cdn.jsdelivr.net/npm/simple-icons@16.38.0/icons/<slug>.svg`.
- Licence of the SVG files: CC0-1.0. Simple Icons' disclaimer: the icons'
  trademarks remain their owners'; using them is subject to the owners'
  guidelines.
- Kept: the single path's data, unchanged, in a 24 by 24 view box. Each
  editor has one file, the same on light and dark surfaces, as Paseo ships
  one icon per editor (the coordinator's ruling on the 467 frames: a mark
  that changed with the scheme read as two products). Where Paseo draws an
  app tile (Zed, Cursor, Android Studio) the path sits on a rounded tile at
  0.68 of its side (Android Studio 0.7); the tile is drawn here, not taken
  from Paseo's PNGs. The kit embeds the files by path
  (`kit/src/agent_mark/editors.rs`).

| Mark | File | Slug | Original SHA-256 | Drawn as | Owner's page |
|---|---|---|---|---|---|
| Cursor | `cursor.svg` | `cursor` | `71572a9be192cc069ec22b48d785e61d754eeed7edb7d082d615f6d5620f70ec` | `#EDEDED` cube on a `#18181B` tile | https://cursor.com/brand |
| Zed | `zed.svg` | `zedindustries` | `90848d3e8d74d4fb3efc7137b8a5181139082e212566b025843379a7cb0550a0` | `#FFFFFF` mark on a `#18181B` tile | https://github.com/zed-industries/zed |
| Android Studio | `android-studio.svg` | `androidstudio` | `465dda2b9f94726978d67e98922ecce2425e0ea3d839949c6657c62f3265024d` | Android green `#3DDC84` on Android navy `#073042` | https://developer.android.com/studio |
| Xcode | `xcode.svg` | `xcode` | `20f03ce67d3bf01bbc95aef5cb6b6af1ed833bcf83337d499ab07caf7085553f` | `#147EFB` glyph | https://developer.apple.com/develop |
| WebStorm | `webstorm.svg` | `webstorm` | `11827451725f77c24128844a001cf557726f42f60fd94e4aa7884242593ae6b3` | `#000000` square with its letters white | https://www.jetbrains.com/company/brand |

Differences from Paseo's icons: Paseo's Android Studio icon is a white
tile (green on white is under 3:1, so the tile is Android's navy); its
WebStorm and Xcode icons are the products' coloured app icons, which no
openly licensed vector carries.

Use: each mark only names the program a path opens in, beside its name
("Open in Zed"), never as Vessel's own brand and without implying
endorsement. The owners' own brand pages (the table's last column, from
Simple Icons' metadata) govern the trademarks; the CC0 licence covers the
SVG files only.

## Visual Studio Code

- Mark: the Visual Studio Code "stable" icon, `vscode.svg` in this folder,
  Microsoft's file byte for byte (the kit embeds it with `include_str!`).
- Source page: https://code.visualstudio.com/brand
- Download: https://code.visualstudio.com/assets/branding/visual-studio-code-icons.zip
  (archive SHA-256 `04c5292a117bbc619f1ce574cc59b6c5e2e26374d6e64b92619e848612f2d9cb`,
  version 2021-06-21).
- Member: `visual-studio-code-icons/vscode.svg`, SHA-256
  `74ad401c6487a0dc5af1f6b2935556ab2d3553ae45effc7e1697b4d117d046b6`.
- Terms: https://code.visualstudio.com/brand. "Visual Studio Code, VS Code,
  and the Visual Studio Code icon are trademarks of Microsoft Corporation.
  All rights reserved."
- Terms summary: action labels take the form "<action> in VS Code" ("Open
  in VS Code"), with the icon on the action; the icon is not modified,
  recoloured, stretched or redrawn, gets no background (so it is not put
  on a tile as Paseo's PNG is), is not paired with the name as a lock-up, and keeps clear space of a quarter of its "fish" (the menu row's
  padding and gap give it). It is never used for Vessel's own brand.
