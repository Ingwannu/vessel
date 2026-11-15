## Batch gate correction

The first full native run on `c3ad12fd` passed fonts, format and quality, then
stopped in native Clippy: the newly one-byte Copy `EmptyFork` was still passed
by reference to its banner. Pass it by value in that private helper and its one
caller. There is no message/layout/state change. Native build/test did not run
in that failed attempt; no full pass is claimed from it. Re-run the full gate
on this correction. The preceding fork move's native narrow Clippy was green.

The local capture build of `c3ad12fd` passed. Initial inspected About captures
use an isolated empty config and private authenticated display, proving only
that normal empty-state page. Shared newer-node fixture binaries failed the
standard fleet's shape preflight; that fleet was rejected without weakening
its assertions and all eight verified owned node/fake processes were stopped.
Those failed fixtures provide no UI/current-node evidence. Before/after,
remaining language/theme/size combinations and live fork screens remain owed.

## Fork typed-recovery correction (source ready; full gate/screens owed)

The diagnostic-preserving fork move is `ec544b88`. This independent correction
projects all wire codes to safe fork recoveries; origin reads have removed,
access, version, unavailable and generic recovery messages. Original diagnostics
stay in host logs/authoritative session state. Unknown empty-fork failures now
carry a typed `Other` reason rather than copying diagnostic text into each frame;
existing protocol classification of the two known empty-fork reasons is unchanged.
No protocol or node behavior changes. An uncertain fork result asks the person to
check the session list before retrying, avoiding an unqualified duplicate retry.

The final fork catalog has 24 messages per language. Tests cover whole English/
Korean recoveries and all eleven wire-code classifications plus absent transport
code. Three native input regressions feed actual fork/origin inner handlers, then
apply the language change while preserving composer text/caret/selection/focus,
retained origin target/retry state and ownership of another area's later notice.
They do not run a node/agent or independently prove the parent act-routing guard.
All-target view Clippy, 833 library tests and ten contract tests pass. Native
format, size, boundaries and tokens pass with no allowance increase; candidate
source coverage has zero missing keys in both languages. Initial generated-doc
Clippy warnings came from two CamelCase translator comments; they now use plain
words. The preceding vocabulary move also passed remote all-target native
Clippy. Full native gate and captures are still owed; no completion claim yet.

## Fork vocabulary migration (source ready; full gate/screens owed)

Reuses the generated catalog, retained-notice pattern and existing fleet events;
no new parser or dependency. Ten whole messages replace the fork table. Source
captions retain one formatted sentence per listed session, refreshed only for
changed titles or language, with first-listed UUID semantics matching the old
`Fleet::title_of`; removed sources are evicted. Rows borrow these captions and
plain getters. A changed caption invalidates cached timeline rows/measurements.
Origin/fork failure text is prepared at the response or language event, not a
frame. Retained global notices keep their ownership check after navigation.

Reference: Paseo's `components/assistant-fork-menu.tsx` and
`hooks/use-fork-agent.ts`. The existing one new-session target, lock, mark/menu
geometry, origin log, remove behavior and navigation are preserved. As with the
rewind move, this first commit retains diagnostic interpolation; typed refusal,
origin-read and unknown empty-fork corrections follow separately before any
complete UI submission. The staged move does not make node diagnostics Korean.

Lead approved the one fallback seam in legacy `conversation/entries.rs`:
`spoken` takes the prepared `Translator`; only the Forked branch changes its
words. Actual callers: three app expressions, three existing failure-test
expressions and eight page-word assertions (the latter were additional callers). No other spoken sentence changes. Paint's former fleet borrow is
replaced by the prepared caption cache in the two existing constructors.
Fork/origin modules, the empty-fork banner, language registration and fleet/act
refresh lines are the other app seams. C's terminal and spacing work are untouched.

All-target view Clippy and 831 library + ten contract tests pass, including
whole English/Korean sentences, rename/language/duplicate/removed-source cache
behavior and zero allocations over 10,000 prepared reads/unchanged refreshes.
Native format, size, boundaries and tokens pass; no allowance was added. The
standalone coverage reporter reads the candidate root with zero missing keys
in either language; this is source-preview coverage, not a compiled registry
claim. App tests, whole native gate and screen evidence remain owed. The first
Clippy attempt's three source warnings were fixed; a later format check found
page assertion wrapping, corrected before committing.

G1's exact Korean PR form-title correction is `175491aa64320833ebff65eeebde4015904462a2`.
Its whole gate and Korean screen evidence remain with G1; no L PR key edit.

# Track L: the app's words as translation data

## Rewind typed-refusal correction

The staged diagnostic interpolation is replaced independently of its move.
All eleven wire error codes map exhaustively to ten safe rewind recoveries;
the node's diagnostic is logged at the host boundary and never interpolated
into the notice. Access, version, missing conversation, current state,
unsupported agent, missing/failed agent launch, unavailability and uncertain
internal failure have complete English/Korean messages. No new wire detail or
protocol is invented. No-code transport failures keep the existing lost-link
path, without a second rewind error signal.

The retained notice now carries its typed recovery and cached text, so its
language refresh cannot expose a diagnostic. The source-language corpus has
14 keys per language. Tests compare every whole recovery and all error-code
classifications. A native GPUI regression feeds a real typed `ActAnswer` to
the actual rewind handler, then uses the existing composer-input preservation
helper to check the Korean notice, cleared pending act, typed value, caret,
selection and focus. This test executes no node or agent; its app execution
remains owed. All-target view Clippy, 828 library tests, ten catalog-contract
tests and native source checks pass. Full native gate and captures remain owed.

The preceding `4380e565` vocabulary move passed remote all-target native
Clippy. The updater correction's two doc-markdown diagnostics were fixed by
`02bbff30`, whose remote native Clippy also passed. Neither narrow check is
reported as executing the app's regression tests.

Later PR wording is coordinated with its active owner: the one Korean
`pulls-form-title` change to “PR 생성” belongs to that area's current slice;
the duplicate Approved fact is its drawing/view correction. This branch does
not edit those same keys while that slice is open. Take the resulting commit
before the later PR vocabulary work.

## Rewind vocabulary migration

The instructed conversation-file split is merged before this migration.
Five rewind messages move from `text/conversation/rewind.rs` to complete
English/Korean `rewind.ftl`: tooltip, warning, mode, copy label and failure
notice. The retired table module is deleted. Reference: Paseo 0.15.4
`components/rewind/rewind-menu.tsx` lines 38–151,
`rewind/use-rewind-agent-mutation.ts` lines 25–65, and
`rewind/composer-restore.test.ts`. Keep the existing one conversation-only
mode, menu/tooltip geometry and composer-restoration behavior. The existing
timeline point/restoration fixtures are unchanged.

Decision: reuse generated getters and the prepared-notice pattern. Plain
labels borrow the shell's prepared catalog; a retained failure formats only
at the answer or language event and keeps shared text. A frame notice that
still belongs to this failure can reword after navigation without resetting
the pending target, composer, selection or focus. Its cache is deliberately
not cleared by navigation, since the global frame notice is not cleared then.

This is a wording-preserving move. It deliberately retains the former node
diagnostic interpolation in the whole failure sentence, including the empty
diagnostic branch; Fluent adds its normal Unicode argument isolation. The
typed-error correction follows separately, before a complete UI submission.
The staged notice is not claimed to make an English node diagnostic Korean.
Tests compare every label and whole notice in both languages, including
rewording the retained outcome. All-target view Clippy, 827 library tests,
ten catalog-contract tests and native source checks pass. App and full native
checks/screens remain owed. No new dependency, protocol, control or token.

Other conversation-file seams: `entries.rs` adds one prepared-catalog field
to `Paint`, with its one-line initialization in `timeline.rs` and
`permission.rs`. The field avoids initializing/resolving a locale in a draw.
The language-change path adds the one-purpose notice refresh. Rewind's own
state/callback holds its cache; no unrelated conversation table is moved.

## Source ready: typed update recovery

The updater's English Display strings previously reached About directly, and
a failed restart copied its Display into the frame notice. This correction
projects the updater's closed typed errors into 13 safe recovery choices,
with complete English/Korean messages in `update.ftl` (now 50 messages per
language). Native classification matches every outer and nested error variant;
original update errors remain in the authoritative state, and restart causes
are logged with their source chain. No diagnostic is interpolated into UI text.
No fetch, signature verification, installation or restart behavior changes.

The pure view projection retains shared recovery text alongside status,
action and progress; a new check clears an earlier recovery. It separates a
small private status-text projection so constructors remain short. The
restart notice is retained separately and retranslated only while it is still
the notice displayed, preserving a later unrelated notice. Language changes
refresh both paths through the existing one-purpose host method. The browser
keeps its idle adapter; an unreachable failed-browser state has a safe
Unsupported classification rather than a raw diagnostic.

Whole-sentence tests cover all 13 recoveries in both languages, including the
prepared error value. The allocation test now includes Failed and its recovery
getter. Native-only synthetic error fixtures cover each update recovery
category and constructible alternate variants without downloads or process
launches; restart ownership and actual cached-problem clearing have their own
host tests. The latter app tests still await execution by the native gate.
Local all-target view Clippy and 825 library tests plus ten catalog-contract
tests pass. Native source checks pass. Status: **source ready, gate owed**;
app compilation, whole native tests, wasm and screens remain unverified for
this correction. There is no installed-app preview hook or fake transport.
The first remote native Clippy run on `802c51a6` reported two doc-markdown
diagnostics on the native classifier's header. Its type names are marked as
code in a separate documentation correction; the failed run is retained.
The instructed conversation split is merged at `fb7f7f2a` (`32eae9f4`, including
`aaae7a03`); rewind/fork source work can now start on that layout.

The About renderer also now borrows static licence values and the unavailable
log-path label through its existing SharedString value slot, avoiding a fresh
String copy of those constants while drawing. Real filesystem-path formatting
is unchanged. No geometry, control, theme token or allowance changes.

V1 reviewed exactly `46ff1c13..7c99ae1f..73fd0c49` as SOURCE/A13 GATE PASS:
all six native steps passed, 1,147 ordinary and two compile-fail tests passed,
zero failed, one existing display fixture ignored. Its scope explicitly
excludes UI approval, a complete About screen, inventory completeness,
`75c4d8dc`'s strengthened guard, and the later updater work. The updater's first
remote Clippy failure on `75cf3b4a` remains recorded; the test-module correction
`bf9f5e77` passed a new remote native Clippy check. No unchanged gate is rerun.

## About inventory: member-local declarations

The self-review found a pre-existing gap in the inventory guard: its normal
dependency traversal was correct, but its final ordered comparison considered
only declarations in the shared workspace manifest. Four already-shipped
member-local dependencies escaped it: `ashpd` 0.13.13 (MIT), `futures-lite`
2.6.1 (Apache-2.0 OR MIT), `image` 0.25.10 (MIT OR Apache-2.0), and `resvg`
0.46.0 (Apache-2.0 OR MIT). Their declarations and licences were checked
against the pinned lockfile and each downloaded package's own manifest.
No dependency, version, feature or lockfile is changed.

The strengthened test retains the former shared-manifest ordering and normal
transitive traversal without changing either helper. It additionally requires
every native member's direct external normal declaration, on every target.
Previously unlisted member-local names follow the shared names alphabetically;
fonts still follow the crates. Development/build-only declarations stay
excluded; a declaration used both normally and for development stays required.
The original fixtures remain, with two additional fixtures proving inline
normal/both membership, dev-only exclusion and a browser-only inline normal
dependency. The old comparison's passing-but-incomplete list is explicitly
compared with the strengthened required list. This adds coverage; it does not
narrow the original comparison or its runtime set.

The list grows from 28 to 32 entries. It is a borrowed static slice instead
of carrying a manually repeated array length. New Desktop, Tasks and Images
roles describe desktop-setting reads, following setting changes, and drawing
the app's images respectively. The complete vocabulary is now 27 messages
per language, with whole-sentence checks. The role fixture is moved outside
the assertion function to keep the function short; no expected sentence is
removed. Native source checks and all-target view Clippy pass; 823 library
tests and ten catalog-contract tests pass. Whole native gate and captures
remain owed for this follow-up. The earlier whole About run on `73fd0c49`
does not include this strengthened guard and cannot prove its coverage.

## Source ready: title-bar and update vocabulary

The About move and role fix are committed as `7c99ae1f` and `73fd0c49`.
Their whole native gate is now submitted against the immutable latter commit;
no current-update working-tree source travels with it. The role fix's local
contract tests have also passed (10), and the compiled coverage command has
exited zero: English/Korean complete, zero missing. V1 has not received a
complete submission or approved these changes.

The next area is the shared updater vocabulary, including the title bar's
four labels, About's card/dialogs/log actions and the updated-on-start banner.
Reference: Paseo 0.15.4 `packages/app/src/screens/settings-screen.tsx`
lines 621–747 (`DesktopAppUpdateRow`), and its English/Korean
`desktop.updates.status` / `settings.about.updates` resources. Keep the
existing deliberate differences: one update feed, the automatic-check switch,
the existing Zed-inspired title-bar labels and the running build's notes only.

[Decision Log]
- Purpose: translate the existing messages without formatting in a frame.
- Existing assets: generated Fluent getters, the updater's typed watch state,
  the normal language-change dispatch and shared kit rows/buttons.
- Alternatives: calling dynamic Fluent messages in the render functions would
  allocate every frame; adding the updater dependency to view would bring in
  native I/O and break the standalone browser boundary.
- Choice: a pure `view::updates::PreparedUpdate` projection, shared Arc text,
  and a host `UpdateUi` that retains the authoritative runtime/state/notice
  and refreshes its text on events and language changes.
- Why: both About and the title bar borrow the same prepared values; the
  browser uses the same idle projection without an updater I/O dependency.
- Cost: a small explicit state projection at the host boundary and one cache
  refresh registration in the language-change path.

The host's four updater fields are grouped with the prepared projection,
shrinking `shell.rs` from 500 to 486 product lines (the repository's size
checker); no allowance grows. Host call sites, a GPUI regression, the pure
view model and 37 messages in each language are installed. Narrow all-target
view Clippy passes, with 821 library and ten catalog-contract tests passing;
the new shared-text allocation test reports zero allocations. Native source
checks pass. The first narrow attempt failed on six test lints (constant
placement and unused black-box results); they were corrected, with the failed
run kept separately. No assertion or deadline changed.
The shared validator also accepts the explicit candidate root, with zero missing
keys in both languages; this is a source-candidate check, not a compiled API
or current-binary claim. App compilation, its new GPUI regression, the whole
native gate and current screens are owed. Status: **source ready, gate owed**.
The first remote all-target native Clippy run on `75cf3b4a` failed on the
new GPUI test's private find access (and its unused import). The regression
moves into find's existing language-test module, where that access belongs;
no production visibility is widened and no assertion is removed. This is a
separate correction to the source-ready commit, with the failing run retained.

The model preserves numbered-build prefixes and plain developer build names;
whole Fluent branches select either presentation. Numeric progress stays a
number. Its getters share Arc values; allocation tests and whole-message tests
cover both languages and the pseudo locale. A new whole-sentence fallback is
used only if message preparation fails, with the formatter's diagnostic in logs.
Fluent's Unicode argument-isolation markers surround variable values; visible
sentences and version-prefix rules are preserved, and the tests compare the
complete returned strings including those markers.
The existing raw updater error and restart-notice Display paths are explicitly
deferred to a separate typed-error correction, not hidden by this migration.

Additional seams approved for this area: `language.rs` one refresh call,
`main.rs` one module line, the `Shell` field grouping and its no-I/O test
fixture, the browser adapter's field access, and General's one `copy_path`
getter. No layout/control/token change. The next rewind/fork migration is
owned here but must start after the conversation-file split `aaae7a03` is
merged. Its raw node-error boundaries also belong to this track; their owner
has no parallel correction planned. Do not touch the terminal work.

## About role correction (after `7c99ae1f`)

The catalogue move is committed independently as `7c99ae1f`, preserving
all 22 old messages. This follow-up changes the five Fluent-stack rows
from Data to Translations and UUID from Data to Identifiers. Their complete
English/Korean labels are “Translations” / “번역” and “Unique identifiers” /
“고유 식별자”. The two additional roles are closed enum variants and cached
plain getters, raising About's vocabulary to 24 messages per language.
An inventory regression pins all five translation rows and UUID's role;
whole-message and zero-allocation coverage includes the new labels.
No dependency, licence, list order, row layout or inventory size changes.
Narrow all-target view Clippy and 814 library tests pass, including the new
inventory/whole-message checks. Native fmt/size/boundary/token checks pass.
Explicit-root coverage through the unchanged production validator reports
English/Korean complete, zero missing. The duplicate contract-test command
and compiled-registry report remain queued; their pending state is not used
as a pass. Whole native gate and captures remain owed over the batch.
Status: **source ready, gate owed**. Next area ownership for the
update table and its title-bar/About callers is confirmed, words only.

## Resume point (2026-11-15)

Published build 0.0.0.0.17 (`56a7cbd3`) is merged without rebasing. The
engine (`5834962`), partial-language fixture boundary (`4b3e180`), merge
preparation (`466ca256`), pairing/language model (`8da03f18`) and its final
evidence record (`4b9c0029`) are all ancestors of that build. Their old
gate and captures remain historical evidence, not checks of today's tree.
The additional instructed merge to `46ff1c13` retains the terminal parser
and width dependencies added to the About inventory after the published build.

The next slice is About's licence vocabulary: 22 messages per language,
including 19 inventory roles. Existing assets: the embedded Fluent catalog,
English-generated getters, warmed plain-label cache, generic coverage gate
and existing About renderer. Decision: **modify/reuse** those assets; no
new dependency, parser, control, wire call or architecture decision.
An exhaustive `Role` adapter chooses cached plain getters rather than a
dynamic Fluent selector, because a selector would allocate on each row draw.
The original strings, including the two new terminal rows, move verbatim.
The five Fluent rows and UUID still retain their old Data role in this
refactor; correcting those meanings is a separate follow-up commit.

References read: Paseo 0.15.4 `packages/app/src/screens/settings-screen.tsx`
lines 473–499 (About card, connected hosts); Vessel's existing
`web/src/features/settings/about.tsx` lines 78–105 (licence inventory),
`web/src/data/about.test.ts` (runtime inventory and licence checks).
The inventory is Vessel's existing addition beyond Paseo's About sections;
this migration changes neither its row layout nor its scope. Update,
connected-machine and trademark sections retain their owners' tables.
This slice therefore migrates the licence vocabulary, not the whole About
screen; pseudo-language will still expose those unmoved shared sections.

Ownership while this slice is open: About's old table, its new locale files,
role adapter, and `view/src/about.rs`'s Role and Fluent/UUID inventory rows.
The app renderer changes only three expressions: obtain `shell.words.about()`,
call `licence_note()` and call `software()`. Removing `AboutText` changes the
two view export lists. No other area table is edited.

Added checks compare all 22 whole messages in English and Korean, extend the
warmed zero-allocation test across every bundled role, and assert that a
key present only in English **or** only in Korean fails the shared validator.
The latter pins both directions of the already-existing contract; partial
candidate languages still retain the separate accepted fallback contract.

Verification: source-only format, size, boundary and token checks pass.
Native file budget: 547 files, 15 listed; boundaries: four crates, seven
external owners, two name rules; tokens: 418 step pixels, 79 other pixels,
45 hex colours, no new allowance. Narrow view all-target Clippy passed;
813 library tests and ten localization-contract tests passed. Coverage:
English/Korean complete, zero missing each. Changed view source was actually
recompiled. All four short-lane commands exited zero, without a timeout.
Root format passed; its size check has 28 existing violations and its
boundary check still lacks `vessel-node -> vessel-agent`. Root sources,
manifests, checking scripts and the architecture's crate tables are identical
to `46ff1c13`; this slice does not fix or waive those baseline failures.
No local
whole-native gate is run under the current build-machine constraint; the
remote native gate command and current-source captures are still pending.
No new UI, fleet or display process has been started. Status: **source ready,
gate owed**, not a complete review submission. The whole native gate and
screen checks will cover the batch once the remote command is available.
Next: the separate role correction, then title-bar/update words and the
newer screens, each area's ownership confirmed before its source changes.

Remaining scope: updater/title-bar vocabulary, legacy area tables and newly
added conversation rewind/fork, sidebar Display, settings, file-editor and
attachment words. Composer and pull-request resources already exist on the
merged line. Inventory each area's callers and current owner before moving
it, one area per commit. Full English/Korean parity of the app is not yet
achieved by catalog coverage alone.

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

### Final implementation/gate result, 21:36 UTC

All five final commands now have exit zero in `s2-final-gate-2.json`:
native format, all-target Clippy, normal build, whole native test (432
ordinary plus two compile-fail), and standalone `vessel-app-view` check for
`wasm32-unknown-unknown`. The final wasm command recompiled view and
completed in 41.25s after its slot wait. This supersedes the pending-target
checkpoints above; the earlier outer-session 143 is not used as evidence.
The full sequential runner has completed with exit zero. Frozen compiled
input hashes remain identical to the source in `8da03f18`; follow-up
commits record documents only. All UI proof and cleanup described above
are complete. No build, private fleet or display is left running.

Send the final immutable range to V1 for the model review. The status is
implementation/gate complete, review pending, not coordinator acceptance.
C/F/E2 have the unchanged source hash and the exact foreign-line report.
The next session begins with V1's findings/approval; only after acceptance
and a clean lead-head merge can the released next areas start. The
About roles and S2 ownership holds listed above still remain pending.
