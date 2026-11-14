# ADR 0042: Reported usage and account limits

Status: accepted for implementation, 2026-11-13 (milestone 12, track N2).

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
