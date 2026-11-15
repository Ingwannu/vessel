# ADR 0048: Official agent marks, with bounded embedded artwork

Status: accepted for implementation, 2026-11-13; provenance and visual
review must pass before integration. The owner requested official logos.

## Context

Agent names shared a generic bot or a letter. Paseo supplies a provider
icon in its model browser, composer, agent rows and schedules. Its assets
are not a source for Vessel: only a vendor's own brand/media page or its
own open-source repository may supply artwork.

The native kit knows no node or view types. Its existing `glyph` uses a
monochrome SVG mask, which would recolour official multi-colour artwork.
An agent's display name and a user-added command are not proof of a brand.
The registry can name arbitrary hosts in its optional `icon` field.

## Options

1. Copy the reference's icon assets: rejected, they may be redraws.
2. Fetch registry icons in the app: rejected, it grants wire input a
   network and image-decoder capability, including private addresses.
3. Reviewed, embedded vendor path data, with an explicit generic fallback.
4. Node-side icon acquisition: possible later, but requires an additive
   content-addressed image wire contract and security review first.

## Decision

Option 3. Source records, retrieval dates, SHA-256 values, terms and
attributions are kept in `docs/design/agent-marks/README.md`.

- `view::AgentMark` is the UI-free closed set of Claude Code, Codex,
  Gemini, OpenCode, GitHub Copilot and Generic. The view owns the sole
  identity-to-mark decision, with tests alongside it; the app adapts that
  enum to the kit's opaque artwork in one place. Built-in identities are matched exhaustively;
  names are not lowercased, split at suffixes, or guessed from labels.
  User-added agents retain Generic even when named like a built-in.
- The kit accepts only its own opaque embedded `Artwork` constants and a
  typed logical size/space, never a path, SVG string or URL from a node.
  It rasterises those reviewed vectors itself, at the device pixels each
  mark lands on (amended 2026-11-14, below). No runtime download.
- Geometry, fills, winding rules and the approved light/dark variant are
  retained. Source wrappers, unused clips and metadata are not executable
  assets in the app. A necessary clear-space view box is added around,
  never by changing, a path. No recolouring with UI accent tokens.
- A mark's minimum size and clear space override Paseo's slot. A 36-point
  row can hold OpenCode's 16 × 20 mark plus its 8-point clear space on all
  sides (32 × 36 total). A 28-point pointer toolbar or short inline slot
  uses Generic; a 44-point touch pill holds the full 32 × 36 box.
  GitHub Copilot is identified by the current official bare GitHub
  Invertocat beside the explicit product name. It is the same embedded
  source the separately typed forge control uses; the retired mascot head
  is not bundled. The full product lockup was evaluated but dominates a
  16/20-point icon family and cannot become legible when shrunk to match.
- Claude uses Anthropic's official Clay Spark. Codex uses its vendor
  OpenAI's official black/white Blossom (not a redraw of the terminal
  prompt). The source's inset is preserved. Gemini uses Google's Gemini
  spark in its four colours, Google's own file as the Gemini app serves it
  (amended twice on 2026-11-14; before, the Gemini CLI app icon, a PNG
  plate, then the blue spark from `google/adk-docs`). Nothing is traced.
- Requested sizes target the reference's painted ink. Full image viewports
  retain their clear space and may be wider than the nominal slot; the
  measured boxes/differences are recorded in `docs/design/agent-marks/ink.md`.
  Existing row/header/pill padding contains the vertical quiet zone. A
  requested20 Claude Spark cannot fit a28-high compact spot with its quiet
  space and uses Generic there; the44-high touch pill has room. Generic
  provider rows use the kit's existing16-point Bot, as ordinary glyph rows do.
- Third-party original files are not committed where redistribution is
  not expressly granted: only the path data the app draws, provenance,
  and attribution. Vendor rights are not relicensed under Vessel's
  source-available licence. Owner-facing permission requirements remain
  explicitly listed; implementing the owner's decision does not claim
  that permission has been granted.

### Registry `icon`: deliberately not implemented tonight

The node currently discards this optional field; the app receives no
trusted bytes for it. All non-built-in registry and user agents use
Generic. No node/wire change is made by this app track.

A future implementation must fetch on the node only, with an explicit
host policy and private-address/redirect checks, bounded concurrent
fetches, connect/body deadlines, a small byte/pixel limit, MIME and magic
checks, and a parser that rejects scripts, external references, entities,
fonts and active SVG content. Prefer bounded rasterisation to canonical
pixels, hash/cache that output, and transmit typed content-addressed
bytes. The app must never fetch an address supplied by the wire. This is
unfinished work, not an assertion that arbitrary SVG can be made safe by
string replacement.

## Why

Local reviewed paths are small, deterministic and available offline. A
kit control prevents each surface from inventing its own dimensions or
fallback, and keeps GPUI out of the view crate. Exact identities avoid
claiming that an arbitrary user command is an official vendor agent.

## Consequences

Each new brand needs its own vendor source and terms review. Some small
slots intentionally show the generic glyph. The About attribution list
must name every shipped mark as its owner's trademark, used to identify
an agent without endorsement. An official raster original may be used unchanged with its source terms
recorded (none is today). An unavailable official asset is reported as
missing, never traced into an unofficial substitute.

## GitHub interpretation for the owner (2026-11-13)

The decision deliberately reads GitHub's standalone-mark and permitted
integration clauses together; it is not a claim that the Invertocat is a
separate Copilot logo. The current source is
https://brand.github.com/foundations/logo (Invertocat and Legal sections).
The standalone-mark sentence permits the mark when the GitHub brand is
already established; its exact relevant excerpt is “where the brand is
already clearly established.” The permitted-use item is headed “Show
integration”; its sentence permits a GitHub logo to tell readers that a
project integrates with GitHub. The full sentences remain at that address.

The separate current Copilot page,
https://brand.github.com/brand-identity/copilot, states:

> The Copilot mascot head was retired in early 2026 and is no longer part of our visual identity.

Prior written permission remains an owner issue, quoted in the source
list's “For the owner” section. The implementation neither asserts approval
nor bypasses the vendor's right to revoke it.

### Agent-name context

The current node has no built-in Copilot identity. User-added commands,
including ones named `copilot`, receive Generic. The bundled Copilot
artwork is identified explicitly in About and the audit inventory. Any
future wire-backed Copilot identity must preserve this context audit:

| Surface without the agent name beside its model/title | Current Copilot rendering | Required context before a future official Copilot identity is mapped |
| --- | --- | --- |
| Composer model pill, including icon-only density | Generic for added commands | Same trigger's tooltip must say GitHub Copilot |
| Model name rows inside a provider view | Generic for added commands | Provider header in the same popup says GitHub Copilot, or row tooltip does |
| Sidebar session row / collapsed sidebar | Generic for added commands | Same row or kit tooltip says GitHub Copilot |
| Board card title | Generic for added commands | Same card's agent line says GitHub Copilot, or mark tooltip does |
| Session/workspace tab | This track adds no provider mark there | Same tab label or tooltip says GitHub Copilot |

There is no current production surface that shows the Copilot Invertocat
without that agent-name context. A future mapping may not silently make
these generic positions into unlabelled GitHub marks. Forge uses explicitly
identify GitHub and use the same kit data through `forge_mark`.

## Amendment 2026-11-14: drawn at the display's pixels; Gemini's spark

The owner saw the marks broken on their screen (likely 125 % or 150 %)
and Gemini shown as a terminal plate rather than Gemini's mark.

- Cause, read in the pinned GPUI: `img()` rasterises an SVG once at twice
  its own view box and stretches that texture onto the bounds; shrunk
  12:1 to 65:1 by a bilinear sampler without mipmaps, strokes break up.
  `svg()` renders at device size but only as a one-colour mask.
- Decision: one path for every mark. `kit/src/agent_mark/raster.rs`
  snaps the mark's box to the device grid, rasterises the embedded vector
  with GPUI's own renderer (resvg) at exactly that many device pixels,
  and paints it 1:1; rasters are cached per asset and device size. Its
  Decision Log names the alternatives (mask for one-colour marks plus
  layered masks, pre-shrunk images, a GPUI patch).
- Gemini: Google's spark from `google/adk-docs` (source, hash and terms in
  the source list), the same in both themes, its ink 16 at 16 like the
  reference's. The PNG plate and its use are removed; the Apache-2.0 text
  stays because the spark comes from an Apache-2.0 repository.
- Proof: `kit/examples/mark_sharpness.rs` at `Xft.dpi` 96/120/144/192,
  each crop compared with librsvg at the same device box (numbers in the
  source list). Google's approval requirement for product icons remains
  the owner's, as before.

## Amendment 2026-11-14 (second): Gemini's mark from Google itself

The owner's word 30: the Gemini mark may of course be used; the
coordinator: Google's own mark, from Google, never from another
product's files (the spark above came from Google's ADK documentation).

- Gemini is now the file the Gemini app loads from Google's CDN, embedded
  byte for byte (source, hash and terms in the source list): the spark's
  vector outline over a JPEG colour field in Google's four colours,
  embedded as `data:`. The kit's rule that embedded drawings are inert
  vectors keeps "no script, no link" for all and allows that one embedded
  raster for Gemini only (`embedded_drawings_are_inert_vectors`).
- Not redrawn as gradients: Google's mark is used unchanged.
- The outline and inset are the old spark's, so its ink and the pills'
  geometry are unchanged; the sharpness sheet at the four scales is the
  same proof as above.
