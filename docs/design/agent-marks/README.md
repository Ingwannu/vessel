# Agent artwork: sources and rights

Retrieved 2026-11-13; Gemini replaced 2026-11-14. These trademarks belong
to their respective owners; Vessel displays them to identify the agent,
without endorsement. Vendor artwork is excluded from Vessel's licence.
Only drawing path data is kept; no original archive or SVG file is
redistributed. Gemini's spark comes from a Google repository published
under Apache-2.0 (text beside this file), with trademark rights reserved.

## Claude Code

- Mark: Anthropic Claude Spark - Clay.svg.
- Source page: https://www.anthropic.com/news
- Download: https://www.anthropic.com/press-kit
- Original member SHA-256: `6d53db4be375e899c937c26cf16684a80d6e869b1928d72b37748bef2560e219`.
- Terms: https://www.anthropic.com/legal/trademark-guidelines
- Terms summary: Anthropic permits trademarks only with prior approval and prohibits alterations to colour, proportions or other features. Keep reasonable clear space and do not suggest endorsement.

## Codex

- Mark: OpenAI Blossom.
- Source page: https://github.com/openai/openai-agents-python/blob/main/docs/assets/logo.svg
- Download: https://raw.githubusercontent.com/openai/openai-agents-python/main/docs/assets/logo.svg
- Original member SHA-256: `b94ea61d860fae6f82f43571f36f17111fcf5d348e8e9cc22ae4b441c7560011`.
- Terms: https://openai.com/brand/
- Terms summary: OpenAI permits identification directly related to its services subject to its brand rules, subordinate to the app brand and without implied sponsorship. Keep approved black or white colours, geometry and clear space; permission is non-transferable and revocable.

## OpenCode light

- Mark: OpenCode logo light.
- Source page: https://opencode.ai/brand
- Download: https://opencode.ai/brand/files/opencode-logo-light.svg
- Original member SHA-256: `0192e1384d474d2d2d4fef81c02bf6aa0d75d2d47b28a529e86c01a1664c842a`.
- Terms: https://opencode.ai/brand
- Terms summary: The official brand page supplies light and dark artwork for product identification while reserving trademark rights. Preserve its two-tone fills and 4:5 proportions, minimum 16 by 20 size and two-stroke clear space.

## OpenCode dark

- Mark: OpenCode logo dark.
- Source page: https://opencode.ai/brand
- Download: https://opencode.ai/brand/files/opencode-logo-dark.svg
- Original member SHA-256: `c6e736136fe3c57f98ea73228907c6ac7cc1ced6216ddef1feb637d84737a2b5`.
- Terms: https://opencode.ai/brand
- Terms summary: The official brand page supplies light and dark artwork for product identification while reserving trademark rights. Preserve its two-tone fills and 4:5 proportions, minimum 16 by 20 size and two-stroke clear space.

## GitHub Copilot / GitHub forge

- Mark: official current bare Invertocat; the agent name explicitly says GitHub Copilot.
- Source page: https://brand.github.com/foundations/logo
- Download: https://brand.github.com/GitHub_Logos.zip
- Black member: `GitHub_Invertocat_Black_Clearspace.svg`; SHA-256: `b5e2f4b3f953f075de7016faa952d6ce3ec7492aa7b32408505ec25857962d89`.
- White member: `GitHub_Invertocat_White_Clearspace.svg`; SHA-256: `ed102562e49ca3ea6e7d79ea10d54360dbecc5beb33de6a60fa578b6c0fac803`.
- Terms: https://brand.github.com/foundations/logo and https://brand.github.com/brand-identity/copilot
- Terms summary: GitHub permits its current mark to identify integration, subordinate to the app brand and without implied affiliation, and states that prior written permission is required for artwork use. The standalone Invertocat is used where the GitHub brand is explicit in the agent name; the retired Copilot head is never used.

## Gemini CLI

- Mark: Google's Gemini spark, the "Standard product icon" export (vector, Google's own gradient `#346bf1` → `#3186ff` → `#4fa0ff`).
- Source page: https://github.com/google/adk-docs/blob/main/docs/integrations/assets/gemini-spark.svg (Google's Agent Development Kit documentation, in the `google` organisation).
- Download, pinned: https://raw.githubusercontent.com/google/adk-docs/94636546e5377a2cd74d90ae81c5b7874739dda6/docs/integrations/assets/gemini-spark.svg (commit "Update integrations assets (#1255)", 2026-02-06; git blob `62f07e26`).
- Original SHA-256: `068b3fb370ae6473fea495e2ea05de028e7372d4bcef2fe3a9d4bc50f15c2d5e`.
- Kept: the path and the gradient (stops, coordinates, transform) unchanged; dropped: the Illustrator comment, the element ids and the empty bounding rectangle. The 192-unit view box and its 8-unit inset are Google's.
- Terms: https://github.com/google/adk-docs/blob/main/LICENSE (Apache-2.0, no NOTICE file) and https://about.google/brand-resource-center/brand-elements/
- Terms summary: the repository distributes its files under Apache-2.0, allowing redistribution with the licence, but does not grant trademark rights. Google's product-icon guidance requires an approval request; keep the icon unchanged, attribute Google, and do not imply endorsement.

Until 2026-11-14 the app showed the Gemini CLI application icon (a dark
plate with a prompt, a 1645-pixel PNG from google-gemini/gemini-cli). The
owner asked for the Gemini mark itself, the spark Paseo and GOAT show; the
spark above replaces it, from Google's own repository, not from Paseo or
an icon set. The PNG and its use are gone from the app.


## For the owner

Anthropic requires prior approval: "You may only use our trademarks as specifically permitted by us and only in materials we approve beforehand."

GitHub: "Do not use GitHub trademarks, logos, or artwork without GitHub’s prior written permission."

The relevant terms addresses are listed with each mark. Shipping implementation is the
owner's decision; this record does not assert that these approvals exist.

Google product-icon approval is also for the owner: the current guidance
at https://about.google/brand-resource-center/brand-elements/ says to
“create a Partner Marketing Hub account to find assets and request
permission through our approval form.” This is a separate condition from
the source repository's Apache-2.0 redistribution licence.

## Drawing boxes and optical alignment

[Measured ink and display boxes](ink.md) records the reference's viewBoxes,
path bounds, official viewport sizes, per-brand display boxes and all
necessary size/clear-space differences. Paths, fills and proportions stay
unchanged. The comparison preview projects each full original at the same
ink scale, including added clear space for Claude and OpenCode; the
OpenCode original is always at its 16 × 20 minimum, never shrunk to fit a
requested compact slot.

Provider rows reserve a common32-wide column. Ordinary headers and model
pills reserve each brand's actual image viewport width, keeping the
reference's label gap. Their existing vertical padding holds the quiet
zone; row pitch is unchanged. Disabled artwork retains its approved
colours and opacity while words/trailing controls receive55% opacity.
Generic/glyph rows keep their existing whole-row disabled treatment.

The current Invertocat source is shared by the agent and typed forge
uses; no second asset is embedded. Gemini's spark is the same in both
themes. Owner permission requirements above remain separate from its
software licence.

## Drawing at the display's pixels (2026-11-14)

Every mark is rasterised from its vector at the device pixels it lands on
(`kit/src/agent_mark/raster.rs`, whose Decision Log says why GPUI's
`img()` broke the marks at 1.25 and 1.5). The proof is
`kit/examples/mark_sharpness.rs`: each mark at 14, 16 and 20 in cells off
the pixel grid, captured at `Xft.dpi` 96, 120, 144 and 192, cropped and
compared with librsvg (`rsvg-convert`) at the same device box. Measured
2026-11-14 over 120 crops (5 marks × 3 sizes × 4 scales × 2 themes): mean
absolute error 0.0024 (largest 0.0081) after, against 0.062 before; the
largest single-pixel difference after is 0.17 on an anti-aliased edge,
where two rasterisers round coverage differently.
