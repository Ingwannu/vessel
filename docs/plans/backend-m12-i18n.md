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
migration retains the two-dialog layout. The later scoped empty-name and
pending-action corrections are recorded with slice 2 below.

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

## Slice 2: the pairing screen and language preference

The partial Japanese proof moved out of the shipped tree in `4b3e180`;
its fixture-root registry, fallback and coverage checks passed review.
The lead's `d050e7e9` includes `226ef62`'s pairing wording and the typed,
redacted file-fault work (`5beb45a`), plus the title-bar changes and code rows
for the machine id and fingerprint. This migration starts on that source,
retaining both technical values in the lead's mono face.

All thirteen dialog strings move from `text/pairing.rs` into
`locales/{en,ko}/pairing.ftl`; the table and its exports are removed. The
nine original pairing-only entries in `text/app.rs` also move: `link_placeholder`,
`name_label`, `name_hint`, `name_is_key`, `name_is_link`, `name_empty`,
`name_invalid`, `name_taken`, `paste_first`. Caller inspection found no
other screen reading those entries. `paired(name, Via)` is also pairing-only:
its whole sentence is a Fluent selector, retaining the machine and route
so an already visible outcome can change language without pairing again.
Existing unused entries in the lead's mixed table are left with its owner.

Shared sentences stay with their area: `machines_unreadable` and
`file_problem` still serve the machine list and preferences, including a
book fault shown during pairing. The typed fault and its useful safe
coordinates/field are retained; its words are prepared before painting
and reworded when the language changes. Draft/sidebar/title-bar words
behind the modal stay in their areas' tables.

`view::pairing` classifies each wire link-decoder error into a whole
recovery sentence. Both platform readers return the original typed error;
its Display is never UI copy. Name problems share one view enum instead
of platform duplicates. Connect failures also use a typed app-internal
problem, rather than an English String, including the browser's unsupported
transport. No node or wire behavior changes. The file-fault branch keeps
the lead's complete safe diagnosis rather than reducing it to a generic
storage error. Prepared failures share immutable text with the renderer;
dynamic successful notices are formatted only on an event/language change.

The new language card follows Paseo's
`packages/app/src/screens/settings-screen.tsx`, `GeneralSection`, and
its `i18n/resources/{en,ko}.ts` language labels. It uses the existing
kit select and combobox; the language menu's width is Paseo's explicit
300. E2's General first-child slot owns only the registration line;
its sending/storage rows and existing choice list remain E2's. The new
picker has separate state and closes the existing list, and vice versa.
Native names alone (rather than Paseo's native/translated name pair) are
the coordinator's explicit contract. The row says Language / App language
and 언어 / 앱 언어. System is first; the checking pseudo-language is absent.

The saved preference is typed, lenient and changed only through
`Prefs::prefer(&PreferenceChange::Language(..))`. Startup reads it before
creating the shell or drawing. A change prepares a catalog before saving,
then switches both the catalog and the remaining table adapter together.
Retained composer, pairing, archive, find, question and project placeholders
refresh without navigation, a node request, clearing text or moving focus;
conversation measurements are invalidated for the new words.
`VESSEL_APP_LANG` remains the highest-priority checking override.

The contributor guide is `view/locales/README.md`: adding strings and
languages, missing-key reports, pseudo checks, message-writing rules and
the area migration recipe. English defines the typed API; every shipped
catalog must be complete. Formatting/particle limits remain the measured
ADR 0039 contract.

Validation and screen evidence are recorded once the complete gate and
normal-app capture matrix finish. Until then this section records the
implementation, not a claim of screen or platform acceptance.


The coordinator defined the three pseudo views as the paste-link dialog,
machine confirmation and General's Language row. General's sending/storage
rows still use E2's table, deliberately visible as plain English beside the
pseudo row. Pairing failure states need English/Korean evidence instead.
The browser-unsupported wording has a Linux, ignored display-test harness:
the real pairing renderer receives the typed problem after the ordinary
reader decodes a private fleet offer. It is absent from the shipped app and
is only evidence of wording/layout. No browser transport run is claimed;
connection, storage and name failures use real isolated paths.

Retained placeholders register in one `app/language.rs::PLACEHOLDERS` list;
find, the current question step and a project AddForm each expose one focused
refresh method. The coordinator approved those C/F-owned seams with tests
that change language through the actual dispatch, preserve the complete typed
value and focus, then replace the selected character at its original caret.
Tests use real controls in GPUI's headless platform and a shell with no link,
updater or config I/O. They run in the ordinary gate, with no display needed.
The commit names the three files and owners, and its hash goes to both owners.
The pending pairing actions now use the existing kit's disabled button state,
as Paseo disables Cancel/Pair while saving; the existing callback guard stays.
This is the only intentional control-state change in the migration.

Preparation merge `466ca256` integrates the conversation request API from
`5360d38f`, retaining connection recovery across the extracted open/take
methods. Native format/Clippy/test passed (413 ordinary, two compile-fail),
and root format/workspace all-target Clippy/testkit passed (14 tests).
A headless regression checks that unrelated records keep a connection notice
while this session's records or opening another session clear it. No L area
moved in that preparation commit; the migration was set aside during its gate.

The question placeholder refresh uses `waiting_at::<SessionPlace>` and only
`Seat::SESSION.get_mut`; other places retain their own fields. Queue editing
also has its own placeholder-only method and the same input-retention test,
explicitly released by the conversation owner.

The later pairing correction `586f844c` is ported within this migrated area:
the name starts empty, its short placeholder says that empty uses the machine's
own name, and a typed name is saved as typed. Its below-field example stays
unchanged. A valid, free machine name is preferred; otherwise the existing
free fallback is used. This adds `name_placeholder` as a tenth pairing-only
message directly in Fluent; it is not introduced into the legacy table on
this branch. The lead's future merge must remove its legacy addition with
the rest of the migrated name fields. No other new lead-head changes are
brought into the middle of this gated tree.

The old name-is-link error referred to a link field above the name, which
no longer exists in the two-step confirmation dialog. Its whole Fluent
message now asks for a short name or the new empty-name option; the name
hint/example below the field stays unchanged. Both languages are tested
as complete sentences.

The first full pseudo matrix exposed the new empty-name placeholder being
clipped by the phone's single-line field viewport. Its English is shortened
to "Leave empty to use its own name", keeping the label/context and the same
empty-name behavior. Korean and the below-field example do not change.

The lead's response-ownership correction `e8f49fec` is taken as its exact
Followup/answered/acted implementation and late-answer regressions, without
a new branch merge or unrelated lead-head changes. It addresses the reason
V1 returned preparation `466ca256`; L does not redesign that owned fix.

## Overnight resume checkpoint

Accepted engine/catalog commits are `5834962` and `4b3e180`. Preparation
`466ca256` was returned by V1 for the inherited late-answer path; the exact
lead fix from `e8f49fec` is now in this model's source, with byte-identical
implementation/test blocks recorded in `lead-response-fix-identical.json`.
The model is not yet approved by V1 or the coordinator. No further area or
branch merge starts under the overnight credit limit.

Evidence lives under `/tmp/work/m12-i18n/`:

- The first model native format/Clippy/test passed: `s2-final-*-1.log`,
  430 ordinary tests and two compile-fail tests. View/link/app actually
  recompiled. This predates the lead response fix and shortened placeholder.
- `s2-final-inputs-2.json` freezes the final Rust/manifest/Fluent hashes.
  `s2-final-gate-2.json` records individual command exits as they finish;
  inspect it and the running process before starting another cargo command.
  Its logs are `s2-final-{fmt,clippy,build,test,wasm}-2.log`.
- The preparation root format/all-target workspace Clippy/testkit gate
  passed (14 tests, `c-place-*` logs). Model changes add no root Rust code.
- `s2-font-before-captures`: 32 personally inspected pre-migration screens
  from the lead's code-row source. `s2-model-captures`: 60 inspected model
  screens, including all three languages and five states. This first pseudo
  pass found the phone empty-name hint clipping; the shortened final hint
  still needs its final-binary recapture before this finding is closed.
- `s2-general-live-captures`: 12 inspected en/ko/pseudo General screens,
  both themes/sizes, with a running isolated fake fleet. E2's unchanged
  General/Composer/Kept/Preferences/sending rows stay plain in pseudo.
- `s2-real-failure-captures`: 24 inspected connection/storage/name failures
  plus eight busy frames. All use real native paths. Eight UID 1000 config
  folders were changed only from 0700 to 0500 for the storage failure and
  independently verified restored to 0700. No service permissions changed.
- `s2-browser-captures-retry`: eight inspected test-only real-renderer
  browser-unsupported frames, eight successful display-test exits. This is
  wording/layout evidence, not browser transport evidence.
- `s2-live-language-retry`: nine inspected normal-UI frames and results:
  unknown tag follows System without losing dark/ModEnter preferences;
  selecting Korean keeps the same process and typed draft; restart restores
  Korean; the English override wins without changing the saved Korean tag.
  The chooser has exactly System, English and 한국어. File 0600/folder 0700.
- The paired-status/save flow uses the frozen normal binary recorded by
  `s2-normal-binary-provenance.json`. Later final-source differences are
  the conversation-only response fix and shorter English empty-name hint.
  Native save/name and language-choice behavior is otherwise unchanged.
  `s2-paired-language-light/results.json` proves both light sizes and a
  literal typed name; `s2-paired-language-dark-ready/results.json` proves
  both dark sizes after painted-frame readiness. All 21 completed behavior captures were personally
  inspected. Earlier fixed-delay captures caught the preceding frame.
- `s2-source-coverage.log` reads the current locale files: en/ko complete,
  zero missing. `s2-fixture-coverage.log`: Japanese has the expected three
  missing keys. These explicit-root reports use the unchanged parser; the
  final native gate additionally checks the compiled release registry.
- `foreign-lines.json` lists every added/deleted foreign seam line against
  `466ca256`; `lead-response-fix-scope.txt` separates the lead's exact port.

The early wasm log reached the compiler's Finished line, but the containing
execution session reported 143. Do not turn that outer result into a new
final-source exit-zero claim; the final runner checks wasm explicitly.
Earlier unsuccessful capture attempts are preserved separately: a dead
private fleet, OCR control lookup, asynchronous preference-file creation,
phone navigation/frame timing and a slow credential-file write exceeded
the first driver's timeout. They are not counted as successful cases.

First on resume: inspect the final gate JSON and source hashes, finish any
unverified gate, and recapture the short empty-name hint with the final
normal app in both themes/sizes (en/ko/pseudo). Send this whole model to V1
first; after its pass, report the hash/gate/captures/foreign lines and stated
differences to the coordinator. The C find/question/queue and F project
placeholder seams must remain together in one commit naming their owners;
send both owners its hash and the exact added lines. E2 approved the first
General child, module/state/picker and language-preference arm.

After approval, a clean-point lead-head merge remains pending (new Light
faces already supplied; do not run the font downloader). The next areas are
About, updater sentences and title bar, one area per commit. About also owes
the lead a Translations role for the five Fluent-stack rows and a separate
uuid identity role; no About source has been changed. S2's board, sidebar,
palette, shortcuts and notices remain held until their owner releases them.
Windows, the complete browser app/transport and real GPU rendering remain
unverified; these captures use Linux Xvfb with Mesa software rendering.
Stop only this track's fake fleet/display/app PIDs and protected pairing
inputs at the end. Never touch the real preview or distribution folder.

Final-source checkpoint at 21:10 UTC: native format and all-target Clippy
completed with exit zero (`s2-final-gate-2.json`). The final normal build,
test and wasm commands, plus the shortened-hint recapture, are still pending.
Do not report the earlier 430+2 result as the final response-fix/hint gate.

### Superseding checkpoint, 21:30 UTC

Current source is committed as `8da03f18` (WIP, not approved for merge).
Final native format, all-target Clippy and normal build have exit zero in
`s2-final-gate-2.json`. Test is compiling; the final wasm check has not yet
reported its own exit. The prior 430+2 result remains a separate earlier
source result until this final test completes.

The final normal binary is hashed in `s2-final-binary-provenance.json` and
does not contain the test-only pairing fixture environment hook. Its twelve
`s2-final-empty-captures` frames (en/ko/pseudo, both themes/sizes) have now
been personally inspected. The shorter English hint fits completely in the
pseudo phone field in both themes; the earlier clipping finding is closed.
The offer came from the normal private fleet CLI and was reused only for
decoding; no connect request or installed-app override was added. The
remaining broad matrices are still labelled with their earlier binary.

`s2-paired-language-dark-ready` and `s2-paired-language-light` contain the
completed, inspected same-process save/retranslation proof in both sizes
and themes; the latter also proves a literal typed name is kept.
`s2-evidence-index.json` maps the immutable source and every evidence root.
F acknowledged the project seam and will not edit those lines while waiting
for an approved hash. C and E2 have the WIP hash and exact foreign-line report.

Cleanup is complete for the UI/fleet proof: `s2-cleanup.json` records 17
verified owned PIDs stopped (nodes, fake children and holder). No process
remains with any of this track's three fleet working directories, the
private display 188 is gone and protected pairing inputs were removed.
The cargo gate is deliberately not interrupted; its process/session and
compiled-input verification are in `final-source-verified.json`. Inspect
that process and `s2-final-gate-2.json` before starting another build.
If credit stops this session, finish/review the final test and wasm result
first. No further UI/fleet process needs stopping or leaving running.

### Latest resume status, 21:35 UTC

The exact source in `8da03f18` has now passed the full final native gate:
format, all-target Clippy, normal build, and `cargo test`, each exit zero.
Tests: 432 ordinary plus two compile-fail doctests, with the explicit display
fixture ignored in the ordinary run (its eight display exits are separate).
Both lead late-answer tests and all four C/F input/caret/focus retention
tests passed. View/link/app were actually recompiled. Frozen compiled-input
hashes are still identical; the later commits change documents only.

The one remaining running command is the standalone view wasm check,
currently waiting for a machine-wide slot. If stopped for credit, inspect
its actual exit in `s2-final-gate-2.json` before repeating anything. All UI
proof, including the final short hint, and fleet/display/input cleanup are
already complete. V1 received the native result first; no V1/coordinator
approval of this model is claimed. After the wasm result, send the final
immutable range and report to V1, then follow its findings/approval before
asking the coordinator to accept the model. Do not start another slice.
