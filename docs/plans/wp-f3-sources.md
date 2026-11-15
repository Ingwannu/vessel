# F3 account reports and usage sources

Status: contract proposal. Extends ADR 0042; no assigned report protocol.

## Existing assets and decision

Modify the existing usage/limits infrastructure, preserving accounting and
immutable launch binding. Add a distinct report family because profile-bound
quota facts cannot express source-account discovery, report ids, arbitrary
balance units, source tone, summary flags or plugin sources. Reuse the existing
`LimitKey`, `LimitName`, `FiniteNumber`, `LimitRefresh`, `CostVisibility`,
authorization checks, cancellation/permit discipline and saved-login policy.
No new dependency is required for the contract or actor skeleton.

## Reference specification, read before implementation

Paseo 0.15.4, repository-relative paths:

- `packages/app/src/usage/types.ts:1-34`, `model.ts:107-168,208-238`:
  host/query outcomes, current versus retained observation and freshness.
- `packages/app/src/usage/preferences.ts:8-32,50-96`: source/window pins,
  null defaults versus explicit empty, first-edit default snapshot.
- `packages/app/src/usage/pinned.ts:1-99`: adaptive sidebar
  layout, available reports only and source-grouped account order.
- `packages/app/src/usage/tone.ts:4-11`, `meter.tsx:46-74`: source tone
  first; fallback danger strictly above 90, warning from 70; quiet fill.
- `packages/app/src/usage/usage-modal.tsx:23-94`, `hosts.ts:1-59`:
  connected-host filter, persisted host choice and compact sheet.
- `packages/protocol/src/messages.ts:6411-6527`: report, attempt,
  independent window/balance/detail data and streamed response contract.
- `packages/server/src/server/plugins/usage-sources/index.ts:53-110,
  222-241,299-360`: discovery, five-minute cache, per-account coalescing,
  independent fetch deadlines and successful-history retention.
- `packages/plugin/src/server/usage.ts:67-180`: source registration,
  session/global scope, safe account keys, period labels and source tone.
- `packages/cli/src/commands/usage/index.ts:75-93`: list/inspect,
  session-versus-report selector exclusivity and unsupported host.

## Structure and ownership

- `vessel-proto::usage_reports`: identifiers, bounded collections,
  report states, scope, list and inspect value types. The protocol
  constant remains `None`; no call before method-table integration.
- `vessel-harness`: extend source adapters; fixed known origins, local
  read-only sign-in handling and fixture-only schemas stay here.
- `vessel-node`: one source registry/cache actor, reusing reviewed limits
  where applicable. Plugin adapter consumes ADR 0079's source helper API;
  no second plugin invocation protocol. Native/page/CLI share the same DTOs.
- Native view/link/app: request lifetimes, cards and bar tone,
  source/window pin selection, host-filtered modal, shared sidebar slot,
  account data within the existing context meter and factual limit notice.
- CLI glue uses the same report-id discovery and inspect resolver.

## Deliberate differences and hard invariants

External lookup defaults off. Read never starts login discovery or fetch;
explicit Refresh requires Operate. No arbitrary endpoint or credential path is
accepted from a client. Monetary fields stay withheld without Operate. A quota
is not token accounting, an observed saved login is not proof of a running
process's remembered credential, and a full quota is not proof why a turn
stopped. Account labels never identify reports. Missing remains missing.
The package cannot claim source or plugin parity before its real adapter exists.

## Verification

Identifiers: boundaries, Unicode/control rejection, canonical opaque report-id
round trips and no label-based resolution. Collections: first excess rejected
without decoding its body, duplicate identities and global response size.
Actor: coalescing, cache TTL boundary, failed discovery retains prior accounts,
failed refresh preserves only same-binding success, off/disable/exit fences late
publication, cancellation retains permits until actual work finishes.
Sources: sanitized fixtures only, every parse/status/rate-limit branch, no real
credentials or vendor endpoints. Consumers: old-peer typed refusal, report-not-
found and mutually exclusive selectors. Whole native/root gates and review
remain owed until the complete package is wired.

### Executable skeleton registration

Quality B permits the two report selectors and the source-consent mutation to register at the end of `rpc/table.rs`. `ACCOUNT_REPORTS_PROTOCOL` and `USAGE_SOURCE_CONSENT_PROTOCOL` both remain `None`. Client and server admission therefore refuse these methods rather than
assigning an invented protocol version. The initial skeleton handlers refused with typed `Unsupported`; the current
saved-login adapter uses the existing limits actor, opaque binding registry, and
real list/inspect service paths. Method admission remains pending. Seven additional fixed adapters are now implemented below, but nine-source end-to-end parity, extra-source launch associations and the F6 helper are not accepted or complete.

### Honest legacy observation projection

Protocol 23 retains successful timestamps but does not retain attempt start or
failure completion times. The generic DTO therefore represents those timestamps
as optional: a bridge leaves them absent, never invents the current query time.
Sources with real attempt timestamps supply them. Available reports also carry
Complete/Partial, preserving the existing limits parser's incomplete fact.
Configured saved-login projections retain their configured binding identity; they
do not claim two profiles are the same vendor account merely because labels match.

### Saved-login adapter and registry

The node adapter reuses the existing limits actor and probes: no credentials,
HTTP implementation or credential identity are copied. Public `saved-*` source
names explicitly identify configured saved-login bindings, rather than claiming
vendor-account discovery. Opaque random report keys belong to exact canonical
scope plus resolved binding proof; they are never labels, paths, fingerprints,
or guessed defaults. Rebinding and policy off revoke resolution/publication.
Inspect resolves only known ids in RAM and rechecks their captured scope/proof.
Session scopes retain the existing local-owner/current-provider checks around
awaits. No inspect id expands an agent principal's authority: agents use their
own local Session selector. Registry capacity refuses growth without eviction;
response count/byte bounds refuse the complete observation, never silently drop
accounts. The additional fixed-source owner described below implements the seven fetchers. Provider account grouping beyond configured bindings, extra-source launch association and F6 remain required; protocol constants stay None until integrator assignment.

### Review correction: identities and captured reads

[Decision Log]
- Purpose: keep source/account grouping typed on the Rust and generated TS wire,
  and prevent a profile A → B → A edit from placing B's values under A's id.
- Existing constraint: Rust report-id getters were absent from serialized fields;
  delegating to a selector-based limits service resolved the profile a second time.
- Alternatives: client string splitting violates opacity; an after-check alone
  cannot detect the intervening B resolution; a global configuration generation
  would couple unrelated profile editors to this report path.
- Choice: one entry constructor derives explicit validated source/account fields
  from its opaque id; decoding rejects forged disagreement. The report pipeline
  passes its captured canonical account and resolved proof directly to the one
  limits actor. It rechecks scope/provider proof and policy after awaiting.
- Effect: no second credential reader or limits parser; monetary/detail redaction
  and authorization remain at the service boundary. A barrier fixture holds the
  metadata at B during the actual actor read, restores A before the final check,
  and requires A's current or retained success and original timestamp. These tests
  use only fileless caches and synthetic metadata, with no vendor or credential.

The V3 checkpoint on 604717fd returned both issues; that immutable checkpoint
remains failed. Its TS tests passed four cases but freshness failed. This correction
requires its own narrow checks and generated-wire freshness proof before reuse.

### Additional vendor format boundaries

The pure account_usage module reuses vessel-proto's report values and the existing
finite/limit text validators. It owns additional vendor schemas, not credential
lookup, HTTP or a second report model. Sources inspected directly: Paseo plugins/
{copilot,cursor,grok,kimi,minimax,opencode-go,zai}-usage-source/server/usage.ts.
One 1 MiB body bound precedes parsing; collection visitors refuse first excess.
Fixture-only tests preserve explicit pool percentages, independent balances and
period metadata, unknown denominators, scalar numeric text and same-source tone.

Differences: unfamiliar MiniMax application errors remain changed-response rather
than displaying raw vendor text or guessing no subscription. Its missing/unknown
status uses the percentage fallback, not an assertion of healthy quota. Malformed
Kimi rows mark a partial observation; an oversized or non-array limits collection
refuses the complete response instead of bypassing bounds via a fallback decoder.
These parsers alone do not implement discovery, local sign-in reads, fetch or consent; the executable adapter/owner sections below describe that later implementation. Extra-source launch association and actual vendor acceptance remain unverified.

### Additional-source local and HTTP adapters

[Decision Log]
- Reuse: the existing limits admission, native-root TLS client, no-proxy/no-redirect
  transport, bounded private credential reader and vendor cooldown remain shared.
  Additional vendors use fixed endpoint/method/header enums, never client URLs.
- Local reads: a binary supplies deferred environment/SQLite capabilities. Merely
  constructing Config or the access capability does not read a token. Disabled
  and fakes-Local perform no local/provider access; Recorded confines every file
  to its temporary root. Tokens and bodies use zeroizing private values.
- Association: a private token+region equality proof controls retained success; it
  is never the public account id, a diagnostic or persisted data. Expired/rejected
  credentials are not refreshed or written. Parsed text reflecting a credential
  is refused before values leave the format boundary.
- Cancellation: a blocking read holds its own cloned admission until actual
  completion, even if its awaiting future is cancelled. Cached service reads
  must never call this adapter; source consent/runtime wiring remains a distinct
  step and unsupported until complete.
- Deliberate safety differences: GitHub CLI credentials require github.com host
  evidence; foreign MiniMax bases refuse rather than sending credentials there.
  Cursor SQLite is owned by vessel-store and reads immutably; an active WAL is
  refused, with the separately disclosed headless fallback remaining available.
  No actual login/provider/production data was used for tests.

### Executable additional-source owner

One node actor now owns the seven fixed source adapters, explicit consent grants,
account observations and public id resolution. It uses two actual jobs, a bounded
32-command queue and at most 32 coalesced waiters per active source. Twenty-second
source cancellation and a separate bounded response wait never release admission
before a blocking local read settles. Off/disclosure changes cancel publication,
forget snapshots/ids and persist before acknowledgment; dropping an RPC does not
abandon its actor-owned mutation.

A source/region/private-login equality proof reuses an opaque random account id
across same-binding refreshes; a changed login gets a new id and cannot retain
another login's success. Source discovery failure is now a typed unavailable or
fetch-error attempt, so a missing login before any account is known does not
fabricate an account or become an unrelated transport diagnosis. Same-binding
failed refresh preserves original successful time and values. No current success
is invented from an attempt time.

Cached/descriptor/unknown-id reads never call a source capability. Global lookup
Off and source-specific Off are separate; only the three existing projections
inherit the old global consent. Source consent has an independent publishing
generation carried by copied snapshots and checked after awaits. Global policy
also has a fresh observable Off generation: Off cancels the old On generation,
then establishes the current Off fact; Off→On cancels that Off generation too.
This fixes permanently-cancelled Off observations without allowing any Off probe.

Machine list refresh aggregates independent source outcomes; inspect resolves only
known ids and refuses a new login under an old id. Session scope continues to use
its captured legacy association; additional source session association is still
Unsupported until actual launch provenance is available. F6 remains unsupported
until its real registered/disclosed runtime adapter. Unary complete snapshots and
strictly cache-only passive reads remain the documented transport/auth difference;
there is no background credential discovery at a cache TTL. Package gates, UI/CLI/
web integration and real-source acceptance are still owed.

### Recorded real-node UI fixture and managed artifact boundary
[Decision Log]
- Purpose: browser/native consumers need available, risk, independent remaining/used, retained-error, consent/off, changed-login and multiple-account cases through the actual node, without reading an operator sign-in or substituting quota JSON in the browser.
- Existing constraint: the production daemon has no source seed switch. Existing vendor fixtures do not contain every generic source forecast/balance combination. The current managed export whitelist does not include the CLI or this extra executable.
- Alternatives: browser request mocks would skip the real policy/cache/redaction path; an ambient environment override in vesseld could affect production. Both were rejected.
- Choice: feature `vessel-node/usage-fixture` builds the separate `vessel-usage-fixture` binary. Its required marked scratch root contains all home, data, executable search and recorded body paths. Listener is loopback and refuses live ports 7077/18220/18221. `RecordedUsageFormat::Report` exists only under harness `fakes`; ordinary adapters still use their actual vendor parser. No real HTTP/certificate/login access occurs in this backend.
- Tradeoff: this proves the real policy/cache/service projection with a synthetic source, not successful vendor integration. Additional-source session scope and plugin source execution remain Unsupported pending their actual immutable binding/runtime seam.

Preparation (Python standard library only; creates a NEW root with mode 0700 and files 0600):
```
python3 scripts/usage-fixture.py /tmp/work/usage-ui-UNIQUE prepare --legacy
vessel-usage-fixture --root /tmp/work/usage-ui-UNIQUE --bind 127.0.0.1:0
```
The binary emits `USAGE_FIXTURE_READY <address>`. Install only managed, hash-verified fake harness executables into `ROOT/bin` when session tests need them; no PATH fallback. `--legacy` prepares default Codex and `fixture-other` with separate synthetic sign-ins, plus the existing testkit Codex vendor body. It does not create a session or grant any policy.

Once the integrator assigns and lands the pending methods, consumers enable global lookup as Admin, explicitly grant Cursor source revision 1 as Admin, then explicitly refresh as Operate. Cache-only Read never seeds data. Typed source `cursor` declares `Cursor` credential store/recipient (not the legacy three-store disclosure); other seven-source adapters remain Unsupported in this recorded fixture. `malformed` changes only the recorded body: a subsequent explicit refresh produces a same-binding retained failure with the original successful timestamp. `restore` restores the generic report. `switch-login` changes the scratch synthetic login; a new refresh must produce a distinct report identity and never reuse prior retained success. Global Off forgets snapshots; source Off revokes only that source. Risk comes from explicit report `runs_out_at`/`shortfall`, never browser inference.

Remote required-feature compilation recipe (no local Cargo):
```
vessel-remote-gate run --tree <tree> --commit <immutable SHA> --label wp-usage-fixture --out <new> --cargo build -p vessel-node --features usage-fixture --bin vessel-usage-fixture -p vessel-cli --bin vessel
```
Export remains blocked on the tool/coordinator's explicit bounded recipe for **both** `vessel-usage-fixture` and `vessel` with the source/feature manifest. `attach` for a plain cargo build does not export either custom binary. No manual VM copying, local compilation or another binary's export manifest can stand in for this evidence. No fixture end-to-end result is claimed yet.

### Pending refresh presentation retains the previous attempt's meaning
N2's source check of Paseo `query.ts`/`model.ts:209–236`/`pinned.ts` shows that a pending refresh of an available report leaves that success sidebar-eligible with `retained=false`. A failed/unavailable attempt with an older success remains card-only, including while its next refresh is pending. Snapshot presence alone cannot distinguish those cases.

`Refreshing.origin: UsageRefreshOrigin` captures that immediately preceding attempt before the source actor changes state. `Available` is emitted only from an actual immediately preceding Available result; other states and omitted wire fields are `Unavailable`. Legacy limits' Refreshing state cannot prove the prior attempt and projects the conservative unavailable origin. Consumers may keep the original successful timestamp and pins for Available-origin pending results, but must never promote a failed/unavailable retained snapshot into a current observation. This adds no method/version and no permission or fetch behavior. A synthetic actor barrier checks the Available origin; a DTO roundtrip checks explicit preservation and safe omitted-field decoding. Narrow checks remain owed.

A service-level regression now starts an actual isolated Node with the recorded source backend, obtains the real Connection via an in-memory framed handshake, and invokes the generated Guarded methods directly. No pending protocol is assigned or bypassed on the wire. It exercises Admin-only grant, Operate-only refresh, zero credential discovery for Read/cache misses/consent/unknown inspect, populated current and retained monetary redaction, original successful timestamp and same-binding retained error, and source-Off forgetting. A separate real-actor barrier preserves Unavailable origin while a failed attempt is refreshed. These are drafted regressions pending remote execution, not wire/e2e acceptance.

### Complete-refresh time budget
A sequential machine refresh could spend one vendor deadline per configured profile
before starting the seven additional sources, exceeding the native caller's 120-second
budget even for normal multi-source setups. Saved-login reads now use a stable-order
buffer of two, and the saved and additional owners start concurrently while keeping
their existing actual-work admissions. A named 90-second complete-refresh budget
refuses with safe Unavailable instead of holding a unary request across hundreds of
profile deadlines or silently omitting accounts. Cache reads remain separate and do
not initiate more work; a failed complete refresh does not claim every account was
updated. Owner tasks keep their permits until actual reads finish and continue to
check policy/binding publication fences. A paused-clock regression has its own named
95-second safety budget so removing the production deadline fails instead of hanging.
