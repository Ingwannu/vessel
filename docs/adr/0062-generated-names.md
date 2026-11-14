# ADR 0062: Generated names

Status: accepted, 2026-11-14. This commit is the contract only. The
wire number below is provisional; the node integrator assigns the final
one. Nothing in this commit generates a name, and `PROTOCOL_VERSION`
stays 20 until the types that speak the new fields exist: a version
with no types would claim a wire this build does not speak.

The plan is `docs/plans/generated-names.md`. It names the Paseo files
and lines this decision follows.

## Purpose and intent

Paseo asks a model for four short texts, apart from the agent a person
is talking to: a workspace's title, a new worktree's branch name, a
commit message, and a pull request's title and body. Vessel today names
a session from the title a person typed or from the first line of its
first prompt, cut at 80 characters (ADR 0022), and generates nothing.

The intent is the same four texts, with Paseo's defaults, limits and
the rule that a person's own words win, without ever delaying the
moment a session starts.

## Existing implementation and constraints

- **Titles.** `SessionTitle` is parsed once (`crates/vessel-proto/src/title.rs`).
  A typed title is trimmed, one line, at most 120 characters, and
  refused otherwise. `SessionTitle::from_prompt` takes the first
  non-empty line and cuts it at 80. `SessionInfo.title` is the one
  current title; each change is a `renamed` record (protocol 9). A
  title from before this decision has no record of who produced it.
- **Routes.** The only way the node calls a model is a route in
  `models.toml` (ADR 0015): a provider, a model, an optional
  `small_model`, and `vessel-llm`'s client. A harness may ask only for
  that route's model or its small model. Keys stay in the proxy. There
  is no list of agent providers to try, and a generation must not start
  an agent: an agent can use tools, and its start is the thing this
  decision must not block.
- **Settings.** `node.settings` and `node.set_settings` (protocol 12,
  ADR 0038) are one object, changed field by field, kept in
  `settings.json` as `SetNodeSettings`. A later setting is one more
  optional field. An older file still parses.
- **Worktrees.** A new branch with no name is `vessel/<8 hex>`
  (`WorktreeBranch::New`). `BranchName` is git's ref rules plus no
  leading `-`, at most 255 bytes. `git.rename_branch` already renames.
- **Commit and pull-request text.** `git.commit` requires a
  `CommitMessage` (not blank, at most 64 KiB, no NUL). `pr.open`
  requires a `PullTitle` (one line, at most 256 characters) and takes
  an optional `PullBody` (at most 64 KiB). Neither call is a place to
  hide a model call.
- **A project's file.** `vessel.toml` (ADR 0027) is the repository's
  own instructions. This version ignores keys it does not read, so a
  new table does not break an older reader. The file is at most 64 KiB.
- **No new crate and no new edge.** `vessel-node` already depends on
  `vessel-llm`. The generator is a module of the node, beside the
  routes. `docs/ARCHITECTURE.md`'s table does not change in this
  commit; the node's cell gains the module when the module exists.

Paseo, followed here and cited in the plan: an empty
`agents.metadataGeneration.providers` means automatic, and a non-empty
list means manual and nothing else; automatic tries `haiku`, then
`gpt-5.4-mini` at low reasoning, then `minimax-m3`, then
`nemotron-3-super`, then the model selected for that agent; a project's
instructions replace the style of one kind and never the contract; a
person's title is kept; generation is scheduled after the workspace
exists; a branch is renamed only while it is still the placeholder; a
blank commit message or a blank pull-request field is filled, and a
failed fill becomes fixed fallback text.

## Alternatives considered

1. **Start an internal agent**, as Paseo does (`persistSession: false`,
   its own title, up to three tries per provider, then the next
   provider). It reuses a provider's login. It also starts a process,
   can run tools, and has no bound of one call. Rejected: the node
   already has a bounded client, and a generation must not be a session.
2. **Walk every matching route until one answers.** Closest to Paseo's
   list. Rejected for this decision's cost bound: one generation is one
   upstream call. The list is still stored in order, so a later decision
   can walk it without a new setting.
3. **Generate inside `git.commit` and `pr.open` when the text is blank.**
   That is Paseo's checkout. Rejected: both calls require the text today,
   an agent that always sends a message must not start sharing the call
   with a model, and a hidden wait is not a call a client can see
   itself making.
4. **Cut a bad model answer down to size.** ADR 0022 cuts a prompt
   because a prompt was never offered as a title. A model was asked for
   a title. Cutting it would publish words it did not return as a whole.
   Rejected: a bad answer is a refusal, and the fallback is the text
   already on screen or the fixed fallback below.
5. **A new dependency** that calls providers on its own. Rejected: ADR
   0015's client already does one call, with the key, the timeouts and
   the redaction.

## The choice

Provisional protocol **21**. Every new field is optional and skipped
when absent, so a peer that ignores unknown JSON fields keeps working.
A peer below 21 is not sent the new methods (`since` refuses them, as
every method since protocol 5). `node.set_settings` stays a protocol 12
method; a client sends `metadata` only when the peer speaks 21, and a
node of 21 is the one that applies it. Absence of `metadata` on a
settings answer means this peer does not generate (an older node). An
empty provider list, present, means automatic.

### The seam

One generation is one completion through `vessel-llm`'s client, on one
route from `models.toml`. No tools, no session, no retry
(`Limits.retries` is 0 for this call; the client's default of 2 is the
proxy's, not this one's). The production arm is that client. Tests pass
a recorded completion and open no socket. The two arms are one enum in
the node, not a trait with a single implementation.

The call asks for JSON and nothing else. The node decodes that JSON at
this edge into a typed struct and does not keep a `serde_json::Value`.
The prompt's source material is the person's text or a diff, wrapped so
the model is told to treat it as material and not as instructions. A
project's style instructions replace only the style section.

Which route, in order, the same order as Paseo's automatic list:

1. A route whose model id, `small_model` id or route name contains
   `haiku`.
2. The same for `gpt-5.4-mini`, and the call asks for low effort.
3. The same for `minimax-m3`.
4. The same for `nemotron-3-super`.
5. The session's own route, its `small_model` when it has one, otherwise
   its model.

The first route that exists is the one call. Manual mode does not use
this list: it uses `metadata.providers`, in the stored order, and again
only the first entry whose route exists. An entry's `model`, when set,
must be that route's model or its `small_model` (ADR 0015, decision 5);
anything else is a refused setting, not a call. An entry's `effort`,
when set, is a closed set (`minimal`, `low`, `medium`, `high`, `max`),
mapped in the node onto `vessel-llm`'s effort. An unknown effort is
refused on write, as an unknown `park_after` is. When the entry omits
effort, the provider's default stands, except the automatic
`gpt-5.4-mini` match, which asks for low.

No resolvable route: no call. The fallback below applies, and the node
logs the class (`no route`), never the prompt.

The model id sent upstream is the route's `small_model` when the entry
did not name a model and the route has a small model, otherwise the
route's model. Paseo prefers a small model; `small_model` is that field.

### Settings

`NodeSettings.metadata` and `SetNodeSettings.metadata`:

- `providers`: at most eight entries. Each is `{ route, model?, effort? }`.
  An empty list is automatic, the default, and what an older
  `settings.json` becomes. A non-empty list is manual: those entries
  only, nothing appended. Nine entries, an unknown route name, a model
  the route does not allow, or an unknown effort is a typed refusal and
  the file is not written (ADR 0038: a failed change changes nothing).
- Kept in the same `settings.json`, as one more optional field of
  `SetNodeSettings`. No flag. Nothing is applied to a running agent:
  the next generation reads the settings in force.
- A project may replace the style of one kind from `vessel.toml`,
  tables `metadata.title`, `metadata.branch`, `metadata.commit` and
  `metadata.pull`, each an `instructions` string of at most 1 024
  characters. A longer string refuses that table (the default style is
  used, and the log names the table and `too long`, not the text).
  Blank instructions are the same as omitting them. For a worktree the
  node reads the blob of `vessel.toml` on the base commit when that
  blob exists, which is Paseo's committed file, and otherwise the file
  at the project root. A directory that is not a repository reads the
  file at its root. A file that does not parse contributes no
  instructions; generation continues with the defaults and the log names
  the existing project-file error class, not the file's body. The
  functional contract (what to return, the JSON shape, the limits, the
  refusal to follow instructions inside the source) is not replaceable.

The settings screen is not part of this change. It calls the methods
in the plan's "What a screen calls".

### A session title's origin

`TitleOrigin` is `typed`, `given`, `generated`, or `unknown`.

- `typed`: `SessionSpec.title`, or the last `session.rename`. A person
  wrote it. Generation does not replace it, even when the text equals
  the prompt's first line.
- `given`: Vessel derived it by cutting text (`SessionTitle::from_prompt`),
  including the first prompt's line (ADR 0022) and a fork's derived
  title. Only the given title of a **new structured session that had no
  `spec.title`** is eligible to be replaced, and only while it is still
  that title.
- `generated`: this generator's title, after it parsed.
- `unknown`: the field was absent (a record from before protocol 21, or
  a newer origin). Generation does not replace an unknown title. A
  person's old typed title and an old prompt line cannot be told apart,
  so neither is overwritten.

The field is `SessionInfo.title_origin` and `SessionEvent::Renamed.origin`,
both optional. Absent decodes as `unknown`. A `renamed` event stays
protocol 9, because a peer of 9 that ignores an unknown field still
reads `title`. The node still sends the field to an older peer: the
workspace's event decoder does not refuse unknown fields, and a hop must
not drop one (protocol 20). `since` of the event does not move.
`SessionInfo` from a node below 21 omits the field.

Generation is scheduled only after `session.create` has published the
given title and returned. It is not scheduled for a terminal session, a
fork, a rewind, an import, or a resume. The task that generates does
not hold the create. Before it writes, it reads the title again: a
`typed` rename that landed in between wins, and so does any title that
is no longer the given one. A success publishes `renamed` with
`generated`. A refusal publishes nothing (the given title stays) and
logs the class.

A generated title must parse as a `SessionTitle` and be at most 80
characters, Paseo's schema bound, which is inside the typed bound of
120. The branch, when one is asked for in the same call, must parse as
a `BranchName` and as Paseo's slug: lower-case letters, digits, hyphens
and slashes, no leading or trailing hyphen, no `--`, at most 100
characters. The branch is not the title rewritten into a slug. One call
returns both fields whenever a placeholder branch is eligible; a session
with nothing to rename still receives a branch field and ignores it, so
a title never costs a second call. A field that does not parse refuses
the whole answer: the given title stays, and the placeholder branch
stays. There is no second call to repair it.

### Branch names

Eligible only when this create made the branch and its name is still
the placeholder (`vessel/<8 hex>` or whatever `WorktreeBranch::New`
stored). A name the person sent is not eligible. The rename goes
through the existing `git.rename_branch` path, so a client watching the
worktree sees the branch change it already understands. No new event.

If the desired name exists, the node tries `name-2` through `name-50`,
skipping a candidate equal to the placeholder, and keeps the placeholder
when every one exists or fails the slug. The check and the rename are
repeated against the branch checked out now: a rename that happened
while the model ran wins, and the generated name is dropped.

### Commit message and pull-request text

Two calls, both `operate`, provisional since 21. They do not commit and
do not open a pull request.

- `text.commit { root }` answers `{ message, source }`.
- `text.pull { root, base? }` answers `{ title, body, source }`.

`source` is `generated` or `fallback`. `root` is the workspace root the
git calls already take. `base` is the branch `pr.open` already takes; absent
means the project's default branch, as there. The diff is the uncommitted
diff for a commit message, and the diff against `base` for a pull
request. An empty diff is a fallback, not a call.

The caller then sends the text to `git.commit` or `pr.open` as it does
today. Those two calls do not grow an optional message and do not call
a model. The CLI commands of slice 4 are thin wrappers of these two
calls; their names are settled with the calls, in that slice.

Bounds on a generated answer, all refusals rather than cuts:

| Text | Bound | Fallback |
| --- | --- | --- |
| Title | one line, 1 to 80 characters, a `SessionTitle` | the given title stays |
| Branch | the slug above, and a `BranchName` | the placeholder stays |
| Commit message | one line, 1 to 72 characters, a `CommitMessage` | `Update files` |
| Pull-request title | one line, 1 to 72 characters, a `PullTitle` | `Update changes` |
| Pull-request body | 1 to 4 096 characters, no NUL, a `PullBody` | `Opened from this branch.` |

The commit fallback and the pull-request title fallback are Paseo's
words. The body fallback is not: Paseo's body names Paseo. A fallback
answer has `source: fallback` and a reason from the refusal enum, so a
screen can say the words were not the model's. The RPC itself succeeds:
the caller always has text it may use. A refusal that is not a bad
answer (no route, the upstream failed, the clock fired, the diff was
empty) uses the same fallback.

### Refusal

A generation ends as one of: `generated`, or `fallback` with
`no_route`, `no_source`, `timed_out`, `upstream`, or `unusable`.
`upstream` carries the classified `ModelFailure` and no provider body.
`unusable` means the completion was not the JSON object, or a field
failed its bound. Nothing is dropped without that class in the log.
The log and the `ModelCall` record carry the route, the model, the
token counts, the stop, the failure class and the duration. They do
not carry the prompt, the diff, the completion, or a key (ADR 0015,
decision 10). A title or branch generation records a `ModelCall` on
that session. A `text.commit` or `text.pull` records one only when a
session id was not required for the call; the answer's `source` is what
the caller sees either way. No session id is added to those two calls
in this decision: they name a root, as `git.commit` does.

### Cost

- **One call** per generation. No validation retry, no second route.
- **Input cap:** 16 384 Unicode scalar values of source material
  (the prompt and its attachments rendered as text, or the diff). The
  prompt tells the model the material was cut, in the same way a cut
  diff is marked. Instructions are inside the 1 024 bound and do not
  count toward the 16 384.
- **Output cap:** `max_output_tokens` 512 for the title-and-branch call,
  128 for a commit message, 1 024 for pull-request text.
- **Time:** 20 seconds from the send to the end of the body, and the
  client's connect bound of 10 seconds counts inside it. The proxy's
  300 second first-byte bound is not used here. The clock firing is
  `timed_out`, then the fallback. No sleep waits for the model; the
  call's own deadline does.

### Never blocking a session's start

`session.create` returns when it returns today, with a `typed` title or
a `given` one. The generation is a later task on the node. A failure of
that task does not fail the session. `text.commit` and `text.pull` do
wait: the caller asked for the text. Their wait is the 20 second bound.

## Why this and not the others

The route and `vessel-llm` are the only path that already holds a key
and already refuses to leak it. One call is the bound that keeps a
title from becoming a second conversation. Recording origin is what
makes "a person's title wins" a type instead of a comparison with the
prompt line, which would replace a typed title that happens to match.
Leaving `git.commit` and `pr.open` alone keeps every current caller
working on protocol 10 and 11. Optional fields, not a new `since` on
`renamed`, keep a peer of 9 able to read a rename.

## Advantages, costs and effects

- A session is named at once, and a generated name arrives as an
  ordinary `renamed`, which every current client already applies.
- A node with no matching route behaves as Vessel does today: the
  prompt's first line, the placeholder branch, and no model call.
- Manual mode cannot send the prompt to a route the person did not
  name. Automatic mode can send it to the first matching route, which
  may be a provider the person enabled for sessions. That is Paseo's
  automatic default, and the setting is how a person leaves it.
- One call means a dead first match does not fall through. The given
  title and the fallbacks are the result. The stored list is the place
  a later decision would walk.
- The input cap drops source material Paseo would have sent (a whole
  prompt, a 120 000 character commit patch, a 200 000 character pull
  request patch). A long diff is named from its head.
- `vessel-workspace` gains a reader for `[metadata]`, in its own file:
  `project_file.rs` is already near the 500 line budget. No new
  dependency. The architecture table gains a phrase when that module
  lands, not an edge.
- User-facing docs (the README, the settings text) change with the
  slice that makes the setting real, not in this commit: a document
  that describes a call the binary does not have would disagree with
  the code.
