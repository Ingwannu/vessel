# F5: General words migration

General's settings behavior belongs to the settings owner. This word package
changes generated catalog calls and owned translation data, preserving input,
preference/store, visibility, ordering, controls and layout. The reference is
Paseo's General/Sending settings sections, already described by the native
General renderer. Existing Vessel wording is the migration source; no reference
translation assets are imported.

[Decision Log]
- Purpose: migrate the existing General labels and delivery explanations with
  the English-generated contract, English/Korean together.
- Existing assets: the Fluent catalog and Settings cached-choice adapter.
- Alternative: formatting a selector each frame adds allocation; retaining a
  duplicate Rust word table gives translators two sources of truth.
- Choice: whole static messages plus exhaustive TurnDelivery-to-cached-getter
  adapters, as the existing Chat display choices do.
- Effect: no dependency, model/range/store, physical-key naming or UI behavior
  change. The unused general_lead field has no caller and is removed.

## Plain-word subfeature

`general.ftl` contains 13 messages in each language: title, Sending, send
shortcut, default send, three delivery labels and three whole explanations,
Kept, Preferences, This launch only. Existing values are preserved. General's
renderer changes only those expressions. The delivery trigger/list changes
only its label expression. Section::General uses the prepared title; other
section mappings stay unchanged. The corresponding Rust fields/methods are
removed. Whole English/Korean goldens verify all messages and delivery cases.

## Remaining General scope

The shared refusal banner's word-error fix is described below. The current-base
Diagnostics compatibility use of kept_why remains until W20's package is
integrated. Overview/Connections have not been touched; E2's terminal-settings
domain is separate.

Current state: General source snapshot 28639559 passed its whole native gate
and standalone view wasm build. All 98 after-capture originals were opened;
the 16 normal English/Korean before/after pairs have identical decoded pixels.
The final review is pending. Earlier checkpoints and failures below are retained
as history, not current pending checks. Later generator producer commits and
the separate F6 helper are outside this General approval request.

## Prepared storage subfeature

The General renderer now borrows a complete `SharedString` hint instead of
formatting the file note or calling `kept_why` during paint. A pure
`PreparedGeneral::new(words, &prefs.kept)` prepares it before Areas exists.
`Areas::new` forwards that one typed value to `SettingsState::new`; neither
constructor guesses a language or creates an unprepared placeholder. Both Shell
constructors, including `without_io` in the language test harness, use the same
preparation path.

The view adapter `General::storage` formats whole messages from `Kept` and
`FileFault`. Four unreadable shapes distinguish a known field from a closed
problem and presence from absence of a source position. No absent field/position
is represented by a fabricated argument. Numeric schema versions and positions
use NUMBER without grouping; positions outside its exact u32 count range return
a typed error instead of being truncated. A complete localized fallback is
prepared on error. No I/O, new dependency or view-to-link edge is introduced.

The existing English/Korean file explanations and recovery remain the same.
Two deliberate word-error fixes are included: NoPath's arbitrary diagnostic is
replaced by one safe translated sentence (the public model is unchanged), and
the newer-file Korean message uses the particle function rather than the
interim `은(는)`. The new ten keys bring General to 23 keys in each language.

`Shell::refresh_general_words` runs after the catalog changes, before paint. It
rebuilds the hint from current facts and retranslates the copied-path notice
only while that notice still owns the frame foot. The actual clipboard path
remains data. Today `Prefs.kept` is discovered at startup and skipped by serde;
SavePrefs logs an error but produces no storage-state event. A future caller
that replaces `prefs.kept` after startup must call this same refresh method.
No synthetic event or change to store outcomes has been added.

Tests cover complete English/Korean explanations for each typed problem,
located/named forms, NoPath diagnostic suppression, newer-file grammar, exact
position rejection, pseudo output, changed typed storage facts, and the real
copy/language dispatch with clipboard and typed input/caret preservation. The
old `kept_why` remains only for the current-base Diagnostics compatibility
caller; W20's incoming package removes that call. It is not used by General.

### Shared seams

- `settings/mod.rs`: one PreparedGeneral field; constructor parameter and
  initialization; helper module/re-export. E2 owns the remaining settings state.
- `areas/mod.rs`: one owned constructor parameter, forwarded unchanged.
- `shell.rs`: prepare from the effective catalog and loaded facts; pass to Areas.
- `shell/language_tests.rs`: the second Areas caller passes prepared English
  default facts. Other test shell behavior is unchanged.
- `language.rs`: one additive refresh after changing the effective catalog.
  K's additive appearance-placeholder registration must be preserved on merge.
- `settings/general.rs`: prepared hint borrow and the copy-notice handler call;
  controls, conditions and geometry are unchanged.

Static size/boundary/token checks pass on the prepared storage source (610
Rust files; 14 existing size exceptions, no new token entries). An old retained
explicit-root checker first rejected numeric useGrouping: 0; the files now use
its supported Fluent useGrouping: "false" option. This is a data correction,
not a relaxation of the gate. Current-source compilation and runtime tests are
still owed; old checked binaries are only source-parser evidence.

## Whole preference refusal notices

The old shared Settings banner concatenated a translated prefix with Refused's
English Display diagnostic. It also retained only that resulting String, so a
language change left the old sentence visible. The following word-error commit
reuses the existing Refused enum and exact MAX_ACTIONS/MAX_MACHINES constants.
`settings-notices.ftl` owns three whole EN/KO messages (two reasons and a safe
formatting fallback). English uses numeric plural selectors, not noun fragments.
A pure view adapter converts the model bound to an exact numeric argument or a
typed range error. No second bound or store rule is introduced.

The app retains one `PreparedRefusal { reason, words }` in the existing optional
refusal field. The existing prefer Err branch prepares it, the existing success
and arrival branches still clear it, and language dispatch prepares it again
from the same cause before painting. Rendering only clones its SharedString;
it never formats or displays Error. The two sentences now explain both limits
in Korean as well as English.

E2 reserved these exact shared seams: SettingsState.refused's type and its
`.child(refused.words.clone())` render expression; the prefer Err preparation
expression; one language refresh registration; the helper module/import. No
change to preference outcomes, allowed ranges, success/arrival clear, S2
lifecycle calls, Overview, or selection behavior is included.

Whole EN/KO reason goldens and singular English messages cover the view adapter.
GPUI tests use the real prefer rejection path and real language dispatch, assert
that a rejected change did not mutate preferences, retain input/caret/focus,
and check that a successful change clears the old refusal. These new tests are
source prepared and still await execution. The earlier immutable 008181b3 suite
cannot prove the later storage/refusal source or its newer package base.

Source static checks: 612 Rust files, the same 14 existing size exceptions,
unchanged token entries, and valid module boundaries. Explicit-root validation
reports English/Korean complete with no missing key. Whole package gate, review
and screen evidence remain owed.

## Borrowed toolkit binding

A source review of pinned gpui-pre-shared-string 0.3.10 found another allocation
boundary: its From<&str> delegates to SmolStr::from and copies long notes, while
new_static retains a static reference. General and the existing Language row
now explicitly borrow every cached plain label/hint at their widget boundaries.
The Delivery trigger and list avoid their old intermediate owned String too;
other settings choice branches retain their existing behavior.

The existing getter allocation test now includes General's three labels/whole
hints and the Language note, in English, Korean and pseudo. A real language/input
test checks that the actual Delivery trigger and menu labels retain the exact
cache addresses before and after the switch. This does not claim an entire GPUI
frame allocates nothing: widget trees, vectors and caller data still have their
own lifetimes. It proves the message getters/binding use the prepared strings.
Current-source execution is still owed.

The future b060-only narrow/whole controllers were stopped before any child or
remote submission (verified command, UID, sleep state and no children); their
metadata remains in the evidence folder. The already registered 008181b3 run
keeps its place unchanged. Final component and package candidates include this
binding correction; b060 is not mislabeled as the final checked source.

## First component result and shared-caller correction

The exact 0e9381266 component build failed before tests ran: Sidebar's
`row_menu.rs:151` also uses SettingsText.path_copied. The initial call-site
search was too narrow and deleting that shared field was incorrect. The field
and its exact original English/Korean values are restored; Sidebar source is
untouched and remains its owner's migration. General keeps its context-owned
prepared catalog notice. The compiler's unnecessary qualification warning in
the test-only second Areas constructor is corrected at the same time.

The original log remains evidence of a real source compilation failure, not a
passing test run. Its controller submitted no Clippy/wasm/whole gate afterward.
The earlier 008181b3 run ended with signal 143 during the operator-approved
build-VM resize/restart, confirmed by the tool owner. That infrastructure
interruption has no compiler/test result. The corrected candidate still owed
its component suite and package gate at that checkpoint.

## Raw Unicode-isolation goldens and base 6

The corrected 3272224f component run compiled and executed the app tests:
75 passed, 4 failed, 2 ignored. The four failures were new expected strings
omitting Fluent's intentional U+2068/U+2069 direction isolates around paths and
counts. The renderer's message output was retained. The test fixtures now assert
complete raw sentences including the controls, in line with existing core,
updates and fork goldens. No engine isolation setting is disabled and no output
normalizer hides a discrepancy. Located problems now also have explicit raw
English/Korean goldens for every case, replacing dependence on the old Rust
formatter. Clipboard tests still assert the original unformatted path.

The announced package base 6, 18eedad4 (incoming native 1404/0/2), was merged
as 3265c841 after the failed component run; no whole gate had started. W12's
layout initialization and G4's test-link constructor additions were preserved,
as were both prepared-General constructor calls. Current whole-gate/export
preparation waits for the lead's announced native lockfile correction; no
--locked command is used against the known stale base-6 lock. A new merged-head
component suite, then one whole package gate/export and captures remain owed.

## Fixed lock and component execution

The lead's fixed lock/menu head dd13ce37 was merged as 34dd9d0f. The announced
incoming native result is 1408/0/2; this is not this package's gate. The merged
component command used --locked and --no-fail-fast. All four native crates
recompiled. App tests passed 86/0/2; view library tests passed 973 with one
failure; the remaining view suites passed 22, 3, 117 and two compile-fail docs.
The sole failure was the new pseudo test calling Locale::from_tag("en-XA"),
which correctly excludes checking-only pseudo from ordinary stored languages.
The test now uses the existing PSEUDO constant, preserving that boundary.

Original failures/logs remain in the report evidence. Clippy, standalone wasm,
the whole native gate/export and actual General captures still await this
corrected candidate. Current noncompiling quality checks pass: 658 Rust files,
the same 14 size exceptions, valid boundaries and no new token-list entry.
All Rust compilation continues only through the build VM.

## First whole package gate

The corrected e35adcc89 settings suite passed 21/0/0, including pseudo.
The first whole native gate passed fonts, formatting and all quality checks,
then Clippy rejected two generated documentation comments: the translator
comments named MAX_ACTIONS and MAX_MACHINES without Markdown code backticks.
The English comments (and their Korean copies) now quote those identifiers.
No message value, variable, generated getter contract or caller changed.

The failed run is retained; its native build/test/export did not run. A
changed-crate Clippy check preceded the corrected whole gate. The accepted
dd13ce37 baseline app remains read-only; its original captures were retained
because this comment-only correction changed no UI text.

## Final bounded package evidence

Exact source 28639559, run `i18n-20261115T134746Z-28639559`: fonts, fmt,
quality, Clippy, build and test all returned zero, with no skipped step.
Both changed crates actually compiled. Raw test totals are 1451 passed,
zero failed, two ignored: 1449 ordinary tests and two compile-fail docs.
Quality checked 658 Rust files with the same 14 size exceptions, valid
boundaries and no new token-list entry. The exported app is bound to that
commit, SHA-256 `18a89df04c82f5f3cdddaee33ef10a9b82f02dc339ff55f4254c6512a14e0e7f`.
The separate run `i18n-20261115T145346Z-28639559` built view for
`wasm32-unknown-unknown` successfully; this is code generation, not a browser
runtime check. No unchanged whole gate is requested for these results.

The author opened all 34 baseline/discovery originals and all 98 after
originals. The after set comprises 92 matrix frames, one initial probe and five
obsolete discovery frames. Normal English/Korean, dark/light, at 390, 800,
1024 and 1280 logical pixels match the baseline's decoded RGBA exactly in all
16 pairs. Pseudo labels and six selected 1.25/1.5/2 scale samples fit the
inspected states. At 800×600 the long English/pseudo path continues below the
initial fold; ordinary wheel scrolling exposes it fully, as captured.

Actual file paths exercise malformed JSON, denied file access and a valid JSON
string of the wrong root kind, with complete English/Korean recoveries in both
themes and phone/desktop sizes. The original `[]` fixture was accepted as
default preferences: its eight frames are normal empty-sequence evidence,
not kind-failure evidence. They are retained separately from the corrected
scalar captures. The first navigation plan also produced a phone Settings-root
discovery frame; it is excluded from General proof. An interrupted probe's
recorded processes were stopped after checking their PIDs and executables.

Real controls expose the three delivery choices, localized copied-path notices
and the whole shortcut-limit refusal after attempting the 65th action. The
pseudo refusal expands while the legacy Shortcuts table remains plain.
General's other remaining plain pseudo words are shared Settings navigation
and Back/Add-a-machine labels, owned by their existing tables; physical key
names and path values are data. No shipping preview branch was added.

All app/display probes ran as UID/GID 1000 against private directories and
preferences at modes 0700/0600, using the read-only mode-0500 export. Broken
and bound-full preference bytes stayed unchanged. Completed probes stopped
their recorded app/display processes. The General source adds no dependency,
lock, range, serialization or store behavior change. The broader frame remains
its owners' work, including the retained Sidebar `path_copied` and Diagnostics
`kept_why` compatibility callers.

Limits: machine-limit, NoPath and Newer outcomes have whole-sentence/model
tests, not live UI producers here. There is no OS clipboard extraction,
real-node/provider/updater run, browser or Windows runtime, real-GPU,
accessibility, first-frame timing, extra-scale before-parity or later combined
integration approval. The clipboard projection test asserts the raw path;
captures prove the visible notice only. Earlier bounded About, Diagnostics,
Chat and fork/rewind reviews retain their own scopes.
