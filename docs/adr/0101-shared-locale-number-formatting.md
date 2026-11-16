# ADR 0101: Shared locale number formatting

Status: proposed, 2026-11-16. The input and parity requirements below are
assigned; the implementation backend is **not chosen until measured**.
This extends ADR 0039's numeric capability without changing its translation
folder, typed-call-site, fallback or preparation rules.

## Context and reference

Paseo 0.15.4, reference commit
`1c00b365c9e0b74c1580eba33d905d693125ae21`, defines `formatAmount` in
`packages/app/src/usage/format.ts:185–199`. Its callers pass the app language
(`usage/balance-bar.tsx:20–67`). The three policies are:

| Source unit | Presentation |
| --- | --- |
| USD | Locale currency symbol/pattern, USD fraction digits |
| Tokens | Locale short compact notation, at most one fraction digit |
| Other units | Locale grouping, at most two fraction digits |

`usage/format.test.ts` covers grouped amounts, currency and compact tokens.
Vessel currently has a folder-owned `NUMBER` formatter, but its public
`Number::count` accepts only `u32`, `Number::decimal` also limits magnitude
and decimal places, and its formatter has no currency or compact engine.
An area-local grouping function or passing a preformatted ASCII count as
text does not close this gap.

The same caller needs exact `u64` token counts and duration counts. Casting
them to `f64` loses integer information beyond `2^53`. A full-finite amount
and an exact unsigned count therefore cannot share an unqualified float
representation. The existing `vessel_proto::conversation::FiniteNumber` already admits
every finite binary64 value; reuse that validation instead of defining
another finite-number wire scalar.

## Fixed contract

[Decision Log]
- Purpose: one shared presentation kernel for amounts and exact counts,
  consumed by Usage and later areas, with locale data outside Rust.
- Existing assets: Fluent's maintained AST/runtime, generated typed getters,
  embedded formatting rules, locale selection, `FiniteNumber`, and prepared
  display caches. Reuse them rather than adding another area formatter.
- Alternatives: widen a float conversion, clamp to `u32`, drop an excessive
  value, or keep English grouping in each area. These lose valid inputs or
  duplicate policy and are rejected.
- Contract: preserve typed numeric values and units until preparation;
  distinguish finite amounts from exact unsigned counts; produce display
  text at state/language changes, then borrow it while drawing.
- Tradeoff: full-range rounding, compact carry and plural selection need a
  more capable numeric representation. Backend cost and policy integration
  must be established before adopting a dependency or publishing an API.

The view boundary will have two distinct input capabilities:

- An amount holds the existing `FiniteNumber` and a closed presentation-unit
  choice: USD currency, compact tokens, or ordinary decimal quantity. This
  choice classifies formatting; it is not a second wire business-unit enum.
  A consumer maps its canonical unit exhaustively and retains that unit.
- An unsigned count retains all `u64` bits. Its ordinary grouped rendering
  never routes through `f64`, a saturating cast, or an ASCII-text argument
  pretending to be a numeric message variable.

Fields are private and constructors accept validated types. The exact public
type names are settled with the measured backend, not by exposing its types
to app callers. The prepared result is immutable display text, separate from
the numeric input; callers never parse it back into a number. View performs
no I/O and takes no GPUI type. Formatter construction/data checks precede
rendering. A valid numeric input must not become a range error or a placeholder.
Any value-based preparation cache must retain the sign of zero; the existing
finite scalar's equality alone does not distinguish the two formatted zeros.

## Rounding and range

Follow the pinned source's `Intl.NumberFormat` options and
[ECMA-402 edition 13](https://402.ecma-international.org/13.0/#sec-numberformat-objects):

- Nearest rounding uses ties away from zero (`halfExpand`). Decimal rounding
  follows the Number-to-decimal semantics used by Intl, not an extra binary
  multiplication followed by a cast. Preserve the sign of negative zero.
- USD has two displayed fraction digits, including trailing zeros; its sign,
  symbol and position follow the selected language.
- Compact tokens have zero to one fraction digits. Scaling and suffixes are
  locale data; rounding that crosses a compact boundary must select the new
  magnitude. Do not apply English K/M/B/T rules to Korean.
- Other units have zero to two fraction digits. Ordinary counts retain all
  integer digits; grouping/digit characters follow the language.
- Accept negative finite amounts, subnormals, both zeros and both signs of
  the largest finite value. Do not replace a large amount by infinity.
- Cover the complete unsigned range through `u64::MAX`, including adjacent
  integers above `2^53`. Display and numeric plural semantics both matter;
  rendering a lossless string while selecting plurals from a rounded float
  is insufficient.

Fluent's built-in numeric carrier is a float. The implementation must document
how exact counts reach `NUMBER` and selectors without losing information.
Any required typed adapter or maintained-runtime extension is part of this
decision; no claim is made that the current `Number` already provides it.
Keep standard `NUMBER`/`DATETIME` names and avoid copying Fluent's parser or
hand-writing plural rules. Date/calendar and timezone behavior is outside
this change.

## Backend alternatives, not yet adopted

1. **ICU4X number components and their data.** Investigate only decimal,
   compact and USD-currency capabilities. Current compact/currency APIs have
   unstable/experimental surfaces; their rounding defaults are not assumed
   to equal the pinned source's explicit fraction policy. Record the exact
   features and data markers. A locale-subset tool run that regenerates Rust
   whenever a translation folder is added conflicts with ADR 0039. Full data
   avoids that workflow but must be measured in the installed app.
2. **A maintained decimal representation plus narrow folder-owned rules.**
   `fixed_decimal` offers unsigned/finite-float input, decimal rounding and
   negative-zero representation without a locale database. Investigate it
   as the kernel, with currency/compact patterns in translation folders and
   existing grouping/digit rules reused. Do not hand-roll float parsing or
   a second plural engine. Narrow tables must still reproduce the required
   complete numeric domain and compact carry behavior.
3. **The current formatter or bounded decimal arithmetic.** Keeping its
   `u32` ceiling or selecting a fixed-precision type that cannot hold valid
   source magnitudes fails the contract. This is not an acceptable fallback.

Primary API references:
[ICU4X compact formatter](https://docs.rs/icu/latest/icu/decimal/struct.CompactDecimalFormatter.html),
[ICU4X currency formatter](https://docs.rs/icu/latest/icu/experimental/dimension/currency/formatter/struct.CurrencyFormatter.html),
[fixed_decimal](https://docs.rs/fixed_decimal/latest/fixed_decimal/).
These identify candidates, not dependency adoption or a maintenance/license
approval. Check exact release manifests and ADR 0014 before selection.

## Measurement before selection

Commit this contract before implementation. Keep comparison sources and
checks immutable and record exact revisions, features and commands.

- Compare baseline, ICU-backed, and narrow-data candidates. Use the app's
  shipping release profile and stripping procedure; do not write a
  distribution directory. A standalone probe can establish correctness or
  preliminary cost, but its bytes are not an installed-app size delta.
- Ensure the candidate formatter is actually reachable after linking; an
  unused dependency eliminated by LTO is not a measurement of that engine.
- Record installed bytes, added dependencies/data, elapsed build time and
  the corresponding wasm32 build/size. Keep cold/warm conditions explicit.
  Reuse the earlier first-frame protocol if startup work is introduced.
- Record the supported locale-data set. Preserve adding-a-folder operation
  and English fallback; neither a new contributor tool run nor a hidden
  Rust locale table may be introduced without a superseding decision.
- The previous ICU cost concerns in ADR 0039 remain evidence. If a new
  measurement contradicts its stated constraints, report the conflict before
  building the product on that candidate. No dead experiment feature remains
  in the selected product.

## Dependency experiment and approval boundary

The coordinator requires a yes/no decision on the ADR diff and measured
numbers **before any product lockfile change**. The controlled build-tool selector provisions only an isolated scratch graph
offline from the build cache, preserving the experiment lock/feature manifest.
It is not permission to resolve or download dependencies through the ordinary
product gate. Its first exact run stopped at dependency-edge admission before
compilation; installation of the selector is not candidate validation.
Missing measurement cells remain missing, not zero.

The narrow candidate starts with these exact requests, all `Unicode-3.0`:

| Crate | Version | Features, with defaults disabled | Purpose |
| --- | --- | --- | --- |
| `fixed_decimal` | `=0.7.3` | `ryu` | Shortest finite decimal, explicit rounding, exact unsigned digits |
| `icu_plurals` | `=2.4.0` | `compiled_data` | Maintained exact-operand cardinal/ordinal rules |
| `icu_locale_core` | `=2.4.0` | none | Checked ICU locale input |
| `icu_provider` | `=2.4.0` | `sync` | Shared prepared data compatible with concurrent bundles |

The ICU decimal/compact comparison additionally requests `icu_decimal`
`=2.4.0`, defaults disabled, `alloc`, `compiled_data`, `unstable`, `ryu`. It uses
upstream baked decimal/compact/plural data, with no locale-subset regeneration.
Both candidates use folder-owned USD patterns; the closed USD-only requirement
does not require an experimental all-currency runtime. The experimental ICU
currency API was investigated, but is not in either requested graph. These
are experiment candidates, **not adopted dependencies**; their full transitive
versions/features, license inventory, data markers, reachable linked code,
release-byte deltas on both targets and compile-time deltas are still owed.

| Candidate | Native installed delta | Stripped wasm delta | Compile-time delta |
| --- | --- | --- | --- |
| Existing Fluent baseline | not yet measured for this base | not yet measured | not yet measured |
| Narrow decimal + plural data | not yet measured | not yet measured | not yet measured |
| ICU decimal/compact + plural data | not yet measured | not yet measured | not yet measured |

The existing Fluent float is rejected as the exact unsigned carrier: adjacent
integers first collide at `2^53 + 1`, and display hooks cannot recover the
lost bit or repair native plural matching. This does **not** imply that every
shipped sentence currently selects a wrong category. English cardinal only
distinguishes one from other, and Korean cardinal is always other; for exact
unsigned integers neither changes category above the boundary. No shipped
catalog currently requests ordinals. However, `NUMBER(type: "ordinal")` is
already supported by the validator/runtime: English ordinal last digits can
change immediately at `2^53 + 1` (exact ending 93 is few; rounded ending 92 is
two). Exact display is required even where categories agree.

The deferred survey locales are `ar`, `es`, `fr`, `ja`, `pt-BR`, `ru`, and
`zh-CN` alongside shipped `en`/`ko` (`gaps.md`, localization section).
Russian and Arabic cardinal rules inspect last digits; Spanish, French and
Portuguese also have integer-million divisibility conditions in current CLDR rules.
They require exact operands. This describes the survey, not a decision to
ship additional languages or copy their translation assets. The maintained
[CLDR rule chart](https://unicode.org/cldr/charts/latest/supplemental/language_plural_rules.html)
is the rule reference, not a second hand-written rule engine.

The coordinator permits a **scratch-only** minimal extension of Fluent
0.16's custom-value numeric options/matching hooks. Stock custom types expose
display only, while numeric matching is hard-wired to the float variant.
Any experiment must preserve the parser/resolver and ordinary numeric path,
avoid downcasts in Vessel code, record touched files/lines, and demonstrate
how the patch is checked/re-applied on the next Fluent release. Vendoring is
a continuing upgrade cost, not a free adapter. Compare it with the cheapest
exact alternative before requesting adoption: an explicit maintained-rule
plural grammar function plus a typed exact-count boundary, or AST lowering
using Fluent's existing parser. Neither alternative may smuggle a lossy
operand into plural selection or require an English-only area formatter.
No runtime patch is installed in the product by this document.

The scratch hook artifact in `experiments/number-format` currently changes
five upstream files, adding 56 lines and removing three. Original/resulting
source hashes and the existing archive checksum are recorded beside it.
It also applies by source context to upstream main
`0f87afbbc3fe7d20c61a3fc75cb5d0e123aefdda`, whose bundle manifest still says
0.16.0. This is not a released-next-version rebase or a compile result.
Every upgrade owes a five-file semantic recheck, both memoizer paths,
stock/default behavior controls, exact matching/options tests, portable
build and changed dependency cost. The artifact README contrasts the
no-vendor explicit-PLURAL alternative and its generated-contract/plugin
policy and numeric-literal expressivity costs. Neither is adopted.

## Acceptance and consumer boundary

Compare whole EN/KO outputs against the pinned source's Intl options, with
the oracle/runtime versions recorded. Add fixed cases for decimal ties,
negative zero, compact thresholds and carry, large grouping, subnormals and
largest finite magnitudes. Test exact unsigned counts around `u32::MAX`,
`2^53` and `u64::MAX`; include a test-only language whose plural categories
can expose a low-digit error. Test language-specific numeric data and fallback.
For the unsigned-count oracle, Intl's exact integer/BigInt input checks locale
rendering without pretending that a JavaScript Number retained all those bits.
Do not copy Paseo's translation assets.

The chosen kernel needs VM-only strict, view tests and wasm checks. Its
dependency licenses must appear in the inventory, and no size/token allowance
may grow. Consumer packages then keep typed inputs, rebuild prepared text on
state/language changes and retain field entities/drafts. Their actual app
integration and captures remain separate from the formatter's pure tests.

## Results

No backend has been measured or adopted for this extension yet. Existing
generator source/View approval does not approve this formatter. Append the
measured comparison and resulting choice here before product adoption.

The cached ICU 2.4.0 decimal/plural data identifies CLDR 49.0.0 and Unicode
18.0.0; the retained Intl oracle below uses CLDR 48.0/Unicode 17.0. Do not
attribute a data-version difference to rounding or assume byte parity merely
because both engines use CLDR. Compare the EN/KO corpus and report any
policy/data differences explicitly before choosing the backend.

### Reference and rejected-carrier characterization

`scripts/locale-number-oracle.mjs` reproduces the pinned source's three option
sets, not its app/framework. Its retained `intl-en-ko.json` fixture records
Node 24.22.1, V8 13.6.233.17-node.58, ICU 78.3, CLDR 48.0 and Unicode 17.0,
34 finite-amount cases and 13 exact `BigInt` unsigned cases. It retains raw
binary64 bits (including signed zero), options and resolved defaults.
Compact grouping resolves to `min2`, and Korean compact includes 천 at 1000,
not just 만/억/조. This oracle is data for later Rust comparisons; generating
it is not a formatter or app conformance pass.

`tests/number_carrier_contract.rs` characterizes stock Fluent independently
of Vessel's future adapter: adjacent unsigned values collapse, Russian
cardinal selection sees the rounded value, and English ordinal selection
also loses a last digit. Positive small-integer controls separate this
numeric limitation from a malformed test bundle. Its changed-source VM
result is pending; no product numeric capability has been extended yet.

## Recorded controls and first scratch result

The unmodified Fluent carrier controls at `08076af53` passed their strict
View integration-test check and all three runtime tests. The runtime run is
`i18n-narrow-20261116T014848Z-08076af5`: fonts and Cargo exited zero, with
actual View compilation and three passed, zero failed/ignored. These reproduce
loss in the rejected float carrier, including English ordinal selection at
`2^53 + 1`; they do not validate a replacement. Compilation took 52.23 seconds
in the test profile; this is not a release compile-time delta.

The narrow feasibility run
`i18n-number-experiment-20261116T014841Z-d633cdd4` resolved offline successfully
but was rejected before compilation. Existing versions/checksums were retained,
while feature-unified dependency edges changed: `icu_locale_core` gained
`serde`; `icu_provider` gained `serde` and `stable_deref_trait`; `potential_utf`
gained `serde_core` and `writeable`; `zerotrie` gained `zerovec`. Five packages
were added: `fixed_decimal`, `icu_locale_fallback`, its data crate, `icu_plurals`
and its data crate. The scratch audit refused the first unapproved existing
edge rather than silently relaxing its graph policy. The build-tool owner
must justify and admit only the exact required edges before a replacement
experiment. The original red receipt, original/result locks and patch remain
evidence. No numeric candidate test ran, and no release/wasm size was measured.

Later unlinked carrier sources prepare source/English numeric results before
binding and use the bundle memoizer only to select the prepared namespace.
This avoids a fallible locale constructor inside FluentType's infallible
display method, and avoids a global registry or Any/downcast bridge. Nested
NUMBER options consume a numeric decimal, not displayed text; compact output
retains its full scaled amount for a subsequent NUMBER. These are uncompiled
experiment sources. Partial numeric-pattern failure at this binding boundary,
actual app reachability, both backend variants, and portable/release costs
remain open; no public API or runtime adoption follows from this design.
