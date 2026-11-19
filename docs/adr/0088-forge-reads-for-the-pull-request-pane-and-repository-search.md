# ADR 0088: Forge reads for the pull request pane, and GitHub repository search

Status: proposed (implementer G, 2026-11-19; the coordinator's rulings of
the same day). The number is the reservation "forge node gaps (G1)" of the
parity plan, which had no file. Part 1 (`forge.repos`) was drafted as
36 and assigned protocol 35 (`REPOSITORY_SEARCH_PROTOCOL`) at its
node-line merge, the next free number in merge order. Parts 2 to 4 are
drafted together as protocol 36 (`PULL_REQUEST_PANE_PROTOCOL`). The
integrator assigns the real numbers at merge.

## Context

Paseo 0.15.4 has four forge reads that Vessel's node lacks (survey
`p3-forge-review.md`; CATALOGUE F-E28, F-E30, F-E72, F-E73, F-E80;
TRACKER 431-433, 440):

- `workspace.github.search_repositories` (`server/src/server/session.ts:7185`,
  `services/github-service.ts:2337`): an empty query lists the signed-in
  account's repositories (`gh repo list`); text searches GitHub
  (`gh search repos <q> --sort updated --order desc`); limit 1 to 50, the
  Add project sheet asks 30. Each row carries id, name, owner/name,
  description, visibility and a clone URL in the protocol that
  `gh config get git_protocol --host github.com` names (`https` when it
  says nothing). The statuses are success, unavailable (`gh` missing),
  unauthenticated and error.
- The pull request pane's checks section lists EVERY check of the head,
  grouped by outcome, each with its workflow and timing
  (`git/pull-request-panel/checks-section.tsx`).
- `pull_request_timeline_request` (`checkout-session.ts:1167`): reviews,
  conversation comments and review threads (path, line, resolved,
  outdated, review), with times and authors; GraphQL first:100 per kind
  and 100 comments per thread; `truncated` when any page had more.
- `checkout.forge.get_check_details` (`checkout-session.ts:1287`,
  `github-service.ts:2234-2330`): one check run's output, one page of 20
  annotations, one page of 100 jobs of its workflow run, the failed ones
  (failure, cancelled, timed out, action required; at most 5) and the tail
  of each failed job's log (200 lines, 16 KiB, a UTF-8 safe suffix).

Vessel's node reaches GitHub with its own HTTP client and the machine's
token (ADR 0029), never `gh` per call. A pull request is reached through
its session, never by a URL a client names. `pr.get` sends check counts
and at most 20 failing checks without ids. Its provider reads reviews and
threads for `pr.feedback` only, without times. `project.clone` exists
(protocol 10), so the Add project sheet clones already, but it cannot
search.

## Decision

### 1. `forge.repos` (protocol 35)

- Unary `forge.repos { query?, limit? }` (`operate`; forwardable; refused
  to an agent principal, as `forge.status`: the node's sign-in is a
  person's). Not `read`: with the machine's token the answer lists the
  account's private repositories and their descriptions, data from outside
  the node that no other `read` call gives (`forge.status` names the login,
  `pr.*` stays within a session's repository). ADR 0112 made
  `folder.suggest` `operate` for the same reason, and the call's one caller
  ends in `project.clone`, which is `operate` (node review R1). `query` is at most 256 characters, one line; the node trims
  it. `limit` is 1 to 50, default 20 (Paseo's server default).
- It answers `RepositoryList { access, repositories }`. `access` is the
  existing `ForgeAccess` of `github.com`: `ready` when it read, else
  `signed_out { gh }` (`gh: missing` is Paseo's "gh is not installed",
  `signed_out` its "not authenticated"), `refused`, `rate_limited` or
  `unreachable` (Paseo's "error"). Clients word these as Paseo does.
- `Repository { path, description?, visibility, updated_at, clone_url }`
  (Paseo's numeric id and bare name are left out: the owner/name path is
  unique in one answer and the name is its last segment): `path` is owner/name (`RepoPath`), `description` one line
  of at most 350 characters (GitHub's own maximum), `visibility` public,
  private, internal or unknown, `clone_url` a `CloneUrl` that
  `project.clone` takes as it is.
- The empty query is `GET /user/repos?affiliation=owner&sort=pushed`
  (the account's own repositories, newest first, as `gh repo list`); text
  is `GET /search/repositories?q=…&sort=updated&order=desc`; `per_page` =
  limit for both. github.com only, as Paseo.
- The clone protocol is `gh config get git_protocol --host github.com`,
  run with the same hygiene as `gh auth token` (no shell, the node's data
  folder, ten seconds, two at once) and kept a minute per host, so a person
  typing a search does not run `gh` per keystroke (Paseo reads it per
  search). `ssh` gives `ssh_url`
  (`git@github.com:o/r.git`); anything else, a missing `gh` or a failure,
  gives `html_url` (`https://github.com/o/r`), as Paseo.
- Search has its own GitHub quota (30 a minute). A limit answer to a
  search holds only search on that host; any other limit answer still
  holds the whole host (secondary limits are the account's).

### 2. Every check on `pr.get` (protocol 36, drafted)

`Checks` gains `runs: Vec<CheckRun>`, every context of the head as read
(at most 100). `CheckRun` gains optional `run` (the check run's id),
`workflow_run` (its GitHub Actions run's id), `workflow`, `started_at` and
`completed_at`. `PullSummary`, the row every client lists, gains optional
`tally: CheckTally { passed, failed, pending }`, the counts `Checks`
already has: Paseo's sidebar fills its check glyph by quarters of
completed over total, and a row has no list of runs (sending up to 100
runs a row in a listing every client polls was rejected). All are optional
serde fields: an older client ignores them, an older node omits them, and
`failing` keeps its meaning.

### 3. `pr.timeline { session }` (protocol 36, drafted)

`read`, open to agents as `pr.get` is, held to the same session rule. One
GraphQL query: reviews (100), comments (100), review threads (100, each
100 comments), with ids, times, author logins, URLs, a thread's path,
line, start line, resolved and outdated, and a comment's review. The
answer carries `access` as `pr.get`, the items in Paseo's stable order
(time, then id), and `truncated` when a page had more. The provider's
4 MiB body bound also bounds the answer; a larger answer is `unreachable`
with no partial list.

### 4. `pr.check { session, run }` (protocol 36, drafted)

`read`, as `pr.timeline`. Both are held to the caller's ownership of the
session, as `pr.get`'s refresh is (each is a forge request on demand).
`run` must be a check of the session's pull request head, as the
watcher's last read named it; the node never reads a check that a client
names freely, and an unnamed one answers no report. The answer has the
check run's output (title, summary, text), one page of 20 annotations,
the failed jobs of its workflow run (at most 5, from one page of 100) and
each job's log tail; a log that cannot be read leaves its job without one
(Paseo attaches what it has). `truncated` when a page was full or more
jobs failed than are read.

The job log is `GET …/actions/jobs/{id}/logs`, which GitHub answers with
a 302 to signed blob storage on another origin (GitHub Enterprise Server
may answer the log itself: read as any answer). The API request goes
through the client as every request does, which keeps a redirect's
`Location` instead of following it. This is the one exception to ADR
0029's same-origin rule:

- the node follows exactly one redirect, only to `https` (never `http`),
  and sends that request WITHOUT the `Authorization` header (the signed
  URL is the credential);
- it asks `Range: bytes=-16384`. If a server ignores the range (an
  answer of 200), the node streams it, keeps a rolling 16 KiB tail, and
  stops at 32 MiB read, marking the tail as cut;
- it has its own 20 s time and keeps a UTF-8 safe suffix of at most 200
  lines, saying when it cut;
- the signed URL is never logged, stored or sent to a client.

## Alternatives

- `gh` per call, as Paseo does. ADR 0029 rejected it for every forge
  call; the search would also need `gh` installed, when a token from the
  environment is enough.
- Making the client compose the clone URL from `owner/name` and a protocol
  (the page does this for typed rows). The node knows `gh`'s configured
  protocol, the client does not, so Paseo's choice is the node's.
- One method for timeline and checks: they load at different times
  (Paseo's rail needs the summary, the open pane the timeline, a click the
  logs), and the log read is a different, costlier thing.
- Following every redirect for logs: the same-origin rule exists to keep
  the token on GitHub. One redirect, without the token, keeps that.

## Consequences

- One more method now, three later. Each adds the version tables,
  samples, guard, policy, forward policy, TypeScript bindings and the
  client's unserved list, as every method does.
- The seeded fake forge (ADR 0060) lists its seeded repositories as
  search results (an optional description and visibility per repository),
  so fleets and end-to-end tests search with no network. The provider's
  tests answer from recorded GitHub bodies on loopback, never the real API.
- Differences from Paseo that remain: GitHub only (no GitLab pipeline or
  approvals); the timeline is bounded by the 4 MiB body as well as by
  page sizes; reads are addressed by session, not by working directory
  (the session row is Paseo's workspace row).
