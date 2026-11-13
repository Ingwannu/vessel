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
