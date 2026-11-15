## Reference ink, measured after the family review

Rechecked 2026-11-13. Measurements use the SVG viewBox and painted path
bounds through librsvg's vector geometry API, with SVG width/height
props overridden as Paseo's `SvgXml` does. Reference values below use that geometric result, not screenshot
estimates. Official C/L/M/H/V paths are also checked by solving cubic
derivative roots: an approximate renderer rectangle can omit an interior
curve extremum, as the Invertocat foot demonstrated.
No Paseo path is an artwork source or a committed asset. The reference
files are `components/icons/{claude,codex,copilot,opencode}-icon.tsx` and
`assets/acp-provider-icons/gemini.svg` under Paseo's app source.

| Reference | ViewBox | Painted bounds (x, y, width, height) | Ink at 14 | Ink at 16 |
| --- | --- | --- | --- | --- |
| Claude | 0 0 24 24 | 0, 0, 24, 24 | 14 × 14 | 16 × 16 |
| Codex | 0 0 24 24 | 1.027, 1, 21.941, 22 | 12.799 × 12.833 | 14.628 × 14.667 |
| Copilot retired head | 0 0 512 416 | 0, 0, 512.004, 416 | 14 × 11.375 | 16 × 13 |
| OpenCode | 96 64 288 384 | 128, 96, 256, 320 | 9.333 × 11.667 | 10.667 × 13.333 |
| Gemini | 0 0 24 24 | 0, 0, 24, 24 | 14 × 14 | 16 × 16 |

The former optical balancing made the family too small. Display boxes
now target the reference's ink while retaining the full vendor viewport
and clear space. No path or proportion is altered. Exact matching in both
axes is impossible where the official mark differs from the reference:
OpenAI's official Blossom matches its height (14.67 at 16), with width
14.80 rather than 14.63; the current Invertocat matches the retired head's
16-wide box, with its own 15.498-high proportions rather than 13. These
are deliberate differences, not stretched substitutes.

| Official artwork | Image/clear-space box at requested 12 / 14 / 16 / 20 | Ink at 16 |
| --- | --- | --- |
| Claude Spark, 93.6 ink inside its 141 clear-space viewport | 18.077 / 21.090 / 24.103 / 30.128 square | 16 × 16 |
| OpenAI Blossom, 484.137 × 479.816 in 721 square | 16.530 / 19.285 / 22.040 / 27.550 square | 14.800 × 14.667 |
| GitHub Invertocat, 96 × 92.989 in 128 square | 16 / 18.667 / 21.333 / 26.667 square | 16 × 15.498 |
| Gemini spark, 176 ink inside Google's 192 square | 13.091 / 15.273 / 17.455 / 21.818 square | 16 × 16 |
| OpenCode | 32 × 36 at every requested size that has room | 16 × 20 |

A provider row still reserves a common 32-wide layout column and keeps
its existing 37-point pointer / 44-point touch pitch. Ordinary artwork's
clear-space viewport occupies part of the row's existing vertical inset;
its ink has the requested size. Header and pill widths reserve the full
image viewport rather than consuming their label gap. The header's
vertical slot remains 16 and its existing padding holds the quiet zone;
its title moves by the extra width in the table. The model pill remains
28 pointer / 44 touch with its original 4-point gap. A compact 28-point
spot cannot hold a requested 20-point Claude Spark plus quiet space, so
that combination uses Generic; the actual touch model pill uses Lockup
and holds it. The first/last picker OpenCode image is centred to the
nearest whole pixel (at most 0.5 point from the 37-high row's centre),
which preserves its mandated integer edges. Name baselines and row pitch
stay identical for every brand.

Generic in a provider row now draws the exact same Bot at 16 as existing
Glyph rows draw Folder and Server at 16. Their different path bounds are
not normalised or redrawn. At 16 their ink bounds are Bot 11 × 10.667,
Folder 13 × 10.333 and Server 13 × 11.667; all use the same 16-point glyph
API. Inline Generic retains its requested size.

### Small-form and light-background rechecks

OpenCode's current own brand page (`https://opencode.ai/brand`, rechecked
2026-11-13) offers logo, padded square and avatar variants, but states a
16 × 20 minimum and 4 × 5 whole-pixel steps for the logo. No smaller UI
or favicon exception is stated. The square adds padding, not a smaller
permitted ink form. Keep the minimum and documented Generic fallbacks.

Google's public Brand Resource Center and product list, its Partner
Marketing Hub product-icon guidance, and the Gemini CLI repository's
asset inventory were rechecked. The public guidance routes product-icon
assets/permission to the Hub. The repository provides the unchanged CLI
plate and terminal-text snapshots, with no located bare or light variant
of that product icon. No public, appropriately sourced bare alternative
was located in this check; this does not assert none exists in Google's
private brand resources. (Superseded 2026-11-14: the owner asked for the
Gemini spark rather than the CLI plate; it is now Google's own file as the
Gemini app serves it, see the source list. Its outline and inset are the
same as the blue spark's before it, so the ink matches the reference's 16
at 16; its four colours are the same in both themes.)
