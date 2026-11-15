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

The method table represents unassigned protocol numbers explicitly. On the
Quality B foundation, both `Method::since()` and `Call::since()` return
`Option<ProtocolVersion>`; `required_protocol()` delegates to the latter.
The metadata field contributes `GENERATED_NAMES` to the existing settings
call, retaining its older fields' requirements. An unassigned contribution
refuses the whole patch; assigned contributions combine with those fields
and the method's version. There is no new method in the session-title package,
so it adds no pending method row. Later new calls use the shared table's
`[Scope, (pending GENERATED_NAMES), shape]` form until integration assigns it.

All three fleet envelopes propagate the inner requirement. Client admission
runs before request ids and the writer queue, and server admission before
dispatch. A pending requirement is `CallError::UnassignedProtocol { method }`,
with code `Unsupported`. The original shared-admission checkpoint introduced
the optional accessor while keeping numeric inspection; Quality B now makes
inspection optional too, with one method table. This package uses that head's
contract, never a second admission rule or numeric sentinel. Existing assigned
methods retain their versions. The pure-Rust refusal opens no network path
and has the same behavior on Windows.

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
call and the provider's default stands. Chat's `max` is likewise omitted
rather than downgraded to `high`; a provider configured without reasoning
fields receives no effort. These omissions apply only to metadata, and do
not change proxy translation. When the entry omits effort,
the provider's default stands, except the automatic `gpt-5.4-mini`
match, which asks for low.

No resolvable route: no call. The fallback below applies, and the node
logs the class (`no route`), never the prompt.

For a manual entry without a model, or the session-route fallback,
the model id sent upstream is the route's `small_model` when it has one,
otherwise its primary model. Preferring `small_model` here is a Vessel
cost choice. An automatic model-id substring match instead keeps the
actual matching model id, primary before small when both match. It must
not substitute an unrelated small model after matching the primary id.
A route-name-only alias match uses the same small/primary cost choice.
Paseo's manual candidate without a model uses the default or first model
(`selectDefaultModel`, 262–264); its automatic path uses the model that
matched the substring, not a separate small-model field.

### Route selection and staging

The node reuses its parsed `Routes` snapshot and the bounded metadata
settings. No provider SDK, discovery request, credential read or model
request is needed to select a tuple. Paseo scans the supplied provider
and model arrays (`structured-generation-providers.ts` 232–249). Vessel
uses the catalog's deterministic route-name order for equal-priority
matches; the primary/small ids take the place of that model array, and a
route name remains the stated label substitute. This is a deliberate
difference for a declared `models.toml` catalog, not a claim to reproduce
provider discovery ordering. The hint order itself is unchanged.

Manual selection never appends automatic choices. On reload, a vanished
route may be skipped for the next stored manual entry, but an existing
selected route with a now-disallowed explicit model or unknown effort is
refused, not silently repaired or sent to another provider. Stored
duplicates cannot change the first resolvable tuple and never authorize a
second request. Selection is not settings validation: saving still needs
the later all-candidate checks, including the actual encoded API budget.
The session fallback supplies a route, not harness-specific thinking
options: absent a metadata preference or automatic hint, its provider
default stands. Paseo can carry the current selection's thinking id
(`structured-generation-providers.ts` 95–135). Vessel keeps metadata effort
explicit at this direct model-route seam; guessing from agent-specific
option keys in the node would cross the harness ownership boundary.

Before wiring, the node selector module and its borrowed catalog iterator
are compiled only for tests. The wiring commit after the shared accessor
removes both attributes and supplies the real caller; no public API or
lint suppression is added to keep unused product code. Tests use the real
catalog parser and typed settings with synthetic endpoints, opening no
socket and reading no credential. The selector alone does not spend.

### Strict complete-response decoding

The model API crate owns response grammar. It will reuse its existing
Anthropic Message and Responses Response types and add the missing whole
Chat completion shape beside that API's existing wire types. A bounded
complete-response decoder returns text, reported usage and a known stop;
the node alone interprets the returned text as the generated-name JSON.
No provider response structs or string-based API parsing live in the node.

The existing streaming decoders deliberately tolerate missing or unknown
end reasons for proxy compatibility. They cannot alone prove that a
structured metadata answer completed normally. Changing their policy would
change unrelated harness traffic; parsing all streamed events a second time
would add another state machine. Instead, metadata uses the existing typed
request encoders with streaming off and buffers one small, bounded reply.
There is no metadata token stream shown to a person, so this loses no
visible progress. Existing proxy decoder behavior remains unchanged.

`MAX_COMPLETION_BYTES` is the one response-byte bound, shared with the
caller's HTTP limit. A normal explicit end/status is required; missing,
unknown or incomplete endings, declared errors, tool/refusal content,
non-text output and multiple Chat choices are unusable. Successfully
decoded API envelopes retain reported usage and known stops when output is
refused, rather than fabricating an end turn. A body that cannot decode as
its API has unknown counts and stop; partial fields are not inferred. A contradictory
tool/refusal block makes the text unusable but does not overwrite the
provider's reported stop. The typed text error carries that refusal. Credentials are checked on both the raw envelope and decoded
text with the existing Redactor; node schema decoding checks the final
value too. Completion text and errors have safe Debug output. JSON causes
stay typed behind a safe diagnostic wrapper because serde messages may
quote provider content; only category, never quoted text, crosses it.
This decoder reads no file, opens no socket and adds no dependency.

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
constructor. In this value-only slice, `MetadataSettings` (through
`RawMetadataSettings`) and `MetadataProvider` are the strict wire/patch
forms: unknown keys are refused at the decoding edge, so a misspelled
preference cannot silently choose automatic defaults. This reproduces
Paseo's strict persisted schemas (`persisted-config.ts` 172–186) at the
patch boundary only. These strict deserializers must not be used to read
`settings.json`. Its decoder stops
constructing candidates at
`MAX_METADATA_PROVIDERS` and refuses an extra element, rather than
allocating an unbounded list before checking its length. Duplicate entries
count towards that storage bound and stay in order; selection deduplicates
later. The cap is a Vessel bound, absent from Paseo's provider array.
Future effort values decode as typed `Unknown` for reading and are refused
by settings validation when the setting is wired. This value-only slice
adds neither settings fields nor registered calls: availability checks and
persistent writes belong to the following settings slice.

The settings wiring slice adds a separate lenient file view, including its
nested metadata and candidate objects. Unknown keys are ignored at every
level while known values are retained; known invalid values still fail
validation, and a later effort reads as `MetadataEffort::Unknown`. The
file view converts through the same bounded constructor and parsed ids,
not through the strict wire deserializers. This is a deliberate difference
from Paseo's strict file schema, required by Vessel's existing settings
compatibility rule: a newer candidate key must not reset unrelated node
settings. No metadata-specific keeping mechanism is added. The shared
settings store owns the `settings.json.later-<UTC>` copy before the first
replacing write when unread fields exist, and the
`settings.json.unreadable-<UTC>` move for an unparseable file.

That slice owes file tests with an unknown metadata key and an unknown
nested candidate key, checking that the route and other node settings
survive read, an unrelated patch and reopen. It also checks a future effort
reading as Unknown, known invalid values still being refused, and strict
wire patches refusing those same unknown keys before write or model work.
The value-only slice has no file reader and does not claim these tests.

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

A session without a given title is left unchanged by design. The task
waits for the accepted turn text that produces that given title; it does
not synthesize an expected title or fill an empty one directly.

An external read followed by an ordinary rename is racy. A generated
proposal therefore carries the expected title to the session actor. The
actor compares its live, unparked, structured, non-resuming session's given origin,
exact title and absent explicit specification title, then publishes the
rename in that same serialized order. A typed rename, even to identical
words, wins. A successful proposal returns `Applied`; a mismatch returns
`Kept` and publishes nothing. No watch borrow crosses the publish await.

The actor also refuses a generated proposal when the current session is
parked or its origin is Imported. Task cancellation alone cannot decide a
proposal already queued with the actor; these guards keep a late proposal
from changing a parked/imported session. A fresh Remote origin is allowed:
forwarding a new create is not an import. The origin match is exhaustive,
so another origin must decide the rule. Ordinary manual renames keep their
existing behavior. Node arming still excludes imports and cancels on park;
the actor check is a second defense, not a replacement for that routing.

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

### Project-file whitespace policy

The project's `vessel.toml` instructions keep Rust `str::trim`, consistent
with Vessel's own project-file text handling. Paseo uses JavaScript trim;
their Unicode whitespace sets differ. The shared JavaScript-style trim
helper, if present, is not applied to this own-format reader. This is a
stated format choice, not a claim of identical whitespace classification.


### Shared request options before complete requests

[Decision Log]
- Purpose and intent: keep provider quirks identical for proxy traffic and
  the later bounded metadata request.
- Existing implementation and constraints: the proxy assembles three
  public upstream option structs and serializes their typed requests.
  The complete response decoder already lives in vessel-llm; no second
  provider grammar belongs in the node.
- Alternatives considered: duplicate the proxy option match in metadata,
  or introduce an SDK. Both would create a second configuration policy.
- Choice: move option assembly to a Provider method and add one closed
  upstream Options enum whose streamed encoder delegates to the existing
  encoders. This extraction changes neither flags nor request bytes.
- Why this choice: the node still owns its catalog; vessel-llm still owns
  API encoding. Existing serde and Bytes suffice; no dependency is added.
- Effects: the proxy shrinks. Complete-request policy will be a separate
  change, with actual encoded-budget checks rather than proxy changes.


### Bounded complete request preparation

[Decision Log]
- Purpose and intent: send a bounded whole-response request through the
  same typed encoders without inheriting streaming flags or a raised cap.
- Existing constraints: upstream encoders select streaming and Anthropic's
  budgeted thinking may raise max_tokens. Checking only the neutral cap
  would not bound the actual provider request.
- Alternatives considered: hand-write a second request format, or lower a
  thinking budget after encoding. Both create a second API policy.
- Choice: the LLM owner takes an owned neutral request and a positive
  OutputTokens value, applies the stated metadata effort omissions, sets
  the cap, delegates to the existing encoder, then verifies the encoded
  cap before serialization. It switches stream off and removes Chat's
  stream_options. A raised or missing encoded cap is a typed refusal.
- Why this choice: the actual request is the cost boundary. Existing
  encoders, serde and NonZeroU32 suffice; there is no new dependency.
- Effects: budgeted thinking that cannot fit is refused, without another
  route or retry. This is a Vessel cost limit; Paseo's structured call
  (worktree-branch-name-generator.ts 114–135) does not specify this cap.
  No request is sent and no node setting changes in this library slice.

### Title material and semantic result edge

[Decision Log]
- Purpose and intent: isolate bounded source material and the generated
  title JSON from routing, HTTP, credentials and actor mutation.
- Existing constraints: GeneratedTitle already checks raw UTF-16 units
  before the ordinary title constructor. CompletionText and JsonFailure
  already seal provider text and decoder diagnostics. ProjectStyles owns
  checked replacement instructions. These types are reused.
- Alternatives considered: accept an unbounded prompt string in the
  transport, decode into a generic JSON tree, or copy Paseo's prompt.
- Choice: capture at most MAX_SOURCE_CHARS Unicode scalars in a redacted
  material type, remember whether it was cut, and construct an original
  tool-free title-only prompt. Checked project style replaces only the
  style section. The shared output limit constant formats instructions.
- Why this choice: the source cap applies before encoding and cannot be
  bypassed by prompt assembly. A blank captured prefix yields no source;
  the observer must not substitute a later turn. Paseo's style replacement
  is preserved (build-metadata-prompt.ts 35–43); the bounded source and
  direct JSON prompt are Vessel's cost and integration choices.
- Effects: the node decodes title JSON once with a typed object, seals its
  serde cause, uses GeneratedTitle, and checks the final title for keys.
  A nonempty whitespace-only title becomes no title, like Paseo's trim
  fallback (worktree-branch-name-generator.ts 132). Empty raw text, invalid
  or oversized text is refused. An escaped lone surrogate is refused by
  serde before the title constructor. Rust title trimming remains the
  existing constructor's rule. Unknown result keys are ignored, as the
  reference object schema does. Branch output is not requested or applied
  in this title-only slice; the branch slice still owes independent use
  of a valid branch when a title is blank.

The node material/prompt/result module is staged under cfg(test) until
shared-accessor wiring supplies the real caller. This edge performs no
file or socket I/O, discovers no route and mutates no session.

### One bounded completion transport

[Decision Log]
- Purpose and intent: bound the one metadata HTTP exchange independently
  of the proxy's long stream budgets and retries.
- Existing implementation and constraints: vessel-llm Client already owns
  TLS, the declared endpoint, credential headers, redirect/proxy refusal,
  response-body bounds and Redactor. Limits are fixed when it is built.
  Reusing the proxy instance would inherit its longer bounds and retries.
- Alternatives considered: another HTTP stack, changing proxy limits, or
  a trait with one production implementation. All introduce a second
  transport policy or change unrelated model traffic.
- Choice: the node constructs one dedicated existing Client with the named
  metadata policy and retains its pool. A private wrapper accepts only
  that policy, adds the total title-call timeout around send and body,
  then delegates whole-response decoding to vessel-llm. Retries are off.
- Why this choice: the same existing transport enforces the byte bound
  while reading, before complete decoding. Endpoint and credential are
  passed in by the later route/permission owner, never discovered here.
- Effects: provider Failure is retained typed behind a sealed diagnostic
  wrapper because its message may quote input; only its class is exposed.
  Deadline keeps its typed elapsed cause. Complete decode results keep
  reported usage and stop even when content is unusable. The only test
  alternatives are cfg(test) recorded and stalled arms; product builds
  cannot answer with a fake. Tests perform no network or real-key access.

This client module remains staged under the generation module's cfg(test)
until shared-accessor wiring installs its production caller. It does not
choose a route, read a key, acquire session permits, record a ModelCall or
apply a title; the later owner must do those operations in that order.

### Reuse initialized TLS trust for a separate bounded pool

[Decision Log]
- Purpose and intent: construct the metadata pool once without repeating
  operating-system certificate reads or retaining proxy connect timeouts.
- Existing constraints: cloning vessel-llm Client shares its HTTP pool and
  the connect bound fixed by its builder. Its current constructors already
  load and configure trust correctly. Re-running root loading inside each
  generation, or a cancelable lazy initializer, can duplicate blocking work.
- Alternatives considered: clone the proxy unchanged, load roots again,
  or add another TLS/root cache. None provides both the separate policy
  and reuse of already initialized trust.
- Choice: first move existing constructors unchanged into their own client
  build module. Separately retain their immutable TLS configuration and
  add with_limits: build a distinct existing HTTP pool with those same
  roots and the requested limits. Ordinary Client cloning stays unchanged.
- Why this choice: the TLS owner still owns every TLS decision and the node
  can construct its one metadata client from the initialized proxy client.
  Standard Arc and existing dependencies suffice; no root file is reread.
- Effects: roots remain fixed for that client's lifetime, as before. A
  certificate change requires the owner to rebuild it. Tests check shared
  trust identity and independent policies without opening a socket. No
  generation, route or credential discovery is added by the factory.

### Prepared title requests and settings preflight

[Decision Log]
- Purpose and intent: make the bounded encoder the only product constructor
  of a metadata request that can reach the transport, and validate every
  manual setting before it can be saved.
- Existing constraints: route selection checks one tuple; writing needs
  all candidates checked. The text edge owns neutral defaults and effort
  mapping, while the LLM encoder owns the actual provider cap check.
- Alternatives considered: duplicate those checks in Settings, send a
  probe to the provider, or let the client take arbitrary bytes.
- Choice: reuse one candidate checker and neutral request constructor.
  One encoder produces a redacted Prepared body with its API identity.
  Actual title preparation supplies captured material and style. Pure
  settings validation uses an unsent empty conversation to check the same
  model/effort/output policy, with no route, key or capability discovery.
- Why this choice: a route's declared flags and allowed model pair can be
  checked without spending. The positive title-output budget is named in
  this owner and converted to OutputTokens before encoding. An API mismatch
  at the transport boundary is refused before any call.
- Effects: empty automatic settings remain valid even without routes;
  every manual entry, including a later entry, must resolve and encode.
  Reload-time selection still skips vanished entries as already stated.
  A typed preflight refusal leaves file and active settings untouched.
  This validates declared local configuration, not remote model availability.
  No setting is registered or saved in this staged preparation slice.

### Shared recording, typed credentials and live admission

[Decision Log]
- Purpose and intent: share existing model accounting and credential policy
  while ensuring an ended task cannot recreate its retired permit entry.
- Existing constraints: the proxy owns neutral usage/stop/failure mappings;
  Models::key currently turns read/join errors into strings. Permits are
  keyed by session id and retirement removes that entry.
- Alternatives considered: copy the record mappings, read credentials in
  generation, or check liveness outside the permit-map lock. Copies drift;
  a separate key path can bypass policy; an external check races retirement.
- Choice: move the existing accounting mappings unchanged to their own
  owner. Keep one Models key reader with typed, sealed read/task causes,
  preserving missing/plain-key diagnostics at the proxy boundary. Admit
  through the original session handle with its live check inside the
  existing permit-map lock; the same node/session semaphores are reused.
- Why this choice: no second writer, credential store, pool or lock crosses
  an await. Existing proxy traffic keeps its policy and known counts.
- Effects: generation records only actual attempts. Known reported stops
  and usage survive semantic title refusal; unavailable/invalid preparation
  makes no ModelCall. Unknown finality uses the existing error/truncation
  representation, never a fabricated normal ending. Records remain best
  effort while live, as already stated; a final status stays the last record.

### Live first-input controller

[Decision Log]
- Purpose and intent: connect the checked preparation, bounded transport and
  actor proposal without delaying session creation or replacing a person's title.
- Existing constraints: the runtime already publishes the first titleable text
  and its Given rename in order, provides an atomic cached/live follower and
  owns title eligibility. Models already owns live call permits and credentials.
- Alternatives considered: use later prompts after losing the first event,
  inspect harness transcripts, duplicate the actor guard in the node, or make
  creation await generation. Each weakens either provenance, privacy or latency.
- Choice: arm only the external fresh structured creation path. Follow from
  the beginning atomically, capture the first titleable text block once, and
  wait for its Given title. Gaps, lag and a blank bounded prefix end the task;
  they never substitute later text. The handle exposes the actor's existing
  eligibility predicate for early cancellation; the actor remains the final
  compare-and-publish authority. Untitled sessions are left alone.
- Why this choice: cached events cover input racing creation. The original
  handle stays with the task, so a resumed run sharing its id cannot inherit
  old work. Info changes cancel outstanding work when ownership is lost,
  parking occurs or the run ends; finishing an agent turn alone does not.
- Effects: one dedicated HTTP pool reuses initialized trust. Shared permits
  bound preparation and calls; no generation task retires a session. One
  actual attempt emits existing best-effort ModelCall accounting, including
  known usage and stops on semantic refusal. Cancellation can leave unknown
  usage, recorded as truncation while live; no record follows final status.
  Project styles resolve through the existing checked Git runner, falling
  back to the working folder outside Git. Files run on blocking workers.
  This differs from Paseo's main-repository style root as already stated.
  No branch, commit or pull generation is enabled by this package.


Cancellation observes bounded event history as well as the current info.
A watch can coalesce a parked/woken pair; that pair must still cancel the
original task. A gap or lag is conservative cancellation, never permission
to take another source. Ready cancellation is polled before preparation.
The actor's current eligibility check remains the final mutation authority.


The settings reply carries optional metadata as a capability distinction:
Some(empty providers) is supported Automatic, and None is unsupported.
Per-peer projection clears the option while unassigned or below the threshold;
it never hides Automatic from a supported peer. Runtime tasks read the settings
owner's policy directly, independently of that wire projection. The nullable
lenient view is shared by file and reply decoding; patches remain strict.

### Physical preparation outlives cancellation

[Decision Log]
- Purpose and intent: cancelling an obsolete title must suppress its result
  without admitting replacement file work before the original read finishes.
- Existing constraints: dropping a `spawn_blocking` join handle does not stop
  its closure. The first controller held model permits only in the async
  caller, and subscription forwarders have a bounded shutdown grace. Neither
  proves physical preparation has ended.
- Alternatives considered: abort a join handle, release capacity on logical
  cancellation, allocate a separate quota, or duplicate the secret reader.
  These respectively fail to stop running file work, over-admit replacements,
  split the existing budget, or split credential policy.
- Choice: share the original session/node admission through an owned lease.
  Borrow a token from the node's hard-drained task tracker, already held by
  Sessions. The style closure retains that lease and is tracked until actual
  completion. A non-aborted tracked key operation retains the same lease
  while awaiting the existing `Models::key` worker to its real completion.
  Dropping the caller's join handle discards the result; it does not abort
  this settling operation. No second secret reader or quota is introduced.
- Why this choice: capacity and shutdown now follow physical completion,
  independently of the original title future. The existing key policy and
  sealed failures remain the credential boundary. HTTP and actor application
  stay in the cancellable caller, so a late file result cannot start a call
  or overwrite a title.
- Effects: ordinary shutdown waits for already admitted file work to settle.
  The HTTP deadline is not claimed to stop filesystem work. Held-real-closure
  tests must cover style and keyed secret reads, actual title/park/node
  cancellation, retained capacity and drain, saturation and no late effects.
  This adds no wire field, call, dependency, model attempt or provider stack.


Generated titles retain SessionTitle's existing normalization, including
Rust's whitespace class, after the raw UTF-16 check. Paseo trims its model
result with JavaScript's whitespace class instead. This stated difference
keeps one canonical title constructor for typed, provisional and generated
origins; the source/style contracts do not claim identical Unicode trimming.

### Workspace commit and pull-request generation

[Decision Log]
- Purpose and intent: provide one Models-owned structured-generation boundary
  for the authorized-root Git caller, without borrowing a session's identity,
  admission or accounting. The session title producer remains private and keeps
  its single attempt. This purpose-specific rule supersedes the earlier
  single-call sketch for future commit and pull-request consumers only.
- Existing implementation and constraints: Models already owns the immutable
  route catalog, provider options, credential reader, initialized TLS client,
  node permits and neutral usage/failure conversions. The reviewed complete
  request/reply machinery owns API grammar. Session ModelCall publication and
  the session usage ledger cannot truthfully attribute a workspace operation.
  The root caller separately owns the checked diff, project instructions,
  original-root revalidation, mutation reservation and product fallback text.
- Alternatives considered: a temporary or borrowed session, widening the title
  driver's private modules, a second provider/credential stack, or returning a
  fallback without running a real generation boundary. Each misstates ownership
  or conceals missing functionality. A callback alone also loses cancellation
  accounting when the requesting future disappears.
- Choice: a separate workspace purpose under the same Models owner, with typed
  original operation/root attribution, bounded prompt and output values, a
  frozen candidate catalog, the existing key reader and HTTP stack, and the
  existing node permit. No SessionId, SessionHandle or per-session permit is
  accepted or manufactured. Root identifiers are correlation facts, not file
  authority; this producer neither opens a repository nor executes Git.
- Candidate policy: Manual uses only its configured entries in order. Automatic
  uses the existing four model hints and an optional explicit current model
  selection; absence of a selection never borrows a session. Resolve against
  one catalog, preserve the existing allowed-model/effort checks, and dedupe
  equal provider/model/effort tuples by their first occurrence. No discovery or
  automatic append to Manual is introduced. The title selector is unchanged.
- Attempt policy: an initial call plus at most two JSON/schema corrections per
  candidate, followed by the next eligible candidate. The limit is one named
  constant at the policy type and appears through that constant in diagnostics
  and tests. Transport retries remain disabled. Transport/finality failures
  move to another candidate, not another same-candidate HTTP attempt. Correction
  prompts retain the base prompt and safe schema diagnostics, never raw failed
  output, credential material or parser text.
- Schema policy: commit message and pull title are raw nonempty strings bounded
  by the reference's maximum of 72 UTF-16 units; the pull body is a raw nonempty
  string within the complete-response envelope bound. Do not trim a successful
  Git response: unlike the branch-title generator, the Git generator returns
  its schema result directly. Unknown response keys are ignored as by the
  reference object schema. Markdown/snippet extraction must follow the shared
  response loop. Git/forge text validation still belongs to the root caller.
- Lifetime policy: a private, non-aborted tracked operation owns admission and
  a token from the existing node hard-drain tracker before spawn. Caller loss
  cancels further attempts. Once the existing key reader starts, the operation
  joins its physical work before releasing capacity, even after cancellation;
  no late result may start HTTP. In-flight HTTP is cancellable, suppresses later
  attempts and cannot mutate a root. There is no new semaphore or tracker.
- Accounting policy: Models owns bounded workspace-purpose reports keyed by
  the original operation, including candidate/attempt identity, route/model,
  duration, known reported stop, failure/cancellation and observed versus
  unavailable usage. Every invoked transport is an attempt, not a claim that
  the provider billed it. Caller drop cannot erase an admitted attempt. Reports
  contain no prompt, diff, response text, endpoint or key. They are transient
  diagnostics, not a fabricated session log or durable usage-ledger entry.
  Any required retention/admission bound is named beside its owning type and
  checked before spawn; active operations cannot be silently evicted.
- Why this choice: it reuses the checked owners while preserving the original
  root's cancellation and attribution. An owned settling operation can await
  physical credentials without keeping an obsolete caller alive. A retained
  report survives that caller's disappearance. Explicit usage availability
  avoids treating an unreadable/cancelled reply as measured zero usage.
- Costs and effects: bounded diagnostics have finite retention and no restart
  durability. Direct complete HTTP and explicit finality remain Vessel choices;
  the reference instead uses an internal agent run. No real agent is launched
  here. Prompt/output/HTTP cost bounds are to be named and justified with the
  implementation, not silently copied from the title-only budget. No setting,
  method, assigned protocol or live root caller is enabled by this contract.

Reference: server/session/checkout/git-metadata-generator.ts:63-83,125-228
(raw Git schemas, patch bounds and purpose calls); agent/agent-response-loop.ts:
307-345,398-483 (schema corrections, availability and candidate fallback);
agent/structured-generation-providers.ts:24-92,136-169,225-264 (ordered resolution,
current selection and defaults), all under packages/server/src/server in the
reference tree. ADR 0087 owns the root caller and its fallback/authority rules.

The service remains unimplemented at this contract checkpoint. Its exact Rust
API is announced only after implementation, isolated fake-provider tests and
review. The inherited title package's settings privacy/write-lifetime union
must be integrated on its designated checked head before either package is
claimed consumer-ready. No moving branch is an accepted producer API.
