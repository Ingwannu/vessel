# Source identity on a form textarea

Decision: modify/reuse the existing input path. The checked FieldId producer
exposes Plain/Identified on one listen subscription; TextareaState is already
used by forms and the composer. A second engine, subscription, callback system
or visit counter would split the existing Enter/IME/Editability semantics.

[Decision Log]
- Purpose: a delayed prompt edit/submit must retain its actual editor origin.
- Existing assets: FieldId, Callbacks::Identified/listen, TextareaState and the
  typed FormMinimum. FieldId is not a form visit or backend permission.
- Choice: form_multiline_with_source feeds the same callback enum to the one
  multiline factory and takes an explicit FormMinimum. Its old constructors
  keep their signatures and Plain callback behavior. Every Self initializer
  retains FormMinimum::Default; Expanded changes only the outer source minimum.
- Why/cost: source-aware lines/textareas share semantics. Constructors and SendOn
  move together into field/multiline.rs, with SendOn re-exported at its original
  path. The root falls within the ordinary budget, so its record is removed
  rather than increased. No input-style/glyph/public caller migration is added.

API: TextField::form_multiline_with_source(placeholder, on_submit, on_change,
FormMinimum::{Default,Expanded}, window, cx). Both callbacks receive the actual
FieldId; the clone/id contract remains the checked producer's. The caller must
retain original form visit plus current source/context/authority. Reusing an
engine keeps its id and does not prove old-form safety.

Source basis: pinned schedule-form-sheet.tsx1040 minimum96; existing GOAT form
minimum68/80/cap remains the default; existing form-field/editability/input rules
are unchanged. This is source identity, not a newly designed input control.

New tests use actual multiline keyboard insertion, plain Enter line break and
platform Mod+Enter submit, both minimum choices, Unicode text/source/clone
identity, and the existing ReadOnly/re-enable path. Original form/source-line
regressions remain unchanged. None of these is OS IME, focused-field form
replacement, authority, consumer UI or a whole-package pass. Narrow checks and
one source disposition are still owed at this source checkpoint.
