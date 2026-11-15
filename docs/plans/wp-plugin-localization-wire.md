# F6 canonical localized declaration contract (proposal)

ADR0079 requires typed Literal/Message client data rendered by kit. Current
ClientDocument has literals only. The installation-local Fluent implementation
and L's shared checked contracts/bundle factory already exist in the view; this
proposal connects them without a second catalog or core translation registry.
It is not implemented or approved by the current canonical-values exception.

[Decision Log]
- Purpose: keep plugin labels current when app language changes, with English
  fallback and preserved edits, without executing React or formatting in paint.
- Assets/constraints: canonical ClientDocument/ClientCatalog, existing literal
  Caption/ContentText/ThemeName, proto FiniteNumber, Fluent contracts and L's
  NUMBER/DATETIME/PARTICLE bundle factory. Core Translator remains closed.
- Alternatives: a second target-to-label override map, duplicated translated
  catalogs, or typed text refs directly in the one canonical document.
- Choice: Localized<T> with Literal(T) or Message(MessageRef) on actual display
  text fields. Existing literal JSON remains a string; a message is a closed
  object. Resources are optional only while all texts are literal.
- Why: no conflicting literal/override sources or alternate wire representation.
  Action/method/identity/schema fields remain outside translation entirely.
- Impact: native Fluent author format and a restricted exposed-key subset;
  client validation/cache/lifetime integration and dependency review still owed.

## Proposed canonical values and ownership

A MessageRef has one canonical ContributionId-shaped message key and at most16
named typed arguments. The existing lowercase-letter/digit/hyphen identifier
is a valid restricted Fluent identifier subset; full resource AST names and
reference/term contracts still use the shared Name scanner, not another parser.
Arguments are ContentText, existing proto::conversation::FiniteNumber, or closed
Gregorian date/local-clock/zone-label variants with validated chrono parts.
The view performs the fallible bridge to core Number/Date and uses L's fluent
bridges. No silent clamp, dropping an argument, private date-encoding copy or
new financial/amount renderer. Generic numeric wire values are NOT capped to
current core Number's u32 ceiling: L owns the separately assigned full-finite
amount/unit formatter and any actual supported formatting expansion.

Resources are typed locale/source records with duplicate-before-insert refusal,
mandatory en, and the existing installation-local limits:16 locales,32KiB each,
256KiB aggregate,256 parsed keys,128 name bytes,16 arguments,128 syntax openers
and32KiB formatted output. Constants must have one owner and be imported by the
view, rather than copied. Existing whole document/catalog/frame caps still apply.
Only the named byte decoder promises a pre-parse byte boundary; generic serde
construction is not advertised as universally pre-allocation bounded.

LocaleTag is a wire resource-language tag, not a core embedded Locale selection.
The existing view decoder uses unic-langid0.9.6 and canonicalizes it. Hoisting
that exact bounded decoder into proto would reuse the existing native parser
and MIT OR Apache-2.0 dependency, with an explicit root workspace alias and
proto edge; no hand-written language table. Coordinator/lead manifest, license,
lock and crate-boundary approval are required before that hoist. L's frozen
core generator is not a locale-wire dependency approval.

## Admission and prepared presentation

Node validation owns structural/bounded canonical data and whole graph identity;
actual Fluent syntax, expanded English key/argument/reference/type contracts and
per-message fallback use the existing view implementation. No Node Fluent
renderer, new parser or locale RPC is implied. Invalid external resources never
enter core Translator. Client-language validation is a separate trust boundary
and cannot be inferred merely from a node's structurally valid catalog.

Before publication, one installation owns its parsed namespace and current
prepared formatter/presentation. Bad client text withdraws that installation's
client contribution eligibility with a safe evaluation problem, without silently
executing an old client graph or claiming its server helper failed. Bounded raw
facts may remain for display/diagnostics; they are not eligible client actions
or themes. This follows Paseo's per-installation client evaluation boundary.
Malformed whole catalog identity/budget rules retain their existing atomic rule.

Raw immutable ClientDocument remains the action-origin authority. Resolved
captions/content/theme names are a sealed derived presentation from that exact
raw graph, not a modified canonical document or another transport schema. Every
button/menu/form intention rechecks the raw node/plugin/generation/graph and
actual feature context; localized text never supplies method or target authority.
Popover presentation lookup must resolve the same validated raw path, including
currently open child bodies, rather than reverting them to raw English labels.

## Language, cache and disposal

The sole Hub observes the core app language once and refreshes installation
namespaces without any locale RPC. Parse/English validation occurs on catalog/
generation admission; formatting/preparation occurs on catalog/language events,
never while drawing. Preparation work remains bounded and retains its admitted
ticket/task/byte ownership until actual settlement, even on caller loss/timeout;
it cannot mint another unbounded queue behind the existing catalog read slots.

Only the active prepared formatter is retained per installation, with parsed
resources reused; repeated language switches do not accumulate bundle caches.
Language changes preserve form drafts, actual editor entities/FieldIds and
current popover lifetime. Close/reopen still follows the separately documented
native draft-reset policy. Disposal retires only that installation's resources/
subscriptions; stale generations cannot publish labels. Cached offline valid
facts remain static presentation only; failed client resources are ineligible.
The owner notifies appearance/commands/sidebar/panels outside paint after borrows
end. No second theme loader, translator registry or catalog subscription exists.

## Readiness

This needs Node-side canonical model/source review first. The private app
exception currently permits EXACT3a8 catalog models only; it does not permit
speculative localization wire edits. No new parser dependency, type, formatter,
cache, field mutation or product wiring is added by this document. Actual locale
changes with preserved edits, partial-key fallback, renderer target bounds,
retired namespaces and current-generation publication require execution on the
real joined Hub/renderer before F6 acceptance.
