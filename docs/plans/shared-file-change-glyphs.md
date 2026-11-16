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
