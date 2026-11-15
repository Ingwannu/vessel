# F6: shared Fluent contracts for installation-owned resources

The plugins package owns installation text, resource bounds, contribution
lifetimes, refresh and publication. The words track supplies two crate-private
view seams. Neither extends the static core registry or creates a second core
translator. The reference is the plugin client translation subscription/cleanup
in `packages/plugin/src/client/translations.tsx:54` and its message contract in
`packages/plugin/src/text.ts`; ADR 0079 owns the native contribution scope.
Reference translation assets are not imported.

## Bundle configuration

[Decision Log]
- Purpose: owned plugin resources need the same NUMBER/DATETIME/PARTICLE and
  pseudo behavior as the core, with independent resource ownership.
- Existing assets: concurrent Fluent bundles, embedded numeric/date rules and
  checked Number/Date arguments already implement that contract.
- Alternatives: a copied formatter drifts; loading plugin keys into Translator
  leaks installations into its process cache and loses generated-key guarantees.
- Choice: `localization::resources::bundles<R>(Locale)` returns empty localized
  and English fallback bundles, with `R: Borrow<FluentResource>` for owned core
  resources and installation-owned `Arc<FluentResource>`.
- Why: validated Locale chooses core-owned data; plugin lookalike message keys
  cannot replace the private DATETIME pattern bundle. English fallback retains
  English plural rules. The pseudo locale transforms both bundles.
- Effects: callers validate/add their resources, prepare values on data/language
  events and own cache/disposal. No dependency, I/O, locale RPC or public
  runtime-string-key API is introduced.

`resolution.rs` owns typed initialization/format diagnostics and generic
resolution below configuration. `formats.rs` uses that lower layer;
`resources.rs` configures bundles; `runtime.rs` uses them for the unchanged
static core cache/API. Number/Date Fluent bridges are crate private and belong
only at the argument boundary. Core FormatError retains static keys; plugin
consumers retain their own owned-key error type.

## Pure source contract

`view/fluent_contract/` is shared source for the maintained Fluent AST walk,
validated Name/Key, opaque Shape, checked Contract and typed ContractError.
It contains no folders, registry, filesystem, environment or toolkit types.
Build tooling includes it directly. A test-only view root registration compiles
it on the target side; F6 promotes that one registration to crate-private when
its production consumer lands, with no unused-code allowance.

The flow is parse/index entries → scan Pattern into Shape → expand references
into Contract → compare English and translated Contracts. Private fields make
comparison of an unexpanded Shape a type error. Expansion checks missing/cyclic
references, incompatible types and local bindings. Its named depth limit is 64;
memoized subtree height preserves that limit on a later deeper path and avoids
exponential traversal of duplicated DAG edges.

Core folder/prefix/context/native-name and released English/Korean completeness
rules remain host policy. Comparison uses Exact variables for core Korean and
Subset for partial installation translations; neither permits variables or
number/date requirements the English caller does not supply. Plugin consumers
still own parsing, duplicate/entry/attribute rejection, namespace, mandatory
English and resource byte/key/count bounds. They must publish message keys,
not expose arbitrary core or term keys as a new app API.

## Function operands and Date encoding

A retained pre-extraction checker accepted three intentionally invalid copied
catalogs: NUMBER with a string operand, a string bound to a numeric term
parameter, and a string bound to a Date term parameter. Upstream Fluent writes
custom callback Error as a function placeholder without adding a resolver
diagnostic (`resolver/inline_expression.rs`). Thus an empty runtime error vector
cannot validate those operands. No shipped-language defect was observed.

The shared gate requires numeric NUMBER operands, retains named term literal
kinds and rejects incompatible numeric/Date bindings. A Date stays a typed
caller value; translation literals never depend on its private encoding.
Every Date read must pass through DATETIME, including message references.
Nested PARTICLE(DATETIME(...)) is valid. Expansion removes lexically bound raw
reads so a local text parameter shadowing a Date name remains independent.
Numbers retain plain interpolation through the numeric formatter. The standard
function names and their implementation are unchanged.

## Verification and integration boundary

Exact `7f94630e1237dea19f50362b9667c6a7154b6ffd`, on package base 4 (`3cd0d3fc`)
merged as `a4157f17`, passes:

- Narrow run `i18n-narrow-20261115T085743Z-7f94630e`: 902 view library tests and
  22 host example tests, 924 passed, zero failed/ignored. Both steps exited zero.
- Narrow run `i18n-narrow-20261115T091030Z-7f94630e`: view all-target Clippy with
  warnings denied; both steps exited zero. Changed view code recompiled in both.
- Formatting and static quality: 597 Rust files, 14 existing size exceptions;
  boundaries pass; token counts 417/74/45 in 96 existing entries, none new.
- Independently identified generated API via executed lib-test dep-info:
  98,908 bytes, SHA-256
  `5407f5c425d160b7b1125c4a4eaa97cd70e6e7fc51cd3788f4083d83c645744d`,
  equal to the retained old generator on the same current corpus. The incoming
  Korean PR-title value change does not affect generated API definitions.

Tests cover core whole sentences, zero-allocation warmed plain getters, Arc
resource release/namespace isolation, independent English plural fallback,
private date patterns, pseudo, graph depth/DAG/cycles/missing references,
variable/type mismatches, invalid operands/bindings and safe Date composition.
The two earlier Clippy failures (terminator/exhaustiveness, then Result-returning
fixture construction) remain recorded; their fixes use no lint exception.
Two earlier local waiters were stopped before compilation for source/base
transitions; they are not compiler results.

Disposition: source and narrow checks ready; the coupled F6 package's ONE whole
native gate and review remain owed. No current app/UI/plugin lifecycle,
publication/generation ordering, subscription disposal, installed-size/startup
benchmark, browser/Windows/real-GPU/accessibility or current wasm execution proof
is inferred. The earlier exact F5 preferred-language gate/frames are separate.
The root crates/manifests/lockfile match the incoming package base; this track
adds no node/protocol work. Contributor steps remain in the locales README.
