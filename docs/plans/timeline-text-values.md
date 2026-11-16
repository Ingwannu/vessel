# Generic ECMAScript text edges

[Decision Log]
- Purpose: question/custom/allow-empty clients and project/native command edges
  must agree on BOM versus NEL without each copying a whitespace table.
- Existing assets: the canonical owner already has trim_ecmascript and exhaustive
  TC39-set tests. Project text had a private equivalent and native commands had
  another explicit table. Rust trim differs on FEFF and 0085.
- Choice: export the existing generic trim helper and use its one private
  predicate for native command splitting too. Delegate project text to it.
- Consequence: no provider parser, form field, RPC, admission or authority is
  introduced. Interior strings and existing parser bounds/errors stay unchanged.

The root helper returns a borrowed slice. FEFF is removed at edges; NEL remains
content. Its original exhaustive whitespace/interior/non-whitespace assertions
are unchanged. Native parser/objective fixtures retain case, whole arguments,
control/extra-field refusal and byte bounds. Project fixtures remain unchanged.
Full owner question/custom validation and the header decoder already use this
root helper; this cut attaches neither to the legacy form.

This normal carrier follows the approved 3f standalone values and announced
bc9 node base. Only its final helper commit is the new producer delta. Complete
text.rs, native_commands/parse.rs and workspace/project_text.rs match the common
owner. New root export/module wiring and this documentation are receiving-only.
The earlier 3f source-blob manifest remains an immutable predecessor checkpoint;
it is not a claim that the consolidated native parse blob is unchanged.

Before consumption: exact changed-source strict, pure tests and root vesseld
build, bounded source review and checked integrator adoption. No priority is
requested. This adds no wire type/field/method, status, Store/schema, runtime,
configuration, positive action or app dependency.

## Consolidated complete-source manifest

| Path | SHA-256 |
| --- | --- |
| crates/vessel-proto/src/text.rs | e1ad4341a33c9b9e3cf550d286f51c1fd6fdfd29e253679b7861218857890d27 |
| crates/vessel-proto/src/native_commands/parse.rs | 8e57cef5ddda0f61dc8272854feb96648161a33fbd061d24ef857417543a92e0 |
| crates/vessel-proto/src/workspace/project_text.rs | 2ab304418af5427b43b96c0822dbdce57ed2253c36dc36d7e5ce04bd23172cab |
