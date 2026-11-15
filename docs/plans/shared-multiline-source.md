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


## Exact580 requested narrow outcome (2026-11-15)

The isolated source chain is dcbdbb018 →6211c07e →75b0cd1d →f5381b23
→580a7781a3dd30bc727049f60d2a1c89fd62617a. Only the final exact source
has this outcome: remote --locked Kit all-target Clippy0 and Kit --lib171/0/0.
Both native-fonts/native-cargo steps were0, no skip; Kit actually checked and
compiled. Runs schedules-20261115T131718Z-580a7781 (Clippy) and
schedules-20261115T131834Z-580a7781 (tests), same schedules short priority suite.
Steps SHA256:90ae9c32bf6eae7f08f5bb09dd2b2531aa423e8d51d2c17c330ae9397c2477ae
and d3314018e82b54d413fed4febb8b72bb651c17e8d1d2dbf1f65f3cb09e0a049b.
The identified newline/Mod-submit/Unicode/source/clone/readonly/re-enable cases
and actual minimum96/growth/cap case explicitly passed. This is GPUI test
backend evidence, not native OS IME, old-form A→B authority, consumer UI, App or
whole F1 proof. Source disposition remains V1-owned until its bounded review.

The seven-path source manifest, unchanged events/identity/identity-keyboard/old
form/editability byte checks, local exact rustfmt/doc scan, original source diff
and all exact run file hashes are in /tmp/work/m12-g2/multiline-final-packet-20261115.
Generate the manifest with git diff --name-only dcbdbb018 580a7781; each entry
uses git show 580a7781:PATH, byte length, SHA256 and git rev-parse 580a7781:PATH. This packet
uses immutable580, not today's moving F1 files. Retain the isolated parent's
picker829 only as its historical baseline; consumers preserve their newer
picker/glyph/exports and apply only these field/source changes. Root485 removes
its exception rather than raising a limit. G3 SearchField waits for the checked
exact handoff; no parallel field writer or second engine is authorized here.

Earlier records remain distinct:621 Kit compile101 (factory/test imports),
75 authorized queued cancellation143 with no compile proof, f538 strict
Clippy101 (five doc identifiers and two unit-statement semicolons). All were
corrected as one complete final source batch; no lint suppression or assertion
weakening. The separate c41 F1 run ended View compile101 and no tests; its
repairs/current changed F1 run are not this isolated producer's evidence.
