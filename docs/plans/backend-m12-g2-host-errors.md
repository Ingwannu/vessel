# Shared partial-host errors in the schedule list

G4's shared Alert presentation is consumed selectively from `eb75a637` (source
`68fac599`, contract `22e9ddbc`). The resulting alert.rs blob is exactly
`943dd08278ed1706f77f76070c7348cbdf7c7d54`; the G4 cumulative plan/branch is
not merged. Existing Alert variants and sizes retain their renderer/defaults.

Purpose: the reference schedules-screen.tsx:334–347/400–417 partial-host notice
has neutral border1, radius12, padding12, sibling gap4, red300 normal Meta12,
and no icon. Ordinary Error/ExtraSmall alerts do not match that presentation.
The reviewed named `alert::host_errors(messages, tokens)` avoids an area notice
renderer or style override. Draw only for nonempty partial-host errors; initial
failed load keeps its own surface, and responsive outside margins remain local.

Verification: the integrated changed-crate all-target Clippy passed, and all
four selected Alert tests passed (0 failed, 129 filtered). G4's isolated source
and native gate disposition is preserved; this consumption adds no new whole
run under A14. Long/multiple hosts, English/Korean and both layouts/themes must
still appear in the final list's first-use captures. No UI pass is claimed here.
