# Viewport toolbar chrome, its glyphs and the window-controls corner

Kit leaf for the image lightbox's toolbar (the coordinator returned W6 B1 for
the toolbar only: boxed `+ − □ ×` squares drawn over the window controls).
Additive on package base 14.6; no existing profile, glyph, path or drawing changes.

[Decision Log]
- Purpose: Paseo's zoomable-viewport toolbar (`zoomable-viewport/toolbar.tsx`
  28-136, used by `attachment-lightbox.tsx` 59-108) is the shared small icon
  chrome (`iconButtonChromeStyle` size small: frame 20, or 32 compact; glyph 14,
  or 18 compact; radius md; `interactionHighlight` when hovered or pressed;
  opacity 0.5 when disabled) on a `surface2` plate, with `foregroundMuted`
  Lucide icons that turn `foreground` on hover. The cluster is shown on hover,
  focus or touch (web) and always on the native app; hidden buttons have
  opacity 0 and no pointer events. Icons: `ZoomIn`, `ZoomOut`, `Scan`, then the
  lightbox's `X`.
- Existing assets: `icon_chrome` (W12's Toolbar and Footer profiles),
  `ToolbarButton` (the guarded handler wrapper), the single glyph inventory with
  `X` already present, and `chrome` (the app's own title bar with its
  minimise/maximise/close cells, 36 wide on Linux and 46 on Windows).
- Alternatives considered: a lightbox-local button (a second button style; the
  return names exactly that); a rest-fill/ink option on `ToolbarConfig` (every
  toolbar caller would carry knobs only one surface uses); aliasing `Plus`,
  `Minus`, `Maximize` (the returned look: they read as window controls).
- Choice:
  - `icon_chrome::Profile::Viewport { config, glyph, presence }` and
    `ToolbarButton::viewport`: the Toolbar's geometry, highlight and disabled
    opacity, rest fill `surface2`, muted adaptive glyph turning foreground on
    hover, and `Presence::{Shown, Hidden}` supplied by the owner (Hidden: opacity
    0, no hover style, and the wrapper refuses a handler as for Disabled).
  - Copies Paseo's pinned style order (icon-button-chrome.ts:79-82: the
    caller's hidden style, then highlighted, then disabled last): a hidden,
    disabled action shows at 0.5 and still takes no pointer. Only an enabled,
    shown action gets the hover/press highlight and the foreground ink, as
    Paseo's disabled `Pressable` has no hovered or pressed state (app review
    finding P3; coordinator ruling, 2026-11-16).
  - Three glyphs appended after `Import`: `ZoomIn`, `ZoomOut`, `Scan`, from
    canonical Lucide 1.50.0 XML (`icons/zoom-in.svg`, `zoom-out.svg`,
    `scan.svg`) with the inherited 1.5 stroke (`toolbar.tsx` sets none, so
    AppShell's `LucideProvider` 1.5 applies). The existing Lucide notice covers
    them; it is unchanged.
  - `chrome::bar_height()` and `chrome::controls_corner()`: the app bar's height
    (zero with the system's bar) and the region its window controls take at the
    top right (Paseo's top-right `WindowChromeObstruction`; empty on macOS and with
    the system's bar). A full-window overlay leaves that corner uncovered.
- Why: one more closed profile keeps one hitbox, one guard and one highlight for
  every small icon action; the glyph roles stay distinct from window and panel
  glyphs by drawing, and tests hold that.
- Impact: the owner decides presence and keeps callbacks, stop-propagation and
  labels. Paseo's 4/6 px `hitSlop` is not reproduced: the hit area is the frame.
  With a spacing-1 gap two slops overlap; GPUI delivers a press to every Normal
  hitbox under the pointer, and an occluding margin would block the owner's
  viewport hover that drives `Presence` (see the lightbox plan in wp-b1.md).
  Paseo's web build has no slop either (react-native-web 0.21.2 `Pressable`
  does not implement `hitSlop`; only the legacy Touchable does), so only the
  native app's compact 6 px is a real difference. `panel_role_tests` helpers gain `pub(super)` (the same three-word change
  as the S2 file-change leaf, so the two merge cleanly).

## Inventory order

The receiving inventory on 14.6 ends `… X, Import`. These three follow it. The
S2 F4 roles (sidebar branch) and SquarePlus/SquareMinus/SquareDot (file-change
leaf) are unpublished appends on other branches; whichever lands later appends
after the earlier ones without reordering published rows. The coordinator's
reservation for ArrowRight/Focus/Move/ArrowDownToLine stays after them.

## Checks

Tests added: `assets::viewport_role_tests` (one asset each, viewBox, stroke count
at 1.5, lens/plus/corner landmarks that keep the three apart and unfilled, a clear
rim at 14 and 18 points over fractional scales), `icon_chrome` viewport style
values in both themes and both sizings, a hidden/shown click probe on
`ToolbarButton::viewport`, and the controls corner. Nothing here was compiled
locally.

Results (2026-11-16, build VM):
- Kit tests, run viewport-chrome-narrow-20261116T063522Z-86ebdcce:
  `test --locked -p vessel-app-kit --lib -- assets:: icon_chrome:: toolbar_button:: chrome::`
  30 passed, 0 failed (all six new cases explicit).
- First strict run viewport-chrome-20261116T063521Z-86ebdcce stopped at native-fmt
  (two layout differences); the format commit followed. Strict run
  viewport-chrome-20261116T064035Z-26584ef6: native-fmt, native-quality and
  native-clippy (`--all-targets -D warnings`, whole native workspace) all 0.
- The lightbox consumer on the receiving tree (W6's input packet on 14.2 plus this
  leaf) passed its own strict, its lightbox tests 5/0 and an app export; its
  captures went to the coordinator. No whole native gate ran for this leaf.

Second round (app review RETURN P3 and the coordinator's style-order ruling):
797524284 + format 253623c4f. Kit tests viewport-chrome-narrow-20261116T070320Z-79752428:
32 passed, 0 failed (new: the responds truth table and the painted scene staying
the same under hover and press on a disabled action). Strict
viewport-chrome-20261116T070453Z-253623c4: fmt, quality, whole native clippy 0
(its predecessor 070301Z stopped at native-fmt on one test layout).
