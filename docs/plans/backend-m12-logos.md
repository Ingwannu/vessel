# Milestone 12, track K: official agent marks

## Before implementation

Existing assets: the kit's `Assets`, `glyph`, picker leading slots and
GPUI/resvg image renderer. Decision: extend these, no image dependency or
network API. Generic fallback remains the kit's bot. ADR 0048 records the
tradeoffs and deferred registry `icon` transport.

The node's `HarnessKind::ALL` is Claude Code, Codex, Gemini and OpenCode.
Settings Agents lists those plus added commands. Its fake fixture adds
`fake-goose`, `ghost-agent`, `check-failed` and `needs-sign-in`; the registry
test uses `example-agent` and an unpinned `floating` entry. These are fake
identities, not brand aliases. GitHub Copilot is the fifth requested mark;
user-added commands do not acquire it by their display name.

## Reference inventory

Reference paths below are relative to `getpaseo/paseo/packages/app/src`.
Source is read only for layout and behaviour, never copied as artwork.

| Surface | Reference | Mark and gap |
| --- | --- | --- |
| Resolver | `components/provider-icons.ts`, `provider-icon-name.ts` | exact built-in IDs, then catalog/snapshot SVG, then Bot |
| Model provider rows | `components/model-browser.tsx` | `ICON_SIZE.sm` 14, shared combobox leading gap 8 |
| Model provider header | `components/model-browser.tsx` | `ICON_SIZE.md` 16 |
| Composer and new-session model pill | `composer/agent-controls/model-sheet.tsx`, `layout.ts`, `control.tsx` | 16 pointer / 20 touch; gap 4; toolbar 28 / 44 |
| Agent and workspace rows | `components/agent-list.tsx` | `theme.iconSize.sm` 14; title row gap `spacing[2]` 8 |
| Settings Agents | `screens/settings/providers-section.tsx` | `theme.iconSize.md` 16 |
| Schedule row | `components/schedules/schedule-row.tsx` | 16 |
| Schedule form provider row | `components/schedules/schedule-form-sheet.tsx` | 16 |
| Imported-session row | `components/import-session-sheet.tsx` | 16, selectors 14 |
| User message control | `components/user-message/index.tsx` | 16 |

Additional provider-icon carriers and their final renderers:

| Carrier / renderer | Size |
| --- | --- |
| `command-center/agent-control-registration.tsx` → `command-center/icon.tsx` → `command-center/command-center.tsx` | 16 |
| `agent-presentation/index.ts` → `panels/agent-panel.tsx` → `screens/workspace/workspace-tab-presentation.tsx` | 14 default |
| `panels/provider-subagent-panel.tsx` → the same workspace-tab renderer | 14 default |
| `subagents/track.tsx` → `WorkspaceTabIcon` | 14 |
| `subagents/parent/use-parent-agent.ts` → descriptor / parent presentation | renderer's 14 default |
| `screens/workspace/workspace-desktop-tabs-row.tsx` → `WorkspaceTabIcon` | 14 default |
| `components/workspace-tab-drag-scope.web.tsx` | explicit 14 |
| `review/feedback/surface.tsx` → `WorkspaceTabIcon` | 14 default |
| `components/terminal-profile-icon.tsx` | 14 default, caller can supply its slot size |
| `screens/settings/host-page.tsx` provider icon wrapper | 16 |

The current Paseo sidebar's `components/sidebar/sidebar-workspace-row-content.tsx`
uses workspace-kind glyphs/status dots (`DensityIcon size="sm"`: 14 pointer,
18 touch), not a second provider resolver. Vessel's requested sidebar
agent mark is an owner-directed addition; its existing status dot stays.
The board follows Vessel's board behaviour, with the agent-row mark size.
`components/combined-model-selector.tsx` carries the same model glyph at
its toolbar's 16/20 size (16 fallback), rather than another asset source.

## Necessary differences

- Brand-owned geometry and minimum/clear space win over Paseo's slots.
  OpenCode needs 32 × 36 total at its minimum; a row can fit it, an inline
  or 28-point toolbar cannot and uses the generic bot.
- GitHub has retired the standalone Copilot head. The current bare
  Invertocat identifies the vendor beside explicit GitHub Copilot wording,
  as ADR 0048 records; the same source serves the typed forge control.
- Registry/snapshot SVGs are not loaded: no safe node image wire contract
  exists (ADR 0048). The generic glyph identifies added commands.

## Evidence and progress

- Vendor source inspection found that the published Google JS sample's
  current `favicon.svg` is an empty document; rejected, not treated as a
  mark. Google's current brand hub asks for sign-in for its Gemini guide.
- An early source-inspection command treated Anthropic's press-kit ZIP as
  HTML and stalled in a regex scan. Stopped that inspection by its own PID;
  only selected SVG members were subsequently read with Python zipfile.
- Google Gemini CLI's official VS Code companion icon is an unchanged
  1645-square PNG under the repository's Apache-2.0 licence. This exact
  raster original is used rather than tracing a substitute vector.
- Implementation and capture review are in progress. No completed gate or
  visual pass is claimed here yet. Job 2 (themes, ADR 0049) follows V1's
  acceptance of job 1.

## Review decisions

V1 requested that identity-to-mark logic live with its tests in `view`,
per the strict area's ownership rule. The app only adapts the resulting
mark enum to the kit's opaque artwork. This deliberately clarifies the
initial assignment's wording that placed the identity mapping in `app`;
no reverse source inclusion or view-to-app dependency is introduced.

The OpenCode source has a full-viewport white path inside its luminance
mask. It is clipping metadata, not painted ink: the hand conversion keeps
only the two visible shade/outline paths. A pixel test checks that both
the clear space and the top counter remain transparent in both variants.
The source path-sequence comparison distinguishes painted paths from
paths inside masks/definitions.

Problems met during the control slice:

- The first cold native `cargo clippy --all-targets -- -D warnings`
  rejected redundant raw-string delimiters and missing/backtick documentation.
  Later checks caught a unary float negation in the token-based row inset
  and audit-example imports/borrows. Those were corrected without adding
  lint exemptions. Intermediate passes are not the final tree's gate.
- Reading the complete Paseo theme source corrected an initial size
  assumption: `xs=12`, `sm=14`, `md=16`, `lg=20`. Provider rows are 14,
  Settings provider marks 16; the composer remains 16 pointer / 20 touch.
  The new Artwork leading uses 14 while existing Glyph16 stays unchanged.
- The first full test command began before the shared build-slot rule was
  announced and was left running as instructed. Every later check/build/test
  command uses the machine's slot wrapper. No second cargo command overlaps.

The control changes shared kit seams only: the `Assets` load/list entries,
`lib.rs` module registration, the opaque `Leading::AgentMark` alternative,
and `PillParts::agent_mark`. Existing glyph and SmallGlyph ownership is
preserved. The OpenCode box uses the row's existing 8-point vertical inset
via margin, leaving its 16 × 20 ink and 8-point clear space unchanged. The
layout regression compares marked rows with an unmarked baseline row.
Visual clipping and hover evidence is still required in addition to that test.

The first full test run caught two assumptions in the new tests: the
original PNG header is 1645 × 1645 (the inspection viewer had resized its
display to 1600), and the row-layout probe lacked the real popup's Body
type style, producing a 42-point default-font row. The source PNG bytes
were unchanged. The test now measures rows in the actual popup typography;
its equality with the existing row height and adjacency assertions remain strict. The new OpenCode pixel
transparency test passed on that run.

The first four mark-family captures were inspected at both scales and
both themes. They exposed that the full Copilot product wordmark dominated
its neighbours; shrinking the lettering would make it illegible. The
coordinator accepted the current official bare Invertocat beside explicit
GitHub Copilot context. The kit uses one source for that agent attribution
and the separately typed forge control. Captures from the rejected full
lockup iteration are retained only as earlier evidence, not final proofs.

A further measurement established the actual native baseline: Body text
is 21 points high, so a row with 8-point top/bottom padding is 37 high
despite its 36 minimum. The marked-row test compares with the unmarked
37-point row; it does not change existing row typography. OpenCode's
36-high clear-space image aligns to the top of the row instead of its
vertical centre, preserving whole-pixel edges rather than introducing a
half-point offset. Both ends remain within that existing row.

The prototype rows also showed OpenCode's label displaced relative to the
other providers. All Artwork row slots now reserve its required 32-point
column, centring smaller marks and Generic inside it, so the text aligns.
This costs 18 extra points compared with Paseo's 14-point provider column;
it preserves the 8-point gap and does not enlarge existing row heights.

## Control-slice validation

The final control tree passed the complete native gate: `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`, and `cargo test` (370 tests:
app 3, kit 24, link 14, view 240, extras 89; no failures). The compiler
caught a single-pattern match and an oversized combobox function in the
last iteration; an if-let and a shared search-row helper resolved them
without lint exemptions. The final gate includes the 14-point row sizing,
32-point common column, header slot and compact alignment changes.

The rebuilt comparison window was inspected in all four theme/scale
combinations: every requested 12/14/16/20 slot beside the official original,
plus the whole family at 16 and 20 with Generic. Gemini's unchanged PNG
retains crisp high-contrast border/prompt edges at these sizes; its plate
remains visible, a deliberate official-asset difference from bare vectors.
OpenCode stays visibly larger because its minimum ink cannot be reduced.

Twenty picker captures cover both themes, desktop and 390-wide touch:
first/last row, selection, hover and bottom scrolling. OpenCode's required
clear space remains within its row, including at the scroll viewport's
ends; the neighbouring rows' content does not overlap it. Family captures
are distinct from the app-surface captures, which belong to the next slice.
The temporary original-comparison files and captures are review artifacts,
not bundled application assets. Every painted path matches the vendor's
original path sequence; the source PNG remains byte-identical.

## Control correction after first review

This supersedes the first control validation's original-comparison and
header evidence. OpenCode's comparison original now always uses its
16 × 20 minimum ink in 32 × 36 clear space. The header uses ordinary
16-point artwork, with a 32 × 36 exception for OpenCode; it no longer
reserves the row's 32-point column for every brand. Its long title now
uses the reference header's single-line ellipsis (Paseo
`adaptive-modal-sheet.tsx`, title `numberOfLines={1}`), so it cannot run
under the popup's edge. Actual back/title/artwork headers are captured.

Disabled official artwork keeps its approved opacity and colours. The
55% treatment applies to its words, selection check and trailing control;
Generic and previous glyph rows retain their whole-row treatment. An
actual GPUI click/key regression proves disabled Artwork, Generic and
Glyph rows do not call back; navigation skips them, enabled selection
still works, stale disabled/out-of-range highlights cannot activate, and
Escape closes. Enter rechecks current rows rather than trusting an old
highlight. The touch pill's existing 44-point target can hold OpenCode's
full box; the short pointer pill and inline rows retain Generic.

The corrected production sources passed native fmt, clippy (all targets,
warnings denied) and all 375 tests: app 3, kit 25, link 14, view 244,
extras 89. The gate also includes the following app slice's uncommitted
sources; the kit files in that manifest match this control commit. It is
not a separate claim of a clean control-only 375-test tree. A separate
normal binary build was interrupted while waiting for a machine slot;
only that build stage is resumed, with no repeat of the passing gate.

Forty replacement captures were inspected: four complete mark families
at scale 1/2 and dark/light; 28 provider-row captures (first/last,
selected/hover/scroll, disabled pointer and keyboard); and eight actual
long-title Claude/OpenCode headers at desktop and 390, both themes.
The source and binary hashes and exact file list accompany the review.
These prove the picker and standalone artwork only. Actual labelled and
icon-only `pill_parts` at touch 44 still need runtime captures; the app
final captures are pending. A Settings ProviderRow
has distinct 56-point/padded geometry and needs a separate fitted adapter
and actual card/adjacent-row evidence before that consumer replaces Bot.

## App slice

The sole semantic mapping is `view::AgentMark::of(&HarnessRef)`: all four
built-in variants map exhaustively, every added command maps to Generic.
Tests cover the reference's custom-Claude profile case, unknown/fake ids,
case and suffix traps, and the fake fleet's long-label custom commands.
The sole artwork adapter is `app::agent_mark`; no surface guesses a logo
from a model name, an agent label, or a registry icon address.

The draft's provider rows, model rows (including global search), provider
header and non-fast model pill use that adapter. The same draft control is
reused by a new session and an open conversation. Fast mode retains its
reference lightning icon. Existing CardDetail already preserves the
node's typed harness; the sidebar and board use it directly. The sidebar
keeps its independent status dot; the board replaces the prior letter tile.
OpenCode uses Generic on these inline surfaces because its clear-space
box would not fit. No schedules or Settings Agents files are edited.

Settings About gains an artwork attribution section using its existing
settings card/rows and source/terms links. All five shipped marks have a
named owner; Google has a separate software-licence link because that
licence does not grant trademark permission. English and Korean whole
sentences live in `view::text::marks`, with translator context and tests.
This is added in the existing About manner: Paseo's settings About and
Vessel's licence list; there is no new interactive control or page.

`scripts/mark-fixtures.sh ROOT TARGET` extends an already-started fake
fleet with long bilingual session titles, Gemini/OpenCode identities, and two custom agent
labels that mention a real brand but retain their added identities. Its
checks read the real node's persisted titles and its CLI's full added-agent
records and saved harness identities, failing if any exact expected shape is missing. It runs no real
agent and uses only the fake fleet's scratch configuration. The first
attempt found the previous scratch node processes had exited; no runtime
success was inferred from stale PID files. A supervised fresh fleet is
used for final captures.

A 390-wide runtime check found that the existing effort card had no anchor
on touch: the tooltip deliberately does not measure there, but the card
needs the same trigger bounds. The model pill now attaches the existing
kit anchor probe on touch. This is a small draw/wire seam, preserving the
reference's absence of mobile tooltips while making the model control open.
Prototype captures document the failure; final captures use the rebuilt
source, not those prototypes. The final changed-source gate passed fmt, all-target clippy with warnings
denied, and all 375 native tests, including the new identity/attribution
tests and real picker callback regression. The normal app binary and final
app-surface captures remain a separate runtime check; their build was
interrupted while queued and only that phase is resumed. The earlier
390-wide calibration successfully opened the touch model card, selected
a long model and OpenCode, and showed the sidebar, board and all five
About records. It is intermediate evidence, not the final binary manifest.
