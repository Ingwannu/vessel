# ADR 0042: Reported usage and account limits

Status: accepted for the reviewed node implementation, 2026-11-14 (milestone 12).
Native screens, live-provider behavior and automatic recovery are outside this
acceptance; integration still assigns final compatibility numbers and runs
the combined gate.

## Context

ACP already supplies `TurnUsage` and `ContextUsage`, including an optional
running session cost. A context size is not billed token usage. Provider
inputs disagree about whether cached tokens are included. Summing context
updates, or adding the proxy's count to the agent's count, doubles usage.

Paseo's `plugins/claude-usage-source/server/usage.ts` reads the selected
local OAuth login and asks Anthropic's usage API; its Codex counterpart
asks the ChatGPT usage API. They discover accounts locally, leave refresh
to the harness, retain the last successful report on failure, and expose
windows separately. `packages/server/src/server/config.ts` enables usage
by default. GOAT also obtains limits from Claude's `get_usage` control
response and Codex's `account/rateLimits/read`, and records rate-limit
notifications. Its recovery banner offers resume and snooze at reset.

## Decision

- Reuse the ACP reports, event writer, SQLite readers and existing
  client authentication. No scraping, price catalogue, billing estimate,
  transcript scan, or inferred token count. Proxy reports are a separate
  source and never added to ACP reports for the same session.
- Commit token samples and cost increments in the event append's own
  transaction. Each sample is keyed by session and event sequence. Count
  a turn once. An absent report remains absent; a reported zero is zero.
  Keep cache reads/writes and reasoning apart as the agent reports them.
- ACP cost is cumulative for the native session (ACP v1 `UsageUpdate`
  and `Cost`), not the process. Keep per-unit baselines across `Ready`
  for the same native session. A changed native identity, a decreasing
  counter, a profile handoff, or a report replayed during startup becomes
  an explicit baseline/gap rather than spending that cumulative amount
  again. Only a session newly created after accounting began, with no
  resume/import origin, has a known zero starting point. Existing
  sessions' first observations are not assigned to the current day.
  Negative amounts are ignored. Never combine different cost units.
  Every increment carries its observation interval: a period cutting
  that interval gets an explicit gap rather than the whole charge.
  Session queries also show the latest cumulative reports independently
  of time-period increments. Their visibility uses the same cost scope.
- The session runtime marks context observations made while loading a
  conversation as `replayed` (the optional protocol-19
  `ContextUsage.observation` field). This includes profile moves, wakes
  and rewinds whose Ready event is deliberately consumed by the replay
  filter. Such an observation can establish a baseline or a gap, never
  spend an increase under the previous account. The runtime sets the
  origin; a vendor cannot choose it. An ordinary observation defaults to
  `current`, omitted on the wire. Older peers receive the original context
  report without this metadata. This reuses the existing replay boundary
  instead of inferring a load from status, counter value or wall time;
  those facts cannot distinguish a hidden Ready from new work. Changed
  native identity or profile still clears the baseline conservatively.
  The existing event/projection storage keeps this metadata; no new
  retention policy, credential access or database migration is needed.
- Keep at most 90 days and 100,000 usage samples per node, independent
  of event byte pruning, with session deletion cascading to its samples.
  The bounded per-session cost baselines remain until session deletion,
  independently of sample retention, so pruning cannot charge old totals
  again. No historical backfill: reports say when accounting began and whether
  their bounded scan was incomplete. Daily means the past 24 hours and
  weekly the past seven days, explicitly labelled rolling periods.
- Each account limit is a latest successful snapshot, not an accumulating
  counter. Report the provider's window identity, used percentage,
  duration when supplied, reset time and observation time. Unknown
  durations stay unknown (Paseo's Codex rule). A failed refresh retains
  the successful snapshot with a separate stale/error state.
- A node may read its agent's stored sign-in only on that agent's machine,
  from the operator-configured profile home. Credentials never enter the
  protocol, relay, database, diagnostic text or logs. No client-selected
  path, endpoint, header or command. The harness owns refresh; Vessel
  never writes the sign-in. Bounded local reads and requests only to the
  provider's fixed HTTPS origin, no redirects or proxy credentials.
- **External lookup defaults off**, deliberately differing from Paseo:
  starting a node must not implicitly expand a stored sign-in's use.
  While off, Usage shows the agent's own reports and one opt-in row:
  it explains that the node reads that agent's saved sign-in on this
  machine and sends it only to that vendor to read account limits.
  Only an administrator changes this policy. Turning it off drops
  cached external snapshots and prevents queued refreshes from publishing.
- `read` sees token aggregates and limits; `operate` additionally sees
  cost in the new usage calls. This does not retroactively change the
  existing conversation/event read contract, which can include reported
  cost. Agent principals may read only their own local session's usage;
  no account discovery, machine totals or another machine's account data.
  Delegated calls use the existing scope intersection at the destination.
- Recovery is explicit: identify all exhausted windows, use the latest
  known reset among them, and offer wait-and-continue only with a known
  future reset. Persist the intent; cancel on an intervening user turn,
  changed account, deletion or explicit cancellation. Recheck state before
  sending once. Offer account handoff only through the existing checked
  profile move; do not offer an action for an unavailable route.
- Protocol 19 provisionally adds usage calls after the protocol-18 node line; the minimum stays 2. Older clients see the
  existing conversation reports. New clients on older nodes receive the
  existing version refusal and show that usage is not available there.

## Alternatives

Read transcript files as GOAT's history view does: rejected because the
node already has a typed, atomic event stream and transcript semantics
vary. Derive costs from tokens: rejected because subscription prices and
cache accounting do not prove what was charged. Send sign-in material to
a fleet home: rejected because routing does not confer credential custody.
Automatic fallback without consent: rejected because it changes the
account that spends money and may change the agent's behaviour.

### Exact cost arithmetic and bounded reads

The input domain is each finite wire number's decimal spelling, rather
than the binary float's expansion. `rust_decimal` 1.43 (MIT, default
features off, std only) validates that spelling with `from_str_exact`;
ordinary parsing and scale-reducing arithmetic can silently round tiny
values. Inputs outside its exact range become explicit cost gaps.
The shared JSON dependency enables `float_roundtrip`: its default parser
can otherwise change a finite number by one ULP while reading the wire or
reloading a stored report, before exact accounting even sees it. A whole
context-usage response fixture checks that its cost survives serialization
and decoding at the tiny and large boundaries. This selects the existing
JSON library's precise parser; it adds no dependency.

`num-bigint` 0.4 (MIT OR Apache-2.0, std only) holds integer multiples of
1e-28 for differences and accumulation. The active dependency closure is
`num-integer`, `num-traits` and the latter's build helper `autocfg` (all
MIT OR Apache-2.0); randomness, serialization and property-test features
are off. With at most 100,000 samples and 32 cost units, each accumulator
needs fewer than 210 bits. An unbounded-sized vendor number never enters
it. This replaces repeated Decimal-to-float partial sums, which lose
independent small costs, and avoids an order-dependent intermediate
rescale or overflow. Maintaining only a checked Decimal sum was rejected
because checked arithmetic can still reduce its scale.

Only the final sum or increment crosses back to a finite wire number.
Its decimal spelling must parse exactly to the expected Decimal value.
A result outside that range or without a lossless wire representation
is omitted, never rounded; an omitted unit counts all its contributing
cost samples in `unaccounted_costs`. Other units remain independent.
Current cumulative reports stay the provider's original observations.

The general reader reuses idle connections; that cache does not limit
concurrent jobs. Usage scans therefore have a separate two-job admission
limit shared by every Store clone on the node. An excess call is refused
immediately with a typed busy error and retry advice. A permit belongs
to its blocking closure while queued and running, so aborting the RPC or
closing its connection cannot admit replacement scans before old work
ends. Other legacy reads keep their existing behavior.

## Consequences and verification

A bounded ledger costs disk space and does not claim complete history
before this version or beyond retention. Limits are provider capabilities,
not guaranteed for every ACP agent. Tests use sanitized fixtures matching
vendor responses and the process-level testkit only. No real credential
file or vendor endpoint may be read during this work. Live-account lookup,
including actual reset and recovery, remains explicitly unverified until
an operator checks it separately.

### Envelope version compatibility

An intermediate peer must understand the typed inner method and its fields.
The common `Call::since` rule therefore takes the maximum of the envelope's
version and the inner call's full requirement for forward, relay and deliver.
Checking only the target would let a forced self-forward execute a new
usage call on an older negotiated connection. The same central calculation
guards client sending and server dispatch; no second version checker is
introduced in the node or renderer. Regression tests cover local refusal,
raw forced requests before dispatch and supported older envelopes.

### Saved-sign-in lookup contract (node-only continuation)

This section defines the saved-sign-in implementation contract. Its node,
client and CLI implementation is under verification; source presence is not
a gate, integration, native-screen or live-account approval.

`usage.limits` selects one configured account binding or the selected profile
of one session. An omitted account profile selects the configured default; it
does not mean all historical profiles, as in `usage.query`. The existing
`profile.list` supplies the bounded enumeration to a Usage screen. Results
explicitly describe the last observation of a saved sign-in, not proof of
which credential an already-running process remembers. Profile reload or an
external login swap can make those different. No percentage alone is accepted
as the cause of a failed turn.

A cached read requires `read` and performs no credential or vendor I/O. An
explicit refresh additionally requires `operate`, because it initiates
credential use and network egress; this deliberate restriction does not hide
already obtained limits from read-only devices. `usage.lookup` requires
`admin`. Cost and credit balances require `operate`; eligibility flags remain
visible, so paid continuation is not mistaken for a hard stop. Agents may
select only their own local session. The destination checks these rules after
delegated-scope intersection, as for usage aggregates.

Session-scoped lookup uses a nonsecret binding captured with the actual launch
profile. A catalogue reload cannot redirect it to a different home; a
successful move replaces it, a failed move preserves it, and park, wake and
rewind keep it. The binding includes its account label atomically, so it
cannot be labelled using a separately read pre-move profile. Use the existing
per-session state, not another mutable registry. This binding is RAM-only and
contains no credential. If a stored session no longer has a known launch
binding, return an explicit unassociated state rather than look up a possibly
different current profile. Account-scoped lookup always uses the current
configured profile, which a Settings page deliberately selected. A file at
either location may still have been replaced while a process remembers its old
token; the response makes that limitation clear.

An owning actor serializes policy changes across RPC cancellation. Only Off/On
is stored in an owner-only atomic `usage-lookup.json`; no token, fingerprint
or snapshot is persisted or synchronized. A malformed or unreadable choice
fails off. The command queue holds at most 32 messages, there are at most two
probe jobs and 256 latest account entries in RAM, and each response holds at
most 128 independent windows. A cancelled RPC cannot drop a job's admission
while its blocking local read is queued or running. Turning lookup off cancels
the current generation and clears its cache before acknowledgement. Old
generation results never publish after Off or a later On. The actor drains
rather than aborts blocked reads.

The harness descriptor selects the supported source. Claude reads its
configured `.credentials.json`, preferring the matching macOS Keychain item
under Claude Code's account/service convention. Custom config paths use the
NFC-normalized directory's SHA-256 prefix and never fall back to the default
service. Codex reads its configured `auth.json`; OpenCode's OpenAI OAuth login
uses its XDG data store. No refresh token is requested or used. Known foreign
routes, API-key-only logins, unknown agents and unsupported stores answer an
explicit safe category. The harness remains responsible for refreshing or
replacing a sign-in.

Only a bounded GET to Anthropic's `/api/oauth/usage` or ChatGPT's
`/backend-api/wham/usage` is allowed. Origins, headers and methods are fixed
in the harness crate, redirects and inherited proxies are disabled, connect
timeout is ten seconds, request timeout fifteen, and each local file/response
is bounded to one MiB. Headers are sensitive and raw bodies are zeroed on
release. Raw credential/parser/HTTP details never reach logs, replies or
diagnostics. Vendor Retry-After limits forced refreshes too. A bounded
per-node vendor/token cooldown applies across profile aliases and policy
toggles; known active deadlines cannot be silently evicted to bypass the
vendor. Use response receipt time for relative Retry-After seconds. A failed
refresh retains a snapshot only after matching the observed saved-login
fingerprint, including the selected account identifier; missing or changed
logins discard it. Internal equality keys stay in RAM and never leave the
harness/node boundary. A credential-free HTTP client can be cached per node;
its certificate loading is blocking work owned by the same admitted job.
Cancellation never detaches a queued/running file or certificate reader: await
these phases, check cancellation, and select cancellation only around
asynchronous network I/O.

Unlike Paseo's default-on behavior, Vessel starts off. A screen should show
`usage.query` observations while off and one working admin opt-in row: "Read
this agent's saved sign-in on this machine and send its access token (and
Codex account identifier, when present) only to Anthropic or ChatGPT to read
account limits." It can name the typed source without exposing a private local
path. It calls `usage.lookup` to change the policy, then `usage.limits` with
refresh; subsequent cached reads perform no I/O.

The wire keeps provider-reported periods separate, leaves unknown lengths
unknown, preserves zero versus missing percentages, scopes model/surface/
feature/code-review allowances independently, and preserves explicit
paid-use/credits/allowed flags. A full-window notice reports identities and
the latest future reset only when every selected full general window has one.
It does not assert that a turn failed because of those windows, or offer an
unimplemented resume/switch action. Persistent automatic recovery remains a
separate unfinished item of the original acceptance list.

The library default backend is Disabled. Binaries explicitly select Local,
while the persisted policy remains off. Test builds with fakes refuse Local
even if accidentally selected. Recorded fixtures are confined to one canonical
temporary root and contain only synthetic vendor-shaped data. No check in this
slice may read a real credential or call a vendor. Actual login, vendor
authorization, macOS Keychain and live reset behavior remain unverified until
an operator checks them.

This continuation deliberately exposes factual notices only. A full window is
not a stopped-turn diagnosis, and no automatic resume or account switch is
advertised before its persisted intent and authority checks exist. The
original recovery acceptance remains unfinished. Native app screens, controls
and page work belong to their own tracks.

The implementation reuses workspace chrono, reqwest/TLS, zeroize, SHA-256 and
Unicode normalization. The latter reproduces Claude's Keychain service
identity; std alone cannot normalize NFC. No new third-party package is
selected. Harness-specific decoding and endpoint choice stay in
vessel-harness; the node owns authorization and lifetime, and vessel-store
owns atomic policy persistence. This avoids teaching the client a vendor
format or moving credentials across a fleet link.

Launch bindings share immutable nonsecret configuration through `Arc`. This
avoids copying path/account-selection strings through each launch future and
cache lookup; it also keeps existing launch futures within their stack-size
bound without weakening tests. Equality compares the captured values rather
than allocation identity. The tradeoff is one allocation per resolved binding;
credential contents remain ephemeral and outside that allocation.

A multi-provider agent's saved login is not evidence of its current provider.
OpenCode account lookup reads its saved OpenAI OAuth login; session lookup
additionally requires exactly one current model selection with the native
`openai/` prefix, as Paseo's Codex source `sessionRoutes` does. Missing or
ambiguous evidence is unassociated, and a known different provider is refused.
The node checks the existing in-memory agent options before local credential
I/O and again after waiting for the probe, including cached reads and parked
sessions. A concurrent provider change cannot publish that account's report as
facts of the changed session. This adds no prompt, credential discovery or
model default inference; account lookup remains independently available.

### Compatibility integration

The accounting query, account-limit methods and replay provenance share the
named usage protocol boundary in `vessel-proto::usage`. The final number is
given at merge. The method table accepts constant expressions so changing
that one boundary cannot accidentally leave replay provenance behind. This
changes no wire shape or supported behavior at the current boundary.
