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
representation. The existing `vessel_proto::FiniteNumber` already admits
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
