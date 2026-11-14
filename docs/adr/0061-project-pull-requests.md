# ADR 0061: Open pull requests of a project without a session

Status: accepted value contract; provider/service staging amendment proposed.
Amends ADR 0029's session-only discovery, not its watch or write rules.

## Context and existing assets

A project's pull-request page needs its repository's open requests when
there is no live session. `pr.list` returns watched sessions, including a
session identifier for every entry. Making up a session or starting one to
fill this page would change the meaning of that call and consume agent
resources for a read.

Paseo's `packages/server/src/services/github-service.ts:2027–2050` lists
requests from a working directory with `gh pr list`, an empty search by
default and a default limit of 20. Its cache default is thirty seconds
(`:104`); identical calls share a load (`:1166–1215`). Tests at
`packages/server/src/services/github-service.test.ts:3980–4085` cover
reuse, expiry, coalescing and directory-specific invalidation. These are
behavior references, not code to copy.

Vessel already has validating `RepoRef` and `PullSummary` values, a
credential policy and bounded HTTP client in `vessel-forge`, registered
project-root checks in `workspace/roots`, and git's hardened remote lookup.
The existing session watcher deliberately refuses a default-branch or
detached checkout; those restrictions cannot decide a repository listing.

## Decision Log

- Purpose and intent: read a registered project's open pull requests
  without creating, finding or watching a session.
- Existing implementation and constraints: retain ADR 0029's host-bound
  credentials, read-only git, explicit write authority and bounded HTTP;
  retain `pr.list`'s meaning for existing clients.
- Alternatives: extend `pr.list` with fictitious session entries; accept a
  repository URL from the caller; run `gh pr list` for every page; or extend
  the existing forge provider with a bounded project read.
- Choice: extend the existing provider and project resolver. Add a separate
  typed project read when its shared unassigned-protocol mechanism exists.
- Why: the project registry remains the authority for paths and remotes;
  the existing provider remains the authority for tokens and forge facts.
  No subprocess-per-page or new provider/dependency is necessary.
- Advantages, costs and effects: no agent or watcher is needed, and page
  limits are explicit. Pagination is a live view, not a frozen snapshot;
  clients deduplicate repository/number and restart the first page to
  reconcile requests whose update time moved between reads.

## Value contract

The initial implementation stage adds values only. It registers no method,
adds nothing to existing replies/events, and changes neither the current
nor minimum protocol version. `PROJECT_PULL_REQUESTS` is
`Option<ProtocolVersion>::None`; a number is assigned at integration only
after the node, client, CLI and consuming PR screen can use the contract.

- `PullCursor` is opaque printable ASCII, nonempty and bounded by its
  `MAX_BYTES` constant. Neither errors nor `Debug` disclose its contents.
  The node never treats it as a URL, path, query or shell input.
- `ProjectPullCursor` pairs that cursor with the `RepoRef` of its page.
  Before continuing, the node resolves the project's remote again and
  refuses a repository mismatch before sending a request to a forge. This
  association is consistency data, not authority to select another host.
- `OpenPullPage` contains one repository, at most `MAX_PULLS` summaries,
  an optional next cursor and a node `read_at` timestamp. Private fields
  and validating construction/decoding reject an oversized page, another
  repository, duplicate numbers, non-open requests and a next cursor on an
  empty page. Malformed pages fail whole; they are never silently cut.
  An empty page without a cursor is a successful empty result.

`MAX_PULLS` follows Paseo's default list limit. The cursor byte cap is a
Vessel input budget, not an assertion about a provider's cursor encoding.
Every bound has one named constant beside its value type; checks, error
messages and boundary tests use that constant.

## Following service and transport stage

The planned method is `pr.project_list`, `ListProjectPullRequests`, Read
scope, returning a typed ready page or an unavailable reason. It takes a
registered `ProjectId`, optional `ProjectPullCursor`, and a named cached or
refresh read mode. The existing client method table's shared
`required_protocol` accessor must land first; until then there is no
`Request` implementation, dispatch arm, CLI flag or client method for it.
Once present, both client admission and node dispatch refuse an unassigned
call with `CallError::UnassignedProtocol`; assignment and older-peer
fixtures are separate from the value stage.

Read-only devices may read. Agents may not list a whole repository, as
ADR 0029 already decides; delegated calls use effective scope intersection
and retain the originating principal's restrictions. No-read is Forbidden.
Unknown/not-ready/non-git/replaced project roots use the existing typed
project errors, before forge work.

Resolve the registered root on every call. Select the checked-out branch's
configured remote, otherwise `origin`, through the existing hardened git
runner; default-branch, detached and unborn checkouts do not require a
session, head commit or pushed branch. Parse the address with the existing
remote parser and discard its raw text. No remote, unsupported host, sign-in
failure, access refusal, rate limit and unreachable service remain explicit
unavailable outcomes; none is converted to an empty page.

The provider reads open requests ordered by update time descending, using
a single bounded GraphQL page of summaries, checks rollup, review verdict
and mergeability. It does not request review bodies, threads or check logs.
It reuses the existing neutral facts mapping. Any GraphQL error, invalid
essential fact or inconsistent page metadata fails the whole read. GitHub's
[pagination contract](https://docs.github.com/en/graphql/guides/using-pagination-in-the-graphql-api)
supplies the next cursor from `pageInfo`, so no offset or total count is
invented. There is no claim that multiple pages are an atomic snapshot.

The node admits a bounded number of calls before root/credential/network
work, refusing Busy rather than accumulating an unbounded queue. A permit
passed into blocking root resolution stays owned by that closure until it
actually exits even if the RPC is cancelled. Successful pages use Paseo's
cache lifetime and coalesce identical requests; both stored pages and
in-flight keys have named finite bounds. Failures are not reused for a
later request. Refresh bypasses a stored result. The key includes the
resolved repository, cursor and credential generation; a changed remote or
sign-in cannot reuse another repository/account's facts. No stale successful
page is presented as fresh after a failed refresh.

## Security and deliberate differences

Only the node's configured host credentials reach the existing provider's
API addresses; enterprise-token host restrictions, sensitive headers,
redirect policy, response byte/time limits and rate controls stay those of
ADR 0029. The caller supplies no executable, arbitrary path or target URL.
This read performs no fetch, push, checkout, merge, prompt or feedback;
repository hooks are not enabled. There is no new listener or exposure.

Unlike Paseo's free working-directory selector and search/limit arguments,
this first contract selects a registered project and a fixed bounded page
of open requests. Explicit pagination avoids a truncated list being shown
as all requests. Search, closed/merged history, arbitrary counts and
project-wide merge/detail actions are deferred. Existing session detail
and pinned merge contracts remain unchanged; a summary does not authorize
a merge. The provider and git subprocess paths have the same meaning on
Windows; no Unix-only implementation is introduced by the value stage.

## Validation and delivery

Value-stage fixtures cover cursor length/character boundaries, safe
diagnostics, empty/full/oversized pages, repository and duplicate/state
invariants, complete serialization round trips and rejecting malformed
whole pages. No existing wire shape is changed by these values.

The later provider/service slice must use actual FakeForge operations and
the loopback GitHub stand-in, not a mock of the new listing function.
Fixtures include more than one page, other repositories, closed/merged
requests omitted, zero sessions, default/detached/unborn checkouts, a moved
root/remote, read-only/agent/delegated/no-read authority, partial GraphQL
errors, no access, rate limit, cancellation and admission saturation.
Cache/coalescing tests use driven time and barriers. The CLI and fake fleet
consume the actual call only after registration; the fixture manifest must
identify projects independently of sessions. No real forge, token or agent
is used by these tests. Native screens are outside this change.

The value stage and following transport stage are reported separately.
Root checks, the explicit forge-fixture feature gate and generated-type
checks apply; adding a callable wire contract also requires both older-peer
directions and the web checks. No stage claims project listing works until
the actual node/client/CLI path is executed.

## Proposed provider and service staging

[Decision Log]
- Purpose and intent: exercise the actual provider, registered-root checks
  and bounded cache before a transport method can safely be registered.
- Existing implementation and constraints: `Forge` is the existing open
  provider interface; GitHub whole reads own the neutral facts mapping.
  `Credentials` already decides host binding, expiry and retry cooldown.
  An unconnected ordinary node module would otherwise be dead product code.
- Alternatives: duplicate GitHub HTTP/auth/facts in the node, change
  visibility to hide dead-code warnings, add a second feature flag, or
  stage only the node entry while extending the existing provider normally.
- Choice: extend `Forge` with a repository-page operation implemented by
  GitHub and the actual shared FakeForge. Factor the common summary decoder
  out of whole reads, preserving their facts and error precedence. Keep
  the unconnected node service under `cfg(test)` until reviewed wiring
  removes that attribute.
- Why: provider behavior remains real and shared, and tests drive the
  intended node actor without adding a product endpoint or a testing-only
  abstraction. No private entry is made public to silence a lint.
- Advantages, costs and effects: staged tests can prove real root,
  cancellation and cache behavior; they cannot establish product wiring.
  The wiring change must re-read and gate the product configuration before
  this service reaches the node integration line.

The GitHub page uses the whole read's head/context limits through named
constants. A typed query envelope replaces ad-hoc JSON construction. Shared
external summary fields decode known states into enums once; both reads use
the existing checks/review/mergeability functions. Missing or malformed
essential values fail whole. Partial GraphQL errors are never accepted as
a complete page. The fixed API target, credential policy, sensitive header,
redirect, byte, time and quota controls remain the existing provider's.

Credential-associated cache keys use an opaque `CredentialIdentity`, with
the validated host and allocation markers, not token text or a token-derived
hash. The credential cache retains a marker when refreshing the same value
and source, and replaces it when credentials change or leave the cache.
Every observed refusal also changes an invalidation marker, even while the
existing retry cooldown suppresses another `gh` lookup. Keys remain bounded
by the page cache; `Debug` discloses neither tokens nor marker addresses.
The loader compares identity before and after its read and refuses to
publish across a change. A global refusal marker may invalidate other
hosts' pages too: conservative extra reads are preferable to associating
facts with an unconfirmed credential. Host token selection and retry
deadlines themselves do not change.

The node actor owns the cache, flights and bounded command channel.
`MAX_ACTIVE` admits requests before filesystem, credentials or network
work. A request owns an `Arc` admission guard; a clone remains inside the
shared blocking root resolver until it really exits. Queued completions
and coalesced waiters retain admission until the actor processes them.
Dropping the RPC receiver cannot release a still-running job's admission
or strand other waiters. Stop closes new admission and cancels owned jobs;
a blocking closure keeps its guard independently until completion.

`MAX_KEYS` bounds retained cache slots. `FRESH` follows Paseo's cache
lifetime; cached responses preserve the original `read_at`. Ordinary reads
may reuse a still-fresh page while refresh runs. Refresh joins an existing
refresh, but does not join an older ordinary load. The actor keeps a typed
load identity until all older flights finish: a late ordinary result cannot
overwrite a newer refresh or resurrect a page after that refresh failed.
Failures are shared with their current waiters and are not reused by a
later request. Slots with active flights are not silently evicted.

Tests of this stage use real FakeForge state and loopback HTTP responses,
driven cache time and barriers for completion/cancellation order, and actual
registered git folders including base, detached and unborn checkouts.
Transport permissions, forwarding, older peers and client/CLI/fleet
consumers remain the following wiring stage. The unassigned feature constant
and method table remain unchanged throughout service staging.
