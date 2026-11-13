# ADR 0039: Translations are data files, not code

Status: accepted as the direction, 2026-11-13 (the owner: when a
language is added later it must be easy to translate, so set it up for
that now). The format and the rules are decided here; the exact crates
are decided by the track that builds it, against the checks below.

## Context

The native app has two languages, English and Korean, and every string
is Rust: a table per area under `spikes/native/view/src/text/` (about
5,800 lines on the lead's branch, more on the tracks'), two constants of
one struct type for the plain strings, and `match (lang, case)` arms
for every sentence with a value in it (183 arms for Korean alone). The
page is English only.

That shape was chosen for one property: a string missing in one
language does not compile. It has served two languages written by the
same hands. It does not serve a third:

- A translator cannot work in it. Adding Japanese means editing every
  area's Rust file, every tuple match, and knowing which arms are
  sentences and which are fragments.
- Sentences are sometimes built from parts in an order the code
  chooses per language (`"{verb} {noun}"` for English, `"{noun} {verb}"`
  for Korean). A third language needs a third branch in code.
- Plurals are an `if count == 1`. That is English; Russian has four
  forms, Japanese none.
- Korean particles after a value (을/를, 와/과) are fixed in the string
  for the label that happens to be there today; reviews of 2026-11-13
  found three of them.
- Nothing tells a translator where a string appears or how long it may
  be; the doc comment above a field is the only context and no tool
  reads it.
- The page cannot share any of it, so "one experience everywhere"
  (ADR 0036) would be translated twice.

## Decision

1. **The words live in data files in Project Fluent's format**
   (`.ftl`), one folder per language, one file per area:
   `locales/<language tag>/<area>.ftl` beside the `view` crate.
   English is the source; every other language is a copy of the same
   keys.
2. **Adding a language is adding a folder.** No Rust is edited: the
   languages are found when the app is built, the language chooser
   lists each by its own name, and a language that is not finished
   falls back to English message by message.
3. **The call sites stay typed.** English is read at build time and
   every message becomes a Rust item with its variables as typed
   arguments, so a key that does not exist, or a variable that is
   missing, still does not compile. What a second language may lack is
   reported by a check, not by the compiler (see 6).
4. **A whole sentence is one message.** No sentence is assembled from
   fragments in code; a choice (singular or plural, which product,
   which state) is a selector inside the message, where the translator
   can reorder it. Counts are passed as numbers and the language's own
   plural rules pick the form.
5. **Language-specific grammar is a function the messages call**, not
   an edit of the caller: for Korean, the particle after a value is
   chosen from how the value ends; other languages add theirs the same
   way.
6. **Checks, in the gate**: every key of English exists in Korean and
   uses the same variables (the two languages the project writes
   itself are complete; a contributed language may be partial and the
   check lists what it lacks without failing); no message of any
   language uses a variable English does not pass; a pseudo-language
   (longer, accented text generated from English) is built in for
   looking at screens, so a string that never went through the files
   and a layout that cannot take a longer word are both seen.
7. **Every message carries its context** as a comment in the English
   file: where it is shown, what the variables are, and whether it
   must stay short (a badge, a button). The existing doc comments on
   the tables are moved there, not dropped.
8. **Plain strings cost what they cost today.** Messages without
   variables are resolved once per language and kept; no parsing or
   allocation happens while a frame is drawn.
9. **The page uses the same files** when it gets its second language
   (the format has a maintained JavaScript implementation), so a
   string is translated once for every surface.
10. **Dates, relative times and grouped numbers** follow the language
    through the same messages (their forms are selectors too), never
    through a second table in code.

## How it lands

A track of its own (L) builds the mechanism beside the present tables
and moves one small area through it as the model, with the documents a
contributor needs: how to add a string, how to add a language, how to
see what a language lacks. After review and the lead's merge:

- New strings go into the files only.
- Each area's owner moves its own tables when it next touches them;
  track L moves the areas nobody is working in. A table and its
  messages never exist side by side for the same string.
- When the last table is gone, `Lang` stops being a two-variant enum
  and becomes the tag of an embedded language.

Until the mechanism is merged, tables are still written, but written
so that moving them is mechanical: whole sentences, counts as numbers,
no particle fixed after a value, a doc comment that says where the
string appears.

## Alternatives

- **Keep the Rust tables, one module per language.** Still code a
  translator cannot touch; plurals and word order still need branches.
  Rejected.
- **A flat file per language (JSON, TOML, YAML) with a small loader.**
  Simple and every tool reads it, but plural forms, word order with
  several variables and grammar functions have to be invented on top,
  and each invention is one more thing only this project knows.
  Rejected.
- **gettext (`.po`).** The widest tool support and proper plurals, but
  the source string is the key (a reworded English sentence orphans
  every translation), variables are positional, and there is no
  maintained pure-Rust runtime that also builds for the browser.
  Rejected.
- **ICU MessageFormat.** As expressive as Fluent; its Rust
  implementation is larger and younger, and translators' tools treat
  the syntax as opaque text. Rejected for now.
- **Project Fluent.** Made for this problem by the people who shipped
  a browser in a hundred languages; asymmetric (a translation may be
  more complex than its source without the code knowing); maintained
  implementations in Rust and JavaScript; read by the common
  translation platforms. Chosen.

## Consequences

- The compiler no longer proves Korean is complete; a gate check does.
  That is a weaker guarantee bought for a larger gain, and it is the
  same guarantee for every later language.
- A dependency is added to `view` (the Fluent runtime and plural
  rules). It must build for `wasm32` (the browser track compiles
  `view`) and its size and start-up cost are measured before the
  decision is final; if either is unacceptable the fallback is the
  same files compiled to Rust tables at build time, which keeps every
  rule above except messages more complex than their source.
- Moving the tables touches every area while eight tracks add strings.
  The order in "How it lands" keeps two hands out of one file; the
  cost is a period in which both forms exist.
- Translators need to see the screens. Captures per area in both
  themes are already made for review; they are kept for this too.

## Checks before this is final

Track L reports, with numbers: the app's size and time to first frame
with and without the runtime; `view` built for `wasm32`; a plural, a
selector and a Korean particle through the function, each tested; the
build failing on an unknown key and on a missing variable; the gate
failing when Korean lacks a key; a third language added as a folder
with one file and two messages, listed in the chooser, falling back
for the rest; the pseudo-language on three screens.


### Track L experiment (2026-11-13; not a final acceptance)

The decision above is unchanged. The engine is staged beside the Rust
area tables; no area has moved in slice 1. The language chooser and three
normal-screen pseudo-language captures belong to slice 2 by the
coordinator's clarification. Slice 1 exposes the toolkit-free registry and
selection rule, and tests pseudo-language expansion.

The experiment uses `fluent-bundle` 0.16.0 for resolution and CLDR plural
selection, `fluent-syntax` 0.12.0 for its maintained AST (build validation
and runtime resources), `fluent-pseudo` 0.3.3 for expanded accented text,
and `unic-langid` 0.9.6 for checked tags. These are the
[Project Fluent Rust implementation](https://github.com/projectfluent/fluent-rs)
and its language-identifier dependency, under MIT or Apache-2.0.
The parser is reused, not reimplemented. Build-time AST inspection defines
Vessel's supported function/variable contract and generates the Rust API.
`allocation-counter` 0.8.1 is native-test-only (MIT or Apache-2.0), with no
shipped allocation hook or unsafe code in Vessel.

An optional `icu-formatters` feature measures GROUP and DATETIME using
ICU4X 2.4.0 (`icu_decimal`, `icu_datetime`, `icu_locale_core`, and
`icu_provider` with `sync`, under Unicode-3.0). Their upstream is
[ICU4X](https://github.com/unicode-org/icu4x). These permissive licenses
satisfy ADR 0014. ICU's default `compiled_data` in this cost experiment
includes the upstream baked locale set, rather than only the embedded
English/Korean/Japanese locales. This is deliberately not an accepted
production data scope. A final ICU implementation needs a provider limited
to GROUP and the Gregorian YMD date fields for embedded locales; if the
installed cost exceeds about 1 MiB, work stops for review before extending
it. The alternative allowed here is language-owned number/date messages
and selectors.

Measurements use Rust 1.98.1 on x86_64 Linux, the native workspace's shipping release
profile (thin LTO, default codegen units, line tables). The release script's
`objcopy --only-keep-debug`, then `--strip-debug --add-gnu-debuglink`,
produces the executable whose bytes are compared. The debug sidecar is
excluded. The lockfile adds packages without changing any existing package
version, so shared dependencies are not upgraded between the variants. All artifacts are in the track's private build output, with no
writes to the distribution directory.

First content paint is observed on a private Xvfb display with Mesa software
Vulkan, a 1280 by 800 English dark window and an isolated empty app
configuration, no updater and no node. A monotonic clock starts before
process creation. X11 image reads of the upper-left content region `(50, 90, 400, 80)`,
sampled with a 2 ms pause, bracket the last empty image and first painted
image. More than 12 distinct byte values in every seventh pixel byte
identify painted content; the baseline black root does not meet that rule.
Nine serial launches report median bounds; shared-machine scheduling and
software rendering prevent a claim about customer hardware performance.

| Release variant | Installed executable bytes | Median first content paint (ms) |
| --- | ---: | ---: |
| Original app | 73,652,328 | 314.459–319.510 |
| Fluent alone | 74,050,200 | 246.470–254.741 |
| Fluent and ICU cost experiment | 75,337,192 | 238.233–242.499 |

Fluent alone adds 397,872 bytes. The timing difference is not evidence of
an improvement: the runs were serial on a shared machine, not a controlled
CPU/GPU performance experiment. ICU adds 1,286,992 installed executable
bytes (1.227371 MiB) over Fluent alone. That exceeds the coordinator's
approximately 1 MiB limit, so work stops before building further on this
backend. The row is the full baked-data experiment described above, not a
measurement of a properly scoped provider and not proof of ICU4X's minimum
cost. No area migration or slice-1 commit is accepted on these results.
The next decision is a genuinely scoped provider and another measurement,
or language-owned number/date messages and selectors, as allowed above.

The standalone `view` release probe builds for
`wasm32-unknown-unknown`, links plain and variable message access, and
retains formatter initialization. With the ICU cost experiment, its raw
wasm is 9,731,510 bytes, or 3,209,874 after `llvm-strip --strip-debug`.
Fluent alone is 7,649,599 bytes raw and 2,281,727 after the same strip
operation: the ICU experiment adds 928,147 stripped wasm bytes. This is
the localization probe, not the whole GPUI browser app. The
protocol's existing UUID needs the `js` feature for this standalone build;
`view` temporarily enables it only for `cfg(target_family = "wasm")`,
matching GPUI's selection in the complete app. The lasting manifest owner
is `vessel-proto`; the browser track should move it there on integration.

The whole native gate passed: format, all-target Clippy with warnings
as errors, 329 ordinary tests and two compile-fail doctests (unknown key
and missing variable). A final gate including the newly added wasm probe
is still pending. The allocation test initializes all embedded languages
and the pseudo-language, then makes 10,000 iterations of three plain
getters and the cache lookup for each: zero allocations and zero bytes.
English/Korean whole-sentence tests cover selectors, relative time and
particles; Russian fixtures distinguish one/few/many plural rules and
English fallback rules. Korean completeness/variable parity, contributed
partial coverage, extra variables and invalid references are gate tests.
The Japanese folder contains one file with exactly two messages, is listed
as 日本語 with `complete=false`, and falls back for eight absent messages.
The coverage command prints each absent key. The missing-Korean-key
mutation of the actual cargo-test gate is still pending.


Review status at the size stop: the prototype's Latin letter-name rule
omits R (알, a rieul ending), and the license test's host-filtered metadata
needs an additional all-target runtime-dependency guarantee. Neither is
accepted for a final slice. They remain explicit follow-ups alongside the
pending final gate and actual missing-Korean-key build probe; passing the
earlier tests is not a claim that these gaps are solved.

### Selected backend after the size review

The coordinator selected Fluent only. ICU4X formatting dependencies,
license rows and the experimental feature have been removed completely.
The historical three rows above remain the measured experiment. The
coordinator rejected both its installed/browser cost and a separate scoped
ICU-data regeneration workflow, which would obstruct decision 2's
folder-only language contribution. The full-data row is not a lower bound
on ICU4X itself.

The runtime registers the standard names NUMBER and DATETIME. NUMBER
retains Fluent's numeric value for language-owned plural matching; a
formatter hook uses checked rules from `formats.ftl`: grouping/decimal
separators, primary/secondary group sizes and ten digit characters. The
ordinary host build generates static rules from the folders, without a
contributor tool run or checked-in generated Rust data. DATETIME receives
typed Gregorian date/local-clock parts and a caller-resolved zone label,
and resolves whole patterns in the same language's folder. Month/weekday
names, order, wording and h12/h24 convention are data. A missing whole
pattern uses English's pattern and convention. Relative times remain
ordinary messages with numeric counts and Fluent plural selectors.

This is a small formatter, not complete Intl. It does not convert calendars
or time zones, discover numbering systems beyond the file's ten characters,
or implement currency, percent or significant-digit formatting. Typed
decimals and numeric catalog literals are finite, within the u32 magnitude
and limited to nine decimal places. The supported NUMBER options are
grouping, minimum integer/fraction digits, maximum fraction digits and
cardinal/ordinal type; rounding retains a number for plural matching.
Date/time patterns currently receive year, month, day, Monday-based weekday,
hour, twelve-hour hour, minute, am/pm selector and zone label. They cannot
express information those parts do not contain, including seconds, other
calendars or zone arithmetic.

The Latin particle rule now includes L/M/N/R, with R taking the rieul form.
The license inventory no longer filters by host: Cargo's declaration-only
metadata includes every target, and the test follows normal local path
dependencies across workspaces. Its fixtures retain the development-only,
missing-runtime and both-kind guarantees, and add another-target runtime,
transitive-path and published-name alias coverage. The original font check
and registry-order comparison remain.

Actual negative-build probes fail as required: unknown generated method
E0599; missing argument E0061; deleting Korean `meta-system` makes the
whole native `cargo test` fail at the catalog gate. The invalid example
was removed and the Korean file restored byte-for-byte before the final
gate. Final release/wasm record and final gate results follow below.


### Final Fluent-only record (same protocol)

| Final variant | Installed executable bytes | Median first content paint (ms) |
| --- | ---: | ---: |
| Fluent with folder-owned number/date patterns | 74,116,816 | 251.905–258.587 |

This is 464,488 bytes (0.442970 MiB) above the original app and 66,616
bytes above the initial Fluent-only experiment. The final release build
took 6m30s with the existing cache; it is not a clean-build comparison.
Its standalone wasm localization probe is 8,092,410 bytes raw and 2,426,698
after debug stripping (38.88s with cache). No browser UI or download-size
claim is made. The same nine-launch X11 content criterion, release profile,
stripping procedure and isolated configuration were used. The observed
lower timing is not attributed to the engine as a speed improvement.

The final complete native gate passed after all negative probes were
restored: format, all-target Clippy with warnings as errors, 332 ordinary
tests and two compile-fail doctests. It includes zero-allocation plain
access, language-owned plurals, selectors/particles, whole English/Korean
date/time/relative-time sentences, numeric-rule variation, schema rejection
and the all-target license inventory. The Japanese folder still contains
exactly one file and two messages. The compiled list exposes 日本語 as
incomplete, with 22 absent messages reported and English fallback tested,
including the English hour convention when its time pattern is absent.
The chooser UI and three normal-screen pseudo-language captures remain
slice 2 by the coordinator's explicit staging contract. No area table
moves in this mechanism commit.

The final formatter names `intl-memoizer` 0.5.3 directly for Fluent's
per-language display hook (Apache-2.0 or MIT); it was already a Fluent
transitive dependency in the initial runtime row, so this adds no package
to that tree. Existing chrono/serde/serde_json supply checked date/clock
parts and the internal value encoding. The final lockfile adds nine
runtime packages for the Fluent stack and one native-test-only allocation
counter over the original app; existing package versions are unchanged.
The installed/wasm measurements above include the generated API, embedded
catalogs, grammar, formatter code and their linked transitive costs.

### Release catalog boundary (slice 2 correction)

The coordinator accepted slice 1, then required its partial Japanese proof
out of the released app. `view/locales/` is release input: every folder is
embedded and listed. The two-message Japanese file now belongs solely to
`view/tests/fixtures/locales/`, alongside a small complete source pair.
Registry, fallback and coverage tests use that fixture root. The ordinary
app ships complete English and Korean; a shipped-catalog test rejects any
incomplete entry. Core validation and English fallback still support partial
candidate/test catalogs, inspected with the coverage tool's `--root` mode.
A contributor checks the current source/candidate root for zero missing
keys and passes the native gate before adding a release folder. While the
legacy tables remain, coverage of migrated messages alone does not prove
an additional language covers the whole app.

### Pairing model and stored language (overnight checkpoint)

The pairing dialogs now read their thirteen original messages from Fluent.
The pairing-only app-table messages and complete paired-status selector also
move; shared file-fault and other areas' tables stay with their owners. Typed
decoder/connection/storage/name/browser problems select complete English and
Korean messages. Dynamic outcomes retain typed data and prepared text so a
language event can reword them without formatting during drawing. The model's
allocation tests include plain pairing getters and shared prepared failures.

General uses the embedded registry for a stored language preference, System
first and native names after it. The pseudo language remains override-only.
The normal UI has verified persistence, first-paint restoration, override
precedence, a retained typed draft and a paired-status sentence changing in
the same process. Headless real-control tests exercise the actual language
change for find, question, queue and project fields, preserving text, selected
character, caret, focus and current step.

The first model gate passed format, all-target Clippy, 430 ordinary tests and
two compile-fail tests; its changed view/link/app crates were recompiled.
The later source also takes the lead's response-ownership fix unchanged and
shortens an English empty-name hint after pseudo inspection found its phone
viewport clipping. At this checkpoint that final source has passed format
and all-target Clippy. Its remaining final gate and short-hint recapture
are recorded in the track notes rather than asserted complete here.

Personally inspected evidence includes 32 pre-migration screens, a 60-frame
English/Korean/pseudo pairing matrix, twelve General screens, 24 real native
connection/storage/name failures and eight busy frames, eight test-only
browser-unsupported renderer frames, nine normal preference-flow frames and
21 completed pairing/save/live-language frames. The three normal pseudo
views are paste-link, confirmation and General. E2's unmigrated sending and
preferences rows remain visibly plain beside the pseudo Language row.
Private storage-failure folders were restored from 0500 to their original
0700 under the same UID; saved preferences/books are 0600. No service account
or production permissions changed.

Browser-unsupported captures prove wording/layout in the actual native test
renderer, not a browser transport. Linux Xvfb/Mesa captures do not verify
Windows, the complete browser app or a real GPU. Release size/paint numbers
above measure the accepted engine experiment; no new installed-size or
performance claim is made for this model checkpoint. The contributor guide
now records adding strings/languages, coverage, pseudo runs and the one-area
migration recipe, including one cached-placeholder refresh method registered
in a single dispatch list.

The subsequent final normal build completed with exit zero. Twelve fresh
English/Korean/pseudo confirmation captures from that binary were inspected
in both themes/sizes: the shortened empty-name hint now fits completely in
the phone's pseudo viewport. This closes that visual finding. The earlier
broad matrices retain their original binary provenance. At this later
checkpoint the final native test and explicit wasm command still await
their recorded exits; no final gate acceptance is inferred from the build.
The isolated fleet/holder and fake children (17 verified PIDs), private
display and protected pairing inputs have been cleaned up.

The final native gate subsequently completed with recorded zero exits:
format, all-target Clippy, 432 ordinary tests and two compile-fail doctests.
This includes the unchanged lead late-answer regressions and all four
actual-dispatch input/caret/focus retention tests. The final standalone
wasm command is still queued at the overnight checkpoint; its eventual
exit is recorded separately rather than inferred from native success.

The final standalone `vessel-app-view` check for
`wasm32-unknown-unknown` then completed with its own recorded exit zero
(41.25s after waiting for a slot). The full sequential native/wasm runner
completed with exit zero, with frozen compiled-source hashes unchanged.
This closes the pending-target checkpoint above; review of the model is
still pending and no broader platform/runtime acceptance is inferred.
