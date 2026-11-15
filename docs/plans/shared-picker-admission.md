# Shared combobox opening admission

Base: announced package8 `91407173`. Isolated producer only; no F1 app,
HostFilter/Form/Content width, model browser, field, asset or node changes.
Coordinator assigned G2 sole picker writer after W2 R223 source P2-2 found
retained A handlers resolving the current B before the area choice guard.

[Decision Log]
- Purpose: a retained combobox frame must never choose, highlight, close,
  refocus or dismiss a parent on behalf of a different explicit opening.
- Existing assets: one ComboboxState, one combobox/rows renderer, canonical
  G4 ReturnFocus, and GPUI's capture/default-focus event paths.
- Alternatives: action-only guards run too late; counters can wrap; a second
  renderer or per-area event interception duplicates shared admission.
- Choice: a fresh private Rc identity for every explicit open/drill, retained
  only as Weak by callbacks. Admission requires open plus exact pointer identity;
  a retained Weak prevents allocation-address ABA without a generation counter.
- Impact: old signatures, geometry, width, animation/priority and row tests stay.
  Open/close/reset move to lifecycle.rs and keys to keys.rs for the size budget;
  picker allowance only lowers to its measured product count, never increases.

Outside/scrim, keys, title/back, row callbacks and the panel's implicit pointer
focus all admit before state, focus or action mutation. Stale recognized keys
are contained, particularly Escape captured under a still-mounted parent dialog;
unknown keys retain propagation. Stale pointer paths prevent default focus and
contain propagation. Callback-created B cannot receive old A's post-choice
restoration. Final choices/outside/Escape reuse the same weak focus helper;
intermediate/drill choices retain current focus. State-only close stays public,
with close_and_restore as the explicit actual-dismiss path. Replacing a mounted
state must acknowledge actual dismissal before dropping its opener cycle.

Authored GPUI regressions use real shared Dialog/TextField/combobox, real window
pointer dispatch and a post-pre-key-redraw interceptor. They exercise same-state
reopen, actual close/reopen, replacement state and drill with a retained A frame;
B's highlight/focus and parent survive; valid B selection/Escape and subsequent
parent Escape still work. Pointer includes desktop outside and compact scrim.
Additional paths cover weak-opener restoration, newer focus and intermediate
choices. These are authored tests until their exact remote outcome is recorded.

Non-compiling checks: direct rustfmt, native size/boundaries/tokens and diff
check. The sole authorized temporary picker-control priority label runs exact
Kit all-target Clippy and Kit lib tests once for this changed producer head;
715 ordinary F1 stays untouched. No new whole package or UI capture is claimed.
Native Linux focused-field F5, IME/physical input, consumer UI and W2 product
behavior are distinct evidence, not inferred from headless GPUI results.


## First isolated check: actual RED, not adoption evidence

Exact a1b1 run picker-control-20261115T143700Z-a1b1b607 ended fonts0,
Kit Clippy101/no tests. The extraction incorrectly inserted the cycle/opener
fields at the first open-argument declaration instead of inside ComboboxState.
The complete compiler batch also found two redundant test prelude imports,
an unnecessary mutable constructor binding and the test parse helper's required
static string lifetime. These are corrected together, without changing geometry,
weak admission, assertions or adding suppressions. This old run is preserved;
none of its source or green font step is corrected-source compile proof.

A local boundary script was also mistakenly treated as non-Cargo: inspection
showed it calls cargo metadata --no-deps --offline. It compiled nothing, but
still violated the all-Cargo-remote instruction. It will not be run locally
again; further boundary/quality checks run through the remote helper. Direct
rustfmt, source-size/token scans and Git checks alone remain local.


Exact eceae233 run picker-control-20261115T144243Z-eceae233 completed fonts0
and Kit Clippy101, solely combo_row becoming101 lines after admission. No tests
ran. Its complete typecheck batch had no prior compile errors. The correction
extracts the unchanged enabled-row binding into one helper, keeping paint/layout
and the same captured lease. No suppression, assertion deletion or minification.
Per coordinator the last priority is spent; any changed-source retry uses the
ordinary short lane, same temporary label, not another priority or whole gate.
