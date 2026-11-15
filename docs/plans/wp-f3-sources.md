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
- Existing constraint: the production daemon has no source seed switch. Existing vendor fixtures do not contain every generic source forecast/balance combination. At introduction the managed export whitelist did not include either the CLI or this executable; the coordinator-approved fixed selection is now installed as described below.
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
The coordinator-approved managed export is now installed (T, 2026-11-15 12:4x UTC):
```
vessel-remote-gate run --tree <tree> --commit <reviewed assigned SHA> --label <own> --out <new> --no-wait --root-binaries vessel-usage-fixture vessel
```
Only these fixed names are allowed. The fixture recipe uses `--locked -p vessel-node --features usage-fixture --bin vessel-usage-fixture`; the CLI recipe uses `--locked -p vessel-cli --bin vessel` after web-ci with `VESSEL_WEB_REQUIRE_BUNDLE=1`. Ordinary lane only, no app or unrelated root build. Attach/export returns new files only, binaries0500 and metadata0400, with source commit/features/pinned toolchain/exact recipe/SHA256; builder/recipient UID1000. `attach` for an ordinary cargo build still does not export custom binaries. Verify the source/feature/hash and actual file modes before using the real-node fixture; never manual VM copy, local compilation or another binary's manifest. Tool regression success is not a product build, assigned wire or fixture e2e PASS; all remain owed.

### Pending refresh presentation retains the previous attempt's meaning
N2's source check of Paseo `query.ts`/`model.ts:209–236`/`pinned.ts` shows that a pending refresh of an available report leaves that success sidebar-eligible with `retained=false`. A failed/unavailable attempt with an older success remains card-only, including while its next refresh is pending. Snapshot presence alone cannot distinguish those cases.

`Refreshing.origin: UsageRefreshOrigin` captures that immediately preceding attempt before the source actor changes state. `Available` is emitted only from an actual immediately preceding Available result; other states and omitted wire fields are `Unavailable`. Legacy limits' Refreshing state cannot prove the prior attempt and projects the conservative unavailable origin. Consumers may keep the original successful timestamp and pins for Available-origin pending results, but must never promote a failed/unavailable retained snapshot into a current observation. This adds no method/version and no permission or fetch behavior. A synthetic actor barrier checks the Available origin; a DTO roundtrip checks explicit preservation and safe omitted-field decoding. Narrow checks remain owed.

A service-level regression now starts an actual isolated Node with the recorded source backend, obtains the real Connection via an in-memory framed handshake, and invokes the generated Guarded methods directly. No pending protocol is assigned or bypassed on the wire. It exercises Admin-only grant, Operate-only refresh, zero credential discovery for Read/cache misses/consent/unknown inspect, populated current and retained monetary redaction, original successful timestamp and same-binding retained error, and source-Off forgetting. A separate real-actor barrier preserves Unavailable origin while a failed attempt is refreshed. These are drafted regressions pending remote execution, not wire/e2e acceptance.

### Complete-refresh time budget
A sequential machine refresh could spend one vendor deadline per configured profile
before starting the seven additional sources, exceeding the native caller's 120-second
budget even for normal multi-source setups. Saved-login reads now use a stable-order
buffer of one, and the saved and additional owners start concurrently while keeping
their existing actual-work admissions. A named 90-second complete-refresh budget
refuses with safe Unavailable instead of holding a unary request across hundreds of
profile deadlines or silently omitting accounts. Cache reads remain separate and do
not initiate more work; a failed complete refresh does not claim every account was
updated. Owner tasks keep their permits until actual reads finish and continue to
check policy/binding publication fences. A paused-clock regression has its own named
95-second safety budget so removing the production deadline fails instead of hanging.
The same complete-refresh bound covers exact inspect refreshes, including the legacy
captured-binding path; authorization remains before admission and cache-only inspect
still performs no discovery. The budget never substitutes a different report id.
The service fixture explicitly sets `bind=None`; its only TCP-independent carrier is
the owned duplex handshake and its Unix endpoint stays in scratch. It never binds a
product/default/live port. The paused deadline assertion allows timer-wheel rounding
inside its named safety budget rather than requiring nanosecond-exact wake timing.

### Deterministic foreign-refresh fixture gate
[Decision Log]
- Purpose: a second mounted consumer must observe a real in-flight source refresh, then its terminal outcome, without a browser payload mock or timing sleeps that guess vendor duration.
- Existing asset: the fixture already supplies the production source owner with a deferred Cursor database capability. Reuse that injection point; no second source parser, node command or protocol is needed.
- Choice: `hold` creates one fixed mode-0600 Unix FIFO and a marked Cursor database stub inside the isolated root. The fixture's injected capability waits on that FIFO, returns no database token after release, and lets the existing headless sign-in parser read the synthetic login. `release` verifies the FIFO identity and writes one byte; its nonblocking connection attempt has a named two-second test-control budget.
- Alternatives: a browser mock would skip the owner; a public pause RPC would enlarge production's contract. Both are unnecessary for this test-only executable.
- Tradeoff: this is a Unix-only, single-held-job fixture. Release it before stopping the fixture, including after a timeout: actual blocking work deliberately retains its permit until it ends. A hold longer than the production source deadline truthfully produces timeout rather than successful refresh.

After explicit Admin consent, run `python3 scripts/usage-fixture.py ROOT hold`, start an explicit Operate refresh, and wait for `USAGE_FIXTURE_GATE_WAITING` on the fixture's stdout. Cached Read from another connection can now observe Refreshing without starting a read. A second explicit additional-source refresh joins the owner; local request pending remains distinct from that cached phase. Run `python3 scripts/usage-fixture.py ROOT release` before the 20-second source deadline for a successful outcome. `USAGE_FIXTURE_GATE_RELEASED` acknowledges the actual callback release, not publication. Legacy `usage.limits` refresh retains its existing immediate snapshot behavior when already refreshing and does not promise a join. No new event or passive vendor polling is introduced. Python FIFO/mode/duplicate-hold checks are local standard-library probes; the Rust executable and end-to-end join proof still require remote build and execution.

Managed exports can additionally use the tool owner's fixed `--root-fleet vessel-usage-fixture vessel` combination when the same source's daemon and fake harnesses are required. This combines already approved names, ordinary lane only, with no app build. It does not replace or duplicate any existing run; manifests must still match each executable actually used.

The gate sequence first needs an actual successful seeded report: grant/refresh once
without a hold, then hold its next refresh to inspect that account's Refreshing
state. An initial credential discovery cannot invent an account id before its
binding is known. Strict run `wp-usage-consent-values-20261115T121135Z-a8fb5218`
reached every changed crate and returned only two source lints: the service
regression exceeded the function budget and fixture preparation borrowed its Args
without consuming them. Extracting its passive-read/consent phase preserves the
same one-Node sequence, all assertions and the outer safety deadline; preparation
now borrows Args inside its owned blocking closure. This is a correction pending
the changed-source strict run, not a Clippy or package pass.

### One actual per-node admission and cooldown context
[Decision Log]
- Purpose: adding a source owner must preserve the existing two-work limit for the entire node, including queued blocking reads and their cooldown ownership.
- Existing defect: both actors constructed a fresh `LimitContext`, which accidentally allowed two saved-login jobs plus two additional-source jobs. Reusing the type alone did not reuse the actual admission.
- Choice: configured assembly constructs one context and passes clones to both actors. Standalone actor fixtures explicitly supply their own isolated context. Each group schedules one read during one machine refresh, using the shared two-work floor rather than racing four immediate admissions; separately admitted concurrent RPCs still receive bounded Busy outcomes.
- Alternatives: raising the semaphore bound would change the established resource limit; adding another scheduler/queue would duplicate ownership. Neither is needed.
- Impact: all permit/cancellation rules remain in the existing context, and shutdown waits for actual blocking work across both owners. Two concurrent source families may take longer, so the existing complete-refresh budget remains a truthful refusal boundary.

The mixed-family regression uses production `Limits::configured`, a recorded
Copilot source and two configured Codex profiles, with only one blocking worker.
The source callback occupies that worker; one saved-login reader can queue with
the other permit, while the next must return Busy before the gate is released.
With independent contexts both saved readers queue and the named safety budget
fails. A release guard unblocks the callback even on failure. No timers coordinate
the interleaving and no node listener, real sign-in or endpoint is started. This
new regression and changed assembly await remote execution and review.

TS run `wp-usage-wire-fresh-20261115T121923Z-ac977308` compiled but refused
the unclassified `UsageDisclosureVersion` alias before generating declarations
(two proto tests passed, one failed; web did not run). The alias is now explicitly
Quantity: it is a numeric disclosure revision, not a node-minted identity. The
generator's existing complete alias classification check covers this boundary.
Its failure is retained; no partial artifact is presented as fresh wire output.
The corrected generator run must produce and verify the declarations remotely.

### Scoped runtime evidence, before the shared-context correction
Run `wp-usage-source-tests-20261115T122718Z-05b7a342` tested exact
`05b7a342f520095e940c8cca5f4cb9961bbf1efe`: harness 75, node 269, proto 325
and store 77 passed, total 746 passed / 0 failed / 0 ignored. Web preparation,
nested fake executables and the test daemon build also passed. The actual guarded
service consent/refresh/read-discovery/redaction/retained-success regression,
both pending-origin barriers, captured-binding ABA and aggregate deadline passed.
The normal test run emitted a Future qualification warning, subsequently corrected;
it is not strict lint proof. This snapshot precedes the fixture FIFO gate and
the shared-context correction. Their changed-head strict, mixed-family runtime,
TS freshness, assigned wire and whole package proofs remain owed. No actual
vendor sign-in, real endpoint, additional-source Session or F6 execution is proven.
