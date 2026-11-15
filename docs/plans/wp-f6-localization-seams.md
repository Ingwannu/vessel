# F6: shared Fluent contracts for installation-owned resources

The plugins package owns per-installation text, lifetime and contributions.
The words track supplies two reuse seams inside view; neither creates a second
core translator nor imports installation resources into its static registry.
Reference behavior is the plugin client translation subscription and cleanup in
`packages/plugin/src/client/translations.tsx:54` and its message contract in
`packages/plugin/src/text.ts`. ADR 0079 owns native plugin contribution scope.
No reference translation assets are imported.

## Bundle configuration

[Decision Log]
- Purpose: plugin text must use the same number/date/particle functions and
  pseudo transform as the core while retaining its own resource ownership.
- Existing assets: concurrent Fluent bundles, embedded numeric/date rules and
  typed Number/Date argument bridges already implement the app contract.
- Alternatives: copying configuration drifts; extending the core registry leaks
  installation state into its process cache and loses generated-key guarantees.
- Choice: `localization::resources::bundles<R>(Locale)` prepares empty active and
  English bundles; `R: Borrow<FluentResource>` supports owned core resources and
  installation-owned `Arc<FluentResource>` without another dependency.
- Why: a validated Locale selects core-owned formatting data; no plugin message
  can replace the separate private DATETIME pattern bundle. Each fallback keeps
  English's own plural rules. Both bundles use the pseudo transform when chosen.
- Effects: the caller owns resource validation/addition, per-message fallback,
  value preparation/cache and disposal. Number/Date Fluent bridges are crate
  private and belong only at that argument boundary. No frame-time parsing,
  global plugin cache, file access or language RPC is added.

`resolution.rs` owns typed initialization/format errors and generic resolution;
`formats.rs` depends on that lower layer; `resources.rs` configures the bundles;
`runtime.rs` uses them for the unchanged core cache/generated API. Installation
callers retain their own owned-key error type: core FormatError's static keys
are not widened into a dynamic string API.

Tests compare Arc-backed NUMBER, DATETIME and Korean PARTICLE results against
whole core sentences for English/Korean/pseudo, English plural fallback for a
missing translated message, namespace isolation, an attempted DATETIME pattern
lookalike and release of the last installation resource Arc.

State: source and narrow checks ready. View all-target Clippy passed; all 21
localization tests passed, including the two new Arc-resource tests and retained
plain-getter zero-allocation, fallback, pseudo and language-selection tests.
Changed view code recompiled. Formatting, size/boundary/token checks passed;
there are no new size/token exceptions, dependencies or core catalog changes.
The consumer package's whole native gate and integration review remain owed.
The earlier preferred-system-language package was gated at exact `8a944a23`;
its gate/captures do not cover this follow-on. Package base 2 (`6036eeb8`) was
merged after that gate completed; its root crates remain unchanged.

## Pure argument/reference contract

`view/fluent_contract` holds only the maintained Fluent AST walk, validated
Fluent Name/Key types, Shape/Kind, reference expansion and typed ContractError.
The host includes that source directly; view tests compile the same source at
the target side. The plugin consumer promotes that test-only registration to a
crate-private module when it starts using it; no dead-code allowance or public
runtime-key API is introduced before there is a consumer.

Reference expansion produces a private-field `Contract` distinct from the raw
scanned `Shape`; comparison requires that checked type, so reference variables
cannot be skipped by calling the comparator too early.

The English argument set is compared with Exact coverage for core Korean and
Subset coverage for partial installation resources. The caller still owns
resource parsing, duplicate/entry/attribute rejection, key namespace and locale
presence/coverage policy; a shape scan does not replace those checks. Core
folder prefixes, context/native-name checks and released EN/KO completeness
remain in build_support and retain their fixtures. The shared module performs
no I/O and knows no core registry or plugin RPC/schema.

The existing reference depth limit is a named constant. Expansion memoizes a
completed subtree plus its height: duplicated DAG edges no longer cause
exponential re-expansion, and a later deeper path cannot use that cache to
bypass the depth limit. Tests include both depth boundaries, that cached-path
case, duplicate edges, cycles, missing references, unbound terms, conflicting
types and the English/translation variable/type contract. API generation uses
explicit escaped strings from the validated names; a retained pre-change
artifact is compared byte-for-byte after the host extraction.

The plugin owner retains resource byte/key/count bounds, fallback/cache/error
mapping, generation ordering, language refresh and subscription disposal.
Neither this pure seam nor its tests prove those plugin lifecycle flows.

Pure seam state: installed source; its narrow target/host checks and generated
API byte comparison are owed. The bundle-only checks above do not cover it.

## Operand and local-binding gate correction

A retained existing checker accepted three deliberately invalid candidates:
`NUMBER("text")`, a string bound to a NUMBER term parameter, and a string bound
to a DATETIME term parameter (all returned zero on explicit copied roots).
Source inspection of fluent-bundle's `resolver/inline_expression.rs` shows that
custom callback Error writes a function placeholder without adding a resolver
diagnostic. Thus an empty runtime diagnostics vector cannot validate operands.
This is not an observed shipped-language defect; shipped EN/KO uses supported
operands. It is a gate gap that matters when admitting installation resources.

The shared AST gate now requires a NUMBER operand capable of remaining numeric
(a typed variable, bounded numeric literal or nested NUMBER result), including
correct inference through inline placeables. Named term arguments retain their
literal kind: a numeric parameter needs a numeric literal; a Date parameter
cannot be supplied by resource literals. Dates stay typed caller values, so
translations never depend on their private Fluent-boundary encoding. Standard
function names/configuration stay unchanged; no parser or dependency is added.
Negative and positive cases run through both the target contract and host gate.
These corrections strengthen admission checks; they do not turn runtime
placeholder text into UI recovery or replace the plugin owner's failure path.

Package base 3 (`6c782cb6`) was merged as `a6c9d0c4`. Incoming node Quality B
and race corrections are preserved unchanged; this track adds no root/protocol
work. Current state: coherent source, formatting and static quality checked;
Clippy, target/host tests, generated API parity and consumer integration owed.

Package base 4 (`3cd0d3fc`, published/candidate app packages) was then merged
as `a4157f17` before submitting the committed-tree narrow checks. The root
crates/manifests/lockfile match that incoming base. Its only FTL difference
from the retained pre-extraction corpus is the already reviewed Korean PR
form-title wording; English API definitions and numeric/native-name settings
are unchanged. This merge is outside the earlier exact F5 approval.

The first committed-tree narrow run on `6cd6aa981` passed 900 view and 20 host
example tests (920/0/0). Its Clippy check reported five terminator/exhaustive-arm
findings; `90c2c6d32` corrects those. The next Clippy check reached test helpers
and found Result-returning fixture construction using assertions/unwrap. That
fixture builder is now separate from the Result-returning scanner; lint levels
are unchanged and both original failure logs are retained.

A typed Date must be read through DATETIME, including references/compositions;
otherwise its private encoding could be printed by a translated plain variable.
The scanner retains raw reads with lexical term bindings removed during
expansion, and the comparator checks translated reads against English Date
arguments. Nested PARTICLE(DATETIME(...)) and a locally bound text parameter
shadowing a Date name remain valid. Numbers may still use plain interpolation
through the shared numeric formatter. New positive/negative cases cover these
rules. The generated API at `6cd6aa981` was independently identified through the
executed lib test's dep-info: 98,908 bytes, SHA-256
`5407f5c425d160b7b1125c4a4eaa97cd70e6e7fc51cd3788f4083d83c645744d`,
identical to the retained old generator on the same corpus. Follow-up checks
remain owed for this final helper/date-read revision.
