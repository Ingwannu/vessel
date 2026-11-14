# ADR 0062: Generated names

Status: proposed, 2026-11-14. Acceptance is recorded when review
passes. This commit is the contract only. The protocol number is the
one given at merge. This decision does not choose it. `PROTOCOL_VERSION`
stays at the base, 20, until that merge. New calls and new fields are
gated on one constant, `GENERATED_NAMES`, in `vessel-proto`: the
integrator sets that constant and no other line of this decision.
Nothing here generates a name yet, so the constant is not in the code
of this commit.

The plan is `docs/plans/generated-names.md`. It names the Paseo files
and lines this decision follows.

## Purpose and intent

Paseo asks a model, apart from the agent a person is talking to, for a
**workspace** title, a new worktree's branch name, a commit message,
and a pull request's title and body. It does not ask a model for an
agent's title. That title is only the explicit one or the first line of
the prompt, cut at 60 (`create-agent-title.ts` 1–48; the import path
uses the same pair, `agent-manager.ts` 694–698). A project key that
used to prompt for an agent title was removed in v0.1.96
(`paseo-config-schema.ts` 69–70). The model runs once per created
workspace, not once per agent: a directory workspace just created with
a prompt (`session.ts` 4522–4531), a `workspace.create` that carries a
first agent (`session.ts` 6921–6930), or a created worktree
(`session.ts` 7493, `scheduleForWorktree`). The write lands on the
workspace record (`workspace-auto-name.ts` 176–188), and only when that
record's title is empty or still the provisional line.

Vessel has no workspace title. The name on the board is the session's
(ADR 0022): a typed title, or the first line of the first prompt cut at
80. This decision generates **that session title**, which is not what
Paseo generates. The difference, the reason and the cost are under
"Why this and not the others". The other three texts follow Paseo's
triggers. A person's own words win, and a session's start does not
wait.

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

The threshold is `GENERATED_NAMES: Option<ProtocolVersion>`, assigned at
merge. `None` means no negotiated peer supports it; it is not a reserved
numeric sentinel. Every new field is
optional and skipped when absent, so a peer that ignores unknown JSON
fields keeps working. A peer below that constant is not sent the new
methods (`since` refuses them, as every method since protocol 5).
`node.set_settings` stays the method ADR 0038 added; a client sends
`metadata` only when the peer speaks `GENERATED_NAMES`, and only a node
that has that constant applies it. Absence of `metadata` on a settings
answer means this peer does not generate. An empty provider list,
present, means automatic. This decision adds that one top-level key and
no other. It does not list the keys other decisions add; those appear
on the settings type when they merge.

### Shared admission foundation

The purpose is to represent a protocol number that is not assigned while
preserving the existing numeric inspection API used by many callers.
The existing `Call::since()` table covers assigned method and field
versions. A numeric sentinel or the current version would pretend a
pending feature had an assigned version; changing every numeric API at
once would disrupt unrelated features. The choice is one additive
`Call::required_protocol() -> Option<ProtocolVersion>`, defaulting to
`Some(self.since())`; feature-specific pending calls return `None`.
All three fleet envelopes propagate the inner requirement. Client
admission runs before request ids and the writer queue, and server
admission runs before dispatch. A pending requirement is the typed
`CallError::UnassignedProtocol { method }`, with code `Unsupported`.
The benefit is an explicit state and one mechanism for every feature;
the cost is that admission must use the optional accessor rather than
numeric inspection. This foundation adds no pending call or title type;
its guards test pending and assigned states, and a feature adds its
end-to-end pending request test when registered. It opens no new network
path and uses the same pure-Rust refusal on Windows.

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

Which route, in order, the same order as Paseo's automatic list.
The match is case-insensitive, as Paseo's is on the model id and its
label (`structured-generation-providers.ts` 232–237). `ModelRoute` has
no label. The route name stands in for it, so the needle is looked for
in the model id, the `small_model` id and the route name.

1. A route whose model id, `small_model` id or route name contains
   `haiku`.
2. The same for `gpt-5.4-mini`, and the call asks for low effort.
3. The same for `minimax-m3`.
4. The same for `nemotron-3-super`.
5. The session's own route, its `small_model` when it has one, otherwise
   its model.

The first route that exists is the one call. Manual mode does not use
this list: it uses `metadata.providers`, in the stored order, and again
only the first entry whose route exists. A later entry that repeats the
same route, model and effort is dropped, as Paseo dedupes
(`dedupeProviders`, line 74). An entry's `model`, when set, must be
that route's model or its `small_model` (ADR 0015, decision 5);
anything else is a refused setting, not a call. An entry's `effort`,
when set, is a closed set (`minimal`, `low`, `medium`, `high`, `max`),
mapped in the node onto `vessel-llm`'s effort. A value outside that set
is refused on write, as an unknown `park_after` is. Paseo's manual
candidate with an explicit model on a top-level provider passes that
model and thinking string through unchanged
(`structured-generation-providers.ts` 136–160). When the model is
omitted it selects the default model, or the first model, and resolves
the thinking id against that model (162–169, 259–276); nested matches
also resolve it. Vessel instead validates the model against the route's
allowed pair and uses a closed effort type, so an unsupported choice can
be refused before saving. This is a deliberate security and cost bound,
not a reproduction of Paseo's explicit-model pass-through. An effort that
the provider's API cannot represent is omitted: Anthropic's wire effort is `low`, `medium`, `high` and
`max` (`wire/anthropic.rs` 376–385), so `minimal` is omitted on that
call and the provider's default stands. When the entry omits effort,
the provider's default stands, except the automatic `gpt-5.4-mini`
match, which asks for low.

No resolvable route: no call. The fallback below applies, and the node
logs the class (`no route`), never the prompt.

The model id sent upstream is the route's `small_model` when the entry
did not name a model and the route has a small model, otherwise the
route's model. Preferring `small_model` here is a Vessel cost choice.
Paseo's manual candidate without a model uses the default or first model
(`selectDefaultModel`, 259–261); its automatic path uses the model that
matched the substring, not a separate small-model field.

### Settings

`NodeSettings.metadata` and `SetNodeSettings.metadata`:

- `providers`: at most `MAX_METADATA_PROVIDERS` entries. Each is
  `{ route, model?, effort? }`.
  An empty list is automatic, the default, and what an older
  `settings.json` becomes. A non-empty list is manual: those entries
  only, nothing appended. An oversized list, an unknown route name, a model
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
Blank instructions are the same as omitting them. The file is read
from the disk at the cwd's repository root, which is what
`readPaseoConfig(repoRoot)` does (`build-metadata-prompt.ts` 45–57,
`paseo-config-file.ts` 284–289): a filesystem read, not a blob from
the base commit. A directory that is not a repository reads the file
at its root. A file that does not parse contributes no instructions;
generation continues with the defaults and the log names the existing
project-file error class, not the file's body. The functional contract
(what to return, the JSON shape, the limits, the refusal to follow
instructions inside the source) is not replaceable.

These four tables are this decision's. ADR 0066's table already names
them (`[metadata.title]`, `[metadata.branch]`, `[metadata.commit]`,
`[metadata.pull]`) and leaves them out of the command digest; that
track does not parse them. A legacy `agentTitle` key is ignored, as
0066's row says and as Paseo only keeps so an old file still parses.
The names differ from Paseo's `metadataGeneration.title`,
`branchName`, `commitMessage` and `pullRequest`, and 1 024 characters
is a bound Paseo does not have. Both are stated below.

The settings screen is not part of this change. It calls the methods
in the plan's "What a screen calls".

### Bounded value contracts

The protocol crate extends its existing parsed route/model identifiers and
`SessionTitle`; no provider SDK or new dependency is needed. A separate
`GeneratedTitle` preserves the model-output contract: its raw UTF-16 unit
count is checked against `MAX_GENERATED_TITLE_UNITS` before trimming or
ordinary title validation. Paseo checks its schema before trimming too
(`worktree-branch-name-generator.ts` 39–42, 132–135). Vessel additionally
keeps ADR 0022's one-line and control-character refusals; allowing a model
to bypass the title rules would make stored and user-supplied titles
disagree. A bad model title is refused, never truncated.

`MetadataSettings` owns its private ordered candidate list and one
constructor. The settings and candidate objects refuse unknown keys, as
Paseo's persisted schemas do (`persisted-config.ts` 172–186), so a misspelled
preference cannot silently choose automatic defaults. Its decoder stops
constructing candidates at
`MAX_METADATA_PROVIDERS` and refuses an extra element, rather than
allocating an unbounded list before checking its length. Duplicate entries
count towards that storage bound and stay in order; selection deduplicates
later. The cap is a Vessel bound, absent from Paseo's provider array.
Future effort values decode as typed `Unknown` for reading and are refused
by settings validation when the setting is wired. This value-only slice
adds neither settings fields nor registered calls: availability checks and
persistent writes belong to the following settings slice.

### Project style reader

The style reader reuses the existing `ProjectFile::read_in` file-opening
policy through one shared byte reader: regular file, contained path and
`MAX_BYTES`, with ADR 0027's same-user check/open race unchanged. It must
not parse or run commands, alter their approval digest, walk ancestors or
claim a stronger sandbox. It blocks; the node calls it through
`spawn_blocking` when generation is wired. Each known table is decoded
independently, so one malformed style does not erase valid siblings.
Unknown project keys, including the legacy key, remain ignored.

A nonblank instruction is limited by `MAX_STYLE_CHARS` Unicode scalar
values before trimming; padding cannot repair an oversized instruction.
Blank instructions select the default style. The cap is a Vessel cost
bound, absent from Paseo's style schema. Errors retain typed parser causes
inside this edge but only table and failure classes are logged, never
project text; style values redact their Debug representation too.

Root resolution is a stated difference when the node wires this reader.
Vessel will use the actual checkout root, or the session's working folder
outside Git, consistently with its existing project-file policy. Paseo's
`resolveRepoRoot` prefers the main repository for its own worktrees and
throws outside Git (`workspace-git-service.ts` 924–933); its prompt helper
then skips overrides (`build-metadata-prompt.ts` 45–57). Keeping a worktree's
own checked-out `vessel.toml` avoids reading another checkout's policy.
A non-repository folder can already contain Vessel project instructions;
only the style is consumed, with the output and tool-safety contract fixed.
The reader itself accepts an already resolved folder; it does not implement
or verify the node's later root-selection path.

### A session title's origin

`TitleOrigin` is `typed`, `given`, `generated`, or `unknown`.

- `typed`: a person supplied `SessionSpec.title`, a schedule name the
  person wrote, or the last `session.rename`. Generation does not replace
  it, even when the text equals
  the prompt's first line.
- `given`: Vessel derived it by cutting text (`SessionTitle::from_prompt`),
  including the first prompt's line (ADR 0022), a side chat's
  inherited label and a fork's derived title. Only the given title of a
  **new structured session that had no
  `spec.title`** is eligible to be replaced, and only while it is still
  that title.
- `generated`: this generator's title, after it parsed.
- `unknown`: an imported specification whose author cannot be attested,
  or the field was absent (a record from before `GENERATED_NAMES`,
  or a newer origin). Generation does not replace an unknown title. A
  person's old typed title and an old prompt line cannot be told apart,
  so neither is overwritten.

The field is `SessionInfo.title_origin` and `SessionEvent::Renamed.origin`,
both optional. Absent decodes as `unknown`. The durable log keeps known
origins without depending on this build's protocol number. At the wire
edge, both `SessionInfo::for_protocol` and `SessionEvent::for_protocol`
replace the origin with `unknown` for a peer below the assigned threshold,
or while it is `None`; serialization then omits the unknown field. A
`renamed` event stays protocol 9 because its title remains readable.
An older fleet hop decodes and re-encodes a typed `Forwarded.reply`
(`forward.rs`), so it can discard additive reply fields too. The next
reader sees `unknown`, never an inferred person or generated title.
Protocol 20's envelope rule protects request fields; it does not preserve
unknown reply fields. Tests cover old-shaped decoding and re-encoding,
and the same adaptation with unassigned and synthetic assigned thresholds.
A mixed-version binary fleet remains a manual verification.

A fresh structured runtime can arm generation after it has been created;
creation has no prompt, so the task waits for the first actual
`AgentEvent::TurnStarted` that produces its given title. Arming does not
hold create or prove that the wire acknowledgement has arrived first.
Terminal sessions, forks, rewinds, imports and resumes are excluded; node
wiring chooses fresh paths explicitly rather than treating every internal
create as fresh.

An external read followed by an ordinary rename is racy. A generated
proposal therefore carries the expected title to the session actor. The
actor compares its live, structured, non-resuming session's given origin,
exact title and absent explicit specification title, then publishes the
rename in that same serialized order. A typed rename, even to identical
words, wins. A successful proposal returns `Applied`; a mismatch returns
`Kept` and publishes nothing. No watch borrow crosses the publish await.

The existing rename API is live-only, and the actor's final status must
remain its last record. A session that ends before the proposal is
committed keeps its given title; the task is cancelled. This differs from
Paseo's persistent workspace title and preserves Vessel's actor lifecycle
and final-log rule instead of adding a second writer. The existing
`ModelCall` API is also best effort while live: an ended actor cannot
record a late cancelled attempt. Such an outcome is classified in the node
log without content, rather than claimed as a durable usage record.
Actual provider billing after cancellation is not known by the node.

A generated title satisfies `MAX_GENERATED_TITLE_UNITS` raw UTF-16 units
before trimming, then the ordinary `SessionTitle` parser and
`SessionTitle::MAX_CHARS`. The branch, when one is asked for in the same call, must parse as
a `BranchName` and as Paseo's slug: lower-case letters, digits, hyphens
and slashes, no leading or trailing hyphen, no `--`, at most 100
characters. The branch is not the title rewritten into a slug. When the session's
title and the worktree's branch are both eligible, one call returns
both fields. When only one is eligible, the call asks for that one, so
a title never costs a second call and a branch never costs a second
call. A field that does not parse refuses the whole answer of that
call: the given title stays, and the placeholder branch stays. There is
no second call to repair it. Paseo's provisional line is cut at 60 and
an explicit title may be 200 (`create-agent-title.ts` 3,
`agent-title-limits.ts` 1). Those stay 80 and 120 here, because ADR
0022 already published them. Moving the provisional cut would change
how new sessions are named; it would not invalidate an already stored
title. Increasing the typed bound would admit titles that older peers
refuse, including a title of 121 characters. This feature preserves both
existing rules.
The existing constructors count Unicode scalar values and retain inner
prompt whitespace (control characters become spaces). Paseo collapses
that whitespace and its provisional clamp uses UTF-16 code units
(`create-agent-title.ts` 4–18). This decision preserves ADR 0022's
constructors rather than renormalizing a session's current title.


### Branch names

`session.create` does not make a branch. `SessionPlace::Worktree` names
a worktree that already exists (`worktree.rs` 385–389). The placeholder
is made by `worktree.create` (protocol 10) when `WorktreeBranch::New`
or `Snapshot` omits the name. `choose` (`worktrees/start.rs`) then
calls `branch_for` (line 118): `vessel/` plus the worktree id's short
hex, stored as `created_branch`.

The stored worktree gains `branch_auto_name`, absent when the person
named the branch or the branch already existed:

- `pending { placeholder }`, written in the same save as the new
  worktree. `placeholder` is that `created_branch`.
- `attempted { placeholder }`, written **before** the model is called,
  including when the branch checked out now is already not the
  placeholder (then there is no call for the branch and no rename).
  A restart therefore tries a worktree at most once, as Paseo marks
  `attempted` before it generates (`paseo-worktree-service.ts`
  211–218).

The seed is the first prompt of the first structured session whose
place is that worktree and whose prompt has text. A generated name
equal to the placeholder is refused (`paseo-worktree-service.ts` 226).
The rename then goes through `git.rename_branch`, which updates the
stored `branch` and `created_branch`. That write is not pushed. Paseo
does notify: `notifyGitMutation(..., "rename-branch")` and
`emitWorkspaceUpdateForCwd` (`workspace-auto-name.ts` 149–152). A
client here sees the new name the next time it reads the worktree.
The session title, by contrast, is pushed: `renamed` is the event a
follower already applies. No new event is added for the branch.

If the desired name exists, the node tries `name-2` through `name-50`,
skipping a candidate equal to the placeholder, and keeps the placeholder
when every one exists or fails the slug. The check and the rename are
repeated against the branch checked out now: a rename that happened
while the model ran wins, and the generated name is dropped.

### Commit message and pull-request text

Two calls, both `operate`, `since` equal to `GENERATED_NAMES` (the
number given at merge). They do not commit and do not open a pull
request.

- `text.commit { root }` answers `{ message, source }`.
- `text.pull { root, base? }` answers `{ title, body, source }`.

`source` is `generated` or `fallback`. `root` is the workspace root the
git calls already take. `base` is the branch `pr.open` already takes;
absent means the project's default branch, as there. The diff is the
uncommitted diff for a commit message, and the diff against `base` for
a pull request. An empty diff is still one call: the source text says
there is no diff, as Paseo's generator sends `(No diff available)`
(`git-metadata-generator.ts`, the shared prompt builder).

The caller then sends the text to `git.commit` or `pr.open` as it does
today. Those two calls do not grow an optional message and do not call
a model. The CLI commands of slice 4 are thin wrappers of these two
calls; their names are settled with the calls, in that slice.

Bounds on a generated answer, all refusals rather than cuts:

| Text | Bound | Fallback |
| --- | --- | --- |
| Title | nonblank line, `MAX_GENERATED_TITLE_UNITS` raw UTF-16 units, then `SessionTitle` | the given title stays |
| Branch | the slug above, and a `BranchName` | the placeholder stays |
| Commit message | one line, 1 to 72 characters, a `CommitMessage` | `Update files` |
| Pull-request title | one line, 1 to 72 characters, a `PullTitle` | `Update changes` |
| Pull-request body | 1 to 4 096 characters, no NUL, a `PullBody` | `Opened from this branch.` |

The commit fallback and the pull-request title fallback are Paseo's
words. The body fallback is not: Paseo's body names Paseo. A fallback
answer has `source: fallback` and a reason from the refusal enum, so a
screen can say the words were not the model's. The RPC itself succeeds:
the caller always has text it may use. A refusal that is not a bad
answer (no route, the upstream failed, the clock fired) uses the same
fallback.

### Refusal

A generation ends as one of: `generated`, or `fallback` with
`no_route`, `no_source`, `timed_out`, `upstream`, or `unusable`.
`upstream` carries the classified `ModelFailure` and no provider body.
`unusable` means the completion was not the JSON object, or a field
failed its bound. Nothing is dropped without that class in the log.
The log and the `ModelCall` record carry the route, the model, the
token counts, the stop, the failure class and the duration. They do
not carry the prompt, the diff, the completion, or a key (ADR 0015,
decision 10). A title or branch generation publishes a `ModelCall` through
that session's live actor. The live-only and bounded-channel recording
limits above still apply; a record after the actor ended is not promised. The record's route is the route the generation called,
which may not be the route the session was launched with. A
`text.commit` or `text.pull` names a root, not a session, so it does not
append a `ModelCall`; the answer's `source` is what the caller has, and
the node log has the class, the route and the duration. No session id
is added to those two calls: they name a root, as `git.commit` does.

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

## What leaves the node

The completion leaves the node only by the path ADR 0015 already
allows: `vessel-llm`'s client, to the base URL of the one route
`models.toml` names, with redirects off and no proxy. The credential
is that route's key, the one `secrets.toml` names, held by the node for
the length of the call. It is not put in the prompt, the log, the
`ModelCall` record, or the session's environment. A missing route, a
model the route does not allow, a key the client refuses as not
distinctive, and the 20 second deadline each refuse the call. The test
arm of the generator's enum is compiled only under `cfg(test)`. A
release build has no fake that answers.

No signal handler, systemd unit or launchd job is started. The wait is
the HTTP client's deadline, the same call on Windows as on any other
system. There is no Windows-only refusal because there is no
Unix-only mechanism.

## Why this and not the others

The route and `vessel-llm` are the only path that already holds a key
and already refuses to leak it. One call is the bound that keeps a
title from becoming a second conversation. Recording origin is what
makes "a person's title wins" a type instead of a comparison with the
prompt line, which would replace a typed title that happens to match.
Leaving `git.commit` and `pr.open` alone keeps every current caller
working on protocol 10 and 11. Optional fields, not a new `since` on
`renamed`, keep a peer of 9 able to read a rename.

The generated name is put on the session, not on a workspace, because
the board reads `SessionInfo.title` and Vessel has no other title for
a place. That is the opposite of Paseo's write. The cost is how often
the model runs: one call per new structured session that had no
`spec.title`, against Paseo's one call per created workspace. Four
such sessions in one worktree are four calls here and one there. A day
that opens twenty is twenty calls, each still one route, 16 384
characters and 20 seconds. A session in a place that already exists
still costs a call here, and costs Paseo nothing.

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
- A given title can be 20 characters longer than Paseo's provisional
  line (80 against 60) before a generation replaces it, and a person
  cannot type the 200 characters Paseo allows (the typed bound stays
  120).
- A branch rename is not pushed. A screen that does not read the
  worktree again keeps showing the placeholder. The session title is
  pushed, as `renamed`, which is the event that screen already applies.
- `vessel-workspace` gains a reader for `[metadata]`, in its own file:
  `project_file.rs` is already near the 500 line budget. No new
  dependency. The architecture table gains a phrase when that module
  lands, not an edge.
- User-facing docs (the README, the settings text) change with the
  slice that makes the setting real, not in this commit: a document
  that describes a call the binary does not have would disagree with
  the code.
