# Agent artwork: sources and rights

Retrieved 2026-11-13. These trademarks belong to their respective owners;
Vessel displays them to identify the agent, without endorsement. Vendor
artwork is excluded from Vessel's licence. Only drawing path data is kept
for restricted vectors; no original archive or SVG is redistributed. The
unchanged official Gemini CLI PNG is redistributed under its repository's
Apache-2.0 licence (text beside this file), with trademark rights reserved.

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

## For the owner

Anthropic requires prior approval: "You may only use our trademarks as specifically permitted by us and only in materials we approve beforehand."

GitHub: "Do not use GitHub trademarks, logos, or artwork without GitHub’s prior written permission."

The relevant terms addresses are above. Shipping implementation is the
owner's decision; this record does not assert that either approval exists.
## Gemini CLI

- Mark: the vendor's official Gemini CLI application icon (PNG, not traced).
- Source page: https://github.com/google-gemini/gemini-cli/tree/main/packages/vscode-ide-companion/assets
- Download: https://raw.githubusercontent.com/google-gemini/gemini-cli/main/packages/vscode-ide-companion/assets/icon.png
- Original SHA-256: `351e9f5b1bf863d738cd7be4ed040a625a1419450ae7fc490143e4042b7c2438`.
- Terms: https://github.com/google-gemini/gemini-cli/blob/main/LICENSE and https://about.google/brand-resource-center/guidance/
- Terms summary: the repository distributes its files under Apache-2.0, allowing redistribution with the licence and notices, but does not grant trademark rights. Google's product-icon guidance requires an approval request; preserve the icon unchanged, attribute Google, and do not imply endorsement.

The icon was inspected as a 1645 × 1645 PNG. Using the exact raster avoids
tracing it into an unofficial vector; the existing image renderer already
supports it. Its pixels and SHA-256 are unchanged in the app. No vector
asset was fabricated from the empty SVG in Google's older JS sample.


## Drawing boxes and optical alignment

The Claude Spark retains every path coordinate and its Clay fill; its
94-square bounds are centred in a 141-square clear-space box. At a nominal
16 this yields about 10.7 points of ink, matching the original OpenAI
Blossom's 484-wide path in its 721-square viewport (about 10.7 points of
ink), and the generic bot's 15-wide path on its 24-unit grid (10 points).
Gemini CLI's only located official icon includes its own gradient border
and dark plate; no plate is removed and no colour is changed. This is a
necessary difference, subject to the full-set capture review. OpenCode's
minimum ink size is larger than its neighbours and must remain so.

The plated Gemini icon is drawn inside a smaller centred image box for
optical balance: 8, 10, 12 and 14 points inside nominal 12, 14, 16 and
20 slots respectively. These are display boxes only; the original PNG
pixels are unchanged.

The Invertocat's original 128-square clear-space viewport is preserved.
It is drawn in image boxes 10.5, 12.25, 14 and 17.5 points inside nominal
12, 14, 16 and 20 slots, to balance its ink with Claude and OpenAI.
`GITHUB` is the one artwork source; `COPILOT` aliases it, and the forge
control uses the same data through a separate typed API.

Google product-icon approval is also for the owner: the current guidance
at https://about.google/brand-resource-center/brand-elements/ says to
“create a Partner Marketing Hub account to find assets and request
permission through our approval form.” This is a separate condition from
the source repository's Apache-2.0 redistribution licence.

In provider rows the reserved mark column is 32 points for every brand and
for Generic, so labels align with OpenCode's necessary clear-space box.
The requested 14/16/20 mark sizes and the reference's 8-point gap remain;
this 32-point column is a documented width difference from Paseo's compact
14-point column. Inline and toolbar slots retain their requested width.
