# Menu presentation and ordinary glyph size

## Decision Log

- Purpose: the Git overflow can explain an unavailable operation when selected, and uses its action glyphs at16; schedule actions retain14.
- Constraint: a quiet idle action is selectable, unlike disabled or pending feedback. A size choice must not alter row pitch, leading-slot width or pending/success glyphs.
- Alternatives: an area-owned menu, disabled rows with a second explanation line, or scale every icon.
- Choice: additive `Presentation::{Normal,Muted}` and `LeadingSize::{Small,Regular}` with explicit builders on the existing item and renderer.
- Why: input guards, mouse/keyboard selection, Close/KeepOpen, hover/press and feedback precedence stay shared.
- Impact: legacy constructors retain Normal/Small; muted idle text is foregroundMuted at0.72. Disabled/pending/success remain0.5 and non-selectable; success retains its fixed green. Ordinary glyph14/16 stays in the same16 slot.

References read: Paseo `components/ui/menu/menu-item.tsx` (idle muted text/opacity and disabled precedence), `git/actions-split-button.tsx` (selectable muted operations), and the whole `git/action-icons.tsx` (Git leading16). No second line or availability decision is added to the kit. The Git view owns the informational explanation and whether any node write is sent.

Tests cover both themes, exact paint and actual SVG style sizes, feedback precedence, stale selection guards and both pointer/keyboard paths with Close/KeepOpen. Native first-use captures,14/16 comparison and pointer/keyboard muted selection remain separately required; source tests do not prove runtime appearance.
