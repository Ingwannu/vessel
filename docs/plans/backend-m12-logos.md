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
- Initial iteration evidence is recorded here; exact later gates and
  review dispositions appear below. Job 2 (themes, ADR 0049) follows V1's
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

## Lead checkpoint before further optical work

The lead integration imports protocol 12, the reviewed session/machine-book
line, embedded Light faces and system reduced-motion handling. The only
conflict is the native README append; both the artwork contract and the
machine-book ownership/error notes are retained. K makes no node or wire
implementation change. Existing official-identity and area seams survive
the automatic source merges. Fonts were supplied with the lead checkpoint;
the downloader is not rerun. A fresh native gate checks this changed tree.
The prior 375-test tree and normal app build remain immutable evidence for
the earlier source slices, not this integration.

The coordinator's subsequent optical review asks for rendered ink bounds
from Paseo, official clear-space-preserving display boxes, a smaller-form
recheck for OpenCode, and a light/bare Google recheck. These are pending
follow-up geometry work; earlier family captures are fidelity evidence,
not approval of the final family sizing or actual app surfaces.

Lead checkpoint validation: native fmt, all-target Clippy (warnings denied),
whole native tests and normal app build all exit 0. The 441 passed test
results include2 compile-fail documentation examples; no tests failed.
The frozen Rust manifest is verified unchanged after build. This is the
lead integration gate, with no K node implementation change or new root
gate claimed. It is not final optical, pill or Settings runtime approval.

## Importing reviewed settings furniture

G3's immutable263d4a5/70bcfd5 section/frame and64167097 red_300 are reused
for the fitted-mark proof. The public section/frame functions and frame
regressions come from that reviewed source. The lead's newer code_row
and private hint-face body are preserved; row_faced delegates to the same
shared frame instead of undoing the lead's API. No G3 area or plan file
is edited or copied. The token is the single owned #fca5a5 in both schemes;
it does not alter semantic danger. These shared-source imports are gated
with the following optical/fitted control tree before committing.

## Optical and padded-settings control follow-up

The coordinator's family review replaces the initial balancing target
with measured Paseo ink bounds. `docs/design/agent-marks/ink.md` records
those numbers, official clear-space display boxes and every size departure.
Only image boxes change: all vendor path sequences/fills and Gemini bytes
remain untouched. Provider-row Generic now equals existing Glyph16; the
common 32-wide column and every default picker row's 37-point pitch stay.
Header/pill widths reserve each image's quiet-zone viewport; the16-point
header vertical slot,4-point pill gap and28/44 heights remain.

The existing shared frame from G3's reviewed70bcfd5 is imported unchanged.
The new kit-only settings mark adapter uses first24/following23 layout
heights, with OpenCode's unmodified 32 × 36 image inside existing padding.
First/following placements are -6/-7; the real draw box starts10 points
below its row top in either case, without half-point OpenCode paint.
Official artwork stays at the first content line when an error grows the
column; Generic retains the old centring/foreground. Name baselines are
compared with the previous actual Chevron 14/Bot 16 contents. All provider
name columns move16 points for the common32 slot, as recorded. G3's
application area remains untouched and Bot stays until reviewed handoff.

The audit adds actual labelled/icon-only pointer28 and touch44 pill_parts,
plus the exact shared row/card composition at1280/390, longEN/KO names,
status/count and expanded errors. These are control fixtures, not G3's
application runtime or a claim that its whole screen passed. A changed-source
gate and replacement family/picker/header/pill/settings captures are pending.
Prepared Rust is installed with current timestamps; gate logs must show
that the changed kit was recompiled before its new tests/captures count.

## Resume record after the credit closeout notice (2026-11-13)

The credit watcher required this worker to finish only its current slice.
No further merge, new control or theme implementation is started after
that notice. This section is the entry point for the next session; the
closeout validation and final commit disposition will be recorded below.

Committed before the current optical/fitted correction:

- `1ad83315`: initial embedded artwork control and ADR 0048. V1 did not
  accept its first comparison/header evidence.
- `a71a88ed`: corrected original minimum, header, disabled colour and
  actual keyboard/pointer regression. V1 cleared source findings and the
  40 standalone/picker captures. That did not prove pill or app runtime.
- `f8b06b8e`: view identity mapping, app adapter, draft/sidebar/board
  seams, About attributions and checked fake fixtures. V1 cleared source;
  final app visual approval remains pending.
- `140eaf16`: corrected the capture-scope wording; V1 cleared it.
- `45b75107`: lead checkpoint `fc71dcf7`, including protocol 12 and Light
  faces. Native fmt, Clippy, tests (441 results including two compile-fail
  doctests) and normal build passed. The Rust manifest stayed unchanged.
  No K node implementation or new root gate is claimed. Generated web
  wire whitespace inherited from the lead was reported and not rewritten.

The current slice enlarges only display boxes to match measured Paseo ink,
adds the fitted Settings mark slot, and supplies real pill/shared-frame
fixtures. Vendor paths/fills and Gemini bytes remain unchanged; the
provenance comparison passes. G3's reviewed section/frame and red[300]
are reused, preserving the lead's code-row hint face. No G3 area is edited.
The current source needs its own gate and new captures; earlier passes
must not be assigned to it. Style/import failures in intermediate runs
are retained in the logs and are not passing evidence.

After the current commit, the remaining order is:

1. Read the closeout validation below and the exact frozen-source gate
   manifest. Finish any unverified native gate before calling it ready;
   check that the changed kit actually recompiles. Prepared source must
   be installed with current timestamps, without a clean workaround.
2. Inspect new whole-family sizing, picker/header, labelled/icon-only
   pointer/touch pills, and padded Settings rows in both themes/scales.
   New artifacts use `shots/ink-*` under the track scratch directory;
   preserve the earlier V1-reviewed `shots/kit*` and their hashes.
3. Capture the existing app seams with the final normal binary: English
   and Korean, 1280 and 390, model list/composer/sidebar/board/About.
   The prepared driver now produces 72 images with dedicated desktop
   sidebar views. The nine lead-checkpoint phone calibration images were
   inspected successfully but are not final optical evidence.
4. Submit the immutable hash, source/binary/artifact hashes, exact gates,
   brand interpretations and remaining limits to V1. There is no full
   Job 1 pass yet. G3 keeps Bot until the fitted API is reviewed; G2 and
   G1 also need the reviewed hash and exact calls. The candidate fitted call is shown below; derive its edge from the
   same first/following condition as the shared frame. The existing adapter
   owns identity mapping and the kit knows no view type.

   ```rust
   let edge = if first {
       SettingsRowEdge::First
   } else {
       SettingsRowEdge::Following
   };
   settings_mark(crate::agent_mark::of(agent), Size::Sixteen, edge, tokens)
   ```

   This is a candidate API, not a reviewed handoff. The forge uses
   `forge_mark(Forge::GitHub, size, tokens)`, sharing the same source.
5. Job 2 is not started. ADR 0049 is reserved. Existing stored `Theme`,
   `Theme::ALL`, `appearance.theme` and `PreferenceChange::Theme` remain
   unchanged with three choices. After full Job 1 acceptance and resumption
   of this track, implement Paseo's eight choices and send the exact
   API to E2 and S2 together. Preserve G3 red[300], G2 green[500], G1's
   SmallGlyph/MonoBody additions and E2's sized-dot addition on integration.
   E2 owns the chooser and generic preview shell; do not duplicate them.

The exact vendor permission conditions are in the source record's
“For the owner” section. Implementation does not assert written approval
from Anthropic, GitHub or Google. No real agent, preview node, production
service, push or release is used by this track. Build/runtime commands
and artifacts use the existing development account; no ownership or mode
change is required. Own fake fleet/display/process cleanup is recorded at
closeout, and stale PID files must never be trusted on resume.

Cleanup during closeout: both owned fake nodes and all 22 descendants
were stopped by verified PID, with matching start times and UID 1000.
The cleanup report has no remaining process. Their stale PID file was
removed. Completed calibration displays/apps were already closed; any
new standalone audit display is still stopped by its own capture driver.
No file ownership/mode, production service, real agent or other worker's
process was changed. A resume must start a fresh fake fleet/book and
update the app capture driver's scratch config path.

### Current slice checkpoint

The optical/fitted source is saved with a `wip:` subject while its test
command waits for a machine slot. Current fmt and all-target Clippy pass;
new tests, replacement control captures and final app captures are not
claimed as passed. This checkpoint prevents the credit stop from leaving
coherent source uncommitted. Verification of this same slice may continue
until closeout; its result and immutable hash follow in the next record.

### Closeout validation after the checkpoint

The native gate for `e8ffeaa2` finished fmt and all-target Clippy with exit 0,
but whole `cargo test` exited 101: 37 kit tests passed and the new ink-bound
fixture failed. The actual padded first/following/last/error row comparison,
name-baseline/frame checks, all-brand row pitch and disabled pointer/key
regressions passed. This is scoped evidence, not a whole-workspace pass.

The failure exposed an inaccurate expected GitHub height. Solving the
unchanged path's cubic derivative extrema gives bounds 96 × 92.9886843692,
which at the unchanged display box yields 16 × 15.498114. The earlier
approximate rectangle gave 92.71875 and the fixture expected 15.45. The
fixture now expects 15.50 with its original 0.02 tolerance; no production
viewport, path, fill or proportion is changed. The source record is
corrected. This corrected test data still needs the full native gate;
time-limited closeout must not turn the earlier failure into a pass.

The normal example binary was rebuilt by that test invocation from the
checkpoint's production code. Any final-family snapshots taken during
closeout identify that immutable binary/input, not a newly passed suite.
The final app rebuild,72 app images, full pill/Settings visual approval,
owner-facing brand review and full Job1 acceptance remain incomplete.
Job2 remains unstarted. Consumers retain their existing fallbacks until
V1 accepts a corrected immutable control.

Closeout family evidence: four new light/dark scale1/2 comparison images
were captured and inspected. Their exact binary/input and PNG hashes are
in the track scratch `ink-family-closeout-manifest.json`. They are the
checkpoint production code; only the corrected test data differs in the
current source. This is standalone family evidence only. New row/header,
pill, fitted Settings and final app visual evidence remains incomplete.
The capture driver closed its application and virtual display.

### Final saved hashes and handoff

- `e8ffeaa2`: coherent optical/fitted control checkpoint, explicitly WIP.
- `d99a376d`: exact cubic-extrema fixture/source-record correction and
  closeout validation, explicitly WIP. The production prefix is unchanged.
- The following closeout note records these immutable hashes; no current
  corrected whole-gate, new full UI or full Job1 pass is asserted.

V1 owns the pending review. G1/G2/G3 must retain their current fallback
until V1 accepts a final control and its source/binary/capture manifest.
E2/S2 still see the original three-theme API. The next session starts at
this resume record and reruns the corrected native gate before completing
new controls/app evidence. Full Job1 acceptance precedes Job2 (ADR 0049).
The old reviewed 40 images remain untouched; four new family images were
looked at, with a separate checkpoint-binary manifest. No new row/header,
pill/Settings or final 72 app review is claimed.

Both own fake nodes and all 22 descendants are stopped; their PID file is
removed. The final comparison driver closed X236 and its applications.
No cargo/gate command remains running after the recorded test failure.
All work used UID 1000 and no ownership/mode change, push or release was
performed. Start a fresh isolated fleet on resume; do not trust scratch
ready/PID sentinels or reuse old node-book ports.

### Lead takeover and closeout (2026-11-13)

Track K stopped at its credit floor; the lead merged `d9eda48f` into
`backend/m12-spike` after the node line (protocol 13) and track L's second
slice, and finished the slice. The `wip:` commits stay in the merged
history; the merge commit and the gate below are the record.

- Ink fixture: computed again outside the kit from the unchanged GitHub
  path (true cubic extrema): bounds 96 × 92.9886843692, so 15.498114 at a
  16-point width. The fixture's 15.50 with its 0.02 tolerance is right.
- Gate: the whole native gate on the merged tree (fmt, all-target Clippy,
  build, test) exits 0.
- The marks in the real app, on the fake fleet (dark and light, Korean and
  English, desktop and a 390-point window): the model list, the composer
  pill, the sidebar's session rows, the board's cards and About's
  attributions. Each mark sits on its label's centre line; the model list's
  rows keep one pitch (37 points on desktop, 44 in the phone's sheet) with
  OpenCode's plate in them and draw marks at 14 (Paseo's `sm` in its model
  rows); the pill's mark is 16 on desktop and 20 at a phone's width
  (Paseo's toolbar glyph sizes); colour marks keep their own
  colours and the generic glyph takes the muted text colour.
- OpenCode (decided): its 16 × 20 plate stays at the brand minimum; it does
  not make its row taller than its neighbours (measured above), so no
  change.
- Gemini CLI on the light theme (decided): the official dark tile reads at
  the same optical size as the bare marks in the model list and its rows'
  pitch is unchanged; it stays as the vendor publishes it, no spacing
  change.
- Differences from Paseo, named: Paseo tints every provider icon (muted, or
  foreground when chosen) and has no marks in its sidebar's session rows;
  Vessel keeps each vendor's own colours (ADR 0048) and shows the mark
  before a session's status dot. A disabled row with a mark fades only its
  words, as K recorded for the vendors' conditions.
