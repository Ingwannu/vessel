# Shared standalone search field

[Decision Log]
- Purpose: Providers catalog and model filtering need Paseo's standalone search
  surface, whose clear affordance belongs to the control.
- Existing assets: TextField, its unstyled bare renderer, identified editing
  engine, focus/listen, and original Search and Remove (Lucide X) vectors.
- Constraints: one input engine and one shared surface; no area chrome or
  competing field writer. W4 identity then G2 multiline were reviewed before
  this additive control.
- Alternatives: style an ordinary form in each area, create a separate editor,
  or accept a multiline field and silently render it as a search box.
- Choice: SearchField privately constructs only the existing one-line TextField,
  renders its bare editor and owns Search/X drawing and clearing.
- Why: input methods, selection, clipboard and undo remain the existing engine's
  responsibility. Ordinary field, composer, multiline/identified callbacks and
  minimum-height APIs remain unchanged.
- Impact: no new dependency, RPC, preferences or OS action. Programmatic clear
  keeps its original silent reset; the person's X uses replace_all so the
  owning query receives the ordinary Change and the edit remains undoable.

## Source and API

Pinned Paseo components/ui/search-field.tsx, whole file (field/input 92–122):
Search14/X14 with inherited1.5 stroke; horizontal12/vertical6/gap8; max420;
input20; radius.md6; surface1/border at rest, surface2/borderAccent focused.
X has hitSlop8 with no new hover/pressed fill. One source geometry in both
layouts: no invented44-point wrapper. The existing native bare input uses
Type::Body metrics; text/control scaling follows the shared font policy.

kit::SearchField::new(placeholder,on_change,window,cx), text(cx),
set_placeholder(placeholder,window,cx), clear(window,cx), and
element(id,tokens,cx). Enter is not a submission API. Callers provide their
localized placeholder and only ordinary state-change callbacks; no external
clear icon or second editing state. Element identity is caller supplied.

## Verification status

Source installed after the checked G2 baton; current timestamps are used.
Both-theme literal returned-style tests and actual engine keyboard/clear/focus/
wide/narrow bounds tests are added. They are uncompiled until the exact changed
head remote narrow completes. Expected constants are source literals, not the
implementation constants alone. X must emit one empty Change, keep focus, remove
its target after clearing and remain inside the narrow field. Locale placeholder
changes preserve editor identity. No native hover pixels, OS accessibility,
keyboard access to X, actual OS IME or consumer UI pass is claimed. Existing
kit-wide keyboard/accessibility gaps are not solved by this drawing control.
No unchanged whole gate or screenshot matrix is requested for the producer;
consumer first-use and final A9 evidence remain separate.


The first exact2e3 Kit Clippy returned101 solely for a missing unit-statement
semicolon in the last test assertion. The sequential runtime stage did not
start. Add that semicolon only; no product body, assertion, expected geometry
or deadline changes. Old red evidence remains. Corrected exact-head Clippy
and focused tests are still owed, without priority or a whole-gate claim.
