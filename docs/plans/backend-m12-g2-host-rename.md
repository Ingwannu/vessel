# The host-name edit button

## Decision Log

- Purpose: a single shared host-name edit control for machine details.
- Constraint: Paseo host-appearance-section.tsx HostRenameButton has no density wrapper or hover/pressed palette.
- Alternatives: MD Ghost geometry, or an area-owned second pencil button.
- Choice: host_rename_button(id, ButtonState, tokens), original existing Pencil14, padding4/radius6, static muted ink, visual22 and outside8 giving hit38 in both layouts.
- Why: those are the source values. This38-point touch target is a source-specific exception, distinct from the ordinary44-point MD button.
- Impact: callers own callbacks and pending state; disabled consumers attach no callback. The control refuses its cursor/handler when disabled and adds no opacity, spinner or paint absent from the source.

Tests check the static geometry, actual SVG ink and returned hit frame with no opacity in both themes. Native evidence must cover outside/inside input, disabled state, adjacent controls and parent clipping. This control does not own a rename editor.
