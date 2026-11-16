# Shared form text and MD textarea

[Decision Log]
- Purpose: pinned Paseo1c00 cadence-editor.tsx106–131 uses mono for raw cron;
  schedule-form-sheet.tsx1040 uses96 minimum with ordinary MD form chrome.
- Existing assets: canonical FieldId, Plain/Identified callbacks/listen, InputState
  and TextareaState, FormMinimum and the lead's presentation/styles split.
- Choice: additive Body/Code form text and Compact/MD textarea presentations,
  through the existing identified factories. Body and Compact remain defaults;
  FieldId, Enter/IME/Editability/SendOn/cap/minimum semantics stay unchanged.
- Why: area font/spacing wrappers or a second editor would separate appearance
  from canonical input identity and focus. MonoBody14/20 is canonical Code;
  MD uses shared ControlSizing vertical geometry5/11, x12, existing32/44 shell.
- Impact: no old caller migrates; only explicit constructors opt in. Literal
  tests cover both schemes/layouts/defaults; a real Code engine test retains
  source identity and typing/submission history. Masked/secret entry is NOT
  implemented by Code and needs a separate reviewed privacy presentation.

Isolated on announced2ebeedf9, only eight Kit paths and this plan; no F1 App,
select/picker, form-field frame, Link, node or area delta. No execution/source
approval is claimed here. Exact narrow checks and V1 remain owed; no native
font/OS/IME/consumer/full F1 proof is inferred from authored tests.

## Exact completed narrow receipt

Exactc81ab50a5a0115a2f05037b67457b9210faf3cc0:
schedules-20261116T004606Z-c81ab50a locked Kit all-target Clippy0 and
schedules-20261116T005345Z-c81ab50a locked full Kitlib246/0/0. Both are ordinary
SHORT priority0, fonts/cargo0/no skip, immutable exact checkout and actual Kit
Checking/Compiling. Three new Body/Code/MD literal and actual source-identity/
typing/submission cases explicitly executed ok. Author verified all collected
file hashes and four step digest prefixes. Same-row V1 execution closure is
requested; no App/F1/secret privacy/native font pixels/OS IME/consumer proof is
inferred. The older multi-crate F1 reds are separate evidence, not overwritten.
