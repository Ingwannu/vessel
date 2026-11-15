# Canonical plugin resource locale value slice

Base is the announced93970acb node head. This slice carries ONLY the existing
bounded resource-language decoder as a proto value, its exact offline dependency
and tests/docs. No larger localized declaration, FTL resource, MessageRef, cache,
core Locale/Translator, key-loader, node method or amount formatter is added.

[Decision Log]
- Purpose: native plugin resources and future canonical data share ONE parser.
- Reuse: existing view plugin translations LocaleTag FromStr, byte cap128 before
  unic_langid::LanguageIdentifier parsing, then canonical to_string spelling.
- Alternatives: a hand-made BCP47 parser/table or retain two implementations.
- Choice: proto LocaleTag; native consumer replaces its decoder after the exact
  value producer is reviewed/adopted. An embedded language remains a separate fact.
- Why: preserve original grammar, spelling, byte bound and typed error source
  without widening core language selection or duplicating parser policy.
- Impact: two root lock records and one proto dependency; native parser/version/
  features already exist. Numeric/date/Fluent formatting stays with L's owner.

Coordinator48627e3 and V3row274 approve the NARROW contract. T independently read
root VM UID1000 registry archives/sparse exact-version records without Cargo,
resolve or downloads: unic-langid0.9.6, impl0.9.6, tinystr0.8.5 and native-pinned
transitives match checksums. The evidence is the shared plugin offline-cache
confirmation packet. This verifies cache presence, not a new feature/lock closure.

Both locale crates are MIT OR Apache-2.0. Root aliases EXACT=0.9.6 with defaults
off, no extra features; the native crate's default feature list is already empty.
Only the two native package blocks and proto's dependency-list entry are copied
into the root lock, preserving all existing package versions/checksums/records.
No local Cargo is used. VM --locked --offline checks must validate this exact
closure; any missing archive, unexpected resolution or extra feature stops the
slice for its owner ruling, never a network fetch or unrelated lock update.

The canonical value retains the original closed LanguageIdentifierError rather
than assuming it is Clone/Copy/Eq (it is not). Error classifications are byte
capacity versus syntax, both input-free. Native's thin conversion maps them back
to its existing Name budget and Locale source variants. Resource duplicate tags
remain an owning namespace refusal, not a scalar parse failure. Scalar serde/TS
roundtrips, original accepted/rejected forms, exact UTF-8 byte boundary, canonical
collision and diagnostic canaries are authored here. Current actual tests, strict,
root build/boundaries/feature-graph equality and proto wasm32 evidence are owed.
The native replacement is not silently included in this node value slice.

No protocol/schema number is assigned. The lead receives a separately reviewed
native adapter after the canonical producer is on the announced line or covered
by an explicit values-consumer ruling. F6's other private graph/runtime/admission
work and its complete package/platform proof remain separate.
