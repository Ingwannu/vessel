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
