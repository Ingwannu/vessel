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
pending; no compiler or runtime pass is claimed yet.
