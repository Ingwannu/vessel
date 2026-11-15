# Bounded generated plain caches

W5's Usage domain has 132 plain messages. One initializer per area made its
generated resolver exceed Clippy's 100-line limit; adding more domains can
also exceed the root cache's limit. This producer correction is separate from
the frozen General translation package.

[Decision Log]
- Purpose: let translation catalogs grow without changing public typed getters
  or weakening the function-length check.
- Existing asset: the English-driven generator already prepares each plain
  message once, owns its boxed string, and exposes a borrowed getter.
- Alternatives: lint allowances hide the problem; runtime maps introduce
  lookups and fallible keys; hand-splitting catalogs moves implementation
  constraints into translators' files. No new parser or dependency is needed.
- Choice: reuse the generator and standard slice chunking to emit private,
  typed cache trees. One named bound limits every node to 64 fields. Recursive
  grouping bounds parent resolvers too, including a registry with many areas.
- Impact: small layouts retain their emitted storage and getter expressions;
  large layouts add private field projections. Messages still initialize once
  in the same order, propagate the first error, and return borrowed strings.
  Nothing parses, formats or indexes a collection when a getter is called.

`build_support/generate.rs` emits the public API.
`build_support/generate/plain.rs` owns cache layout and getter projections.
`build_support/generate/tests.rs` exercises that same emitter with synthetic
132-key, 4097-key and 132-area catalogs, plus an area resembling a private
node name. It invokes real Clippy and Rust compilation, executes every getter,
checks retained addresses and resolution counts, and verifies an initialization
error across a chunk boundary. A deliberately oversized function must fail
Clippy first, so an inactive length check cannot produce a false pass.

The compiler subprocesses run within the remote Cargo test job. They require
the toolchain's Clippy component, use temporary standard-library-only fixtures,
and introduce no shipped dependency. Run the localization contract test and
the native strict Clippy step through the prescribed remote gate. The W5
package's final gate also checks its actual Usage catalog and callers.

The first narrow run, `i18n-narrow-20261115T142509Z-af9f7c87`, passed fonts
and failed Clippy on two discarded projection-map insertion results; tests
did not run. The insertion results are now explicitly retained, including the
same pattern in the fixture. Corrected narrow and W5 integration results are
pending; no compiler or runtime pass is claimed yet. The corrected run
`i18n-narrow-20261115T143045Z-b1dd1a48` then found an ambiguous fixture-map
value type after changing its default insertion. The fixture now declares its
string map explicitly. Further validation uses the ordinary lane.

The deep fixture's getter/key tables are shared statics iterated by reference,
so the regression neither copies large const arrays nor depends on Clippy's
large-const-array policy. The obsolete queued fixture check was superseded
before taking a slot; that cancellation is not a compiler result.

Run `i18n-narrow-20261115T145206Z-b92a3bf3` passed remote quality and failed
Clippy on three discarded command-builder references in the test helper.
The helper now threads those returned references into the final process call.
Its regression has still not executed; corrected ordinary validation is owed.

## Canonical Shape-line packet

Run `i18n-narrow-20261115T152755Z-123e60a0` later passed quality,
strict Clippy and native tests (1452 passed, zero failed, two ignored),
including the real compiler regression above. That tree used a checked
`Contract` parent; its result does not validate the line's `Shape` representation.
V4 retained that evidence under HOLD rather than approving line integration.

The canonical packet starts at the announced green line `2b806402`. It carries
the five producer commits followed by a mechanical representation adaptation:
the line keeps its existing `Shape` alias, string variable names and dynamic
argument emitter. Only the new plain-cache module reads `Shape.variables`
instead of a checked-contract accessor. There is no second cache producer,
new contract validator, F6 change, General migration, catalog edit or dependency
change in this packet. This preserves the line's existing validation boundary
while sharing the same bounded cache algorithm with Usage and Schedules.

The exact adapted tree still owes remote strict Clippy, native tests and the
synthetic compiler regression before V4 review. Consumer package compilation
and combined-line validation remain separate evidence.

## Exact Shape validation

The adapted source is `2407b6932`, six straight producer commits on
`2b806402`. Run `i18n-narrow-20261115T160707Z-2407b693` completed all four
selected steps (fonts, quality, strict native Clippy and native tests) with
zero exit status and no skip. Independently summed raw results are 1465 passed,
zero failed and two ignored: 1463 ordinary tests and two compile-fail doc tests.
The named large-cache compiler regression executed successfully; View, Kit,
Link and App actually recompiled. This is the adapted Shape tree's evidence,
separate from the earlier checked-Contract run.

The final three generator/test source files match the previously supplied
Shape compatibility patch byte for byte. The dynamic argument emitter and
registry helpers match the line baseline byte for byte. This packet contains
no dependency, lock, catalog, runtime formatter, General or F6 validator change.

This is a four-step producer suite, not a six-stage native whole/export. V4
review and the lead's combined-line gate are still pending. Actual Usage and
Schedules consumer package checks remain their owners' responsibility. No new
UI capture, release-size measurement, browser/wasm/Windows execution or plugin
validator approval is claimed. Source work and static checks ran as the
repository owner; compilers and test subprocesses ran only in the build VM.
No service permissions or live data were changed.

V4 returned the adapted packet on T9: its generated lexical namespace was
selected by opaque boolean arguments. `Namespace::{Current, Parent}` now
names that choice at every storage/initializer call. The fixture's compiler
and catalog-layout selectors use named private enums for the same reason.
These substitutions preserve the emitted text, case counts, compiler modes
and all assertions. Exact changed-source strict and compiler-regression
validation is owed; the earlier 1465/0/2 run is retained as prior evidence.

## Consumer strict-lint gap

G2's actual Schedules compile, run `schedules-20261115T171745Z-d092330e`,
reached a three-child generated root and rejected the common `child_` field
prefix under `clippy::struct_field_names`. The author compiler fixture enabled
the length check but omitted the project's pedantic lint group, so its earlier
green result did not cover this naming rule. No consumer code or suppression
is the fix.

New split nodes use private typed tuple slots, including their leaves. Fixed
`.0`/`.1` field projections are checked by Rust; they are not collection indexes
or runtime lookups. This models ordered cache slots without artificial names
and avoids forcing similarly prefixed translation keys to obey Rust's private
field-name lint. Small named layouts and the public getter API stay unchanged.
The named namespace enum remains in place.

The same compiler regression now denies Clippy all, pedantic, field-name and
length lints. A three-field bad-prefix control must fail with the actual naming
check before the existing large/deep/many-area cases compile and run. Original
value, pointer, one-resolution and first-error assertions remain unchanged.
The obsolete queued `afa790f4` check was verified queued and cancelled before
taking a slot (143), with its records retained. Exact corrected validation and
V4 review are owed; the prior 1465/0/2 result is not borrowed as new evidence.
