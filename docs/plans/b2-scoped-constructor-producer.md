# Minimal scoped composer constructor on package base13

[Decision Log]
- Purpose: allow the sole resident factory to construct its one retained TextField with a captured callback, without selecting or swapping global resources.
- Existing assets and constraints: canonical FieldId/Identified callbacks, the same Plain event listener, existing TextField factories, W12 SessionContext refresh and one SessionResources map. Session body rendering already lives in body.rs; the incoming parent cap is673.
- Alternatives: import the whole B2 branch, allocate a second editor and swap it, recapture selected globals, or raise the old size record. Those lose ownership or hide receiving-line changes.
- Choice: box a captured immutable Plain submit callback once in the same listener, extract only SessionView construction into resource.rs, and add with_composer plus literal cfg(test) new_scoped. Identified and source/IME registry stay unchanged. Keep the existing bare Composer Enter propagation stop so native fallback cannot insert a leading newline after submit.
- Effects: current local input refreshes only captured resident+claim and requires Visible/Focused; the field, selection and draft survive local moves. No network authority, registry, delivery manager or product resource cutover is added.

This portable cut is based on announced2ebeedf9. body.rs, Open/Pending, follow, submit and lifecycle bodies stay byte-identical to that baseline. Session root shrinks673→639; no cap is raised. The existing800px exception transfers from parent to child, with the total unchanged. Plain callback capture and the guard use the current canonical field modules; no old unsplit field body is restored.

Prior exact2fb77abf4a0efa11da3edddef99af0d46a443abb had four-native-crate all-target strict0, actual App110/0/2 and Kit215/0 in runs wp-composer-scoped-constructor-20261115T182201Z-2fb77abf and wp-composer-scoped-tests-20261115T182201Z-2fb77abf. Its original fb83 execution failed on an extra leading newline; the guard correction retained the literal draft assertion. The resource constructor and sequential scoped test here are byte-identical to corrected2fb; those prior results do not prove this receiving graph. New exact receiving strict/runtime checks are required.

Scope excludes Surface, SubmitPress, Markdown, attachments, retention snapshots/Hold adapters, live minter/resource attributes, selected-route fallback, global clone/swap, network leases and product resident promotion. Removing cfg(test) belongs only to W12's coordinated union after its own validation. Existing source/identity/multiline/form tests remain unchanged except the additive captured-Plain-submit test.
