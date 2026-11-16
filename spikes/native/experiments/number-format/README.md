# Number-format experiment inputs

These files are an ADR 0101 experiment, not an installed runtime patch.
No manifest names them, no path override is active, and no product lockfile
has changed. Only the build-tool owner's controlled offline scratch runner
may materialize the candidate graph for measurements.
`measurement-contract.json` fixes the requested graphs, data/profile/strip
scope and required evidence. It is the measurement specification. The fixed feasibility selector is
installed; phase 2 still requires actual reachable app/example overlays.
Neither selector installation nor an unlinked source is a measured result.

`decimal_probe.rs` and its child tests are unlinked candidate arithmetic
sources for that controlled runner. They reuse `FiniteNumber`, maintained
round-trip decimal conversion/rounding, and an independent exact unsigned
carrier. Compact scales are validated/injected data, with no language branch
in the arithmetic. A carry recomputes from the original value to avoid double
rounding. Tests compare arithmetic against the retained Intl corpus and
reject malformed scale data. They do **not** yet test grouping, currency
patterns, plural selection, Fluent integration, artifact reachability or
startup, and have not compiled. Normal View checks cannot cover unlinked
experiment files. No production public API or backend choice follows from
these sources.

`fluent_hook_probe.rs` is a companion integration-test input for the patched
upstream crate, not an app module. The controlled runner must register it as
an explicit upstream test target if the archive disables test autodiscovery.
Its two memoizer tests check actual selector dispatch, nested option merging,
literal/category order, ordinary numbers and default non-numeric custom
behavior. Its category mock deliberately contains no language rules: passing
it would prove hook mechanics, not exact plural arithmetic. It is also
uncompiled at present. The earlier missing selector-loop gate would fail its
positive direct-selection assertion.

`plural_probe.rs` is a separate test-only module for the ICU candidate graph.
It passes exact `u64` directly to maintained `category_for`, including English
ordinal, Russian/Arabic last-digit cases and Spanish/French/Portuguese million
divisibility neighbors. It also characterizes why shipped EN/KO cardinal
categories do not change at large unsigned values. It is uncompiled and
unregistered; no extra language ships and no native/plugin selector has been
connected by preparing these tests.

`icu_decimal_probe.rs` is the larger candidate's unlinked grouping/compact
adapter and has child corpus tests. It uses the same maintained rounding and
compact-carry code through a typed scale-provider trait; ICU supplies the
exponent data and actual localized output. Exact unsigned formatting is
compared to the BigInt oracle. USD tests compare numeric bodies only, with
currency patterns still owed. The comparison graph explicitly enables
`icu_decimal/alloc`, required for its string-output API. These sources are
not native app overlays, reachable-artifact evidence or compiled results.

`feasibility-inputs.json` pins the candidate sources and the two scratch-only
test overlays. The common overlay adds only dev dependency requests and
explicit `cfg(test)` imports to view; the ICU overlay adds its dependency and
module. Both apply to a temporary index without changing product sources.
Only the controlled runner may resolve the isolated offline lock and run the
named tests. This phase cannot produce native installed-byte deltas: actual
app callback overlays and a reachable release/wasm entry are a later packet,
not implied by the test registrations or a passing filtered test count.

`placement_probe.rs` and its child tests prepare the later narrow backend's
digit placement over `UnsignedDecimal`: injected grouping widths/separators
and ten digit symbols, ordinary/disabled/compact-minimum-two grouping, and
full unsigned digits. There is no float conversion, parser, rounding or
language table in it. It is not registered by the frozen stage-1 descriptor
and remains uncompiled; its source/tests must be included in a later exact
candidate check, not credited to stage 1's eight/ten tests.

`patterns/{en,ko}/number-cost.ftl` prepares whole quantity/USD/compact sign
and unit patterns. It is outside the embedded locale root. Compact scale
choices and suffixes live in those files; a later candidate must read them
with the maintained Fluent AST and preserve whole English fallback (including
the English numeric body/scale). These resources are not yet parsed by a VM
check or wired to a callback. Do not count source preparation as reachability
or compare the unlinked files' byte count with an installed binary.

`pattern_probe.rs` prepares maintained-AST scale extraction and whole Fluent
pattern rendering; its compact category is an explicit six-variant choice,
computed from a numeric coefficient by the maintained plural engine. The
default body is a complete amount reference, allowing a contributed language
to specialize a category without joining fragments in Rust. `kernel_probe.rs`
combines arithmetic, placement and those patterns for a single locale, with
whole-corpus tests, unsigned glyph/category tests and signed-zero presentation
keys. Both remain unlinked/uncompiled and outside the frozen stage-1 graph.
They do not yet supply app callback reachability, a wasm entry, current
generated-message unsigned arguments, or a consumer cache.
Those integration requirements and all dependency/adoption gates remain open.

The later `fallback_probe.rs` source adds whole numeric-pattern fallback and
an English-baseline completeness check. A partial source may keep its own
quantity while missing USD/compact messages reprepare the English digits,
scale and pattern together. Its two child tests are unlinked/uncompiled,
not part of stage 1. Fallback for whole app messages and their generated
unsigned arguments is still a separate open integration requirement.

## Custom-number hook candidate

`fluent-0.16.0-custom-number.patch` applies to the unpacked upstream
`fluent-bundle` 0.16.0 archive named by `fluent-hook-manifest.json`.
The archive checksum matches the existing native lock entry. The original
crate remains MIT OR Apache-2.0; this patch does not replace its notices.
The patch touches five existing files: 56 added lines and three removed lines.
It adds no dependency, parser, numeric algorithm or plural-rule table.

- Custom values may return a typed value with NUMBER options applied,
  without the caller downcasting its representation.
- The resolver admits a custom value to the selector loop. Literal/keyword
  matching may then delegate to an opt-in custom value, through the
  corresponding ordinary/concurrent memoizer. Defaults remain false.
- Existing String/Number comparisons and the native float/plural path remain
  unchanged. The patch alone supplies neither exact arithmetic nor a formatter.

Check the patch against a fresh unpacked archive before applying it. Verify
every original and resulting file against the manifest. Source copies used
for subsequent builds must have current modification times; an old artifact
is not evidence that this patch compiled.

### Upgrade cost

This is a standing five-file upstream patch, not a one-time free adapter.
At each Fluent upgrade:

1. Verify the new archive, notices and dependency graph before any override.
2. Re-apply/check the five contexts. Review custom-value object safety,
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

A no-vendor alternative preserving the existing authored syntax is to lower
numeric expressions using `fluent_syntax`'s maintained AST and serializer.
It must separate numeric payloads from final display at explicit private
function boundaries, rather than globally guessing whether arbitrary text
is a number. It also must preserve nested NUMBER rounding/options and term
arguments. Numeric selectors would carry their ordered literal/category
variants to an exact matcher; that matcher uses the maintained plural engine
and returns a private selection identifier. Transforming only the category
or always preferring a literal would change Fluent's existing ordered match
semantics. The same lowering policy would have to serve core build resources
and W16's validated plugin resources. Extra initialization parsing must be
measured, and plain getters must remain pre-resolved field borrows.

| Design | Upstream patch | Translation/API cost | Evidence still owed |
| --- | --- | --- | --- |
| Typed custom-number hooks | Five files, +56/-3 | Existing authored NUMBER/select syntax retained; private numeric adapter required | Both memoizers, all options/literal precedence, upgrade and artifact costs |
| Explicit PLURAL | None | New exact-count inference/function policy; native literal overrides need an additional design | Complete expressivity, shared plugin contract, actual implementation/cost |
| AST lowering | None | Existing authored syntax retained; shared compiler lowering and private typed payload protocol required | Nested/term/literal equivalence, prepared-cache/startup and artifact costs |

The patched resolver source confirms that matches are tried in variant order.
An exact candidate must preserve that order, including a category preceding
an exact numeric literal. No candidate may treat a formatted digit string as
the operand used by its plural engine.

This comparison is not an implemented alternative or a recommendation yet.
The numeric kernel, actual hook tests, exact alternative, dependency graph,
release/wasm bytes and compile-time deltas remain to be evaluated before the
coordinator chooses a product design.

The first source-only artifact omitted the resolver's earlier String/Number
eligibility gate, so adding a match hook alone could never dispatch it. The
corrected artifact includes that fifth file. Neither artifact has compiled;
preserve that correction when citing the source-context apply result.

## Later carrier preparation (not in the frozen stage-1 graph)

`options_probe.rs`, `kernel_prepare_probe.rs` and `carrier_probe.rs` prepare
the opt-in hook carrier. Validated NUMBER options operate on a maintained
decimal; rendering and plural operands share the same rounded value. Source
and English displays/categories are prepared before binding, and an
infallible memoized namespace selects between them. Compact preparation
retains its scaled amount, so nested NUMBER does not turn 10K into 10. Four
carrier integration controls cover exact unsigned digits/categories, English
ordinal fallback, ordered literal/category matches, nested rounding and
per-language compact scale. These modules/tests are unlinked and uncompiled.
They are not included in D633's eight/ten-test descriptor.

The first D633 narrow run stopped after offline metadata at an unapproved
existing dependency edge (`icu_locale_core`); it attempted no compilation.
The exact graph delta and original red receipt are retained by the build
tool owner. No retry or ICU run has been submitted on that uncorrected policy.
See ADR 0101 for the full edge list and the separate stock-carrier control.

## Phase-2 source overlays (not a successful measurement)

`measurement-existing.patch` adds only the fixed standalone View example to
base13. `measurement-narrow.patch` and `measurement-icu.patch` prepare scratch
normal dependencies, the checked five-file Fluent path override/exclude, real
Number/Translator binding and callback, generated numeric arguments, and the
EN/KO numeric pattern files. Product source/manifests/locks remain unchanged.
Both candidates share arithmetic, option handling, carrier and fallback;
`backend_contract_probe.rs` separates the folder-owned backend from ICU's
maintained decimal/compact formatter. ICU owns its digits/grouping/compact
symbols rather than treating the folder's overrides as its data source.
That difference and its compiled data cost belong in the eventual decision.

The candidate example feeds exact unsigned input and each of the three typed
amount policies through an existing generated getter and the registered
NUMBER callback. The baseline example uses only its existing u32 API. The
wasm artifact is a standalone probe, not the full app or browser runtime;
its entry bytes differ explicitly because baseline lacks the wider API.
`runtime_binding_probe_tests.rs` adds whole oracle sentences, exact unsigned
sentences, actual core English fallback, and signed-zero/unit keys. Source
and English compact operands retain their exponent for maintained CLDR rules;
the numeric payload itself remains lossless. The backend's u8 compact exponent
bounds folder scale declarations to255, not finite numeric input magnitude.

These overlays passed temporary-index apply checks and Rust formatting only.
They have NOT compiled, run tests, passed source/V3 review or produced release
artifacts. A readiness packet and every measured value remain separate from
these prepared patches. The frozen D633 feasibility inputs remain pinned to
their earlier source commit; none of these additions is counted in its8/10.
