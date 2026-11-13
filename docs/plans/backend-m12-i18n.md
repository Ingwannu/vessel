# Track L: the app's words as translation data

## Contract (slice 1)

ADR 0039 is the decision. Existing Rust area tables stay in place during
this slice; `pairing` is the first migration, in slice 2, because its two
small dialogs exercise short controls and long safety explanations without
changing the meaning of any node call.

[Decision Log]
- Purpose: adding a language must be a data-only change with typed callers.
- Existing assets and constraints: `Lang` switches all existing tables;
  every other active track still owns its table. `view` has no I/O or UI
  dependency and must build for WebAssembly.
- Alternatives: the current tables cannot provide language-owned plurals;
  an invented message parser would duplicate maintained Fluent tooling.
- Choice: the maintained Fluent parser at build time, generated typed
  accessors, and Fluent bundles with CLDR plural rules at initialization.
- Why: English defines the API; translated messages may have different
  selectors and word order without changing the caller. All plain values
  are retained after initialization and borrowed by accessors.
- Costs: more build dependencies and a runtime bundle per used language;
  measured below and in ADR 0039 before accepting the mechanism.

The new locale registry exposes tags, native names and completeness.
Selection is an override (for checking), else a preference, else the
system's tag: exact tag, its language, English. The pseudo-language is
only reachable by the override. `Lang` remains the legacy table switch.
No wire or node change. English and Korean remain complete for migrated
messages; partial languages fall back per message, not per area.

The coordinator clarified the UI seam: slice 2 adds a typed language
preference through the existing preference change path and a General
settings row with System first and native names. Changing it applies
immediately. The pseudo-language's three normal-screen captures belong
to slice 2; slice 1 proves its transformation in tests.

## References and deliberate differences

Paseo's `packages/app/src/i18n/locales.test.ts` supplies the selection
cases (explicit choice; Korean regions; unsupported and missing system
locale). Its resource tests check interpolation parity. The registry is
built from folders rather than their hard-coded language list, as ADR
0039 requires. The chooser uses native names only, as the coordinator
specified, instead of Paseo's native and translated name pair.

Paseo's `hosts/add-host/internal/pair-link-modal.tsx` and
`i18n/resources/{en,ko}.ts` define pairing's words and context. This
track changes no pairing behavior or layout.

## Slice 1: measurements and verification

The coordinator selected Fluent only after the following experiment.
ICU formatting dependencies, license rows and the experimental feature are
removed. Slice 1 is being finalized; no area table has moved. The following
historical rows precede the folder-owned formatting implementation; its
final release measurement is recorded separately below.

| Release variant (x86_64 Linux) | Installed executable bytes | Median first paint bounds (ms) |
| --- | ---: | ---: |
| Original app | 73,652,328 | 314.459–319.510 |
| Fluent alone | 74,050,200 | 246.470–254.741 |
| Fluent and ICU full-data experiment | 75,337,192 | 238.233–242.499 |

Fluent adds 397,872 bytes. ICU adds another 1,286,992 bytes (1.227371 MiB),
exceeding the approximately 1 MiB stop condition. Its default baked data
includes upstream locales beyond the embedded English/Korean/Japanese;
this does not satisfy the required final scope and does not establish the
minimum cost of ICU4X. The coordinator rejected ICU4X: a scoped-data generation workflow would
also break the folder-only language contribution contract. The selected
implementation uses standard NUMBER/DATETIME names, folder-owned rules and
whole date/time patterns, with the language's selectors.

The lower observed timings are not a performance-improvement claim: the
shared-machine/Mesa runs are noisy. All nine sample bounds are retained
with the measurement logs. Build durations were 21m19s for the original,
24m34s for Fluent, and 11m00s for ICU; caches and concurrent machine load
make those non-comparable as clean-build benchmarks.

The standalone wasm localization probe builds in both configurations:
Fluent is 7,649,599 raw / 2,281,727 stripped bytes; ICU is 9,731,510 raw /
3,209,874 stripped bytes, an additional 928,147 stripped bytes. These are
linked localization probes, not browser-app/download sizes.

### Module boundaries

- `view/build_support/`: discovery, Fluent AST inspection, reference and
  variable validation, coverage, and English API generation. It is a host
  build script and is also compiled into the gate tests and reporter.
- `view/src/text/localization/language.rs`: embedded tags, own names,
  completeness, and preference/system/override resolution. A preference
  cannot hold the pseudo-language; an unknown stored tag becomes System.
- `runtime.rs`: bounded process cache, bundles and per-message English
  fallback. Each language has its own bundle, including a separate English
  fallback, so missing Russian messages do not use Russian plural rules
  on English sentences. Both diagnostics are retained if both fail.
- Generated `Plain`: one retained value per variable-free message. Its
  getters are field borrows. A native-only allocation-counting gate test
  repeatedly accesses them and the initialized process cache.
- `grammar.rs`: Korean object, subject, topic, with and direction particles.
  Hangul syllables use their final consonant; 0–9 use Sino-Korean readings;
  Latin uses the final letter's Korean name (L/M/N/R closed, others open).
  Final ㄹ takes 로. Trailing punctuation is ignored; empty values and
  other scripts use the open form. The table test states these limits.
- `number.rs`: NUMBER retains its numeric type and plural operands; a
  display hook uses generated, checked folder-owned separators, group sizes
  and digit characters. No language branches or target I/O.
- `formats.rs`: DATETIME supplies typed Gregorian/local-clock parts to
  folder-owned whole patterns, including month/weekday names and h12/h24.
  An independent pattern bundle prevents DATETIME recursion. A missing
  whole pattern uses English's pattern, names, digits and clock convention.
- `build_support/format_rules.rs`: validates static rule values and the
  supplied date/time part contract; generates them as part of the ordinary
  build. There is no contributor regeneration command or checked-in generated
  Rust data.

The supported Fluent subset uses a key per context rather than attributes;
terms may bind their parameters locally; message references carry their
transitive variable contract. Cycles, missing references, unknown functions,
unknown message keys, and translated variables not passed by English fail
the same validator used by the coverage command. NUMBER selectors generate
numeric arguments; text selectors generate enums. Counts are never passed
as preformatted text. Calendar dates have a typed caller and are encoded
only at the Fluent function boundary.

### Measurement protocol

The coordinator authorized release measurements in this track's own target
directory. Three variants use the unchanged shipping release profile:
original app; Fluent alone; Fluent with ICU4X. Executables are split using
`objcopy --only-keep-debug` and `objcopy --strip-debug --add-gnu-debuglink`,
the release script's operations. No distribution artifact is written.

A private Xvfb and Mesa software Vulkan run with an empty, isolated app
configuration, no updater and no real node. A monotonic clock starts before
process creation; repeated image samples detect the first painted content.
Each sample retains the start of the last empty capture and end of the
first painted capture so polling uncertainty is visible. The fixed window
position allows reading the upper-left sidebar content directly through X11, without launching
a screenshot program per sample. Nine launches report the individual
brackets and median; these are software-renderer observations on a shared build machine,
not a claim about a customer's GPU or Windows startup.

Before migration, the pairing link dialog was captured and inspected in
English and Korean, both themes, 1280×800 and 390×844. The words fit in all
eight. These empty-book captures establish layout before migration; they do
not replace slice 2's checks against the fake fleet and its confirmation
step.


### Gate and remaining acceptance work

After restoring the negative probes, the complete native workspace passed
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and
`cargo test`: 332 ordinary tests plus two compile-fail doctests (unknown key
and missing argument). The wasm/coverage examples are included in the
all-target Clippy gate. No lint exception was introduced. Final release and wasm measurements also
pass, as recorded below.

The allocation test proves zero allocations/bytes for 10,000 iterations
of three plain getters and initialized cache access, in every embedded
language and the pseudo-language. Coverage lists Japanese by its native
name with one file/two supplied messages and 22 English fallbacks; English
and Korean are complete. Actual `cargo build` probes fail with E0599 for
an unknown generated method and E0061 for a missing variable. Removing the
Korean meta-system key makes the complete `cargo test` fail; the temporary
example was deleted and the Korean file restored byte-for-byte before the
final gate.
Pseudo-language tests pass; its three normal-screen captures are slice 2.

Two self-review findings were corrected: R (알) now takes the closed/rieul
forms alongside L, M and N, with table fixtures; the inventory uses Cargo's
all-target declarations (`--no-deps`) and walks normal local path edges,
including other workspaces. It does not filter to the host or fetch foreign
packages. Development/build declarations do not initiate traversal. Fixtures
cover dev-only, missing runtime, both roles, a different target, transitive
paths and renamed published packages. The original font inventory test and
registry-order comparison are retained.

The formatting limit is explicit: Gregorian dates and pre-resolved local
clock readings at minute precision, the folder's ten digit characters,
primary/secondary grouping, decimal separators, a small documented NUMBER
option subset, and patterns over supplied parts. No other calendars,
zone arithmetic, currency/percent/significant-digit formatter or automatic
numbering-system selection. Relative times are ordinary numeric messages
with Fluent's language-owned plural selection.



The standalone browser build initially stopped in the protocol's existing
UUID dependency: its native v4/serde features did not select a browser RNG.
A wasm-family-only UUID dependency activates the crate's maintained `js`
backend in `view`; no protocol, node, or native RNG changes. It adds no new
native runtime crate (UUID already arrives through the protocol) and is
registered in the existing license inventory.

The browser track confirmed GPUI already selects UUID `js` under
`cfg(target_family = "wasm")` in the full app. The local seam uses that same
condition so standalone `view` does not rely on GPUI's feature unification.
The lasting owner is `vessel-proto`, which generates ids: the browser track
will move this setting there when it integrates. This track leaves the main
workspace untouched.


## Final Fluent-only record

Installed executable: 74,116,816 bytes, 464,488 bytes (0.442970 MiB) above
the original app, and 66,616 bytes above the initial Fluent-only experiment.
Nine-launch median first-content bounds: 251.905–258.587 ms. Same release
profile, stripping and X11 protocol as the historical table; this is not
evidence of improved speed. Cached release build: 6m30s.

Standalone wasm localization probe: 8,092,410 raw / 2,426,698 debug-stripped
bytes; cached build 38.88s. It links the localization layer, including its
formatter initialization, without a UI or node. It is not the whole browser
app. ICU formatting packages are absent from the final dependency graph;
ICU normalization still arrives through the original URL stack; it is not
used by the formatting backend.

The final gate is format PASS, all-target Clippy PASS, full tests PASS
(332 plus two compile-fail doctests). The three real negative-build probes
fail with the expected causes; the catalog/example inputs were restored
before the gate. No root workspace source was changed, so this report does
not claim a main-workspace gate. All runs are local, with isolated config;
no service-account permissions or deployment were changed.

## Shared seams in slice 1

The following is the complete handwritten seam inventory against the base;
new engine/catalog/test files are this track's implementation. Native
Cargo.lock is generated by Cargo from the declared dependencies.

| File | Added/replaced lines | Purpose |
| --- | --- | --- |
| docs/ARCHITECTURE.md | 1031–1044 | Native localization boundary |
| native README.md | 19, 255–293 | Crate responsibility, staged workflow and limits |
| native Cargo.toml | 95–106 | Fluent stack, memoizer, existing UUID version, allocation-test tool |
| view/Cargo.toml | 22–45 | Runtime/build/test declarations and wasm-only UUID js seam |
| app/src/main.rs | 54–59, 85–94 | Prepare the chosen catalog before drawing; resolve override/system |
| view/src/lib.rs | 47 | Export localization |
| view/src/text/mod.rs | 29 | Register the module beside legacy tables |
| view/src/about.rs | 88–94, 124–125 | Six runtime license rows and inventory-test module |
| view/src/about/tests.rs | Entire new file | Preserve font/order checks; all-target runtime/development fixtures |

Cargo.lock's generated additions are lines 162–167, 1822–1866,
3436–3454, 6899, 7221–7229, 7253–7270, 7566, 7568–7571, 7575, 7577 and
9255. Existing package versions are unchanged.

The About test formerly at lines 118–178 moved to its own module. Its
font check is retained; only runtime classification/order fixtures changed.
The architecture's localization section and this workspace README describe
the staged engine and formatter limits. No preference/settings (track E)
file or area table is changed in slice 1.

## Next slice order

The coordinator clarified that the model is the pairing screen as a person
sees it: its thirteen table strings, pairing-only strings currently in the
lead's app table, and typed read-link errors move; shared strings stay with
their current area and are listed. The lead is first unifying the machine
wording. Slice 2 starts only after the coordinator supplies that commit and
it is merged; this avoids editing a table while it is being deleted. No
slice-2 code has been started on the older wording.


## Slice 2, first change: keep proof languages out of releases

[Decision Log]
- Purpose: a person must not select a proof language and receive English.
- Constraint: partial catalogs must still prove registry discovery, English
  fallback and coverage without weakening the folder-only contribution path.
- Choice: move the two-message Japanese file unchanged to a fixture root,
  with a small complete English/Korean source alongside it. Tests and the
  reporter's `--root` mode read that root; ordinary builds read only locales.
- Why: production and test inputs have explicit boundaries, without a hidden
  preview environment variable or a production-only language allow-list.
- Cost: a candidate must pass coverage and the native gate before its folder
  is added to the shipped source; all shipped catalogs must be complete.

The coordinator accepted slice 1 at 5834962, then required this correction
as the first feature commit of slice 2. The lead's b3c0f93 is merged first
(QR and Fluent additions both retained). No pairing wording is moved in
this correction. General's language row is reserved with E2 as its first
child, using a separate L-owned module/picker. S2's actively changing tables
remain untouched; the lead released About/updater/titlebar for later moves.

Merge preparation is commit ef8799d. Its native gate passed (392 ordinary
tests and two compile-fail doctests); the root fmt/workspace all-target
Clippy and testkit gate passed (14 tests). The merge index was frozen at
tree 141d7bdf81f728be3053ea324827cf47c7996243 while this native-only fixture
change was prepared separately; that exact tree is the merge commit.

This correction's full native gate passes: format, all-target Clippy with
warnings as errors, 394 ordinary tests and two compile-fail doctests. The
default report lists only complete en/ko, both zero missing. The explicit
fixture report lists en/ja/ko; Japanese has exactly two messages and lacks
meta-sessions/meta-time/meta-welcome, whose English fallback is tested.
Coverage uses the shared validator and validates numeric settings when the
candidate root includes the source formatting area. No new screen is
introduced by moving a test fixture. Root source is unchanged by this
correction, so its root gate is not repeated. The coordinator's machine-wide
build slot wraps subsequent cargo commands; it is not a host-specific path
in the public contributor script.
