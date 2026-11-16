# Canonical file-change glyph roles

[Decision Log]
- Purpose: G1's shared file-section header needs the pinned added/deleted/
  modified family, not window buttons, generic plus/minus or area SVGs.
- Existing assets: the single declarative Glyph inventory, inline line_glyph
  renderer, canonical-role raster fixtures and existing Lucide/Feather notice.
- Alternatives: aliases to visually adjacent icons or a separate renderer would
  lose the aligned square family and duplicate the source of truth. No new
  dependency, stroke API or color knob is needed.
- Choice: append SquarePlus, SquareMinus and SquareDot after the full receiving
  inventory; use the canonical Lucide1.50 XML child geometry with the existing
  inherited1.5 line stroke. Keep every older variant/path/vector/order unchanged.
- Why: pinned Paseo1c00 components/file-change-icon.tsx:1–59 has no stroke
  override. app/_layout.tsx:926 supplies LucideProvider, and styles/theme.ts:602
  sets ICON_STROKE_WIDTH1.5. Upstream SVG default2 is therefore not effective ink.
- Impact: the caller owns role14, statusSuccess/statusDanger/statusWarning,
  translated accessibility labels, optional counts, target context and actions.
  These assets add no header, menu, dispatch, domain state or appearance policy.

Canonical sources: lucide-icons/lucide tag1.50.0 icons/square-plus.svg,
icons/square-minus.svg and icons/square-dot.svg. The shared rounded18-unit
square at3,3 is retained exactly. The plus has horizontal and vertical paths;
minus only the horizontal path; dot is a stroked radius1 circle, not a filled dot.
Preserve docs/design/brand/lucide-LICENSE.txt unchanged.

This leaf starts on announced base13.1, separate from the already reviewed
Import packet and the unpassed F4 branch. Before the first remote run, reconcile
the newest announced inventory without replacing another owner's appended rows.
No Cargo or priority follows from the writer assignment. The current placement
job and recorded Main runtime remain ahead in the serialized build sequence.

Verification must compare the full old prefix and old drawings after removing
only these three new arms. Reuse the existing canonical-role vector parser and
raster helper in tests; retain every existing assertion. Check unique paths,
parsed inherited stroke, plus/minus/dot landmarks, rim and fractional14-point
scales. Light/dark source sheets are source-only, not native header first use.
Actual strict/runtime, V1, coordinator comparison as requested, lead receiving
gate and G1 native consumer evidence remain separate and currently owed.

Source checkpoint: the complete receiving100 prefix is byte/order/path exact,
followed only by these three roles. Removing their three new SVG arms and one
source comment restores the old drawing file byte-for-byte. Every canonical
XML child tag/attribute matches the retained upstream response; the source
provider audit explains the sole root-stroke mapping2 to1.5. The license is
unchanged. Existing panel-role parser/raster functions gain test-only sibling
visibility; their bodies and every old assertion are unchanged, avoiding a
second raster harness. Three new tests are authored but UNEXECUTED.

Both800x330 light/dark source sheets were personally opened. They compare each
actual inline asset with its canonical source at14 and scales1/1.25/1.5/2,
using the pinned status colors. The plus/minus/dot family remains distinct and
unclipped in those SVG-source images. They are not native Header captures or
Rust test results. No new Cargo request has been submitted for this leaf.

Before the first check, merge announced base14.1 b54abbdc. Its full101 prefix,
including the accepted Import role and its private2.2 arm, stays intact; these
three unpublished declarations follow it. Only their unpublished iteration
positions shift from the earlier source checkpoint. All old public positions,
names, paths and SVGs remain exact. The original100-prefix proof and sheets
are retained as prior-source evidence; final source checks use the receiving
101 prefix. This composition is not yet compiled or runtime-tested.

## Quota-stop resume checkpoint

Current receiving source de3cca679fc3b1f1470a8404ece0d6dc437dfd7b on b54
is source-only. No Cargo, native first use, V1 source/runtime review or approved
consumer hash exists. The old full101 prefix, old drawing bytes and canonical
XML children are independently source-checked; source images are not native
Header evidence. The current three glyphs are not part of the passed Import
packet. G1 waits for exact checks/review and receiving-line handoff.

Future ArrowRight/Focus/Move/ArrowDownToLine are a separate coordinator284
reservation after these three. W22 has a prepared eleven-row consumer snapshot
7be03b61, NOT applied/compiled. No drawing or build for those four is started.
Read Main docs/plans/wp-f4-sidebar-resume.md on backend/m12-wp-sidebar and query
the already-queued placement025223/3958 before any new build. Latest14.4 is
recorded but no new merge occurs during quota closeout. All source is committed.
