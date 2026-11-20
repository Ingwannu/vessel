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
- Kept: the single path's data, unchanged, in a 24 by 24 view box.
  Simple Icons ships no colour in the file; Vessel fills each path with its
  Simple Icons brand colour where that colour has 3:1 contrast or more
  against every menu surface of the scheme (WCAG 2.1, measured over the
  kit's tints), and with black (light) or white (dark) otherwise, as Simple
  Icons' monochrome use.

| Mark | Slug | Original SHA-256 | Brand colour | Light | Dark | Owner's page |
|---|---|---|---|---|---|---|
| Cursor | `cursor` | `71572a9be192cc069ec22b48d785e61d754eeed7edb7d082d615f6d5620f70ec` | `#000000` | `#000000` | `#FFFFFF` | https://cursor.com/brand |
| Zed | `zedindustries` | `90848d3e8d74d4fb3efc7137b8a5181139082e212566b025843379a7cb0550a0` | `#084CCF` | `#084CCF` | `#FFFFFF` (brand 1.77:1) | https://github.com/zed-industries/zed |
| Android Studio | `androidstudio` | `465dda2b9f94726978d67e98922ecce2425e0ea3d839949c6657c62f3265024d` | `#3DDC84` | `#000000` (brand 1.62:1) | `#3DDC84` | https://developer.android.com/studio |
| Xcode | `xcode` | `20f03ce67d3bf01bbc95aef5cb6b6af1ed833bcf83337d499ab07caf7085553f` | `#147EFB` | `#147EFB` | `#147EFB` | https://developer.apple.com/develop |
| WebStorm | `webstorm` | `11827451725f77c24128844a001cf557726f42f60fd94e4aa7884242593ae6b3` | `#000000` | `#000000` | `#FFFFFF` | https://www.jetbrains.com/company/brand |

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
  recoloured, stretched or redrawn, is not paired with the name as a
  lock-up, and keeps clear space of a quarter of its "fish" (the menu row's
  padding and gap give it). It is never used for Vessel's own brand.
