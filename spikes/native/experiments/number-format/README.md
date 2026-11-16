# Number-format experiment inputs

These files are an ADR 0101 experiment, not an installed runtime patch.
No manifest names them, no path override is active, and no product lockfile
has changed. Only the build-tool owner's controlled offline scratch runner
may materialize the candidate graph for measurements.

## Custom-number hook candidate

`fluent-0.16.0-custom-number.patch` applies to the unpacked upstream
`fluent-bundle` 0.16.0 archive named by `fluent-hook-manifest.json`.
The archive checksum matches the existing native lock entry. The original
crate remains MIT OR Apache-2.0; this patch does not replace its notices.
The patch touches four existing files: 55 added lines and two removed lines.
It adds no dependency, parser, numeric algorithm or plural-rule table.

- Custom values may return a typed value with NUMBER options applied,
  without the caller downcasting its representation.
- Literal/keyword matching may delegate to an opt-in custom value, through
  the corresponding ordinary/concurrent memoizer. Defaults remain false.
- Existing String/Number comparisons and the native float/plural path remain
  unchanged. The patch alone supplies neither exact arithmetic nor a formatter.

Check the patch against a fresh unpacked archive before applying it. Verify
every original and resulting file against the manifest. Source copies used
for subsequent builds must have current modification times; an old artifact
is not evidence that this patch compiled.

### Upgrade cost

This is a standing four-file upstream patch, not a one-time free adapter.
At each Fluent upgrade:

1. Verify the new archive, notices and dependency graph before any override.
2. Re-apply/check the four contexts. Review custom-value object safety,
   selector operand order, resolver matching order and both memoizer kinds;
   a context match alone does not prove semantics.
3. Re-run stock numeric/text/custom controls and actual exact-count tests,
   including literal precedence, nested NUMBER options, cardinal/ordinal,
   fallback, concurrent use, and the portable build.
4. Re-measure installed code/data cost when upstream or data changes. Remove
   the patch if upstream provides the required exact numeric hooks.

On 2026-11-16, a source-context check also applied unchanged to upstream main
`0f87afbbc3fe7d20c61a3fc75cb5d0e123aefdda` (2026-09-27). That manifest still
reports bundle version 0.16.0. There is no measured next-release rebase here:
this is an unreleased-source apply check, not compilation or conformance.

## Cheapest no-vendor alternative to evaluate

Keep a typed exact unsigned scalar at the view boundary and use a maintained
plural engine in an explicit `PLURAL($count)` grammar function. The standard
`NUMBER($count)` function prepares exact display text; Fluent then selects on
the category returned by PLURAL rather than receiving a rounded float. This
can avoid an upstream fork and preserves language-owned whole sentences.

That alternative has costs too: the English contract must infer an exact
unsigned argument from PLURAL, combine it with NUMBER uses, and reject an
unsupported bare numeric selector instead of silently misinterpreting it.
Plugin contract validation/function policy must share that rule. Native
numeric-literal override syntax is not automatically preserved by a
category-only function; a separate exact comparison capability or maintained
AST lowering would be necessary where messages need it. Adding an ad hoc
number parser, lossy numeric fallback or a per-area formatter is not allowed.

This comparison is not an implemented alternative or a recommendation yet.
The numeric kernel, actual hook tests, exact alternative, dependency graph,
release/wasm bytes and compile-time deltas remain to be evaluated before the
coordinator chooses a product design.
