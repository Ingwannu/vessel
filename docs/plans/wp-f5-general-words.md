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

State: plain-word source committed as e9116d6ca; its immutable merged checkpoint
008181b3 has a remote component suite pending. New storage source below still
owes its own component checks. Full General completion, its package gate/review
and before/after screen proof remain owed. F6 helper evidence does not approve
these new resources.

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
