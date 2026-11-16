#!/usr/bin/env node
// ADR 0101: reference data, not an implementation of the Rust formatter.
// Reproduces the three Intl option sets in the pinned usage/format.ts function.
// It reads no translation assets and formats integer inputs as BigInt directly.
import assert from "node:assert/strict";

const REFERENCE = "1c00b365c9e0b74c1580eba33d905d693125ae21";
const LOCALES = ["en", "ko"];
const OPTIONS = {
  usd: { style: "currency", currency: "USD" },
  tokens: { notation: "compact", maximumFractionDigits: 1 },
  decimal: { maximumFractionDigits: 2 },
};
const BASIC_AMOUNTS = [
  ["zero", 0], ["negative-zero", -0],
  ["smallest-subnormal", Number.MIN_VALUE],
  ["negative-smallest-subnormal", -Number.MIN_VALUE],
  ["smallest-normal", 2 ** -1022],
  ["usd-half", 1.005], ["usd-negative-half", -1.005],
  ["decimal-half", 2.675], ["decimal-negative-half", -2.675],
  ["round-to-zero", 0.0049], ["negative-round-to-zero", -0.0049],
  ["round-from-zero", 0.005], ["negative-round-from-zero", -0.005],
  ["compact-under", 999.94], ["compact-carry", 999.95],
  ["thousand", 1000], ["example-tokens", 1234],
  ["example-usd", 1234.5], ["example-quantity", 12345],
  ["compact-one-fraction", 12345.67],
  ["ko-carry-under", 9999.94], ["ko-carry", 9999.95],
  ["ko-ten-thousand", 10000],
  ["en-million-carry-under", 999949], ["en-million-carry", 999950],
  ["en-million", 1000000],
  ["ko-million-carry", 99995000], ["ko-hundred-million", 100000000],
  ["en-trillion", 1e12], ["ko-trillion", 1e12],
  ["large-compact", 1e16], ["large-exponent", 1e100],
  ["largest-finite", Number.MAX_VALUE],
  ["negative-largest-finite", -Number.MAX_VALUE],
];
// Freeze additional boundary cases before a candidate executes them. The
// original default output remains byte-identical; --extended is a new corpus.
assert.ok(process.argv.length <= 3 && (!process.argv[2] || process.argv[2] === "--extended"));
const EXTENDED = process.argv[2] === "--extended";
const THRESHOLDS = [
  ["ko-thousand-to-man-under", 9949.99],
  ["ko-thousand-to-man-at", 9950],
  ["ko-thousand-to-man-over", 9950.01],
  ["ko-negative-thousand-to-man-under", -9949.99],
  ["ko-negative-thousand-to-man-at", -9950],
  ["ko-negative-thousand-to-man-over", -9950.01],
  ["ko-man-to-eok-under", 99999499.99],
  ["ko-man-to-eok-at", 99999500],
  ["ko-man-to-eok-over", 99999500.01],
  ["ko-negative-man-to-eok-under", -99999499.99],
  ["ko-negative-man-to-eok-at", -99999500],
  ["ko-negative-man-to-eok-over", -99999500.01],
  ["negative-compact-under", -999.94],
  ["negative-compact-carry", -999.95],
  ["en-negative-million-carry-under", -999949],
  ["en-negative-million-carry", -999950],
  ["usd-digit-carry", 9.995],
  ["usd-negative-digit-carry", -9.995],
];
const CORRECTED_NAMES = {
  "ko-carry-under": "ko-already-above-thousand-to-man-carry-a",
  "ko-carry": "ko-already-above-thousand-to-man-carry-b",
  "ko-million-carry": "ko-below-man-to-eok-carry",
};
const AMOUNTS = EXTENDED
  ? [...BASIC_AMOUNTS.map(([name, value]) => [CORRECTED_NAMES[name] ?? name, value]), ...THRESHOLDS]
  : BASIC_AMOUNTS;
const COUNTS = [
  0n, 1n, 2n, 11n, 21n,
  4294967295n, 4294967296n,
  9007199254740991n, 9007199254740992n, 9007199254740993n,
  9007199254741001n, 18446744073709551614n, 18446744073709551615n,
];
const bits = (value) => {
  const buffer = Buffer.alloc(8);
  buffer.writeDoubleBE(value);
  return buffer.toString("hex");
};
const format = (value) => Object.fromEntries(LOCALES.map((locale) => [
  locale,
  Object.fromEntries(Object.entries(OPTIONS).map(([style, options]) => [
    style, new Intl.NumberFormat(locale, options).format(value),
  ])),
]));
const en = new Intl.NumberFormat("en");
assert.equal(en.format(COUNTS.at(-1)), "18,446,744,073,709,551,615");
assert.notEqual(en.format(9007199254740992n), en.format(9007199254740993n));
assert.equal(format(1234).en.tokens, "1.2K");
assert.equal(format(1234.5).en.usd, "$1,234.50");
assert.equal(format(12345).en.decimal, "12,345");
process.stdout.write(`${JSON.stringify({
  schema: 1,
  reference: { commit: REFERENCE, file: "packages/app/src/usage/format.ts", lines: "185-199" },
  scope: "Reference option sets only; not a Rust, browser, app or locale-data conformance pass.",
  runtime: Object.fromEntries(["node", "v8", "icu", "cldr", "unicode"].map((key) => [key, process.versions[key]])),
  options: OPTIONS,
  resolved: Object.fromEntries(LOCALES.map((locale) => [locale, Object.fromEntries(
    Object.entries(OPTIONS).map(([style, options]) => [style, new Intl.NumberFormat(locale, options).resolvedOptions()]),
  )])),
  ...(EXTENDED ? { corpus_revision: 2, parent_fixture_sha256: "c7ec252d8ad5577a33ba83bb4b8988540821cceef2e07c8689723adfb6544a22", note: "Adds Korean carry boundaries and negative cases; corrects three misleading names, preserving all prior numeric outputs." } : {}),
  amounts: AMOUNTS.map(([name, value]) => ({ name, bits: bits(value), decimal: Object.is(value, -0) ? "-0" : String(value), output: format(value) })),
  counts: COUNTS.map((value) => ({ unsigned: String(value), output: Object.fromEntries(
    LOCALES.map((locale) => [locale, new Intl.NumberFormat(locale).format(value)]),
  ) })),
}, null, 2)}\n`);
