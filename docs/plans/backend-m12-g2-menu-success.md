# An acknowledged result in an open menu

## Decision Log

- Purpose: direct Git writes keep their result where the pending action was shown.
- Constraint: Paseo menu/menu-item.tsx success is not the semantic status.success colour.
- Alternatives: area-owned success rows, a second alert, or substituting semantic green.
- Choice: additive ItemStatus::Success(label) / MenuItem::success(label), existing CircleCheck glyph16 in leading slot16, shared palette GREEN_500 (#22c55e) for both text and icon in both themes.
- Why: source resolveLeadingContent and itemTextSuccess use that exact palette entry; success is effectively disabled with overall opacity0.5 and no hover/activation.
- Impact: the caller supplies complete localized result words and owns timing/reset. Dismissal stays unchanged; KeepOpen leaves the acknowledged result visible. No second CheckCircle alias or vector is introduced.

Value tests cover both themes and ordinary/destructive item tones. Existing pointer/keyboard guard tests include Success and stale highlights; explicit Escape/outside dismissal remains available. Native menu probes are control fixtures; actual Git write success still needs G1's consuming-screen evidence.
