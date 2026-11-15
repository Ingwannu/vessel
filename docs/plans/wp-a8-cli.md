# A8 CLI permit

Status: proposed; parser and selection skeleton only. Product registration
waits for the reviewed ADR 0084 rich response contract and W17 output helper.
W4 owns these CLI files; W23 owns permission/question proto and node runtime.

References read: Paseo 0.15.4
`packages/cli/src/commands/permit/index.ts:8-39`,
`allow.ts:82-181`, `deny.ts:42-125` and
`packages/client/src/daemon-client.ts:6237-6263`.

## Decision Log

- Purpose: list and answer pending requests without weakening rich provider
  semantics or claiming a timed-out answer succeeded.
- Existing assets: session selectors, `session.conversation`, the existing
  session answer command, opaque RawJson, and W17's common output seam.
- Alternatives: reuse `session answer --decline` directly (wrong batching,
  no rich denial/input policy or durable receipt); copy output helpers
  (drift); accept edited JSON then silently drop unsupported edits (unsafe).
- Choice: a dedicated permit command edge, typed asymmetric selection,
  validated opaque input and reviewed shared output/response contracts.
- Why: flags validate before connecting and only acknowledged results may
  be described as allowed or denied. The provider alone interprets edits.
- Effects: no new dependency or crate edge; no node calls invented. The
  skeleton is cfg(test) until product dispatch is complete, so no fake or
  incomplete command can answer in a product build.

Behavior to retain: allow with no request means all; deny needs a request
or explicit --all; explicit --all wins over a supplied request; denial
message and interruption stay distinct; --input JSON parses before any
connection. Session ids/prefixes are Vessel's agent identity. Pending
questions are included alongside permission requests using the reviewed
typed node snapshot rather than inventing messages.

Stated differences: Vessel's existing selectors reject ambiguous prefixes
instead of Paseo's first-match request lookup, to avoid granting a different
request based on list order. Supplied valid JSON is always forwarded for
provider validation, including scalar/null values; it is not silently
dropped by JavaScript truthiness. Unsupported shapes must refuse before
effects. Root/native runtime proof is still owed.

The list model reuses the existing inbox's authoritative request ids and
source keys. Payload-budget references remain visible. The page state
machine refuses nonadvancing cursors, repeated request identities and
unknown item kinds, and only releases rows after the final page. This
prevents an invalid traversal being printed as a complete result. Ordinary
outcome items are excluded. Rich presentation labels await ADR 0084.

Next: wire snapshot/list output, W23's durable response call, bounded
confirmation and reconciliation, batch partial-result reporting and tests
against the in-memory node. Use W17's reviewed output helpers, not a second
table/JSON/YAML formatter. No real agents or production nodes.


Read-only collection now uses a typed Source seam implemented directly by
NodeClient::inbox and inbox_request, with in-memory page/form fixtures. Listing
returns rows only after all pages succeed. A maximum of 10,000 pending rows or
4,096 pages explicitly refuses the command without publishing partial output.
Every selected form is reloaded by its exact InboxKey before a response batch;
resolved, substituted and still-compact forms refuse preflight. The response
runtime is intentionally absent until W23's reviewed rich receipt contract.

Selection is scoped to the resolved agent SessionId before request-prefix lookup.
An ambiguous prefix refuses before form lookup or mutation, including a future
collision between permission/question UUID text. `allow` omitted scope remains
all; denial still needs explicit scope. This intentionally follows Vessel's
existing unambiguous selector contract rather than Paseo's first-match shortcut.


Question --input has a typed staged edge: accept either the canonical accepted
QuestionAnswer or {answers:Record<header,string>}. Reject mixed layouts, extra
provider-edit fields, non-string header values and declined answers in permit
allow before mapping or response effects. Payload Debug remains redacted.
Canonical value decoding reuses QuestionAnswer rather than a parallel value enum.
The shared client fill_headers helper will resolve header/title/name and actual
option values, reject ambiguity/missing answers and apply Question::check; W23
owns that helper (intended, not checked yet). CLI never guesses comma splits or
constructs provider prompt messages. Permission tool --input stays opaque for
provider-side unsupported-input refusal; a scalar is not silently discarded.


Preflight also caps aggregate retained encoded forms at64MiB, in addition to the
listing's10,000-row/4,096-page limits. Otherwise a valid large form per request
could make an all-request batch retain many node-frame budgets at once. A private
checked writer accounts the actual encoded request/question without allocating
another JSON payload; oversized batches fail before any response effect. Existing
comparison/report writers are private and have unrelated scopes, so this avoids
widening their APIs. The cap is an encoded-payload budget, not an exact heap claim.
Smaller explicit request selections remain available. Tests cover exact cumulative
boundary and escaped text counting rather than relying on raw string length.
