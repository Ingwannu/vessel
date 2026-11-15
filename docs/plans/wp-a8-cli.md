# A8 CLI permit

Status: staged parser, typed selection, complete read-only collection/preflight,
question-input decoding and exact-outcome confirmation. Product registration
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


CLI acknowledgement confirmation reuses the existing typed PermissionOutcome /
QuestionOutcome and authoritative Question::check. A durable Settled reply will
be compared against exact request id and selected option/accepted field values
before printing success; cancellation, another client's different answer, an
unoffered option or an invalid empty form is unconfirmed. The private helpers
are staged under the same cfg(test) module until W23's checked RPC and W17's
reviewed output seam arrive. They neither invent a wire method nor turn the
legacy Ack into durable proof. Regression fixtures explicitly cover these cases.


Envelope-key presence is distinct from JSON null. Serde Option<IgnoredAny> would
map both missing and explicit null to None, accidentally accepting mixed header
and canonical layouts such as {answers:{},answer:null}. The strict outer decoder
now tracks answer/values key presence explicitly and requires any present answers
key to decode as its real map. Null/mixed-envelope regression fixtures refuse
before header mapping, connection or response effects. Canonical field-value
decoding still belongs to QuestionAnswer; no duplicate answer schema is added.

Schema decode failures preserve their typed serde source through DecodeFailure,
while its Debug and user-facing Display redact the answer content. Semantic
mixed-layout refusal remains a separate cause-free policy category. A regression
checks both source retention and Debug privacy; the shared output adapter must
render the safe top-level category rather than expanding untrusted source text.

Exact40d44f71c narrow wp-side-node-narrow-20261115T094531Z-40d44f71 reached
vessel-cli after rebuilding its current root dependencies, then failed solely
on a test-local Expected enum declared after statements. The enum is moved to
the function's item section without changing the fixture cases. This is not a
passing CLI result; the latest null/source-privacy change is newer than that run
and still needs strict compilation and execution.


Question-input diagnostic privacy must cover main's complete error chain, not
only the top-level Debug. serde's unknown-variant cause may echo answer content.
DecodeFailure retains the original serde_json::Error privately, reports only its
safe category in Debug, and deliberately stops automatic source traversal at its
opaque boundary. InputError still carries that typed boundary as its cause. This
follows the shared selected-connection error boundary's treatment of untrusted
peer detail; no parser cause is discarded. The regression verifies retained Data
category and walks every printable cause with a synthetic private-answer canary.
It would expose that answer through the previous inner #[source] annotation.
The CLI module remains test-only pending checked DTO/output integration; execution
of this new regression is still owed on the build VM.


The actual clap argument edge also redacts its InvalidValue context for invalid
--input JSON. Changing only ModifiedInput's Debug or InvalidInput's Display is
insufficient: clap's standard formatter normally quotes the entire rejected
argument. The opt-in TypedValueParser delegates to clap's existing String parser
and ModifiedInput decoder, preserves the returned clap error and original cause,
and replaces only the printable argument value with a fixed redaction marker.
No JSON is decoded twice and valid values retain their original opaque bytes.
A regression walks the complete actual clap error chain for malformed JSON
containing a private canary. Source inspection uses pinned clap_builder4.6.7's
public TypedValueParser/Error::insert boundary; VM execution remains pending.


Permit command Debug also omits a denial's provider-visible message while keeping
request scope and explicit interrupt intent. Allow keeps ModifiedInput's existing
redacted Debug. The actual parsed denial-message regression verifies the original
text remains available for the future typed provider call but cannot appear in
Command diagnostics. No product registration or send is added by this change.


Read-only preflight errors also keep the original CallError for later typed-code
projection while stopping arbitrary peer messages at the CLI diagnostic boundary.
SourceError::Node uses a safe message and Debug's closed remote ErrorCode only;
it does not automatically expose CallError's Display through main's cause chain.
The original variant/message remain stored, as in the shared selected-connection
error boundary. A canary regression verifies both retained typed data and private
Display/Debug/source traversal. Listing/lookup still returns failure and no partial
successful output. No response side effect or protocol change is introduced.


Merged the integrator-announced node head82ae7c91 containing the exact reviewed
shared output seam5b3ca1d2, through the required node-line route. The sole conflict
was main's module registrations; preserve parity/presentation and the still-test-
only permit module. Gate46g's reported Rust2389/0/12, e2e68/68 and forge4/0 are base
evidence; the coordinator counted one named web load timeout, so this is not an
all-steps-green claim or validation of A8's own code. Actual formatter wiring must
use OutputOptions/Presentation/Headers/Row/list/render_list/single and admit only
real consuming permit handlers in presentation::check before connection effects.
