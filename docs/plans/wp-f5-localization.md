# F5: localization mechanism and package words

Owner: the words track. This plan owns localization selection and the core
translation contract, not the other F5 owners' settings/theme/font behavior.
The earlier app-i18n plan and ADR 0039 retain their measured history.

## Specification and existing skeleton

The reference is the localization mechanism survey and Paseo's
`i18n/{provider.tsx,locales.ts,sync-language.ts,i18next.ts}`. Settings is
client-local, supports System or an explicit language, resolves before
children draw, bundles offline resources and keeps plugin localization separate.
There is no host language RPC or new link Command/Event for this feature.

The already built skeleton is `LanguagePreference::{System,Language}` plus
`Locale` and the generated typed catalog. English defines the API; the shipped
catalog completeness/variable contract is tested in both directions. A prepared
Translator lives before the first frame; plain getters borrow cached strings.
Initialization and formatting failures have separate typed enums. Current
preference-change dispatch retains input values, caret, selection and focus.
Unknown stored tags become System without losing other preferences.

## Preferred system locales

[Decision Log]
- Purpose: an unsupported first system language must not hide a supported
  later preference, matching the reference's ordered locale loop.
- Existing assets: sys-locale already exposes get_locales on every supported
  host, including navigator.languages on the browser; the view resolver
  already validates against the folder-generated registry.
- Alternatives: a second locale detector/dependency or parsing an environment
  language list ourselves would duplicate the provider and differ by platform.
- Choice: add an ordered pure resolver, retain the old single-tag adapter and
  pass the existing provider iterator at startup/preference changes.
- Why: exact tag, then base language, then the next system tag share one rule;
  English is the final fallback only after all system candidates are tried.
- Effect: no dependency, node call, preference schema or frame-time work is
  added. Explicit preference and the checking override keep their precedence.

Tests cover unsupported-first then Korean, supported English first, supported
Korean first, case/POSIX spelling, no supported language, an empty list, owned
host strings, invalid stored choices, explicit choices and override precedence.
The old single-tag/exact-region tests remain unchanged. The reference's
locales and synchronous-language tests were read: explicit choice, regional
Korean, English ahead of a secondary language and preparation before children
remain the contracts; no reference translation resource was imported.

## Package boundaries and Korean review

Each package owns its own new `view/locales/en/<area>.ftl` and Korean sibling,
its typed view adapter, event-time value cache and normal screen callers.
Existing generator/runtime/registry files belong to this track. Do not add
Rust string tables or a formatter/parser in a screen. Call the prepared
`shell.words` catalog; dynamic sentences retain typed input facts and reformat
only on data/language events. The locales README is the contributor contract.

A package sends its final English/Korean key diff and source commit to this
track once before closing. Review checks context, whole sentences/selectors,
same keys/variables, short labels, terminology, Korean particles and typed
recovery messages; it does not replace the package's behavior review. The
package's single whole native gate includes the ordinary catalog tests.
Narrow checks belong to its commits; no separate whole gate per wording commit.

Core resources are build-time embedded and cannot be extended at runtime.
A plugin consumer takes the validated `Locale` from `Translator::locale()`
and resolves its own contribution text in the plugin's namespace. Only the
plugin's wire boundary serializes `Locale::tag()` to external text. It must not add
arbitrary provider/plugin keys to core Fluent or override core functions.
The future plugin owner owns that resolver and contribution refresh; its live
integration remains separate from the core's offline mechanism proof.

## Deliberate differences and close conditions

The coordinator retained ADR 0039's English/Korean release boundary for this
parity batch. Additional languages wait until the parity packages and must be
complete before shipping; the partial-language proof remains a test fixture.
No reference translation asset is copied. Language choices retain the earlier
approved System-first/native-name contract rather than a hardcoded translated
name matrix. Regional alias rules for languages not embedded are not claimed.
The checking override/pseudo locale is a Vessel verification extension and is
never a stored choice or chooser item.

Paseo memoizes system tags per provider mount. Vessel reads its provider at
startup and explicit preference changes, without subscribing to live OS locale
changes. Browser/native use the same pure resolver; current browser/Windows/OS
integration is not inferred from native unit tests. Older legacy areas still
use their bilingual tables; package owners add new words through FTL, and
ownership is coordinated before old tables move.

Current state: the ordered-list implementation is written over the accepted
translation packages. View/app all-target Clippy passes; localization execution
tests are queued, so this is a WIP checkpoint, not a package pass. Eight retained
before frames reproduce the unsupported-first English fallback and English-first
control in both themes and desktop/phone sizes; every original was opened and
hash/dimension checked. The capture apps/private display were stopped. Whole
gate/review and the targeted after
normal-app unsupported-first locale check are owed at package close. Merge the
announced app-base head before that close; no gate of an older base is a claim
about the combined application. Plugin runtime integration belongs to F6.
