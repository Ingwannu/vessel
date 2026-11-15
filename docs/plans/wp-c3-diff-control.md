# C3 readonly mapped editor producer

Status: prepared isolated source, **unchecked; not consumer-ready**. Base is the
announced e1b73c9e line. The source bytes are copied with current timestamps from
G1's owned C3 control files only; no View/Link/app or root runtime is included.
No new dependency, lock edit or ratchet exemption is introduced.

[Decision Log]
- Purpose: diff selection/copy/wrap stays on the existing source editor engine.
- Existing assets: gpui-base EditorState/CodeEditor, LineSyntax, shared fonts/
  semantic status/terminal selection tokens and VirtualList owner scrolling.
- Constraint: automatic buffer numbers cannot represent old/new source lines;
  styled row lists duplicate selection and copy. InputEditorStyle's complete
  external literal must remain compatible.
- Choice: patch7 optional immutable line-presentation provider on the existing
  highlight resolver, defaultNone and transparent wrapper, captured once per
  visible buffer row. No second engine, input edit model or syntax mapper.
- Effects/costs: mapped readonly parent clipping retains complete layout and
  selection/copy; explicit offclip position/IME bounds are absent. A cold longest
  source line is measured once per width-cache key for horizontal metrics;
  very long minified lines have proportional first-measurement cost. Ordinary
  None paths and gutter/space measurements stay unchanged.

## References and limits

Paseo utils/diff-layout.ts/test supplies source pairing; diff-document palette.ts
supplies statusSuccess/statusDanger15%/10%, muted numbers and terminal-blue
selection. paint.web.ts uses independent old/new editors; native actual-font
measurement replaces the approximate0.62 digit multiplier. This first control
retains the engine six-point gutter gap. Sparse-hunk syntax context is incomplete;
final source/reference geometry, cross-file selection and
consumer body/cache/scroll policy remain C3 work, not claims of this producer.

The lead and V1 source-prechecked the optional resolver/viewport/coordinate
patches; that is not installed-source Clippy/runtime approval. Private vendor
fixtures were removed at the lead's ruling because dependency dev-tests cannot
run from this workspace. Public kit window fixtures cover old complete literal,
transparent nested/clear resolver, gutter text origin, mapped tint/wrapped height,
readonly selection/clipboard, actual owner/cached scroll, bounded provider rows,
offclip/empty/direct IME queries, boundary rows, ordinaryNone coordinates, owner
keyboard reveal and actual wheel packets. These current fixtures have NOT run.

Exact label right alignment and selection/touch paint require post-line Linux
captures; folded-array order remains source-only, and the profile disables folding.
Patch7 README has concrete steps via optional mapped_diff_audit. The lead's
Linux mouse capture cannot prove another platform's touch path or OS/IME behavior.
The example is supplementary control evidence, not a fake-node Git screen.

## Verification plan

Use the prescribed remote VM only. After the outstanding C3 App/Kit source batch
returns, run one changed-source Kit all-target strict Clippy and Kit code_editor
window-test selection on this isolated producer. Preserve exact source/log digests
and actual changed-crate compilation. Do not label older App fa1/0af checks as
this producer's pass. The coordinator/lead alone integrates the reviewed minimal
control range; never the moving G1 package branch. No whole C3 gate/review or UI
pass is inferred from a bounded control handoff.

The package's prior fa1 Clippy reached Kit and found a partial doc span,
redundant Render qualification and two fixture API spellings. The exact same
three corrected source blobs are installed here. This does not relabel fa1 as
a pass or execute any window test; the isolated producer still needs its gate.

## Header foreground correction

Paseo diff-document/palette.ts codeTextColor uses foregroundMuted for hunk
headers. Reuse the existing Base TextDecorationCollection over UTF-8 header
ranges, preserving the complete buffer and existing two-sided LineSyntax.
Only nonempty Header rows receive a foreground override; notices and source
syntax are unchanged. A shared colour cache prevents identical redraws or
clones from repeatedly setting the collection and notifying its owner. Theme
changes replace that one existing collection. No new syntax kind, palette,
parser or vendor API is needed. Tests exercise byte ranges with Hangul and
actual engine notifications across unchanged/changed themes and clones.
This correction is newer than the exact27194 queued tests and remains
uncompiled/unexecuted until its own immutable source is checked.

## Source-copy consumer gap (not covered by engine-preservation tests)

Paseo diff-document/hit-testing.test.ts119–170 expects unified copy to retain
headers, but split new-side copy to omit headers/null old-only cells; old-side
copy retains its own header and source. Header select-all prefers new, with an
old-side fallback for deletion-only hunks. hit-testing.ts selectedCellRanges
skips null cells; model.ts lineSources emits the hunk header in one cell.

The current C3 BufferColumns repeats a header in both columns and uses empty
engine rows for missing cells. Ordinary native Copy therefore includes that
header/empty padding. The queued27194 fixtures assert selection/clipboard equals
the admitted buffer, not exact Paseo split-source extraction. They remain valid
engine-preservation checks but cannot be reported as source-copy parity.

This is unresolved C3 consumer work, not a silently accepted difference or a
reason to change patch7's engine selection/copy model. A possible Kit-only
projection of the same selected byte range through immutable row metadata has
been sent to the lead/V1 before any implementation. No competing selection
engine, vendor API widening, failing/blessed reference fixture or fabricated UI
proof is introduced by this note.

Lead ruling after tracing paint.web.ts270: that one header cell spans the full
viewport (columnWidth = viewportWidth / cells.length). The proposed native
split has two independently clipped half-width engines; merely blanking its
new-side header would clip the remaining header to half width. The split body
is not mounted, so this source-model gap is not reachable in the current
summary-only registry. Keep the duplicated model unchanged in this packet.

Follow-up is one renderer-mount packet with a plan first to lead+V1: a full-width
header over the two immutable engines (display-only virtual rows, synchronized
scroll/row height, no second selection engine), shared action-capture copy
projection and the four actual hit-testing source-copy fixtures. No such
renderer/virtual-row/copy seam is implemented or approved by this ruling.
The current bounded control packet may proceed with this explicit exclusion;
this does not make full C3/split/copy parity passed.
