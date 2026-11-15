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

Current state: source implementation `eabf5ed4` passes view/app all-target
Clippy and all 19 localization tests, including the two new preferred-list
regressions, retained single-tag behavior, pseudo/fallback behavior, released
catalog completeness and warmed plain-getter zero-allocation checks. Changed
crates actually recompiled. The current explicit-root data report has English
and Korean complete with zero missing; the partial Japanese fixture has its
expected three missing messages and is not shipped.

The approved package base `8d5513e4` was merged as `5b36d990`; the locale
implementation is retained unchanged and root crates/manifests/lockfile/page/
scripts match the incoming base exactly. Its contract adaptations and link/event
splits are incoming ownership, not new locale features.

The exact merged source `8a944a237c5f7377b7387c9782793b2a934dbeeb` passed its
one whole native A13 run `i18n-20261115T065422Z-8a944a23`: fonts, formatting,
quality, Clippy, build and tests each exited zero, without a skipped step.
Changed view/link/kit/app crates recompiled. Raw test totals are 1,213 passed
(1,211 ordinary and two compile-fail documentation tests), zero failed and one
ignored. Both preferred-order regressions, released catalog completeness and
plain-getter zero-allocation tests passed. An earlier unregistered submission
was retired during the build-tool owner's authorized submission repair; it was
not a compiler/test failure or a second registered package gate.

All 16 startup originals were opened and hash/dimension checked: eight retained
before frames and eight after frames, both themes at desktop/phone sizes.
With no override or stored choice, `LANGUAGE=zz-ZZ:ko-KR:en-US` starts in Korean
instead of English; the `LANGUAGE=en-US:ko-KR` control stays English. Private empty
configurations and owned display/app processes were used and stopped. Captures
show settled startup before interaction, not a frame-one timing/flicker proof
or a whole geometry comparison across the accepted base merge.

Standalone view's wasm library check and full library build/code generation
passed on the same source. The unstripped dev rlib is 26,980,654 bytes; this is a
library artifact, not an installed-app or browser-bundle size. Neither proves
browser app execution. V1 accepted this exact package with bounded source,
native gate and inspected startup visual approval; it independently opened all
16 originals and verified their hashes/dimensions and retained app binary. Current Windows/OS change subscriptions,
real provider/node/updater flows, persisted-preference I/O, accessibility, real
GPU and plugin runtime integration are not inferred from these tests or frames;
the per-installation plugin integration belongs to F6.

## Coordinated package word seams

- A3 owns new `chat-outline.ftl`, with plain label/hint getters. Its Chat switch
  follows the display row; it preserves the existing Chat catalog and adapter.
- A7 owns new `chat-reasoning.ftl`, a third display-choice key in `chat.ftl` and
  the exhaustive `Chat::display_choice` arm for its new closed enum variant.
  The existing six Chat messages stay unchanged; its reasoning row is first.
- E4 owns `onboarding.ftl` and `diagnostic-flow.ftl`; underscores are Rust getter
  spellings, never area filenames. It consumes the existing translated
  Diagnostics header/Run/Copy/Refresh and keeps technical report schema separate.
  Its revised final diagnostic-flow wording at `c2000083` passed the words
  review and explicit-root EN/KO contract check; remaining typed-problem
  sentence tests were requested. This is not complete-package approval.
- A9 owns its new `models.ftl` domain and typed helper, without a new formatter
  or dependency in the core localization mechanism.

These are word/API seams, not behavior or whole-package approvals. The package
owners coordinate shared Chat edits directly; this track does not write their
screens or preferences in parallel.

## Accepted core package and later work

V1's disposition is limited to exact `8a944a23`, its one native gate and the
inspected startup states above. It explicitly excludes frame-one/flicker or
geometry parity, Windows/real GPU/accessibility, node/provider/updater flows,
new persisted-preference I/O and plugin localization. No unchanged gate or
capture matrix was requested. Documentation-only closure commits retain this
boundary rather than claiming a current combined application approval.

The subsequent package-base-2 merge `55ef3679` brings `6036eeb8` without changing
the preferred-locale implementation or root crates. Its incoming machine-node
identity and field editability/listener seams are not part of this accepted
locale feature. The following F6 shared bundle/contract work is a separate
consumer seam, documented in its own plan; its checks must not borrow this
package's earlier gate or startup approval.
