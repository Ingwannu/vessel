# ADR 0029: Pull requests through a forge, signed in as the machine is

Status: accepted, 2026-11-12 (milestone 11, wave 2, track B2,
`docs/plans/backend-m11.md`, "Track B2: contract" and its review; the
wire is protocol 11's `crates/vessel-proto/src/forge/`, the provider
`crates/vessel-forge`, the node's part `crates/vessel-node/src/forge/`).
Amends ADR 0019.

## Context

agent-orchestrator, GOAT and Paseo follow a session's pull request: its
checks, reviews and whether it can merge. agent-orchestrator's board,
whose reducer Vessel ported in milestone 9 (ADR 0019), has three lanes
(Validating, In review, Ready) that only those facts fill; it merges on
request and routes failing checks and review comments back to the
session. GOAT and Paseo open, show and merge pull requests. A Vessel node
knew no pull request, so those lanes stayed empty.

Questions:

1. How does the node sign in to a forge, and what if the machine's `gh`
   is missing, signed out, or signed in as someone else?
2. One trait for every forge: what is in it?
3. How do a pull request's facts reach the board without a second place
   that decides a lane?
4. How is watching bounded?
5. How is merging kept explicit and exact?
6. What is sent back to a session, when, and how is it kept from
   becoming a way for strangers to prompt the agent?
7. What may an agent do?

What the references do is in the plan's "Track B2: contract" ("What the
references do"), each with its source paths.

## Options

**Signing in**

1. The machine's `gh` for every call (`gh api`, `gh pr …`), as Paseo:
   no token in Vessel's memory; a process per request (about 30 ms of CPU
   each, a few a second while watching), and conditional requests, time
   limits and rate-limit headers only by parsing `gh`'s output.
2. A token the machine already holds, used by the node's own HTTP
   client, as agent-orchestrator and GOAT: `GH_TOKEN` or `GITHUB_TOKEN`,
   else `gh auth token`.
3. A token Vessel keeps as a secret (ADR 0012's machinery), as GOAT's
   settings and agent-orchestrator's `/github/pat` do: one more copy of a
   credential, a page to paste it into, and rotation by hand.

**Where the facts enter the board**

1. Re-derive the board where the node fills `SessionInfo` (as
   `workspace` is filled): the reducer needs facts only the session's
   actor holds (interrupted, signal), so it would be a second, partial
   reducer.
2. A per-session `watch` from the node to the session's actor, whose
   tracker keeps the facts and whose one reducer call reads them.

**Feedback's delivery**

1. Typed into the agent's input, as agent-orchestrator does (it does not
   wait for the agent to be idle).
2. Through the node's queue (ADR 0024), after the running turn, as GOAT
   wakes its agent.

## Decision

**1. Signing in (option 2).**

- For `github.com`: `GH_TOKEN`, then `GITHUB_TOKEN`, as the binary read
  them from the node's environment (a library reads no environment);
  else `gh auth token --hostname github.com`. For another host:
  `GH_ENTERPRISE_TOKEN` or `GITHUB_ENTERPRISE_TOKEN` only when `GH_HOST`
  names that host (GOAT's trap: an enterprise token must not go to a host
  taken from a remote's URL), else `gh auth token --hostname <host>`. A
  host with no token gets no request: it is `not_supported` or
  `signed_out`. Vessel stores no forge token of its own.
- `gh` runs with no shell, in the node's data folder (never a
  repository's), `GH_DEBUG` emptied (it would print the token),
  `GH_PROMPT_DISABLED=1`, `GH_NO_UPDATE_NOTIFIER=1`, ten seconds, two at
  once; its output (the token) is never logged; its errors are reduced to
  `missing`, `signed_out` or `failed`.
- The token is held in memory five minutes (a miss ten seconds),
  dropped on a 401 and asked for again at most once a minute; it travels
  only in an `Authorization` header marked sensitive; its `Debug` says
  only that it is one; redirects are followed only to the same origin.
- Signed in as someone else: `GET /user` names the login, shown in
  `forge.status` and `access: ready { login }`; a repository that login
  cannot see is `refused { no_access }`, and single sign-on `refused {
  sso { url } }`.

**2. One trait (`vessel_forge::Forge`)**, object-safe, an open extension
point (`Arc<dyn Forge>`, which AGENTS.md section 2 allows): GitHub's
provider serves every host for now (github.com, `*.ghe.com` and GitHub
Enterprise Server by their API addresses) and the token decides whether
a host is reached at all; a later provider is chosen by a remote's host.
It answers who the token is; the pull requests of a branch
(conditional); whether one changed (conditional); one read whole (its facts and its discussion: reviews,
threads, failing checks); open; merge at a pinned head; whether a login
may write; and whether the forge can pin a head for a method (GitLab
cannot for rebase, Gitea not at all). Its values are the wire's, named
so GitLab's `iid`, pipelines and approvals and Gitea's states fill them.
GitHub is built (REST for what has conditional requests and for writes,
one GraphQL read); a test double (`fake`) implements it for the node's
tests, so nothing in the tests calls a real forge, and the GitHub
provider is tested against a stand-in on loopback. A person's request is
never conditional: the `ETag`s are shared with the watcher's looks, and
a 304 would answer a person with nothing (an agent's refresh stays
background traffic, conditional).

**3. The facts enter through the session's actor (option 2).** The node
gives each watched session a `ForgeView` through a `watch`; the actor's
tracker keeps the facts and `board::derive` reads them
(`PullRequests::Seen` once the node read the repository; `Unseen` before,
and whenever it cannot see). The fleet-policy facts follow the feedback
switch (agent-orchestrator drives both with one flag). At the final
status the facts are dropped before that record's board, and
`SessionInfo.pull_request` is cleared, so an ended session is `exited`
in building exactly as the store places a stored one.

**4. Watching is bounded.** Per repository and branch, at most 64 per
node (a person's sessions first); live sessions only, not on the
project's default branch; finding a branch's pull request every 120 s
(300 s after a merge or close), watching one every 60 s (300 s after 30
quiet minutes, jittered) with three conditional requests (a 304 is free)
and a GraphQL read only on a change or every 5 minutes; 4 requests at
once (one kept for a person), 5 a second per host in the background,
30 s each, 4 MiB a body; background reads stop under 10 % of a
resource's quota; a limit answer (429, a 403 with none left, a
`retry-after`, the words, GraphQL's `RATE_LIMITED`) cools the host down
(as said, else to the reset, else 60 s doubling to 15 minutes); errors
back a watch off from 60 s to 30 minutes. A session's watch is dropped at
once when it is archived or deleted, and at its next due time when it
has ended.

**5. Merging** is a person's call naming the pull request's number, the
method and the full head commit the person looked at: refused if the
head moved (before the request, and by the forge itself through the
pinned `sha`), if it is a draft or not open, or if the repository
requires a merge queue. Nothing in the node merges on its own; it never
deletes the branch, retries, or waits for checks.

**6. Feedback (option 2), off by default.** Failing checks (per head and
check name), changes requested (per review), unresolved comments (per
comment) and conflicts (per head, re-armed once the forge knows the pull request
no longer conflicts, not while it is still computing that), each
signature prefixed with the repository, sent once. Text is quoted only
from people with write access (asked of the forge, kept an hour), with
control and format characters removed and set apart as quoted data;
others' and bots' are counted, never quoted. One prompt at most waits in
the session's queue, edited as more arrive; never into a running turn,
never started while the session waits on a permission or a question.
The switch and the signatures sent are kept in the store (schema 10), at
most 1,000 per session: near the bound those of events the last read no
longer has go first, then the oldest.

**7. Agents** read any session's pull request (as they read every
session's info) and refresh their own; they open one for a session they
may operate, but the node never pushes for them: the branch must already
be on the remote at the folder's `HEAD`. They cannot merge, switch
feedback, list every pull request or see the node's sign-in.

## Why

- **A token the machine holds, used by Vessel's own client**, because
  watching makes a request every few seconds: a process each is real CPU
  on a node that otherwise idles, and conditional requests (the only
  free reads GitHub offers) need the headers. The machine's own sign-in
  is what all three references use; a token of Vessel's own would be one
  more copy of a credential to leak and to rotate.
- **The actor**, because the reducer reads facts only it has, and ADR
  0019's rule that one function places a session is what keeps every
  client's board the same.
- **The queue**, because the agents disagree on what input during a turn
  does (ADR 0024), and a person cancelling a turn must hold what waits.
- **Quoting only writers**, because on a public repository anyone can
  comment or review, and what feedback quotes becomes the agent's prompt.
- **No push or merge for agents**, because the push runs the
  repository's hooks with the user's credentials outside the agent's
  sandbox (`git.push` is refused to agents for that reason), and a merge
  publishes to a branch others share; the owner asked for merging to be
  explicit. An agent can still run its own `git` and `gh`: this is least
  privilege for a confused agent, not containment (ADR 0013).

## Consequences

- Protocol 11 (`docs/PROTOCOL.md`): six methods, `SessionInfo.
  pull_request`, no event. A protocol 8 to 10 client ignores the field
  and draws the lanes it already knows.
- ADR 0019 amended: the board is no longer only a projection of the
  session's records; the node's pull request facts are its one other
  input, and an ended session's are dropped.
- A machine without `gh` and without `GH_TOKEN` sees no pull request;
  `forge.status` says so. A node run as a service needs `GH_TOKEN` in its
  unit, or `gh` signed in as the service's user.
- Store schema 10 keeps the feedback switch and the signatures sent;
  nothing else of a forge is stored.
- Not built here: GitLab and Gitea (the trait is shaped for them), forks
  (a head in another repository), merge queues and auto-merge, pull
  requests not found through a session (a pull requests page of the
  repository's own), a page (track B2-page).
