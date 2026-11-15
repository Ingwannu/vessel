# App translations

English defines the typed Rust API; Korean must translate every message with
exactly the same variables. Files are embedded at build time. Never edit the
generated Rust: adding a string or language needs only Fluent files and its
normal typed caller.

## Add a string

1. Add a key to `en/<area>.ftl`, with a comment naming its screen, context,
   values, and whether it must stay short. Add its whole Korean message to
   `ko/<area>.ftl` with the same variables.
2. Use the generated method: `catalog.pairing().title()` for a plain label.
   A value produces a required, typed argument; count selectors receive
   `Number::count(count)`, not a formatted string. Unknown keys and omitted
   arguments do not compile. Check an existing area for the generated enum
   names and the `Result` returned by a message with values.
3. Prepare the catalog before drawing. Plain getters borrow cached strings.
   Format messages with values when the value changes and keep the result;
   handle a formatting error at that boundary, never by printing Display in
   the screen. Run the coverage report and the complete native gate.

A whole sentence is one message. Use Fluent selectors for a count or a state,
with whole sentences in every branch; do not join translated fragments. A
translator may change the selector and sentence order. Give separate keys to
places with different meanings even if their English words happen to match.
Keep context comments and mark buttons, badges and other short labels.

A Korean particle whose sound depends on a value uses `PARTICLE($value, "topic")`
(or `subject`, `object`, `with`, `direction`). The [grammar table](../src/text/localization/grammar.rs) states the Hangul, digit and Latin rules.
Particles that do not vary, such as 의 and 에, can follow a value directly.

`NUMBER` and `DATETIME` use the language's `formats.ftl`: separators, group
sizes, digits, month/weekday names, h12/h24 and whole date/time patterns.
English is the fallback for absent formatting rules. Dates are typed Gregorian
parts already converted to the viewer's time zone by the caller. Other
calendars and automatic time-zone/numbering-system discovery are not provided.

## Add a language

Prepare `<tag>/` with a short `meta-name` written in that language, then its
area files. No Rust language list or enum is edited. Work in a candidate root
outside this directory, alongside current copies of `en/` and `ko/`, and run
from `spikes/native`:

```bash
scripts/locale-coverage.sh --root CANDIDATE_ROOT
```

The report lists each missing key. A candidate may be partial and missing
messages fall back individually to English, using English plural rules.
A folder in this directory **ships and appears in General's language chooser**
by its own name, after System. Move it here only when the report says
`0 missing`, `complete=true` for every shipped language, with no syntax,
variable or format-rule errors; run the complete native gate before committing.
Coverage covers migrated messages only: until every table moves, remaining
screens still use the English/Korean tables. The partial Japanese folder lives
only in `view/tests/fixtures/locales/`, never in the shipped registry.

```bash
scripts/locale-coverage.sh                           # shipped catalogs
scripts/locale-coverage.sh --root view/tests/fixtures/locales
```

## Check a screen

With the normal app and your private fake fleet, set `VESSEL_APP_LANG=en-XA`
when starting it. Accented, expanded labels expose tight layouts. The pseudo
language is only a checking override and never appears in the chooser or a
stored preference. `VESSEL_APP_LANG=en` or `ko` also overrides a stored choice;
unset it when checking that General's language change applies immediately.
Use dark/light themes and desktop/phone sizes as described in the native README.


## Move an area

Audit each string's callers first. Move only that area's own messages; shared
words stay with their owner until that area moves. Copy both languages and
translator context into `<area>.ftl`, retaining whole-sentence tests and all
states. Use the screen's existing prepared `shell.words` catalog for plain
getters; prepare messages with values in view when data changes, retaining
typed data so the existing outcome can be retranslated on a language change.
`view/src/pairing.rs` and its whole-sentence tests are the model.

Delete the old area table and its exports after its callers have moved.
A mixed table such as `text/app.rs` loses only the audited area-only entries.
Capture the same real screen before and after, English/Korean, dark/light,
desktop/phone, long/empty/loading/failed states and the pseudo override.
Inspect each capture against its reference. Change one area per commit;
run the entire native fmt/Clippy/test gate and the coverage report, document
shared strings and every other owner's registration seam, then send it to
review before starting another area. Coordinate ownership before touching
an actively edited table.

A field that retains a translated placeholder exposes one refresh method.
Register it in `app/src/language.rs`'s `PLACEHOLDERS` list. Refresh only the
placeholder; retain the typed value, caret, selection, current step and focus.
Test that retention with the actual language-change dispatch and input keys
(the find, question, queued-edit and project-form tests are examples).

For value messages, retain the input facts and shared prepared text at state and
language events; invalidate cached rows/measurements when that text changes.
`view::updates` and `view::fork::Headers` are examples. A frame reads the
prepared value instead of formatting it again. Evict cache entries with their
source data, and test unchanged refreshes as well as renames/language changes.

Project errors from typed variants/codes into complete recovery messages.
Keep original diagnostics in the host's logs or authoritative state, never in a
translated sentence's argument. Test every variant in both languages. When a
wording-preserving move exposes this old defect, make its typed correction a
separate commit before submitting the screen as complete. Retained global
notices refresh only while they still own the visible notice, preserving any
later message from another area. A language change must not resend a request.
