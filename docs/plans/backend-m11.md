# Milestone 11: what the references do, and fast

Status: in progress, 2026-11-11. Built on `backend/m11`, stacked on
`backend/m10` (milestones 8 to 10 are not merged yet).

Read first: `AGENTS.md`, `docs/plans/backend-m10.md`, ADRs 0017 to 0022,
`docs/design/web-ui/DESIGN.md`.

## Goal

The owner's two instructions, after looking at milestone 10's page:

1. Support nearly everything that agent-orchestrator, GOAT and Paseo
   support.
2. It has to be fast, and the speed has to come from the Rust side being
   built for it, not assumed.

Milestone 10 made the page look and behave like the references for what
Vessel could already do. This milestone closes the distance in what it
can do, and puts numbers on how fast it does it.

## Before writing code

1. **What exists.** `README.md`, `docs/ROADMAP.md` and
   `docs/ARCHITECTURE.md` say what Vessel does today. Three inventories of
   what the references do, each feature confirmed in their source, are
   being gathered; the feature tracks below are written from the
   difference, not from memory.
2. **Reuse or new.** Decided per feature in its track.
3. **Outside.** The three references are the specification for behaviour
   (their code is not taken; two are MIT and one Apache-2.0, Vessel is
   PolyForm Strict).
4. **Decision.** Per track.

## Track P: speed (`backend/m11-perf`)

Nothing here had been measured before this milestone. What a first look
on 2026-11-11 found, on the machine that runs the preview:

| What | Found |
| --- | --- |
| Release profile | cargo's defaults: no `[profile.release]`, so no link-time optimisation and 16 codegen units |
| Benchmarks | none in the workspace |
| The UI server's responses | not compressed: the page's script is 1,147 KB on the wire to a client that asks for gzip, brotli or zstd (335 KB if it were gzip) |
| The page's script | one file for every screen, the terminal and Markdown included |
| Pretendard | one 2,010 KB file, fetched whole the first time Hangul is drawn |
| Long lists | no windowing: a session's timeline renders every entry |
| A node at rest | 21 MB resident, 3 s of CPU in six hours |
| The page's shell from loopback | first paint 56 ms, loaded in 185 ms |

The rule of the track: **no optimisation without a number before and a
number after**, and the numbers are kept (`docs/PERFORMANCE.md`).

1. **Measure.** Benchmarks for the node's hot paths (appending to and
   reading a session's log, deriving and publishing a session's state,
   encoding and decoding a frame, fanning terminal output out to
   subscribers, the bridge forwarding a stream) and one load test: many
   sessions emitting events through a node and a bridge to a client,
   reporting throughput, latency at the median and the 99th percentile,
   and memory. For the page: load over a slow link, a timeline of
   thousands of entries, a terminal under heavy output, a board of
   hundreds of cards. The baseline is recorded before anything is
   changed.
2. **The cheap, large ones first**: the release profile; compressed
   responses (compressed once at build, not per request); the page's
   script divided by screen where ADR 0017's defence of the bundle can be
   kept (if it cannot, it is not divided, and the document says why);
   Pretendard in the subsets its package ships, so a page draws Hangul
   without fetching every glyph.
3. **Then what the measurements point at**: windowing the timeline;
   batching a stream's events per frame instead of one render each;
   batching terminal output on the node and in the page; whatever the
   profiler shows in the node.
4. **Nothing else changes.** Same behaviour, same wire. An optimisation
   that needs a wire change is described and left for the coordinator.

## What the references do, and what Vessel does

Three inventories were made on 2026-11-11, one per reference, each feature
confirmed in that project's source (agent-orchestrator and GOAT at their
`main`, Paseo 0.15.2). The tables are their union, by area. "Vessel" is
what `backend/m10` does. The last column is the track that builds the
feature (the tracks follow the tables), or a dash where Vessel already
has it.

Marks: `●` ships it, `◐` ships it partly (behind a flag, off by default,
one platform, or a narrower form), `–` does not. AO is agent-orchestrator.

### Sessions and the conversation

| Feature | AO | GOAT | Paseo | Vessel | Track |
| --- | --- | --- | --- | --- | --- |
| Start a session with an agent, model, effort, permissions and a prompt | ● | ● | ● | ● | – |
| A session as a conversation or as the agent's own terminal | ● | ◐ | ● | ● | – |
| Change model, effort or permissions on a running session | ● | ● | ● | ● | – |
| Queue messages while a turn runs; edit, reorder, remove, send now | ● | ● | ● | – | A |
| Steer the running turn without stopping it | ● | ● | ● | – | A |
| Interrupt a turn | ● | ● | ● | ● | – |
| Start an ended session again with its conversation | ● | ● | ● | ◐ only when moved to another profile | A |
| Rename | ● | ● | ● | ● | – |
| Pin, archive and bring back, delete, mark unread | ● | ● | ● | ◐ ended sessions are listed as an archive | A |
| Questions the agent asks, answered in the session | ● | ● | ● | – | A |
| Context meter; compact | ● | ● | ● | – | A |
| Plans shown as the agent revises them | ● | ◐ | ● | ◐ on the wire, drawn plainly | A-page |
| Edit an earlier message and continue from there (rewind) | ● | ● | ● | – | A2 |
| Fork a conversation into a new session | – | ● | ● | – | A2 |
| Import conversations from the agent's own history | ● | ● | ● | ◐ from another machine's Vessel only | A2 |
| Find in a session | ● | ● | ● | – | W |
| Search every session's messages | ● | ● | ● | – | A2 |
| Park an idle agent process and wake it on the next prompt | ● | ◐ | ● | – | A2 |
| Sessions started by a session, shown under it | ◐ | ● | ● | ◐ the card says who started it | D |
| Change the agent mid-session with a handoff | ● | ● | ◐ a skill | ◐ same agent, another account | A2 |
| Switch one session between conversation and terminal | ● | – | – | – | L |
| A side question that stays out of the history | ◐ | ● | – | – | L |
| One prompt to several models at once | – | ● | – | – | L |
| Usage-limit notice: wait for the reset or fall back to another model | ● | ● | – | – | F |
| Files the session delivers, HTML shown in the conversation | ● | ● | – | – | L |

### The composer

| Feature | AO | GOAT | Paseo | Vessel | Track |
| --- | --- | --- | --- | --- | --- |
| Attach images and files | ● | ● | ● | – | A |
| `@` a file of the project | ● | ● | ● | – | B, B-page |
| `/` the agent's commands and skills | ● | ● | ● | – | A |
| Drafts kept per session | ● | ● | ● | – | W |
| Up and Down recall sent messages | ● | ● | ● | – | W |
| Enter or Mod+Enter sends, by choice | ● | ● | ● | – | W |
| A selection in a diff or a file, sent as a comment | ● | ● | ● | – | B-page |
| Dictation | ◐ macOS | ● | ● | – | V |
| Spoken conversation | – | – | ◐ off by default | – | not built |

### Agents

| Feature | AO | GOAT | Paseo | Vessel | Track |
| --- | --- | --- | --- | --- | --- |
| Agents it knows | 36 | 10 | 6 | 4 | G |
| Any agent that speaks ACP, by command or from the ACP registry | – | ● | ● | – | G |
| Is it installed, is it signed in; install and sign in from the page | ● | ● | ◐ | ◐ `vessel init`, and installed or not | G |
| Several accounts of one agent | ● | ● | ● | ● profiles | – |
| Defaults per agent | ● | ● | ● | ◐ the last choice, per browser | W |
| Every agent on every model | – | ◐ | – | ● the routing proxy | – |

### Approvals

| Feature | AO | GOAT | Paseo | Vessel | Track |
| --- | --- | --- | --- | --- | --- |
| Approve or refuse in the session, with the agent's own choices | ● | ● | ● | ● | – |
| Everything waiting on you, from every machine, in one place | ● | ● | ● | ◐ a board lane | E |
| Answer from the command line or from another agent | – | ● | ● | ◐ agents can, the CLI cannot | A |

### Projects, git and files

| Feature | AO | GOAT | Paseo | Vessel | Track |
| --- | --- | --- | --- | --- | --- |
| Projects: add a folder, clone, start new | ● | ● | ● | – a session takes a path | B |
| A worktree and branch per session | ● | ● | ● | – | B |
| Per-project setup and teardown commands, scripts, environment | ● | ● | ● | – | B |
| Diff: working, staged, everything since the base branch | ● | ● | ● | – | B |
| File tree, viewer, editor, open by name, search in files | ● | ● | ● | – | B |
| Commit, push, pull, branches, stash, discard | – | ● | ● | – | B |
| Open a pull request | – | ● | ● | – | B2 |
| Pull request facts (checks, reviews, mergeable) on the board | ● | ● | ● | – three lanes stay empty | B2 |
| Merge a pull request | ● | ● | ● | – | B2 |
| Failing checks and review comments sent to the session | ● | ● | – | – | B2 |
| A checkpoint per turn; restore the files | – | ● | ◐ | – | B2 |
| A reviewing agent | ● | ◐ | – | – | L |

### Terminals, scripts, preview

| Feature | AO | GOAT | Paseo | Vessel | Track |
| --- | --- | --- | --- | --- | --- |
| The agent's terminal | ● | ● | ● | ● | – |
| Shell terminals beside a session | ● | ● | ● | – | C |
| Project scripts and services, their ports found, a preview opened | ● | ● | ● | – | C |
| Find in a terminal; scrollback size | ● | ● | ● | – | C |
| Agents create, read and type into terminals | – | ◐ | ● | – | C |
| A browser panel | ● desktop | ● | ● desktop | – | C2 |
| Agents drive the browser | ● | ● | ◐ | – | C2 |

### Machines and phones

| Feature | AO | GOAT | Paseo | Vessel | Track |
| --- | --- | --- | --- | --- | --- |
| Several machines in one page | ◐ | ● | ● | ● | – |
| Pairing by link or code | ● | ● | ● | ● | – |
| A relay, end-to-end encrypted, with no account | – | – | ● | ● | – |
| SSH, installing the runtime on the host | – | ● | ◐ | ● | – |
| Move a session to another machine | – | ● with uncommitted changes | – | ● the conversation | B2 carries the files |
| Agents start sessions on other machines | – | ● | ● | ● | – |
| Instructions, skills, MCP servers and secrets shared across machines | – | ◐ | – | ● fleet sync | – |
| Spread new sessions over machines by load | – | ● | – | – | L |
| A phone app | ● | ● | ● | ◐ the page fits a phone | ADR 0020 |
| Push notifications | ● | ◐ | ● | – | E |

### Automation

| Feature | AO | GOAT | Paseo | Vessel | Track |
| --- | --- | --- | --- | --- | --- |
| Agents start, prompt, wait on and stop sessions | ● CLI | ● MCP | ● both | ● MCP | – |
| The starter is told when a session it started finishes | ● | ● | ● | ◐ it has to wait | D |
| Schedules: every so often or at a time, a new session or a prompt | ● | ● | ● | – | D |
| A recurring prompt to oneself | – | – | ● | – | D |
| Saved prompts | ● | ◐ | – | – | D |
| A webhook starts a session | – | ● | ◐ | – | D |

### Overviews, notifications, usage

| Feature | AO | GOAT | Paseo | Vessel | Track |
| --- | --- | --- | --- | --- | --- |
| A board by status | ● | – | – | ● | – |
| Sidebar: group, sort, filter, pin | ● | ● | ● | ◐ by machine | W, A |
| A notification inbox that survives a reload | ● | ● | ● | – | E |
| Desktop notification and sound | ● | ● | ● | – | W |
| Usage: tokens, cost, each account's limits | ● | ● | ● | ◐ a record per routed call | F |
| Pull requests page | ● | ● | – | – | B2 |

### Settings and the rest

| Feature | AO | GOAT | Paseo | Vessel | Track |
| --- | --- | --- | --- | --- | --- |
| Light, dark, system | ● | ● | ● | ● | – |
| More themes | 19 | editor, import | 8 and plugins | – | T |
| Font sizes, content width, how much of a turn is shown | ● | ● | ● | – | W |
| Shortcuts, rebindable | ● | ● | ● | ◐ fixed | W |
| Languages | 9 | 1 | 9 | 1 | I |
| Command palette | ● | ● | ● | ● | – |
| Update from the page | ● | ● | ● | ◐ `vessel node update` | L |
| Diagnostics | ● | ● | ● | ◐ `vessel status` | L |
| Plugins | – | – | ● | – | H |

### Not built, and why

- **Their hosted services**: agent-orchestrator's cloud sandboxes, teams,
  share links and billing; GOAT Connect (disabled in their builds), Boat
  machines, and the Claude, Cursor and Devin cloud providers; Paseo's
  Hub. Vessel has no hosted service and no account (`docs/VISION.md`).
- **Telemetry**: two of the three send usage and errors unless turned
  off. Vessel sends nothing.
- **Bots** (GOAT's Slack and iMessage bots and its standing "GOAT Bot"):
  one reference only, and a schedule or a webhook covers what they
  start. Revisited as a plugin.
- **Notes and a memory engine of Vessel's own** (GOAT only): the fleet
  already shares instructions and skills and copies the agents' own
  memory. Not a gap in what all three do.
- **Bound to a desktop app or to macOS**: computer use, simulator and
  emulator streaming, driving the user's own Chrome through an
  extension, importing browser profiles, the tray, keep-awake, opening
  a folder in an editor. Decided with the shells (ADR 0020).
- **Spoken conversation** (Paseo, off by default): after dictation.
- **Their own migrations** (imports from their earlier versions).

## Feature tracks

Three waves. A track is one worktree and one branch off `backend/m11`,
merged back by the coordinator. A track that adds to the wire builds the
Rust side, the CLI and the agent tools, and stops there; its page is a
second step ("A-page", "B-page"), started when its wire has been reviewed
and merged, so no screen is built on a contract that then changes.

### Rules for every feature track

- **Protocol 10 is this milestone.** Every track's additions are
  protocol 10; the first track merged raises the number, the others find
  it raised. Nothing of this milestone is released until all of wave 1 is
  in, so no node exists with half of protocol 10. A client still treats
  "this machine does not know the call" as a typed failure, as it does
  for an older node.
- **Store migrations** take the next number on the track's own branch.
  The coordinator renumbers at merge when two tracks took the same one.
- **One module per responsibility, and its own files.** A track adds its
  wire types in modules of its own in `vessel-proto`, its calls in files
  of its own in `vessel-node`, and touches a shared type (`SessionSpec`,
  `SessionInfo`, `SessionInput`) with the smallest addition that works.
- **Contract first.** Before code: a "Contract" section for the track in
  this plan (the types, the calls, each call's failures, what an old
  client or an old node sees), committed alone, and read by a reviewer
  that did not write it. Then an ADR for each decision that could have
  gone another way.
- **Truth over neatness** (milestone 10's rule): what an agent can do is
  asked of the agent, not written in a table. A thing an agent cannot do
  is not offered for it, and is not imitated under the same name.
- **Every path a client names is checked** against the folder it has to
  stay in, after resolving links; every call checks the caller's scope
  (a device, ADR 0010; an agent, ADR 0013).
- The gate of `AGENTS.md`, whole, and the docs (`docs/PROTOCOL.md`,
  `docs/ARCHITECTURE.md`, `README.md` where a command or a behaviour is
  new) in the same branch.

### Wave 1 (started 2026-11-11)

**Track P: speed.** Above.

**Track A: the conversation** (`backend/m11-conversation`). Rust, wire,
CLI, agent tools.

1. **Queue, steer, interrupt.** A prompt sent while a turn runs is, by
   the sender's choice, queued, steered into the turn, or sent after
   interrupting it. The queue is the node's (all three references keep
   it where it survives the page closing; two keep it on the server): it
   is part of the session's state, every client sees it, and an entry can
   be edited, moved, removed or sent now. Steering is offered only where
   the agent takes input during a turn; interrupt-and-send is its own
   choice, not steering under another name.
2. **What a prompt carries.** Images and files as typed content, offered
   by what the agent says it accepts. Bytes go from a client to the node
   in bounded pieces, are kept with the session and removed with it.
3. **The agent's commands.** The commands an agent advertises are session
   state, so a client can complete `/`.
4. **Questions.** A question the agent asks is a typed request with a
   typed answer, like a permission, and first answer wins.
5. **Usage and context.** What the agent reports of tokens and context
   size, typed, per turn and as the session's current figure.
6. **Start again.** An ended session is started again with its
   conversation where the agent can resume (what moving to another
   profile already does).
7. **Pin, archive, unread, delete.** Kept by the node, so every client
   agrees. Archiving a running session ends it (agent-orchestrator and
   Paseo both do); deleting removes its log.
8. The CLI and the agent tools for each of these where one makes sense
   (`vessel send` with the delivery, answering a permission from the
   CLI, `vessel sessions` to restore, archive, pin, delete).

**Track B: projects, worktrees, git, files** (`backend/m11-workspace`).
Rust, wire, CLI, agent tools.

1. **Projects.** A node keeps the folders it works in: add one, clone
   one (progress reported), start a new one. A project's own file in the
   repository says how a worktree is set up and torn down, names its
   scripts and its environment.
2. **Worktrees.** A session starts in the folder as it is, in a new
   worktree on a new branch from a base, or on an existing branch. Setup
   runs on creation and its output is the session's to show. A worktree
   is removed with its session only when nothing would be lost;
   otherwise it is kept, or a snapshot of it is (GOAT's refs).
3. **Git.** Read: status, diff by scope (working, staged, everything
   since the base), per file with typed hunks, branches, log. Write:
   stage, commit, push, pull, switch and create a branch, stash, discard.
4. **Files.** List (honouring ignore files), read, write, create, rename,
   delete, find by name (for `@` and open-by-name) and by content.
5. How git is run (the `git` program or a library) is ADR 0023.

**Track W: the page, with what the wire already has**
(`backend/m11-page`). The page's origin is new on every launch
(ADR 0017), so what a user chooses cannot live in the browser: the track
starts with a preferences document the bridge keeps beside the node
book, typed and versioned, and builds on it.

1. Drafts per session and for the new-session page; Up and Down recall
   sent messages; Enter or Mod+Enter sends.
2. One registry of actions with their keys, a Shortcuts section in
   Settings that lists and rebinds them and refuses a clash.
3. Find in a session, over the conversation's model and not the
   document, so it keeps working when the timeline is windowed (Track
   P), and an outline of the prompts.
4. Notifications from the page: the browser's own, a sound, and the
   tab's title, when a session starts to wait on the user or finishes
   while the page is not in front; a Notifications section in Settings.
5. The sidebar's grouping, sorting and what it hides; font sizes,
   content width and how much of a turn is shown, in Appearance;
   defaults per agent.
6. Milestone 10's leftovers: an untitled session's name, Settings on a
   phone-width page, `bridge/client.ts` divided, wire ids as branded
   types.

### Wave 2 (after wave 1's wire is merged)

Written out, with its two halves and the numbers each track takes, in
"Wave 2: tracks" at the end of this plan; this list is the outline it
was written from.

- **A-page**, **B-page**: the screens for tracks A and B, taken from the
  references as milestone 10's were: the queue above the composer, the
  attachment tray, `/` and `@` menus, questions, the context meter; the
  project list and the new-session page's place picker, the diff panel,
  the file panel, the git menu.
- **A2**: rewind and fork where the agent can; import from the agent's
  own history; search across sessions; parking idle agents; changing
  the agent with a handoff.
- **B2**: a forge behind one trait, GitHub first: open, read, merge a
  pull request; its facts fill the board's three empty lanes
  (ADR 0019); failing checks and review comments sent to the session;
  a checkpoint per turn; a moved session takes its files.
- **C**: shell terminals beside a session; scripts and services with the
  ports they open; terminal tools for agents; find and scrollback.
  **C2**: a preview panel for a port on the machine, through the bridge;
  browser tools for agents.
- **D**: schedules, the recurring prompt, saved prompts, webhooks; the
  starter told when its session finishes; started sessions shown under
  their starter.
- **E**: one inbox of what waits on the user, kept by the node; push to
  a phone through the browser's push service.
- **G**: any ACP agent by command and from the ACP registry; install and
  sign-in state, and doing both from the page.

### Wave 3

- **F**: usage (tokens, cost, each account's limit windows), and what to
  do at a limit.
- **H**: plugins. An ADR first: what a plugin may add, how its page code
  reaches every client without an update, and what the page's content
  policy (ADR 0017) then allows.
- **I**: languages, Korean first. Last on purpose: it touches every
  string every other track writes.
- **T**: themes beyond light and dark. **V**: dictation (a speech model
  on the node; an ADR).
- **L**: the rest of the tables: interface switch, side question, one
  prompt to several models, delivered files and HTML, a reviewing
  agent, spreading by load, updating and diagnostics from the page.

## Track A: contract

Protocol 10, this track's half of it (Track B adds its own; the rules
above). Written 2026-11-11 from the three references and the agents'
own ACP code; every "the agent says" below is read from the handshake or
from what the agent sends, never from a table of Vessel's.

### What the agents do (read from their ACP code, no prompt sent)

| | Claude (`claude-agent-acp` 0.100) | Codex (`codex-acp`) | Gemini (`--experimental-acp`) | OpenCode (`acp`) |
| --- | --- | --- | --- | --- |
| Prompt during a turn | queued by the adapter | queued | cancels the running turn | refused |
| Steering (`_meta.steering.supported`, `_session/steering`) | yes | yes | no | yes |
| `promptCapabilities` | image, embedded context | image, embedded context | image, audio, embedded context | image, embedded context |
| `available_commands_update` | yes, `{name, description, input.hint}` | yes | yes, name and description | not found |
| `usage_update` | `used`, `size`, `cost` at a turn's end | `used`, `size` | `used`, `size` at a turn's end | `used`, `size`, `cost` at a turn's end |
| Questions (`elicitation/create`, form mode) | `AskUserQuestion`, only when the client advertises form elicitation (otherwise the tool is disabled) | yes | none (permissions only) | only when the client advertises form elicitation |
| `loadSession` | yes | yes | yes | yes |
| Per-turn tokens in the prompt's answer | yes | yes | yes | no |

What follows from it:

- **Vessel never sends `session/prompt` during a turn.** The adapters
  disagree on what it means (queued, cancelling, refused), so the node
  keeps the queue itself and sends a prompt only between turns.
- **Steering is the ACP steering extension, nothing else.** An agent
  whose `initialize` answer carries `_meta.steering.supported: true`
  can steer; Vessel sends `_session/steering` with `idleBehavior:
  promptRequired` (agent-orchestrator's use of it). Gemini cannot steer,
  so it is not offered there; "interrupt and send" is offered to every
  agent as what it is.
- **Questions are ACP form elicitations.** Vessel starts advertising
  `elicitation: { form: {} }`. URL elicitation (used for an MCP server's
  sign-in) is not advertised: the node would be asking a person to open
  a link the agent chose, from a page on another machine; the agents
  then do without, as they do today.
- **Usage is `usage_update`** (context used and its size, and a cost
  when the agent says one), and the prompt's answer `usage` where the
  agent sends it (ACP's `unstable_end_turn_token_usage`, which Codex and
  Gemini fill and Claude's adapter too; read with ACP's own
  default-on-error rule, so an agent that changes it costs only the
  figure). This turns on the ACP crate's `unstable_end_turn_token_usage`
  feature: no new dependency, one schema field; if ACP drops it, the
  figure is absent and nothing else changes.
- **Compacting is the agent's own `/compact` command** where it
  advertises one; there is no call of Vessel's for it (no ACP request
  exists, and two references send the text too).

### Types (`vessel_proto::conversation`, new module)

Ids, node-minted UUIDv4 (`id.rs`): `QueueEntryId`, `QuestionId`,
`AttachmentId`, `UploadId`.

Bounded values, parsed where a frame is decoded (a bad one does not
decode):

- `CommandName`: 1 to 128 characters, no whitespace, no leading `/`.
- `AttachmentName`: a file name: 1 to 255 bytes, one path component
  (no `/`, `\`, NUL or control character), not `.` or `..`, no leading
  or trailing space or dot, not a Windows device name. Used for display
  and as the stored file's name; never joined to a path a client chose.
- `MimeType`: `type/subtype`, lowercase ASCII token characters, at most
  127 bytes, no parameters.
- `CostUnit`: 1 to 16 letters, digits or `_` (`USD`, `credits`).
- `FieldName`: 1 to 128 characters (an elicitation property's key).
- `FiniteNumber`: a JSON number that is finite (no NaN or infinity), so
  types holding it stay `Eq`.

What an agent can do, from its `initialize` answer:

- `AgentAbilities { steer, resume, image, audio }`, each a bool:
  `steer` is `_meta.steering.supported`, `resume` is
  `agentCapabilities.loadSession`, `image` and `audio` are
  `promptCapabilities`. Files are offered to every structured agent (a
  resource link, ACP's baseline content).

A prompt's content:

- `ContentBlock::Attachment { attachment: AttachmentRef }` (new variant):
  `AttachmentRef { id, name, kind, mime, size }`, `kind` one of `image`,
  `audio`, `file`. In a prompt, a steer and a queue entry; never from an
  agent (what an agent sends stays `opaque`, as now).

The queue:

- `QueuedPrompt { id: QueueEntryId, content: ContentBlock[], queued_at,
  revision, by: Creator }`.
- `PromptQueue { entries: QueuedPrompt[], held?: QueueHold }`; `held`
  absent means the queue moves on its own. `QueueHold`: `cancelled` (a
  person cancelled a turn), `failed` (a turn failed), `ended` (the
  session ended with prompts waiting), `asked` (`session.queue hold`).
- `Delivery`: `start`, `queue`, `steer`, `interrupt`. `Delivered`:
  `started`, `steered`, `queued { entry }`.
- `QueueChange` (tagged `change`): `edit { entry, revision, content }`
  (`revision` the one the edit was made from), `move { entry, before? }`
  (before another entry, or last), `remove { entry }`, `send_now { entry,
  delivery }` (`start`, `steer` or `interrupt`), `hold`, `resume`,
  `clear`.
- `QueueChanged { queue, delivered? }`: the reply to `session.queue`.

The agent's commands:

- `AgentCommand { name: CommandName, description, hint? }`, as
  `available_commands_update` lists them (`hint` from `input.hint`). A
  command is used by sending a prompt that starts with `/name`.

Questions (an ACP form elicitation, typed):

- `Question { id, message, tool_call?: ToolCallId, fields:
  QuestionField[] }`.
- `QuestionField { name: FieldName, title?, description?, required,
  kind }`, `kind` (tagged `kind`) one of:
  - `text { min_length?, max_length?, format?, pattern?, default? }`,
    `format` one of `email`, `uri`, `date`, `date_time`; `pattern` is
    passed on, not checked by the node;
  - `number { minimum?, maximum?, default? }` and `integer { minimum?,
    maximum?, default? }` (whole numbers, `i64`), apart so a bound or a
    default cannot be fractional;
  - `boolean { default? }`;
  - `choice { options: ChoiceOption[], multiple, min_items?,
    max_items?, default? }`, `ChoiceOption { value, title,
    description? }` (ACP's `enum`, `oneOf` and multi-select, in the
    agent's order);
  - `unknown`: a kind a newer ACP has; a client can only decline a
    question that has one.
- `QuestionAnswer` (tagged `answer`): `accept { values }` (`values` a
  map from field name to `AnswerValue`: `text`, `number`, `integer`,
  `boolean`, `choice` (one value) or `choices` (several)), `decline`.
- `QuestionOutcome` (tagged `outcome`): `answered { values }`,
  `declined`, `cancelled` (the turn or the session ended first).

Usage:

- `ContextUsage { used: TokenCount, size: TokenCount, cost?: Cost }`,
  `Cost { amount: FiniteNumber, unit: CostUnit }`: the agent's last
  `usage_update`, cost as the agent's running total for the session.
- `TurnUsage { total, input, output, thought?, cached_read?,
  cached_write? }` (each a `TokenCount`): a turn's tokens as the agent
  reports them in the prompt's answer. Not `TokenUsage` (protocol 7):
  that one is the node proxy's own accounting with defined cache rules,
  and agents do not agree on whether `input` counts cached tokens.

The whole conversation as it stands:

- `Conversation { queue: PromptQueue, commands: AgentCommand[], usage?:
  ContextUsage, permissions: PermissionRequest[], questions:
  Question[], abilities?: AgentAbilities, limits: AttachmentLimits }`:
  `permissions` and `questions` are the ones still waiting.
- `AttachmentLimits { image, audio, file, per_prompt, inline_per_prompt,
  per_session, chunk }`, sizes in bytes, `per_prompt` a count (the
  values in "Bounds").

A session's marks, kept by the node:

- `SessionMarks { pinned, archived_at?, unread }`.
- `Mark` (tagged `mark`): `pin`, `unpin`, `archive`, `unarchive`, `read
  { through }` (`through` the next record the reader has not seen;
  required), `unread`.

Uploads:

- `Upload { upload: UploadId, chunk }`; `Written { received }` answers a
  piece; `attachment.finish` answers an `AttachmentRef`.

### On existing types (the smallest additions)

- `SessionInfo.marks: SessionMarks` (omitted when nothing is marked) and
  `SessionInfo.abilities?: AgentAbilities` (from the last handshake, kept
  by the store, so an ended session says whether it can start again).
- `HarnessOptions.abilities?: AgentAbilities`: the probe's, for a new
  session's composer.
- `AgentEvent::Ready.abilities?` and `AgentEvent::TurnEnded.usage?:
  TurnUsage`, optional, so older clients ignore them.
- `SessionInput::AnswerQuestion { id: QuestionId, answer }` (`session.send`,
  `ack`): first answer wins, as for permissions.
- `SessionInput::Prompt` keeps meaning `start` (protocol 2's behaviour).
- `SessionInput::CancelTurn { turn? }`: the `seq` of the `turn_started`
  meant. A cancel that lands after that turn ended (the queue started
  the next) holds the queue and cancels nothing. Absent: whatever runs,
  as before; a cancel naming its turn needs protocol 10.

### Events

| Event | Since | What it is | A version 2-9 client gets |
| --- | --- | --- | --- |
| `queue_changed { queue }` | 10 | the whole queue after any change (sent, dispatched, edited, held) | `vessel: queue: 2 waiting (held: cancelled)` |
| `commands_changed { commands }` | 10 | the whole list, each time the agent sends one | `vessel: commands: /compact, /review and 12 more` |
| `usage_changed { usage }` | 10 | each `usage_update` | `vessel: context: 41,200 of 200,000 tokens` |
| `agent { steered { content } }` | 10 | a message added to the running turn | `vessel: steered: <first line>` |
| `agent { question_asked { .. } }` | 10 | a question waiting for an answer | `vessel: the agent asks: <message>` |
| `agent { question_resolved { id, outcome } }` | 10 | answered, declined or withdrawn | `vessel: question answered` (declined, cancelled) |
| a `turn_started` or `steered` holding an attachment | 10 | the prompt as sent | the same event, each attachment as `resource_link { uri: "attachment:<id>", name }` |

The stand-ins keep the record's `seq`, as protocol 5's do; `since` for an
agent event now looks at its content (an attachment makes a
`turn_started` a version 10 record).

### Calls

| Method | Scope | Request | Reply |
| --- | --- | --- | --- |
| `session.prompt` | operate | `{ session, content, delivery }` | `Delivered` |
| `session.queue` | operate | `{ session, change: QueueChange }` | `QueueChanged` |
| `session.conversation` | read | `{ session }` | `Conversation` |
| `session.resume` | operate | `{ session }` | `SessionInfo` |
| `session.mark` | operate | `{ session, mark: Mark }` | `SessionInfo` |
| `session.delete` | operate | `{ session }` | `ack` |
| `attachment.begin` | operate | `{ session, name, mime, size }` | `Upload` |
| `attachment.write` | operate | `{ upload, offset, data }` (base64, at most `chunk` bytes) | `Written` |
| `attachment.finish` | operate | `{ upload }` | `AttachmentRef` |
| `attachment.read` | read | `{ session, attachment, offset }` | `BlobChunk` |

All are forwardable. The `operate` calls naming a session are
ownership-checked for agents (ADR 0013); `attachment.write` and
`attachment.finish` belong to the caller that began the upload (another
gets `not_found`, as for transfers). Every call answers `not_found` for
an unknown session, `unsupported` for a terminal session (except
`session.mark` and `session.delete`, which every session takes).

**`session.prompt`**: `content` not empty; each attachment of this
session, of a kind its agent takes, within the per-prompt limits.

- `start`: starts a turn. `conflict` while one runs (the call
  `session.send` makes today).
- `queue`: starts a turn when nothing runs, nothing waits and the queue
  is not held; otherwise appends (`queued`), held or not.
- `steer`: during a turn, sends `_session/steering`; `injected` is
  `steered` (logged as `steered`). When the agent answers
  `promptRequired` (the turn ended first) or no turn runs, the prompt
  starts a turn (`started`), as GOAT, Paseo and agent-orchestrator's
  steer-or-send do. `unsupported` for an agent without `steer`, before
  anything is sent; an agent that answers "method not found" anyway is
  `unsupported` too.
- `interrupt`: cancels the running turn, waits for it to end (at most 10
  s), then starts this prompt. The queue is not held by it. `unavailable`
  when the turn does not end in time (it was cancelled; nothing is
  sent).
- Failures: `invalid_request` (empty content, an attachment not of this
  session, a kind the agent does not take, over a limit), `conflict`
  (ended, moving to another profile, a turn runs for `start`, the queue
  is full), `unsupported` (as above).

**The queue moves** (decided in the session's actor, ADR 0024):

- After a turn ends (`turn_ended` with any reason but `cancelled`) and
  the queue is not held, its first entry starts, one per turn (GOAT's
  order; agent-orchestrator merges every waiting entry into one turn,
  which loses where one prompt ends).
- A turn a person cancels (`session.send cancel_turn`) holds it
  (`cancelled`); a failed turn holds it (`failed`); a session that ends
  with entries keeps them, held (`ended`), and `session.resume` brings
  them back held. GOAT holds in all three cases;
  agent-orchestrator drops them at a cancel. Nothing waiting is ever
  sent after a person said stop.
- `resume` lets it move again and, between turns, starts its first
  entry; `hold` stops it; `clear` empties it.
- `edit`, `move` and `remove` of an entry no longer queued (it was sent,
  or removed) are `not_found`; an `edit` from another revision is
  `conflict`. `send_now` takes the entry out and delivers it as
  `session.prompt` would (`queue` is not a delivery here:
  `invalid_request`), the reply's `delivered` saying how; when that
  delivery fails, the entry goes back where it was.
- The queue is the node's: every client sees it (`queue_changed`,
  `session.conversation`), the store keeps it (it survives a restart of
  the node), and it goes with the session.

**`session.conversation`**: for a live structured session, what its
log says now, kept by the session as it publishes (no log read): the
queue, the last commands and usage, the permissions and questions still
waiting, the abilities. For an ended one, from the store: its queue
(held), its last usage and abilities; no commands, nothing waiting.

**`session.resume`** (start again, ADR 0026): an ended structured
session's agent is started again under its current profile with
`session/load`, the way `session.move_profile` restarts one (ADR 0011):
same id, same log (it continues after the final status with `starting`,
the agent's replay dropped), same title and choices; its queue comes
back held. The call answers once the agent is ready (bounded by the
handshake timeout). An archived session is brought back too.
`unsupported` when the agent cannot resume (its recorded `resume` is
false) or the session never had a harness session; `conflict` while it
is live, or while its log is still being written after its end (a retry
works); `invalid_request` for a profile that no longer exists;
`harness_not_installed`; `spawn_failed` when the agent does not start or
`session/load` fails: the attempt is in the log (`starting`, then
`failed` with the reason) and the session stays ended.
`unavailable` while the node drains. An agent's limits (ADR 0013) count
it as a live session again.

**`session.mark`**: `pin`, `unpin`, `unarchive` change only the mark.
`archive` on a live session stops it first (agent-orchestrator and
Paseo do) and answers once it has ended. `read { through }` marks every
record before `through` seen; `unread` marks the session unread until
the next `read`. Every mark is kept by the store and is the same for
every client. Idempotent.

**`session.delete`**: removes an ended session: its log, its marks, its
queue and its attachments (files included). The agent's own transcript
in its home is not touched (it is the agent's history, which a later
import can read). `conflict` for a live session (archive it first, as
agent-orchestrator says).

**Attachments** (ADR 0025):

- `attachment.begin`: the session is live and structured; the kind
  follows the type (`image/png`, `image/jpeg`, `image/gif`,
  `image/webp` are images, `audio/*` is audio, anything else a file)
  and must be one its agent takes (`unsupported` otherwise); `size` is
  within the kind's limit and the session's (`invalid_request`, naming
  the limit and the size); at most 16 uploads are open on a node
  (`unavailable` beyond).
- `attachment.write`: `offset` is what has been received so far (a
  piece that ends at or before it, sent again after a lost reply, is
  answered with it; a gap or an overlap is `invalid_request`), the data
  at most `chunk` bytes, never past `size`.
- `attachment.finish`: all `size` bytes are there (`invalid_request`
  otherwise); an image's first bytes must be its declared type's
  (`invalid_request`). The bytes are then the session's, under the
  node's data directory, and the store records them. An upload idle for
  10 minutes, or open at a restart, is dropped.
- Given to the agent: an image as an ACP `image` block and audio as an
  `audio` block (the bytes, base64, in the prompt); a file as a
  `resource_link` to the stored file (`attachments/<session>/<id>/<name>`
  under the node's data directory, so two `image.png` never collide and
  the path ends in the file's name), which the agent reads with its own
  tools, as GOAT and Paseo hand files over from their state directory
  (agent-orchestrator copies them into the worktree instead). The log
  keeps the reference, never the bytes or the path.
- A prompt names an attachment by its id; the node replaces whatever
  else the client sent (name, kind, type, size) with its own record, so
  a client cannot relabel an image as a file to pass the inline limit.
- The node reads an attachment back (for the agent or for
  `attachment.read`) without following links, only as a regular file of
  the size and SHA-256 it recorded at `finish`: the agent can replace
  the file it was handed with a link to the node's token, and a device
  with `read` must not then fetch the token.
- `attachment.read`: a session's attachment in `BlobChunk`s, so another
  device shows what was sent.

### The board, and ended sessions (ADR 0026, changing ADR 0019's rule)

Ended no longer means put away: an ended session can start again. As in
agent-orchestrator, whose reducer Vessel ported, a session is in the
archive lane (`terminated`) when it is archived; an ended session that
is not is in building with the phrase `exited` (its activity `exited`),
until it is archived or started again. The schema migration archives
every session that had ended before it (`archived_at` = its end), so the
lanes look as they did. `SessionFacts.worker.is_terminated` now reads
"archived".

### Unread, without a race

`unread` is derived: the session is unread when it was marked so, or
when a record that asks for attention (a turn's end or failure, the
session's end) has a `seq` at or after the last `read`'s `through`. The
store keeps the attention point as a projection of the log and the
`read` point as a mark, so a `read` and a turn ending at the same moment
agree in any order.

### Old clients, old nodes

- A client sends none of the new calls, an `answer_question`, or a
  prompt holding an attachment (`session.send`) to a node older than 10
  (`Call::since`); a node refuses them on an older connection
  (`unsupported`, "needs protocol 10").
- A version 2-9 client reading a version 10 node gets the stand-ins
  above; `marks`, `agent`, `abilities` and `usage` fields it ignores. It
  sees ended sessions in building as `exited` (a phrase it has had since
  protocol 8) until they are archived.
- A question waits for an answer only a version 10 client can give; an
  older client sees the log line, and the turn waits as it waits on a
  permission nobody answers (cancel ends it).

### Bounds

| What | Bound | Beyond it |
| --- | --- | --- |
| Queue | 32 entries (`conflict`), 128 KiB of content in all (`invalid_request`) | refused |
| Commands kept | 512, a description cut at 1 KiB, a hint at 256 bytes; logged only when the list changes | the rest dropped, logged |
| A question | 32 fields, 64 options a field, text cut at 4 KiB (message) and 1 KiB (titles, descriptions) | cut, logged; more fields than that: declined |
| An answer's text value | 16 KiB | `invalid_request` |
| Waiting permissions and questions a session tracks | 64 | the oldest answered `cancelled` |
| An image or audio attachment | 10 MiB | `invalid_request` |
| A file attachment | 50 MiB | `invalid_request` |
| A prompt | 16 attachments, 25 MiB of images and audio | `invalid_request` |
| A session's attachments | 100 MiB | `invalid_request` |
| Open uploads on a node | 16, at most 4 per caller, each dropped after 10 idle minutes | `unavailable` |
| A node's attachments | 4 GiB in all; a session's count includes its open uploads' declared sizes | `invalid_request` |
| An upload chunk | 1 MiB (`BlobChunk::MAX_LEN`) | `invalid_request` |
| Interrupt-and-send's wait | 10 s | `unavailable` |

(Image sizes are agent-orchestrator's and GOAT's, files GOAT's and
Paseo's; none of the three bounds its queue, Vessel does.)

### Store and numbers

- Schema version 7 (the next free one on this branch; the coordinator
  renumbers at merge if Track B took it): on `sessions`, `pinned`,
  `archived_at`, `read_seq`, `attention_seq`, `marked_unread`, `queue`
  (JSON), `abilities` (JSON), `usage` (JSON); a table `attachments`
  (session, id, name, kind, mime, size, created_at), deleted with its
  session; the bytes under the data directory's `attachments/<session>/`.
- ADRs: 0024 (the queue, delivery and steering), 0025 (attachments),
  0026 (ended, archived, started again; unread by sequence). 0023 is
  Track B's.

### Track A: the contract's review (2026-11-11)

An independent reviewer read the contract above against the references,
the agents' ACP code and Vessel's wire, and found fourteen problems. What
changed (the passages above are already amended where they said
otherwise):

1. **Older peers through the fleet.** Only top-level `session.read`
   pages and `device.list` were rewritten for an older connection; a page
   passed back through `fleet.forward`, `fleet.relay` or `fleet.deliver`
   reached an older home unchanged and broke its connection (a hole since
   protocol 9). Every reply is now rewritten by one function that walks
   into the envelope's nested reply (`Reply::for_protocol`).
2. **Starting again after a final status.** "A final `status` is the
   last record" becomes "the last record of a run": `session.resume`
   continues the log after it. The store's writer reopens the row
   (`live`, no end) before anything is published, refusing one that
   retention or `session.delete` removed, so retention never prunes a
   session that is starting again. A subscription the node serves from
   the store ends `lagged { resume_from }` (which every client already
   resubscribes on) when the session was reopened while it was served,
   and `session_ended` otherwise, so it neither misses the new run nor
   loops.
3. **Starting again, for whom.** The session's agent tools get a new
   credential, granted as a session the caller creates would be (ADR
   0013): under the owner for the owner or a device, under the calling
   agent for an agent, which counts it against that agent's live
   sessions. `created_by` stays the original creator. A refusal is the
   same error as at `session.create`.
4. **Questions nobody can answer.** Advertising form elicitation turns
   Claude's `AskUserQuestion` on. A session an agent created (ADR 0013)
   runs unwatched, and before this milestone such sessions never asked;
   so the node advertises questions only for sessions the owner or a
   device creates. The agent tool `wait_turn` returns `question_asked`,
   and an agent tool answers one.
5. **Questions the agent withdraws** (Claude cancels the request when a
   hook answered or a subagent closed): the node watches the request's
   cancellation and resolves it `cancelled`. The `harness.options` probe
   never advertises questions.
6. **Steering.** `startedNewTurn` (the agent ignored `promptRequired`
   and started a turn Vessel did not send, so its end would never be
   logged) is `internal`, with a log line. When the agent answers
   `promptRequired`, the prompt starts a turn if none runs, and goes to
   the head of the queue (`queued`) if the queue already started one.
7. **Cancels, the queue and interrupts.** A cancel may name its turn
   (`CancelTurn.turn`, above). An empty queue is never held, so messages
   typed after a stop still start (agent-orchestrator's behaviour). Every
   delivery, the queue's included, goes through the session's actor one
   at a time, so an interrupt waiting for its cancel is not overtaken by
   the queue's next entry, and a steer or interrupt sent meanwhile waits
   its turn. `move` names the entry to go before; `edit` names its
   revision. Prompts sent while the session starts wait for its
   handshake, as `session.send` does. An entry is checked again when it
   is sent (after a move or a start again the agent may take less); one
   that no longer fits is not sent, the queue is held (`failed`) and a
   log line says why. `session.queue` on an ended session is
   `conflict`: the queue comes back with `session.resume`.
8. **Attachment paths.** As amended above: stored per id, read back only
   as the regular file recorded at `finish`, and the reference claim
   about agent-orchestrator corrected.
9. **Client-made references.** As amended above: the node's record wins.
10. **Deletion, retention, uploads.** Retention's pruning also removes a
    session's attachment files, and the node sweeps orphaned attachment
    folders and unfinished uploads at start. Retention takes archived
    sessions first, then ended ones, and pinned ones last. `session.delete`
    while an ended session's log is still being written is `conflict`, as
    is `attachment.finish` for a session that ended meanwhile. A node
    keeps at most 4 GiB of attachments, a caller holds at most 4 of the 16
    upload places, a session's size check counts its open uploads, and a
    repeated piece is answered with `Written` instead of refused.
11. **The board for older clients.** A version 8-9 peer gets every ended
    session in the archive as before (`SessionInfo::for_protocol`).
12. **The log's growth.** Commands are logged only when the list
    changes; the queue holds at most 128 KiB.
13. **Unread.** A permission or a question waiting on the owner also
    asks for attention. `read.through` is required and only moves
    forward. Agents may not mark read or unread (`forbidden`): an agent
    reading a session is not its owner seeing it.
14. **Types and claims.** Integer fields apart from numbers; `pattern`
    carried; `SessionInfo.abilities`; Claude does report a turn's tokens
    (the table is corrected). An agent's item that does not parse (a
    command name, a cost unit, a field name) is left out and logged; a
    question whose form names a required field it does not define is
    declined. An agent's cost restarts at zero when its process does (a
    start again, a move): the figure is the agent's. A session stored
    before schema 7 has no recorded abilities, so `session.resume` tries
    and the handshake's answer decides.

Rejected, and why:

- **Logging a steer before it is sent** (so the agent's answer to it can
  never come first). The node would then log a message the agent may
  refuse or never take. `steered` is logged when the agent says
  `injected`, in the order the agent's own messages arrive; Claude's
  adapter answers before the model reads the message.
- **The queue logged as changes per entry** instead of whole. Whole
  queues keep every reader's fold one line and the store's projection a
  copy; the 128 KiB bound caps a record.
- **An idempotency key on `session.prompt`.** No Vessel path retries a
  call by itself; a forwarded call that may or may not have run says so
  to its caller, who sees the queue and the log and decides.
- **A separate input type for a prompt's attachments.** The node already
  replaces a client's reference with its own record; a second content
  type would double every place that reads a prompt.
- **Agents marking read** and **devices with only `read` marking read**:
  not given. A read-only device is an observer; its views do not clear
  the owner's unread.

Left as known limits:

- A form's fields arrive in the order ACP's crate keeps them (by key), not
  the order the agent wrote them.
- A held queue and the marks stay on the node; a session moved to
  another node (import, then stop) leaves them behind.
- Track B: a session's worktree must be removed when the session is
  deleted, not when it ends, or starting it again finds no folder. Said
  to the coordinator in the report.

## Track B: contract

Protocol 10 (shared with track A). Everything below is new: wire
types in `vessel_proto::workspace` (one module per area: `project`,
`worktree`, `git`, `diff`, `files`, `path`), two ids in `id.rs`, one
optional field on `SessionSpec` and one on `SessionInfo`. No session
event is added: track B logs nothing new into a session's log except
`vessel:` notice lines, which every client already reads.

### What the references do (read 2026-11-11)

- **How git runs.** All three spawn the `git` program with an argument
  vector; none links a git library. Time limits: agent-orchestrator 15 s
  for reads, GOAT 30 s by default and none on push, Paseo 30 s and 120 s
  for commit, push and pull. Prompts: agent-orchestrator sets
  `GIT_TERMINAL_PROMPT=0` and `GCM_INTERACTIVE=Never` on every run and
  adds `-c core.hooksPath=/dev/null -c diff.external= -c
  core.fsmonitor=false --no-optional-locks` to reads; GOAT and Paseo turn
  prompts off only for background fetches, so a user's push that wants a
  credential waits for the time limit (Paseo) or forever (GOAT). GOAT
  reports a fixed reason (`authentication_failed`,
  `push_rejected_by_remote`) instead of git's stderr, because stderr can
  echo a credential. Nobody checks git's version.
- **Projects.** agent-orchestrator adds a folder, clones (temporary
  directory beside the target, renamed when done; progress parsed from
  `--progress`; cancel kills the tree), or initialises one. GOAT: new
  (git init), clone with progress and cancel, open a folder. Paseo:
  add, clone (staging directory, no progress, five-minute limit).
- **The project's file.** Paseo: `paseo.json` at the repository root,
  `worktree.setup` and `worktree.teardown` (a string or a list),
  `worktree.terminals`, `servicePorts`, `scripts`; unknown keys pass
  through. GOAT: `goat.json`, `scripts[]` with `runOnWorktreeCreate`,
  `runOnSettle`, `async` (default: the agent starts while setup runs),
  read from the checkout being set up, at most 50 scripts.
  agent-orchestrator keeps `postCreate`, `preRemove`, `symlinks`, `env`
  and the default branch in its own store, set from its page: nothing is
  read from the repository.
- **Trust.** Paseo runs `paseo.json`'s setup for every workspace except
  one checked out from a fork's pull request, which waits until the user
  runs setup. GOAT runs `goat.json`'s scripts whenever the project has no
  scripts of its own in GOAT's settings. agent-orchestrator runs only
  commands the user typed into its page. None asks again when the file
  changes.
- **Worktrees and tidying.** Paseo removes the worktree when its last
  workspace is archived. GOAT removes a worktree by rules (deleted,
  settled for N days, merged) but keeps it when it has uncommitted
  changes (unless a snapshot covers them), commits that did not ship,
  an operation in progress, an open terminal, or a branch that changed;
  snapshots are commits under `refs/goat/snapshots` built through a
  temporary index, refused above 100 MiB per file and 256 MiB in all.
  agent-orchestrator refuses to remove a dirty worktree and keeps a
  commit of it under `refs/ao/preserved/<session>`.
- **Diffs.** Paseo parses hunks on the server (`add`, `remove`,
  `context`), 1 MiB per file and 2 MiB in all, an oversized or binary
  file sent as a marker without hunks; scopes "uncommitted", "since the
  base", per commit. agent-orchestrator sends a patch per group of up to
  100 files, 2 MiB, with deferred files (binary, oversized); scopes
  combined, committed, uncommitted, staged, unstaged, untracked; 5,000
  files at most. GOAT sends the file list first and patches per file,
  1 MiB.
- **Git writes.** GOAT: commit (stage given paths), push (no force,
  sets the upstream), `pull --ff-only`, create and switch branches.
  Paseo: commit (`add -A` by default), push, pull, merge, switch (clean
  tree only), stash save, list and pop, discard (unstage, restore,
  `clean` for untracked). agent-orchestrator: none; its agents run git.
- **Files.** List: agent-orchestrator through `git ls-files`, GOAT with
  ignored entries labelled by `git check-ignore`, Paseo everything.
  Read: 256 KiB (agent-orchestrator), 1 MiB (GOAT, Paseo); binary by a
  NUL byte, or invalid UTF-8. Write: text, at most 256 KiB or 1 MiB,
  refused when the file changed since it was read (a fingerprint, or
  `mtime` and size); writes are atomic; `.git` is refused; paths are
  resolved with `realpath` and checked against the root. Find by name:
  agent-orchestrator a substring over the cached `git ls-files` list,
  GOAT a native index (25,000 entries, 15-minute idle life), Paseo a
  walk of at most 20,000 entries with a cache of 8 s; none cancels a
  superseded search on the server. Search in files: `git grep`
  (agent-orchestrator), an index (GOAT), none (Paseo).

### Types (`vessel_proto::workspace`)

Ids (`id.rs`): `ProjectId`, `WorktreeId`, node-minted UUIDv4.

Parsed on decode (a bad one does not decode, so nothing downstream holds
an unchecked string):

- `ProjectName`: a display name, trimmed, 1 to 80 characters, no
  control characters.
- `FolderName`: one folder to create (a clone's): the bundle path rules
  for one segment (`docs/PROTOCOL.md`, version 5: no `\ : * ? " < > |`,
  no control character, not `.` or `..`, no trailing `.` or space, no
  Windows device name or `NAME~1` short name), no `/`, no leading `-`, at
  most 255 bytes.
- `BranchName`: git's `check-ref-format --branch` rules, checked here:
  not empty, at most 255 bytes, no ASCII control or space, none of
  `~ ^ : ? * [ \`, no `..`, no `@{`, not `@` or `HEAD`, no part starting
  with `.` or ending with `.lock`, no leading, trailing or doubled `/`,
  not ending with `.`, and **no leading `-`** (it would read as an
  option).
- `RefName`: the same rules (`HEAD` allowed); a branch, a
  remote-tracking branch (`origin/main`) or a tag.
- `CommitId`: 7 to 64 hexadecimal digits, kept in lower case. The node
  resolves one with `rev-parse --verify --end-of-options <id>^{commit}`
  and refuses it when the result does not start with the digits given
  (a branch named like a commit id).
- `RemoteName`: one part of the ref rules, no leading `-`.
- `RelPath`: a path inside a root, `/`-separated on every platform: not
  absolute, no drive (`C:`), no backslash, no control character, no
  empty, `.` or `..` part, at most 4,096 bytes and 255 a part, and no
  part that git itself refuses as `.git` (git's `verify_path`): `.git`
  in any case, also with trailing dots or spaces, `git~1`, and with the
  zero-width characters HFS+ ignores. The empty path is the root itself,
  accepted only where a call says so. On a Windows node the node also
  refuses a part with `:` (a data stream of another file), a device name
  or a short name.
- `CloneUrl`: `https://`, `http://`, `ssh://`, `git://`, or the scp form
  `[user@]host:path`; no white space or control character, no leading
  `-` (on the URL, its user or its host), no `<transport>::` form, no
  `file://` and no local path (a folder on the node is `project.add`),
  no user information at all in an `http(s)` URL (a token is often the
  user), no password in an `ssh` one.
- `CommitMessage`: not blank, at most 64 KiB, no NUL.
- `ProjectCommand`: one shell command from the project's file, not
  blank, at most 4 KiB, no NUL.
- `CommandsDigest`, `ContentHash`: 64 hexadecimal digits (SHA-256).
- `FindQuery` (1 to 256 characters), `GrepQuery` (1 to 1,000), one line.
- `SearchKey`: a client's number naming a search, so a newer one with
  the same key replaces it.

Every reply enum that will grow has an `unknown` variant a client skips
or shows as such (protocol 9's rule for `OptionKind`): `ProjectState`,
`CloneStage`, `CloneFailure`, `GitSupport`, `SetupState`,
`SetupFailure`, `KeptReason`, `WorktreeRefusal`, `FileState`,
`ChangeStatus`, `DiffContent`, `EntryKind`, `FileContent`,
`GitRefusal`.

Projects:

- `ProjectInfo { id, name, root, git, default_branch?, state,
  created_at, revision }`. `root` is the absolute folder on the node (the
  repository's top level when the folder added is inside one); `git`
  says whether it is a git repository (a folder that is not has files
  but no git and no worktrees); `default_branch` the branch new
  worktrees start from (below). `revision` changes at every change, for
  waiting.
- `ProjectState`: `ready`; `cloning { stage, percent? }` with `stage`
  one of `connecting`, `counting`, `compressing`, `receiving`,
  `resolving`, `checking_out`; `clone_failed { reason, message }` with
  `reason` one of `needs_credential`, `not_found`, `network`,
  `timed_out`, `other` (`message` git's last lines, credentials
  removed); `missing` (the folder is gone).
- `ProjectList { git, projects, more }`; `git` is `GitSupport`: `found {
  version }`, `too_old { version, minimum }` (`minimum` 2.25.0) or
  `missing`, checked once when the node starts.
- `ProjectCommands`: `none`, `listed { setup, teardown, digest,
  approved }`, or `invalid { message }` (the file does not parse;
  nothing in it runs).
- The default branch: the remote-tracking branch `origin/HEAD` points
  at when there is one (`origin/main`), else the local `main`, else
  `master`, else the branch checked out in the project's folder. Kept by
  the node and found again by `git.fetch` and `project.get`.

Worktrees:

- `WorktreeBase`: `ref { name: RefName }` or `commit { id }`.
- `WorktreeBranch`: `new { name?, base? }` (a new branch, named
  `vessel/<8 hex>` when no name is given, from `base`, the default
  branch when absent), `existing { name }`, or `snapshot { worktree,
  name? }` (a new branch at the snapshot's `head`, with the snapshot's
  files put back as uncommitted changes).
- `SetupChoice`: `run` (the default) or `skip`.
- `WorktreeInfo { id, project, path, branch?, created_branch?, base?,
  created_at, created_by, created_for?, sessions, setup, kept? }`:
  `branch` what is checked out now (absent when detached);
  `created_branch` the branch the node created for it, if it did (the
  only branch ever deleted, and the one tidying checks); `base` the
  commit and name it started from; `created_by` who asked (ADR 0013's
  `Creator`); `created_for` a session it was made for; `sessions` the
  live sessions whose folder is in it (a placed session, or one whose
  `cwd` resolves inside); `kept` why tidying left it.
- `SetupState`: `none` (nothing to run), `skipped`, `running {
  started_at }`, `succeeded { finished_at }`, `failed { failure,
  finished_at }` with `failure` one of `exit { command, code? }`,
  `timed_out`, `spawn`.
- `KeptReason`: `uncommitted { files }` (a count), `unique_commits {
  commits }`, `operation { operation }` (`merge`, `rebase`,
  `cherry_pick`, `revert`, `bisect`), `submodules`, `flagged_files`
  (assume-unchanged or skip-worktree entries, whose changes `status`
  does not show), `worktree_refs` (`refs/worktree/*`, `refs/bisect/*`),
  `in_use`, `inspection_failed`.
- `CreateWorktreeOutcome`: `created { worktree }`, `needs_approval {
  commands }` (nothing was created), `refused { reason }`, with
  `WorktreeRefusal` one of `not_git`, `not_ready`, `no_commit`,
  `branch_exists`, `checked_out_elsewhere { path }`, `no_such_base`,
  `no_such_branch`, `no_such_snapshot`, `limit` (too many worktrees).
- `RemoveWorktreeOutcome`: `removed { snapshot? }`, `kept { reasons }`
  (nothing was removed; `reasons` every `KeptReason` that applies).
- `IfLost`: `keep` (the default), `snapshot`, `discard`.
- `SetupOutput { from, bytes, next, truncated, setup }`: a page of the
  setup and teardown output (stdout and stderr together, as
  `TerminalBytes`).
- `Snapshot { worktree, branch?, commit, head, created_at }`, kept at
  `refs/vessel/snapshots/<worktree id>`.

Sessions (the only shared types touched):

- `SessionSpec.place?: SessionPlace`: `project { project, dir?: RelPath
  }` (the folder as it is) or `worktree { worktree }`. With a place,
  `cwd` must be absent; the node fills `spec.cwd` with the folder the
  place resolves to, so a client that predates places still sees where
  the session runs.
- `SessionInfo.workspace?: SessionWorkspace { project, worktree?,
  branch? }`.

Git (`WorkspaceRoot`: `project { project }` or `worktree { worktree }`;
every git and file call names one):

- `GitStatus { branch?, head?, upstream?, base?, operation?, files,
  more }`: `upstream` and `base` are `Distance { name, ahead, behind }`
  (`base` for a worktree whose base is a ref); `files` each `StatusEntry
  { path, old_path?, index, worktree }` with `FileState` one of
  `unmodified`, `modified`, `added`, `deleted`, `renamed`, `copied`,
  `type_changed`, `untracked`, `conflicted`.
- `DiffScope`: `unstaged` (working tree against the index, untracked
  files as added), `staged` (index against `HEAD`), `uncommitted`
  (working tree against `HEAD`, untracked as added), `base { base? }`
  (working tree against the merge base of `HEAD` and the base, a
  worktree's own or the default branch: committed and uncommitted
  together), `commit { id }` (against its first parent). Every diff call
  also takes `ignore_whitespace`.
- `DiffSummary { from?, files, more }`, each `FileChange { path,
  old_path?, status: ChangeStatus, additions?, deletions?, binary }`,
  `ChangeStatus` one of `added`, `modified`, `deleted`, `renamed`,
  `copied`, `type_changed`, `conflicted`.
- `FileDiff { path, old_path?, status, mode?, content, more_from? }`:
  `mode` `{ old, new }` when the file mode changed; `content` one of
  `hunks { hunks }`, `binary`, `submodule { from?, to? }`, `too_large {
  bytes }`, `unchanged`. A `Hunk { old_start, old_lines, new_start,
  new_lines, section?, lines }`; a `DiffLine { kind, text, cut }`,
  `kind` one of `context`, `added`, `removed`, `no_newline`.
- `BranchList { branches, more }`, `Branch { name, remote?, current,
  default, head, time, upstream?, worktree? }`. `CommitPage { commits,
  more }`, `CommitInfo { id, parents, author, time, subject }`.
  `StashList { entries, more }`, `StashEntry { index, commit, message,
  time }`.
- `GitRefusal`: `needs_credential`, `timed_out`, `non_fast_forward`,
  `diverged`, `would_overwrite { paths }`, `conflicts { paths }` (at
  most 100 paths each), `nothing_to_commit`, `no_identity`, `hook_failed
  { output }` (at most 2 KiB), `no_upstream`, `detached`,
  `checked_out_elsewhere { path }`, `nothing_to_stash`, `stash_moved`
  (the entry at that index is not the commit named), `branch_exists`,
  `no_such_ref`, `operation { operation }`, `remote_failed { message }`
  (credentials removed), `unknown`.
- Outcomes: `CommitOutcome` (`committed { commit }`, `refused`),
  `PushOutcome` (`pushed { remote, branch, upstream_set }`, `refused`),
  `PullOutcome` (`pulled { from, to }`, `up_to_date`, `refused`),
  `FetchOutcome` (`fetched`, `refused`), `SwitchOutcome` (`switched {
  branch }`, `refused`), `RenameOutcome` (`renamed { branch }`,
  `refused`), `StashOutcome` (`done { stashes }`, `refused`), each
  `refused { refusal: GitRefusal }`.
- `PathSet`: `all` or `paths { paths }` (at most 1,000).
- `CommitStage`: `staged` (the default), `tracked` (`add -u` first),
  `everything` (`add -A` first).
- `StashAction`: `save { message?, untracked }`, `apply { index, commit
  }`, `pop { index, commit }`, `drop { index, commit }`: the entry must
  still be that commit, since the stash is shared by every worktree of a
  repository and its indexes move.

Files:

- `DirListing { entries, more }`, `DirEntry { name, kind, size?,
  ignored }`, `EntryKind` one of `file`, `dir`, `symlink`, `other`.
- `FileVersion { size, modified, hash? }`: `modified` in nanoseconds
  since the epoch (Paseo's revision), `hash` the content's `ContentHash`
  for a file of at most 8 MiB (a change within the same tick of a
  coarse clock keeps the size and time; the hash still differs).
- `FileRead { version, content }`, `FileContent` one of `text { text,
  next? }` or `binary`.
- `WriteExpect`: `absent` (create only), `version { version }`, `any`.
- `FoundFiles { matches, indexed, complete, superseded }`, each
  `FoundFile { path, positions }` (`positions` in Unicode scalar values).
- `GrepMode` (`literal`, `regex`: extended), `GrepCase` (`sensitive`,
  `insensitive`, `smart`). `GrepResult { matches, more }`, each
  `GrepMatch { path, line, column, text, cut }`, `line` and `column`
  from 1, `column` in Unicode scalar values.

### Calls

Every method is protocol 10, forwardable through `fleet.forward`, and in
`vessel-web`'s list. "Agents" says whether an agent principal (ADR 0013,
local or delivered) may call it; the guard refuses the others with
`forbidden` before the call runs. An agent already has its own file and
git tools, so it gets what orchestration needs: the projects, and a
worktree for a session it starts.

| Method | Scope | Agents | Request | Reply |
|---|---|---|---|---|
| `project.list` | read | yes | `ListProjects {}` | `ProjectList` |
| `project.get` | read | yes | `GetProject { project, changed_from? }` | `ProjectInfo` |
| `project.add` | operate | no | `AddProject { path, name? }` | `ProjectInfo` |
| `project.clone` | operate | no | `CloneProject { url, parent, folder?, name? }` | `ProjectInfo` |
| `project.init` | operate | no | `InitProject { path, name? }` | `ProjectInfo` |
| `project.remove` | operate | no | `RemoveProject { project }` | `Ack` |
| `project.commands` | read | yes | `GetProjectCommands { project, base? }` | `ProjectCommands` |
| `project.approve` | operate | no | `ApproveCommands { project, digest, approved }` | `ProjectCommands` |
| `worktree.list` | read | yes | `ListWorktrees { project? }` | `WorktreeList` |
| `worktree.create` | operate | yes | `CreateWorktree { project, branch, setup }` | `CreateWorktreeOutcome` |
| `worktree.output` | read | yes | `ReadSetupOutput { worktree, from, wait? }` | `SetupOutput` |
| `worktree.setup` | operate | no | `RunSetup { worktree }` | `WorktreeInfo` |
| `worktree.remove` | operate | no | `RemoveWorktree { worktree, if_lost }` | `RemoveWorktreeOutcome` |
| `worktree.snapshots` | read | no | `ListSnapshots { project }` | `SnapshotList` |
| `worktree.drop_snapshot` | operate | no | `DropSnapshot { project, worktree }` | `Ack` |
| `git.status` | read | no | `GetStatus { root }` | `GitStatus` |
| `git.changes` | read | no | `GetChanges { root, scope, ignore_whitespace }` | `DiffSummary` |
| `git.diff` | read | no | `GetFileDiff { root, scope, path, ignore_whitespace, from_hunk? }` | `FileDiff` |
| `git.show` | read | no | `ShowFile { root, at, path, offset, limit? }` | `FileRead` |
| `git.branches` | read | no | `ListBranches { root }` | `BranchList` |
| `git.log` | read | no | `ReadLog { root, from?, skip, limit? }` | `CommitPage` |
| `git.stashes` | read | no | `ListStashes { root }` | `StashList` |
| `git.stage` | operate | no | `Stage { root, paths }` | `GitStatus` |
| `git.unstage` | operate | no | `Unstage { root, paths }` | `GitStatus` |
| `git.discard` | operate | no | `Discard { root, paths, untracked }` | `GitStatus` |
| `git.commit` | operate | no | `Commit { root, message, stage }` | `CommitOutcome` |
| `git.fetch` | operate | no | `Fetch { root, remote? }` | `FetchOutcome` |
| `git.push` | operate | no | `Push { root, remote?, set_upstream }` | `PushOutcome` |
| `git.pull` | operate | no | `Pull { root }` | `PullOutcome` |
| `git.switch` | operate | no | `Switch { root, branch, create? }` | `SwitchOutcome` |
| `git.rename_branch` | operate | no | `RenameBranch { root, from?, to }` | `RenameOutcome` |
| `git.stash` | operate | no | `StashChanges { root, action }` | `StashOutcome` |
| `file.list` | read | no | `ListDir { root, dir, ignored }` | `DirListing` |
| `file.read` | read | no | `ReadFile { root, path, offset, limit? }` | `FileRead` |
| `file.write` | operate | no | `WriteFile { root, path, text, expect }` | `FileVersion` |
| `file.create_dir` | operate | no | `CreateDir { root, path }` | `Ack` |
| `file.rename` | operate | no | `RenamePath { root, from, to }` | `Ack` |
| `file.delete` | operate | no | `DeletePath { root, path, recursive }` | `Ack` |
| `file.find` | read | no | `FindFiles { root, query, key?, limit? }` | `FoundFiles` |
| `file.grep` | read | no | `SearchFiles { root, query, mode, case, dir?, limit? }` | `GrepResult` |

`session.create` takes a `place` from agents too: `project` on any
project, `worktree` only on a worktree the agent created or one created
for a session it may operate (ADR 0013's ownership). An agent may have
at most 8 worktrees it created. It cannot approve commands, so a
worktree it creates runs setup only when a person approved the set.

What each call does, and its failures. Every call can also answer
`forbidden` (scope or agent), `not_found` for a project, worktree or
snapshot the node does not have, `unsupported` when the node has no
usable git and the call needs it, `unavailable` when the node's git
queue is full (below), and `internal`.

- `project.list`: every project, oldest first (at most 1,000; a node
  refuses to add more).
- `project.get`: one; with `changed_from`, waits (at most 30 s, counted
  in the connection's and the origin's waits as `session.read` waits
  are, and answered at once when the call was forwarded) until its
  `revision` differs from it.
- `project.add`: `path` absolute and an existing folder. Inside a git
  repository the root is the repository's top level. A root already
  known answers that project. `invalid_request` for a relative path, one
  that is not a folder, and a root that is a file system's root, the
  node's home folder, or contains the node's data folder (a read-only
  device could otherwise read the node's token and become its owner).
- `project.clone`: answers at once with the project `cloning`; the clone
  runs in the node (it continues when the caller goes away) as `git clone
  --no-recurse-submodules --progress -- <url> <temporary folder>` with
  `GIT_ALLOW_PROTOCOL` limited to `https:http:ssh:git`, into
  `<parent>/.<folder>.vessel-clone-<8 hex>`, renamed to
  `<parent>/<folder>` when it succeeds (`folder` defaults to the URL's
  repository name, which must make a `FolderName`). `invalid_request` for
  a `parent` that is not an existing absolute folder, or a default folder
  name the URL does not give; `conflict` when `<parent>/<folder>` exists.
  A failure leaves the project `clone_failed` and removes the temporary
  folder.
- `project.init`: creates the folder if needed (`conflict` when it is
  not empty) and runs `git init`; the same root rules as `project.add`.
- `project.remove`: forgets the project; never deletes its files.
  `conflict` while it has worktrees or live sessions. A project still
  cloning is cancelled: git is stopped and its temporary folder removed.
- `project.commands`: the commands the project's file names at `base`
  (the blob read with `git cat-file`, never a checkout) or in the
  project's folder as it is, with whether that set is approved.
- `project.approve`: records (or, with `approved: false`, withdraws) the
  approval of the set whose digest is `digest`, and answers the folder's
  commands. A digest is approved as given, not looked up, since the set
  may be on a branch the folder does not have checked out.
- `worktree.list`: every worktree, or a project's (at most 1,000).
- `worktree.create`: answers at once. `needs_approval` when `setup` is
  `run` and the base's commands are listed but not approved (nothing is
  created); otherwise `git worktree add` (hooks off) into `<data
  folder>/worktrees/<project id>/<8 hex>`, then setup in the background
  (it continues when the caller goes away), reading the commands from
  the blob at the base, never from the checkout.
- `worktree.output`: a page of at most 64 KiB from `from`; with `wait`
  (seconds, at most 30, counted like `project.get`'s), waits for more
  output or the end of setup.
- `worktree.setup`: runs setup again (after an approval, a failure, or
  `skip`), with the commands of the worktree's file as it is now.
  `conflict` while setup or teardown runs, or when that set is not
  approved.
- `worktree.remove`: removes the worktree by the rule below. With
  `if_lost: keep`, a worktree where something would be lost answers
  `kept`; with `snapshot`, a snapshot is kept first (`kept` when the
  snapshot is refused); with `discard`, it is removed anyway and its
  branch deleted. `kept` with `in_use` while a live session is in it or
  setup runs.
- `worktree.snapshots`, `worktree.drop_snapshot`: a project's snapshots
  (at most 1,000); dropping one deletes its ref. A snapshot is restored
  by `worktree.create` with `branch: snapshot { worktree }`.
- `git.status`, `git.changes`, `git.diff`, `git.show`, `git.branches`,
  `git.log`, `git.stashes`: `unsupported` for a project without git;
  `invalid_request` for a base, ref or commit that does not resolve.
  `git.show` pages a file at a commit or ref as `file.read` pages one on
  disk (the old side of a diff).
- `git.stage`, `git.unstage`, `git.discard`: answer the status after.
  `discard` restores tracked paths from `HEAD` (index and working tree)
  and, only with `untracked: true`, deletes the untracked ones it names
  (never ignored files). `unavailable` when another git holds the index
  lock.
- `git.commit` with the repository's configured identity
  (`user.useConfigOnly`, so `no_identity` rather than a guessed one);
  the repository's hooks run, as they would for the user. `git.fetch`:
  the remote (default: the branch's, else `origin`), pruning nothing.
  `git.push`: the current branch to its upstream, or to `remote` with
  `set_upstream`; never forced; `pre-push` runs. `git.pull`:
  fast-forward only (`diverged` otherwise). `git.switch`: an existing
  branch, or a new one from `create` (a start point). `git.rename_branch`:
  `from` (the current branch when absent) to `to`; a worktree's
  `created_branch` follows. `git.stash`: as named.
- `file.list`: one folder's entries, folders first, then by name; `.git`
  left out; `ignored` marks what git ignores (asked with `git
  check-ignore --stdin -z`), listed only when `ignored` is asked.
- `file.read`: a page of at most 1 MiB (256 KiB by default) from
  `offset`, cut back to a character boundary; `binary` when the first
  8 KiB hold a NUL byte or the page is not UTF-8.
- `file.write`: at most 1 MiB of text, written to a temporary file in the
  same folder and renamed over the target (keeping its permissions). A
  target that is a symbolic link is written where the link points when
  that is inside the root (an editor saving `CLAUDE.md` that links to
  `AGENTS.md`), never replacing the link. `conflict` when `expect` does
  not hold.
- `file.create_dir`, `file.rename` (never over an existing path:
  `conflict`), `file.delete` (a folder only when empty, unless
  `recursive`; never the root; a link is removed, not what it points to).
- `file.find`: by name over an index of the root's files: `git ls-files
  --cached --others --exclude-standard -z` for a repository, a walk
  otherwise; built on the first search, kept while used and rebuilt in
  the background when older than 10 s. A newer `file.find` with the same
  `key` from the same principal replaces one still running (`superseded:
  true`, no matches).
- `file.grep`: `git grep -e <query>` over tracked and untracked files that
  are not ignored (`--no-index` for a folder without git), binary files
  left out.

**Every path is checked.** A `RelPath` that decodes cannot be absolute
or climb; the node joins it to the root part by part, resolves the
result with links followed (for a target that does not exist yet, its
parent), and refuses a result outside the root's resolved folder, under
any `.git` below the root, or inside the node's data folder (except a
worktree's own folder). A path that leads out and a path that is not
there get one answer, `not_found` ("no such path in this folder"), so a
caller cannot learn what exists outside. These are checks inside the
node, not a sandbox: a process of the same user can still race them (as
ADR 0016 says for the language servers).

**How git runs** (ADR 0023): the `git` program, never through a shell,
arguments after `--end-of-options` or `--` where they could read as
options or paths, with `GIT_LITERAL_PATHSPECS=1` (no `*` or `:(magic)`
in a path), `GIT_TERMINAL_PROMPT=0`, `GCM_INTERACTIVE=never`,
`GIT_ASKPASS=` and `SSH_ASKPASS_REQUIRE=never` (a credential helper
that needs no person still works), `GIT_NO_LAZY_FETCH=1`, standard input
closed, its own process group, and `-c core.fsmonitor=false -c
core.hooksPath=<an empty folder of the node's> -c diff.external= -c
log.showSignature=false` (and `--no-ext-diff --no-textconv` on diffs):
no read and no worktree creation runs a program the repository's
configuration names. Only `git.commit` and `git.push` run the
repository's hooks, as they would for the user. Limits per run: 30 s
(local), 120 s (fetch, push, pull), 1 h (clone); 16 MiB of standard
output (a call that reads less says so); the last 64 KiB of standard
error. Two pools: 8 local runs and 4 network runs at once, each with at
most 64 waiting; past that, `unavailable`.

### Where a session runs, and tidying

- `session.create` with a `place` resolves it and starts at once: never
  waiting for setup. In a worktree whose setup is `running`, it fails
  with `conflict` (wait on `worktree.output` first); one whose setup
  failed or was skipped starts, and says so. The session's log starts
  with `vessel:` notices: the folder, the branch and its base, and
  setup's result with its last lines.
- To start in a new worktree a client calls `worktree.create`, follows
  `worktree.output` until setup ends, then `session.create { place:
  worktree }`; the CLI and the agent tool do these three steps for their
  callers.
- **Tidying** applies to a worktree created for a session, when that
  session is archived or deleted (track A's calls) or its log is
  dropped by the node's retention, and no other live session is in it.
  It is the `if_lost: keep` removal:
  1. inspect: no operation in progress; `git status --porcelain
     --untracked-files=all --ignore-submodules=none` prints nothing; no
     submodule; no assume-unchanged or skip-worktree entry; no
     `refs/worktree/*` or `refs/bisect/*`; and no commit reachable from
     `HEAD` or `created_branch` that no other branch, remote-tracking
     branch, tag or Vessel snapshot has (`rev-list --not --exclude=<own>
     --branches --remotes --tags --glob=refs/vessel/*`);
  2. teardown, when its commands are approved;
  3. inspect again;
  4. `git worktree remove` without `--force` (git refuses a changed
     worktree itself, the last word), then `git branch -D` of
     `created_branch`.
  Ignored files (build output, dependencies) are not counted: they are
  what setup makes again. Otherwise the worktree stays, `kept` saying
  why, until a person removes it.
- A **snapshot** is one commit whose parent is `HEAD` and whose tree is
  the working tree as `git add -A` would stage it (untracked files
  included, ignored ones not), made through a temporary index (the
  worktree's own is untouched), by a fixed identity (`Vessel`), and kept
  at `refs/vessel/snapshots/<worktree id>`; the ref also keeps every
  commit the branch had. Refused, so the worktree is kept, above 100 MiB
  for one changed file or 256 MiB in all, with an operation in progress,
  and when the working tree holds a nested repository (it would become a
  bare gitlink and its files would be lost). Staged and unstaged changes
  are not told apart in it.

### The project's file and its trust

- `vessel.toml` at the repository's root (the folder's, for a project
  without git). TOML, as the node's other files are (`profiles.toml`,
  the node book), with comments. Only `[worktree]` is read now:

  ```toml
  [worktree]
  setup = ["npm ci", "cp \"$VESSEL_PROJECT_ROOT/.env\" ."]
  teardown = "docker compose down"
  ```

  A string or a list of strings each. Any other table or key is left
  for later tracks (`[scripts.*]`, `[services.*]`, `[env]`), so a file
  written for them parses here. At most 64 KiB, 32 commands per list.
- Commands run in order, in the worktree, through the platform's shell
  (`/bin/sh -c` on Unix; on Windows `cmd.exe /d /c` on a temporary `.cmd`
  file holding the command, so no quoting is involved), with the node's
  environment (as a terminal the user opens) plus `VESSEL_PROJECT_ROOT`,
  `VESSEL_WORKTREE` and `VESSEL_BRANCH`, standard input closed, as a
  process group stopped together. The first failing setup command stops
  setup. Limits: 30 minutes for a setup, 5 for a teardown; the output
  log 1 MiB per worktree.
- **Trust.** A repository's commands are someone else's code. They run
  only when a person has approved that exact set for that project. The
  digest is SHA-256 over `vessel.toml worktree commands 1\n`, then for
  each of `setup` and `teardown` its name, its count and each command,
  every count and length as 8 bytes little-endian: any change to a
  command, its order or its list is another set, asked again (a new
  branch, a pull, an agent editing the file in its worktree). The
  commands that run are the ones parsed from the blob that matched the
  approved digest, never a re-read. `project.approve` needs `operate`
  and is refused to agents. What an approval cannot cover: a command
  that runs a script of the repository (`./setup.sh`, `npm ci` running
  package scripts) runs whatever that script says on that branch.

### What an older client and an older node see

- A client asks no protocol 10 method of a node that speaks 9 or less:
  it fails with `NotInProtocol` before sending. A `session.create` with
  a `place` needs 10 (`Call::since`): an older node would start the
  session in its home folder instead.
- A protocol 9 client reading a protocol 10 node sees sessions with a
  `cwd` (the node filled it) and ignores `place` and `workspace`.
- No event is added, so no stand-in is needed.

### Bounded, and by how much

| What | Bound | Beyond it |
|---|---|---|
| A git run | 30 s, 120 s (network), 1 h (clone); 16 MiB of stdout, the last 64 KiB of stderr | killed as a group; `timed_out` or `internal` |
| git runs at once | 8 local, 4 network, 64 waiting each | `unavailable` |
| Projects; worktrees; snapshots | 1,000 each; 8 worktrees per agent | refused; `more: true` |
| Status entries, files in a summary | 5,000 | `more: true` |
| A file's diff | 5,000 lines per call; 4 KiB a line; 4 MiB of git output | `more_from`; `cut`; `too_large` |
| Branches; a log page; stashes | 1,000; 200 commits; 100 | `more: true` |
| Paths in a refusal; a hook's output | 100; 2 KiB | cut |
| Folder listing | 5,000 entries | `more: true` |
| File read; git.show; write | 1 MiB a page; 1 MiB | `next`; `invalid_request` |
| Find by name | 200 results; 500,000 paths per index, 8 indexes, idle ones dropped after 5 minutes | `complete: false` |
| Search in files | 500 matches; 512 characters a line; 4 MiB of git output | `more: true`; `cut` |
| Setup and teardown | 30 and 5 minutes; 1 MiB of output per worktree; 64 KiB a page | `failed { timed_out }`; `truncated` |
| A long poll (`project.get`, `worktree.output`) | 30 s, inside the node's wait shares | answers as it is |

Store: schema 7 adds `projects`, `worktrees` and `approvals` (the next
free number; the coordinator renumbers if track A takes it too). (It
did: 9 after the merge, the plan's "Integration (node)".)

### Not in this track (wave 2), and the doors left open

- A forge (pull requests): `WorktreeInfo` and `GitStatus` gain fields.
- Checkpoints per turn: snapshots are built the way a checkpoint would
  be (a commit through a temporary index under a Vessel ref).
- Carrying a worktree when a session moves to another machine: the
  session's `workspace` names what to carry.
- Scripts and services: `vessel.toml` keeps every key it does not read;
  the digest's version line changes when more of the file runs.

### Review (2026-11-11)

An independent reviewer read the first version against the references
and Vessel's wire. Taken, with the change above: git's hardened
invocation (hooks, `fsmonitor`, external diffs, text conversion,
signatures and lazy fetches off; hooks only on commit and push; the
project's file read as a blob); roots that would expose the node's
token refused; `FolderName` for a clone's folder; literal pathspecs and
`-e` for grep; `session.create` never waits for setup (the three-step
start, `new_worktree` removed from `SessionPlace`); tidying on archive
or deletion of the session a worktree was created for, not on its
process ending (track A's "start again" resumes in it), counting every
live session inside it; the inspection widened (submodules, flagged
files, per-worktree refs, the created branch rather than `HEAD`) and
run again after teardown, with git's own non-forced removal last;
snapshots refused over nested repositories, made by a fixed identity,
restorable as uncommitted changes, and `if_lost: discard` for when one
is refused; typed outcomes for `worktree.create` and `worktree.remove`;
`unknown` on every reply enum that will grow, and `missing` for a
project whose folder is gone; stash entries named by index and commit;
`GIT_ASKPASS=` and `user.useConfigOnly`; any user information refused in
`http(s)` URLs, clones without submodules and with the protocols
limited; git's `verify_path` rules for `.git`, any `.git` refused after
resolution, one answer for missing and escaping paths; `git.fetch`,
`default_branch`, `git.rename_branch`, `git.show` and
`ignore_whitespace`; a content hash in `FileVersion`; writes through an
inside link; search keys instead of "per connection"; a network pool,
bounded queues, waits inside the node's shares and answered at once
when forwarded, "differs" rather than "past", every list bounded, eight
worktrees per agent; the digest's encoding and the limit of what an
approval covers stated; units for positions and columns; `unchanged`,
`submodule` and mode changes in a file's diff; the worktree folder
keyed by project id.

Not taken, and why:

- *Not forwardable `project.add`, `clone`, `init` and `approve`.* A call
  delivered from another node holds at most what this node's owner
  granted that node's devices (ADR 0013's intersection), and a fleet is
  one person's machines; GOAT and Paseo manage projects across machines.
  The roots that would widen a reader's reach are refused for every
  caller instead (above).
- *Recording a repository's executable configuration at `project.add`
  and asking again when it changes.* `.git/config` is written by git on
  a clone and by the user otherwise; it is not repository content. The
  reads Vessel makes switch off every configured program they do not
  need (above), which is what agent-orchestrator does. Filter drivers
  (`git lfs`) still run on status, as they do for the user's own `git
  status`; ADR 0023 says to add only folders one would run git in.
- *A new session per git process (`setsid`)*: it needs `unsafe`
  `pre_exec` or a new crate. On a node started from a terminal, an ssh
  or gpg prompt opened on `/dev/tty` stops the process (a background
  group reading its terminal), and the time limit ends it; a node run
  as a service has no terminal.
- *A filtered environment for setup commands.* They are the user's
  approved commands and need the user's tools (`PATH`, version
  managers), as a terminal the user opens has; harnesses get the same
  environment.
- *Counting new ignored files as something lost* (GOAT's
  `any-local-files`, off by default there too): `node_modules`,
  `target` and the like would keep every worktree forever. Stated
  instead: what must survive must not be ignored.
- *Merging from the base* (Paseo only) and *snapshot expiry* (deleting
  data without a person): later, if asked for.
- *Keeping setup's output in the session's log*: the session's log
  gets setup's result and last lines as notices; the whole output stays
  with the worktree, which outlives a session and is shared by the
  sessions in it.

## Track W: contract

Track W adds no call to a node and does not change protocol 10 (or the
protocol number). Everything below is between the page and `vessel ui`'s
bridge (ADR 0017): ops and one pushed frame in `vessel-web`'s wire
(`crates/vessel-web/src/wire/prefs.rs`), generated into
`web/src/wire/bridge.ts`. The page and the bridge ship in one binary, and
a restarted UI refuses old tabs (a new origin and new keys), so a page
never meets a bridge of another version. The versions that can differ are
those of two `vessel` binaries reading one file; the rules for that are
under "The files".

The decisions that could have gone another way are ADR 0028 (taken as
0025 on the track's branch and renumbered at the integration: 0023 and
0027 are track B's, 0024 to 0026 track A's). A reviewer read this contract
against the code and the references; what changed and what was not taken
is under "Review" at the end of this section.

### Where things are kept

`NodeSource` (the seam through which the CLI gives the bridge its nodes)
gains one method:

```rust
/// Where the page's preferences and drafts are kept. A field that is
/// `None` is kept in memory for this launch. The default keeps both
/// in memory (a source with no disk, such as the bridge's tests).
fn preferences(&self) -> Result<PageFiles, Self::Error> { Ok(PageFiles::default()) }

pub struct PageFiles { pub preferences: Option<PathBuf>, pub drafts: Option<PathBuf> }
```

`vessel-cli`'s `BookSource` answers:

- **Preferences** in the book's directory, `ui-preferences.json`, so
  `--config-dir` moves them with the book.
- **Drafts** in the local data directory (`--data-dir`, default
  `~/.vessel`), `ui-drafts.json`. A draft is prompt text: the data
  directory already holds the sessions' logs, while the configuration
  directory is often a dotfiles repository, and on Windows it roams to a
  server.

The bridge asks once when `vessel ui` starts:

| Answer | Kept | The page says (Settings, Appearance and General) |
| --- | --- | --- |
| a path, the file absent or readable | in that file | where |
| `None` | in the bridge's memory, for this launch | "Kept until vessel ui stops: this machine gives the page no place to keep them." |
| `Err(error)` (no home directory, say) | in memory, for this launch; the reason logged once at start | "Kept until vessel ui stops: " and the error |
| the file cannot be read (too large, not JSON, not a regular file, a newer `version`, an I/O error) | in memory, for this launch; the file is not touched | "Kept until vessel ui stops: " and why, naming the file, with "Fix or remove it, then restart vessel ui." |
| a read or write fails later in the launch | from then on in memory, for this launch; every tab is pushed the new state | as above |

### The files

- JSON, one object each: `{ "version": 1, ... }`. **A version is raised
  only by a change an older reader would misread.** A reader that finds
  a higher version keeps that file's contents for this launch only and
  never writes it; additive changes (a new setting, a new value) keep
  version 1 and rely on the next rule.
- **What it does not know is kept.** A key this bridge does not know is
  kept as it was (`RawJson`, opaque) and written back. A known key whose
  value it cannot read (a value a newer page wrote) is kept raw the same
  way, and the page is given that setting's default; a change to that
  setting replaces it. The unit kept whole is a setting; in a map, one
  entry (one action's keys; one machine's defaults; one agent's defaults
  under it, an unknown agent kind or choice kind included); one draft.
- At most 256 KiB for preferences and 1 MiB for drafts, read and
  written. Read safely: the path's metadata must say a regular file, the
  open handle's must agree, and at most the bound plus one byte is read.
  A change that does not make the document larger always succeeds; one
  that would pass the bound fails (`failed`) and changes nothing. The
  bounds below keep every known setting far under it.
- Written owner-only: a temporary file in the same directory created
  with mode `0600` (on Unix; on Windows it takes the directory's
  permissions, the user's profile), flushed, then renamed over the old
  name, which replaces whatever is there, a link included, and never
  writes through it. A directory the bridge creates is `0700` (the last
  component only; parents are left as they are). The book is written
  with temp-and-rename too, but with the default mode; these files do
  not follow it there.
- No lock between processes, as the book has none: each change reads
  the file right before it, applies itself and renames (drafts: read,
  merge this process's writes by key, rename). Two `vessel ui` processes
  changing one setting in the same instant can lose one change.

### What the preferences hold

Each section's fields have defaults; a missing section or field is its
default.

| Section | Field | Values | Default |
| --- | --- | --- | --- |
| `appearance` | `theme` | `system`, `light`, `dark` | `system` |
| | `interface_size` | whole pixels, 12 to 20 | 14 |
| | `code_size` | whole pixels, 10 to 18 | 12 |
| | `content_width` | `comfortable` (768px), `wide` (1152px), `full` | `comfortable` |
| | `conversation` | `full`, `collapse_tools` | `full` |
| `layout` | `sidebar` | `shown`, `hidden` | `shown` |
| | `inspector` | `shown`, `hidden` | `shown` |
| `composer` | `send` | `enter`, `mod_enter` | `enter` |
| `shortcuts` | (map) | `ActionId` to at most 2 `KeyBinding`s; an empty list is "not set"; an absent action has its default. At most 64 actions | empty |
| `notifications` | `alerts` | `off`, `system`, `sound`, `system_and_sound` | `off` |
| | `title` | on or off: a count in the tab's title | on |
| `sidebar` | `group` | `machine`, `status` | `machine` |
| | `sort` | `created`, `activity` | `created` |
| | `ended` | on or off: list ended sessions | off |
| `defaults` | (map) | `NodeId` to `MachineDefaults`. At most 32 machines | empty |

`MachineDefaults { agent: HarnessKind?, agents: HarnessKind to
AgentDefaults }`; `AgentDefaults { profile: ProfileName?, choices:
ChoiceKey to ChoiceValue }`, at most 8 choices. The choices are the
agent's own option ids and values, remembered from the last session
started with that agent on that machine (agent-orchestrator's and
Paseo's "last used"), and applied only while the agent still offers
them.

Types parsed where the frame is decoded:

- `ActionId`: 1 to 48 characters of `a-z`, `0-9` and `-`, starting with
  a letter. The page's own ids; the bridge does not know the set.
- `KeyBinding`: one chord or two in sequence, separated by one space, at
  most 32 bytes. A chord is modifiers then a key, joined by `+`; the
  modifiers are `Mod` (Command on a Mac, Control elsewhere), `Ctrl`,
  `Alt`, `Shift`, `Meta`, each at most once and in that order. **A key
  names a physical key** (the browser's `KeyboardEvent.code`), written
  as it is on a US keyboard without Shift: `A` to `Z`, `0` to `9`,
  `` ` - = [ ] \ ; ' , . / ``, or one of `Enter`, `Escape`, `Tab`,
  `Space`, `Backspace`, `Delete`, `Up`, `Down`, `Left`, `Right`, `Home`,
  `End`, `PageUp`, `PageDown`, `F1` to `F12`. So `?` is written
  `Shift+/`, and each key press has one spelling. Matching by position
  is what makes `Alt+[` and `Alt+1` work on a Mac (Option types another
  character) and `N` work while a Korean input method is on; on a layout
  that is not QWERTY-shaped, a shortcut is where the key is, not what it
  types.
- `InterfaceSize`, `CodeSize`: integers in their ranges.
- `ChoiceKey`: an `OptionId` of at most 64 bytes; a select's
  `ChoiceValue` of at most 64 bytes. Longer ones are not remembered.
- `NodeId`, `HarnessKind`, `ProfileName`, `ChoiceValue`: the protocol's
  own.

### Ops and the pushed frame

| Op | Result | Failures |
| --- | --- | --- |
| `preferences` | `preferences { preferences: PreferencesState }`, after reading the file again; another process's change found there is pushed to every tab | `busy` |
| `prefer { change: PreferenceChange }` | `prefer { preferences: PreferencesState }`: the whole document after the change | `unreadable` (the change does not decode: a size out of range, a bad binding, too many entries in the change); `failed` (more than 64 actions or 32 machines after it, past the size bound; nothing is changed); `busy` |
| `drafts` | `drafts { drafts: DraftsState }`: every draft, after reading the file again | `busy` |
| `draft { key: DraftKey }` | `draft { text: Option<String> }`: one draft, read again | `busy` |
| `save_draft { key: DraftKey, text: DraftText }` | `save_draft` | `unreadable` (over the bound, a bad key); `busy` |

- **Order.** Ops on preferences and drafts are handed, from the tab's
  read loop and in the order its frames arrive, to one task that owns
  each file (a bounded queue of 64; a full queue answers `busy`). They
  do not take the tab's permits for calls to nodes, so a draft saved as
  the page is hidden is not refused because node calls are in flight.
  `save_draft` is answered when queued; its write follows.
- `PreferencesState { revision, kept: Kept, values: Preferences }`.
  `revision` rises by one with every state the bridge publishes in this
  launch; the page drops a reply or a push older than what it shows.
  `Kept` is `file { path }` (the path as display text) or `this_launch {
  why: LaunchOnly }`, and `LaunchOnly` is `no_place`, `no_path { message
  }`, `unreadable { path, message }` or `newer { path, version }`, so the
  page tells them apart without reading words. `DraftsState { kept:
  Kept, drafts: Vec<Draft> }`.
- `PreferenceChange` (tagged `set`, its value in `value`), each replacing
  one setting: `theme`, `interface_size`, `code_size`, `content_width`,
  `conversation`, `sidebar_panel`, `inspector_panel`, `send`, `alerts`,
  `title_count`, `sidebar_group`, `sidebar_sort`, `sidebar_ended`;
  `shortcuts { edits }`, at most 16 `{ action, to }` where `to` is
  `bindings { keys }` or `default`, applied together (a reassignment
  moves a key from one action to another in one change);
  `reset_shortcuts` (every override, unknown actions' included);
  `default_agent { node, agent }`; `agent_defaults { node, agent,
  profile, choices }`; `forget_defaults { node }`.
- `ServerFrame::Preferences { preferences: PreferencesState }`: every
  state the bridge publishes (after a change, or a change found in the
  file), to every tab, the tab that asked included. Held as a `watch`
  (the latest only): a tab that reads slowly skips states it would have
  overwritten anyway. Sent once to a tab after its welcome.
- `DraftKey` (tagged `kind`): `new` (the new-session page, one) or
  `session { session: SessionId }`. A session id is minted by its node
  and unique across machines, so a renamed machine keeps its drafts.
  `DraftText`: at most 128 KiB of UTF-8 (agent-orchestrator's limit for a
  message); the empty text removes the draft. `Draft { key, text, at }`,
  `at` when it was last saved.

### Two tabs, and two processes

- Preference changes from every tab go through one task, in the order
  they arrive; each replaces only its own setting, on the file read right
  before it, so two tabs changing different settings both keep their
  changes and the same setting ends as the later change set it. Every
  tab is pushed the whole result with its revision.
- Drafts are not pushed (a push would overwrite what another tab is
  typing). The page reads all drafts when it connects, reads one again
  when a composer opens and nothing has been typed into it yet, and
  writes each through. A tab's unsaved text is never replaced by a read,
  and is saved again after a reconnect. Two tabs typing into one draft
  end with the later save.
- Between processes, as for the book: read right before, rename after.

### Drafts

- The page keeps a draft in memory on every key and saves it at most
  once per 500 ms of typing, and at once when the page is hidden
  (`visibilitychange`, `pagehide`), the composer is left, or the text is
  sent. Sent text is cleared only once the send (or the new session)
  succeeded. The page measures the bound in UTF-8 bytes (`TextEncoder`)
  and sends well-formed text (`toWellFormed`); a longer draft stays in
  the tab's memory and the composer says it is not kept.
- The bridge writes at most once a second, taking every save queued
  meanwhile in one read-merge-rename. At most 64 drafts and 1 MiB: past
  either, the drafts saved longest ago go first; a draft not saved for
  30 days goes too (a session deleted elsewhere leaves its draft
  behind).

### The first paint

The page's origin is new on every launch (ADR 0017), so a theme kept in
the browser is gone each time. The bridge writes the appearance into the
index it serves, on `<html>`: `data-theme` (absent for `system`),
`data-ui-size`, `data-code-size`, `data-content-width`, and the
stylesheet has a rule for each value, so nothing is inline. The values
come from the preferences task's current state (memory, no read per
request), each from a closed set or a bounded integer written by the
bridge; they are spliced at the one `<html lang="en">` of the built
index (a test checks there is exactly one; the placeholder page is
served as it is). The index is served before a tab proves its key, to
anything on this machine that can reach the port (by the launch's
address or a literal loopback name) and to whatever a port forward
exposes it to; these values say nothing but how the page looks. The page
reads them on boot.

### On the page (no wire)

- **Shortcuts.** One registry (`web/src/data/shortcuts/`), one listener.
  Actions, groups and default keys (Mod is Command on a Mac):
  General: the command palette (`Mod+K`, `/`), new session (`N`),
  Settings (`Mod+,`). Navigation: the board (`G S`), Machines (`G M`,
  `G F`), Sync (`G Y`), Devices (`G D`), previous and next session
  (`Alt+[`, `Alt+]`), session 1 to 9 (`Alt+1` to `Alt+9`, fixed).
  Session: find (`Mod+F`), focus the composer (`Mod+L`). Panels: the
  sidebar (`Mod+B`), the summary (`Mod+Shift+B`). Rules: a chord holding
  Mod, Ctrl, Alt or Meta fires while typing in a field; one without
  (`N`, `/`, `G S`) stands down. No key fires while an input method is
  composing. In a terminal off a Mac, nothing fires (the terminal owns
  Control). A sequence's second key comes within 1.5 s. A chord is
  compared after Mod is resolved for the platform (`Mod+Ctrl` is one
  modifier off a Mac). A key a person records needs Ctrl, Alt, Command or
  Meta, and is refused when the browser keeps it (`Mod+N`, `Mod+T`,
  `Mod+W`, `Mod+Q`, `Mod+1` to `Mod+9`, `Mod+Tab`) or text editing uses
  it (`Mod+A`, `C`, `V`, `X`, `Z`). A key another action holds is offered
  to be moved (agent-orchestrator's "Reassign"); a fixed one is refused;
  a sequence and the key it starts with clash. Overrides are matched
  before defaults; a clash found in what is kept (two tabs, or a newer
  default) is shown on both rows, not resolved silently. The defaults
  take some keys a browser would otherwise use, on purpose, and only
  while the page has them to do: `Mod+F` (find, in a session), `Mod+L`,
  `Mod+B`, `Mod+Shift+B`, `Mod+,`, `Alt+1` to `Alt+9`; the end-to-end
  tests press each in Chromium.
- **Composer.** Enter sends and Shift+Enter starts a line; with
  `mod_enter`, Enter starts a line and Mod+Enter sends. Mod+Enter also
  sends in `enter` mode (track A's steering may take it later). A key
  that ends an input method's composition never sends. Up and Down
  recall the session's own prompts (`data/recall.ts`): from an empty
  composer, no modifier, not while composing; within a recalled
  multi-line prompt the arrows move the caret until its first or last
  line; Down past the newest empties the composer. The new-session page
  has no history.
- **Find** over `Timeline.entries`' prompts and messages (the page holds
  a session's whole log, windowed or not), never the document; the
  timeline exposes one seam to reveal an entry by key. **The outline**
  lists the prompts beside the timeline (GOAT's minimap, Paseo's
  outline), from the same entries.
- **Notices** (`data/notices.ts`): "needs you" is the board's `blocked`
  family (ADR 0019) entered between two polls; "finished" is a session
  that was working and is now neither working nor waiting (its turn
  ended, or it exited). A session seen for the first time is never
  news; a node older than protocol 8 only ever "finishes", when its
  session exits. While notices are on, a hidden page keeps polling
  (browsers slow a hidden tab's timers to about once a minute after five
  minutes, so a notice can be that late). A system notification carries
  a tag per session and kind, so two tabs show it once; the title's
  count is of notices since the page was last in front. The browser
  grants notifications per origin and the origin is new each launch, so
  permission is asked again every launch, from a click (Firefox asks
  only from one); the page and the docs say so. Sound plays after the
  page's first click or key (browsers allow no sound before).
- **Below 768px** the sidebar (and with it Settings' sections) opens as
  a sheet from a button in the title bar, as GOAT's does; picking a row
  closes it.
- An untitled session without a folder is "Untitled session", not `~`.
- `bridge/client.ts` is divided (done: `1a141db`).
- **Branded ids** (their own commits, after the rest): the generator
  (`vessel-proto/src/ts.rs`) writes an opaque handle the node or the
  bridge mints and the page only hands back as `string & { readonly
  __brand: "SessionId" }`. Every alias of a primitive in the generated
  files is classified in one table, branded or plain with its reason
  (typed by a person, a quantity, a secret), and a test fails on one in
  neither. The page gives a brand only through a checking function at
  its two edges (route parameters, frames), and one helper for test
  fixtures. No Rust wire type changes.

### Bounded, and how

| What | Bound | When reached |
| --- | --- | --- |
| The preferences file | 256 KiB (the bounds below fit in under 100 KiB) | a change that grows it past fails; a file past it is not read (this launch only) |
| Shortcut overrides | 64 actions, 2 keys each, 16 edits per change | too many edits: does not decode; a 65th action: `failed` |
| Defaults | 32 machines, 8 choices per agent, ids and values of 64 bytes | too many choices: does not decode; a longer id or value is not kept; a 33rd machine: `failed`, and its sessions start with nothing remembered |
| The queue of preference and draft ops | 64 | `busy` |
| Drafts | 64, 128 KiB each, 1 MiB in all, 30 days unsaved | a longer draft is refused (the page keeps it in memory only); otherwise the oldest go |
| Draft writes | one a second | saves queued meanwhile are merged into it |
| Pushes to a tab | the latest state only (`watch`) | earlier ones are skipped |

### Review

An independent review (Opus, read-only) found sixteen problems in the
first version (commit `7794f8e`). Changed in response:

- Notices could never fire from a hidden tab (the fleet polls only while
  visible): it keeps polling while notices are on, and the delay a
  hidden tab's timers cause is said.
- Ops were run as concurrent tasks, so two changes from one tab could be
  applied out of order, and a reply could overwrite a newer push: one
  queue per file, fed from the read loop in order, and a `revision` the
  page uses to drop stale states. The `preferences` op reads the file
  again and publishes another process's change.
- Drafts were read once per connection, so a long-open tab could
  overwrite newer text: `draft` reads one again when a composer opens,
  unsaved text is never replaced and is saved again after a reconnect,
  and every drafts write reads and merges the file first. Draft ops do
  not use the node calls' permits.
- The bounds contradicted the file's size: ids and values are bounded,
  the counts lowered (32 machines, 8 choices, 64 actions) and the file
  raised to 256 KiB, so every known setting fits; a change that does
  not grow the document always succeeds.
- `version` now means an incompatible change (a newer one is read for
  this launch only and never written); the unit kept whole is named.
- Keys match by physical position (`code`), one spelling per press;
  sequences time out; Mod is resolved per platform before comparing;
  the keys taken from the browser are listed and tested.
- Drafts are keyed by session id alone (a renamed machine kept stale
  keys), live in the data directory (not a roaming or dotfiles
  configuration directory), expire after 30 days, and are cleared only
  after the send succeeds.
- Defaults separate the default agent from an agent's choices, keep the
  profile, and hold choices as a map (one value per option).
- A failure in mid-launch moves storage to this launch and says so; the
  read checks for a regular file and stops at the bound.
- Missing behaviours written in: recall, the send key with input
  methods, the outline, notices' meaning, tags and title count, Settings
  below 768px, the untitled name.
- The first paint uses data attributes, read from memory, spliced at a
  place a test pins; the exposure is stated as it is.
- Smaller: `set`'s value is in `value`; the path is display text; the
  draft bound is measured in UTF-8 bytes on the page; draft writes are
  once a second at most.

Not taken, and why:

- A typed `full` failure for a document at its bound: with the bounds
  above it cannot be reached by this page's settings, only by unknown
  keys a newer page wrote, and the page then says the message as it is.
- Reporting which settings were read as defaults because their stored
  value was unreadable: only a newer page writes such a value, and this
  page could not show it anyway; the newer page reads its own value back
  unchanged.
- A helper in `vessel-client` shared with the book's writer: the book
  keeps its own mode (another track's file), and this track's scope in
  `crates/` is `vessel-web` and the one method in `vessel-cli`.
- Cleaning up a temporary file a crash left between create and rename:
  another `vessel ui` may be writing it at that moment; it is a hidden
  file of at most the bound, named by its process.
- The panels (`layout`) per tab: agent-orchestrator keeps the sidebar's
  state for the app, and the preferences are the page's only memory
  across launches; a toggle is a human-rate write.

## Left open

- The desktop and phone shells (ADR 0020, still proposed). All three
  references ship a phone app; Vessel's answer until the decision is the
  page on a phone plus push (track E).

## Track P: progress (2026-11-11)

Built: the benchmarks (`benches/` in vessel-store, -proto, -rpc,
-session, -web), the load test (`vessel-cli`'s `load`), the page's
measurement (`web/e2e/perf.ts`), and the baseline, all in
`docs/PERFORMANCE.md`. Changed, each with its number before and after
there: compressed responses made at build; Pretendard in its subsets;
the page divided by screen with every chunk pinned by the index's import
map (ADR 0017's amendment); memoised timeline entries, a windowed
timeline and the reducer's index by id; an unchanged session list draws
nothing; fat LTO with one codegen unit for release builds. Measured and
left: the terminal (xterm's renderer), the store's write path, fan-out.
Described and left: a session list subscription (a wire change).

## Track A: progress

Built in seven slices, each committed with its tests; every call,
event and stand-in of the contract is built. (While a slice was not
built, its calls answered `unsupported`, the refusal an older node
gives.)

- Wire (protocol 10): every type, event, call and stand-in of the
  contract, the envelope's per-peer rewriting, the TypeScript, and the
  page's model carrying the new events (`web/src/data/conversation.ts`,
  stored, not drawn). Store schema 7 with every column and the
  attachments table.
- Slice 1, what the agent reports: abilities from `initialize`
  (steering from `_meta.steering.supported`), commands, context usage
  and a turn's tokens (`unstable_end_turn_token_usage`), deduplicated;
  `session.conversation` from the session's projection while it runs
  and from the store after; the CLI shows them. Agents send their
  commands when they like, before or after their answer to
  `session/new`, so nothing orders them against `Ready`.
  A probe that fails now reads the agent's last stderr lines after the
  disconnect (they are read apart and could come later; a test of
  protocol 9 failed now and then on it).
- Slice 2, delivery and the queue: `session.prompt` (`start`, `queue`,
  `steer` through `_session/steering` with `promptRequired`,
  `interrupt` bounded at 10 s), `session.queue` (edit by revision, move
  before an entry, remove, send now with the entry put back on failure,
  hold, resume, clear), and `cancel_turn`'s `turn`. Every prompt and
  cancel now goes through the session's actor, which awaits the agent
  while still publishing its output (the move's `pump`). The queue moves
  from the actor's loop once a turn's end is published; any `cancelled`
  end holds it (an interrupt's excepted), a failed one too, and the
  session's end holds what waits before the final status. `fake-acp`
  steers its `wait` turn (`finish` ends it).
- Slice 3, questions (contract item "Questions", review items 4 and
  5): form elicitation is advertised in `initialize` only when the
  session asks for it (`AgentSetup::questions`, off by default; a probe
  never does). The node turns it on for a session the owner or a device
  creates (a delivered call of either too), never for an agent's, here
  or delivered (`profiles::launch::watched`); the runtime keeps it for
  the session's life, so an agent a move starts again asks as the first
  did (the actor sets it from its own launch, not from the move's
  request). The driver maps the form to a `Question` within the bounds
  (`mapper::question`), holds the responder (`questions.rs`, at most 64,
  the oldest withdrawn `cancel`), checks an answer with
  `Question::check` before the agent sees it, and resolves a question
  `cancelled` when the agent withdraws it (`$/cancel_request`: one small
  task per question waits on the responder's cancellation marker or on
  the question being settled, and reports through the loop's queue), or
  when its turn is cancelled, ends, or the connection closes.
  `session.send`'s `answer_question` goes straight to the agent, as a
  permission answer does: `invalid_request` for an answer that does not
  fit, `not_found` for an unknown id, `conflict` once settled.
  `fake-acp` asks with `ask`, withdraws with `ask withdraw`, sends a
  URL elicitation with `ask link`, and says on stderr when the client
  takes questions.
  Decided here (the contract did not say):
  - A mode Vessel did not advertise (URL always; a form in a session
    without questions) is answered with a JSON-RPC "invalid params"
    error, MCP's answer to a mode the client did not declare, rather
    than `decline` (nobody declined) or `cancel`. A form Vessel cannot
    show whole (more than 32 fields, a required field it does not
    define or whose name Vessel cannot carry) is answered `decline`, as
    the contract says. Both leave a `vessel:` line in the session's log
    saying why, never the URL (a sign-in link can carry a secret).
  - An optional field whose name Vessel cannot carry is left out; a
    default that is not one of a choice's options is dropped, so a
    client never preselects an answer the node would refuse; option
    values and defaults are never cut.
  - The 64-request bound now holds for permission requests too (the
    driver withdrew none before; the session's projection already
    dropped the oldest): `vessel_agent::MAX_WAITING`, which the
    projection now takes, so both drop the same one.
  - `SessionError::NotBuilt` is gone: nothing in the session runtime is
    unbuilt any more (the node answers its own unbuilt calls).
  Known limits: the bound past 64 waiting requests is not covered by a
  test (a responder cannot be built outside the SDK, and the fake asks
  one question per turn); an agent that asks outside a turn keeps its
  question until the next turn ends or the session does, as a
  permission request would; an older (2-9) client sees the stand-in
  line and cannot answer, so the turn waits until cancelled (as the
  contract says).
- Slice 4, marks, the board's rule for ended sessions, unread, delete
  and retention's order (contract: `session.mark`, `session.delete`,
  "The board, and ended sessions", "Unread, without a race"; review items
  2's store side, 10 and 11; ADR 0026, amended with what follows):
  `Store::mark` (one `UPDATE .. RETURNING` per mark: `read` keeps the
  furthest `through` and clears `unread`, `archive` keeps its first time)
  and `Store::delete_session` (row, log, queue, marks, attachment rows by
  cascade, then the session's `attachments/<session>/` folder once
  committed; a row still live is refused). Every `SessionInfo` the store
  builds carries its marks, `unread` derived by `KeptMarks::marks` from
  the attention point (`SessionEvent::asks_for_attention`, now the one
  rule the store's projection and the runtime share) and the last
  `read`. The board: an ended session is `SessionBoard::exited` (building,
  `exited`) until archived, `SessionBoard::archived` after, in the
  runtime's tracker, the store's reader and the reducer's own test; the
  version 8-9 rewrite (`SessionInfo::for_protocol`, from the wire commit)
  now has a test through a real node. The node applies a mark in the
  store first and hands the store's answer to a session the runtime still
  has (`SessionHandle::set_marks`; `SessionHandle::info` adds the marks,
  with the attention point the publisher reads off its records), so
  `session.list`, `session.get`, a subscription's info and the mark's own
  answer agree; `archive` on a live session stops it and answers once it
  has ended; agents marking `read` or `unread` are `forbidden`.
  `session.delete` answers `conflict` for a session the runtime still has
  (live, or ended with its log on its way to the store). Retention keeps
  pinned sessions longest and prunes archived ones first, through an
  index (schema 8, which replaces schema 1's `sessions_ended`; a test
  reads the query plan), and removes a pruned session's folder too.
  Decided here (the contract did not say; ADR 0026's consequences):
  - No `read` yet reads as a `read` through 0: every record asking for
    attention is unread until the session is first read; sessions from
    before protocol 10 have no attention point and are not unread.
  - A `read` past the end of the log is `invalid_request` (it could never
    be taken back); the end is the ring's for a live session, which a
    follower's records never pass, and the row's for a stored one.
  - Where a live session's marks are kept: the store, with the session
    handle holding the store's last answer; marks on live sessions go
    through one node lock from the store's write to the handle, so two
    marks cannot land in different orders. The info watch carries no
    marks (its sender is gone once the session ends, and an archive comes
    after the end); `SessionHandle::info` adds them.
  - `archive` answers when the final status is published, not when the
    store has the whole log; the store does not check that an archived
    row has ended (the node stops the session first).
  - The web board's end-to-end test now finds a killed session in its
    lane as `Exited`, not in the archive: the page has no archive action
    yet (the page track's).
  Known limits: a deleted session in the middle of an agent's chain of
  sessions cuts the chain, so that agent can no longer operate on the
  sessions beyond it (ownership follows `created_by` through the
  sessions that exist); the folder removal on retention runs on the
  store's writer and holds its next batch for a few unlinks; the CLI and
  the agent tools for marks and delete come with their slice.
- Slice 5, start again (contract: `session.resume`; review items 2 and
  3; ADR 0026, amended with what follows): `session.resume` starts an
  ended structured session's agent again under its current profile with
  `session/load`. The store reopens the row first (`Store::reopen`: one
  `UPDATE .. RETURNING`, `live`, `starting`, no end, out of the archive,
  only for an ended row whose log ends where the caller read it;
  `not_found` once retention or `session.delete` took it), so retention
  never prunes a session that is starting again and its log continues
  after its final status (`Store::ended_session` reads what the run
  continues from in one read transaction). The runtime
  (`SessionRuntime::start_again`, `vessel-session`'s `again`) reuses a new
  session's launch (`run_structured`) and the move's replay dropping
  (`MoveState::replaying`), publishing this run's `ready` and `running`:
  the ring and the publisher start at the stored `next_seq`, the
  conversation's projection from the stored queue, context report and
  abilities, the attention point from the stored one; title, choices and
  creator are the session's. The node (`sessions::again`,
  `service::again`): draining is `unavailable`; one start again per session
  (a claim, `conflict`); a session the runtime has is `conflict` (live, or
  its log still on its way to the store); terminal, an agent whose
  recorded `resume` is false, or no conversation to load is `unsupported`
  (`vessel_session::resumable`, read from the stored row before anything
  starts); a profile no longer defined is `invalid_request`; the agent's
  `vessel` server gets a new credential from `Agents::reserve` with the
  caller's principal, as `session.create` grants it, `created_by`
  unchanged; questions as for a new session of its creator; a routed
  session keeps its route (`unsupported` if its profile's changed, as a
  move). The call answers once the status leaves `starting`: `running`,
  or `spawn_failed` with the reason the log ends with (`starting`, then
  `failed`; the session stays ended). A subscription the store served ends
  `lagged` from its cursor if the runtime has the session again, reads on
  if its stored log grew, and ends `session_ended` otherwise
  (`Sessions::stored_end`). `fake-acp` refuses `session/load` with
  `FAKE_ACP_LOAD_FAILS` and offers no `loadSession` with
  `FAKE_ACP_NO_LOAD`.
  Decided here (the contract did not say):
  - The queue comes back held `ended` (`Desk::continuing`): it stays so,
    or becomes so when the node stopped before the session's end could
    hold it, whatever held it before, as a session's end does; a person
    lets it move with `session.queue resume`.
  - The order, so readers find the session in one place throughout: the
    runtime registers the session with its agent already starting, but it
    publishes nothing until a gate opens (`vessel_session::Gate`); under
    the marks' lock the node makes it visible, reopens the row and hands
    the handle the store's marks; then the gate opens. A row the store
    will not reopen drops the gate: the session ends without a record and
    its hand-over frees its places. Reopening first would leave a moment
    where the store shows a live row the runtime does not have, which a
    stored subscription cannot tell from a row a failed store left live.
  - An agent that cannot even be executed (its `spawn` fails) is refused
    before the row is reopened, so that attempt leaves no record; one that
    starts and fails its handshake or `session/load` leaves `starting`,
    then `failed`.
  - A stored session's mark now takes the marks' lock and looks the
    session up again under it, so a mark never lands in the store for a
    row its running session's handle does not know of.
  - The agent's credential and model token are retired in the hand-over
    (they already refused once the end was visible), no longer by a task
    of their own: that task could run after a start again and take the
    new run's places.
  - A live session's info now carries the abilities of its agent's
    `ready`, as the store's row does; a session started again keeps its
    stored ones until its new `ready`.
  Known limits: the e2e test of a subscription the store was serving
  cannot order the start again before that subscription's end (the node
  reads a short log in one page), so it accepts either end and checks the
  client reaches the new run; the `lagged` decision is pinned by a unit
  test through `Sessions` (`sessions::tests`). Starting again after a
  move uses the moved-to profile; the
  session's files must still be in that profile's home. The CLI and the
  agent tool for it come with their slice.
- Slice 6, attachments (contract: "Attachments", the bounds; review
  items 8, 9 and 10; ADR 0025, amended with what follows):
  `attachment.begin`, `.write`, `.finish` and `.read`, and prompts naming
  attachments. The node's `attachments` module: uploads (`uploads`,
  `staging`) held by whoever began them (`transfers::Holder`; anyone else
  gets `not_found`), 16 open on a node and 4 a caller (`unavailable`), a
  declared size counted against its session's 100 MiB and the node's
  4 GiB with what the store keeps until its row is in (`invalid_request`),
  each staged in an owner-only file under `uploads/` that a piece is
  appended to and hashed into on the blocking pool, dropped after 10 idle
  minutes and at every start; `finish` checks every byte came, an image's
  first bytes (png, jpeg, gif, webp), and that the session still runs,
  then `Store::keep_attachment` moves the file to
  `attachments/<session>/<id>/<name>` without following a link, checks
  its size and SHA-256 and records the row for a live session only.
  `Store::read_attachment` reads one back only as that regular file of
  the recorded size and SHA-256 (Unix: folders opened relative to each
  other with `O_NOFOLLOW`; a file of an unchanged identity is not hashed
  again), for `attachment.read` (a piece at a time) and for the agent.
  Folders no row names are swept when the store opens; delete and
  retention already removed a session's folder (slice 4). A prompt's
  references become the store's records where the prompt enters the node
  (`session.prompt`, a queue `edit`, `session.send`: an attachment not the
  session's is `invalid_request`); the session's actor checks the
  per-prompt bounds and the kinds its agent said it takes before it
  queues or sends a prompt, and right before it sends one asks the node's
  loader (`vessel_session::AttachmentLoader`, `attachments::load`) for the
  bodies, which the agent's driver puts in the prompt (`image` and
  `audio` blocks, base64; a file as a `resource_link` with its `file:`
  URI), while the log keeps the references. A queued prompt is checked
  again when it is sent; one that no longer fits holds the queue
  (`failed`) with a line saying why. `fake-acp` says what a prompt held
  (`describe`) and takes audio with `FAKE_ACP_AUDIO`. Client calls:
  `begin_upload`, `write_upload`, `finish_upload`, `attach` (all three),
  `read_attachment`.
  Decided here (the contract did not say; ADR 0025's "As built"):
  - The node reads the bytes, never the session's actor: a bounded
    channel to the node's loader, awaited through `pump`; loading before
    the prompt is handed over would make a queued prompt hold its bytes.
  - A piece that ends at or before what has come is answered with it; a
    gap or an overlap is `invalid_request` (the contract's line said "a
    repeat is `invalid_request`", review item 10 said otherwise; the
    review wins, and the contract's line is amended).
  - An agent still starting takes files only (images and audio are
    `unsupported` at `begin` until its `ready`).
  - A file changed on disk is `conflict` (for `attachment.read` and a
    prompt); an image that fails its first bytes is `invalid_request` and
    its upload dropped; one attachment named twice in a prompt is
    `invalid_request`.
  - Dependencies, none new to the tree: `rustix` with `fs` for
    `vessel-store` (Unix; `vessel-fleet` already builds it so), `base64`
    and `url` for `vessel-node` (an inline body, a `file:` URI), `bytes`
    for `vessel-client` (a piece), `base64` and `url` for
    `vessel-testkit`'s `describe`.
  Known limits: on Windows the read-back checks the path with
  `symlink_metadata` before opening it (no stable relative open there),
  the window `vessel-fleet` accepts too, and hashes every read; a file in
  place may be written into within one timestamp tick of its last
  verification without being hashed again (the bytes are then the
  writer's own, never another file's); the node's 4 GiB is checked
  against declared sizes when an upload begins, so finishing several at
  once never passes it but a store edited by hand could; a steer the
  agent hands back (`promptRequired`) loads its attachments again for the
  turn it starts; the e2e test of the node-wide 16 and the 4 GiB is a unit
  test through a real store (`uploads::tests`), as is the idle drop. The
  CLI and the agent tools come with their slice.
- Slice 7, the CLI and the agent tools (contract item 8): every call of
  protocol 10 from the command line, and the ones an agent should have as
  tools.
  - CLI (`vessel-cli`'s `conversation/`, arguments in `args/conversation.rs`):
    `vessel send --queue|--steer|--interrupt` and `--attach FILE`
    (repeatable), the outcome printed (`started`, `steered`, `queued
    <entry>`); `vessel session queue ID` (each entry's place, id,
    revision, who queued it, the start of its text and its files) with
    `edit`, `move [--before]`, `remove`, `send-now [--steer|--interrupt]`,
    `hold`, `resume`, `clear`, each printing the queue after; `vessel
    session answer ID REQUEST [field=value ...] [--decline]`; `vessel
    session resume|pin|unpin|archive|unarchive|read|unread ID`, each
    printing the session's line with its marks, which `vessel sessions`
    now shows too; `vessel session delete ID` (asks unless `--yes`, and
    without a terminal refuses to ask, as `node update --now` does).
    Entries, questions and requests are named by an id or a unique prefix
    among the ones there are (`selector::IdPrefix`).
  - Agent tools (`vessel-mcp`'s `tools/conversation.rs`): `send_input`
    takes a `delivery` (`session.prompt`, answering `from` and
    `delivered`), `read_conversation`, `change_queue`, `answer_question`
    (values as JSON, a list for a field of several choices),
    `resume_session`, `archive_session` (`unarchive` too).
  - `vessel_client::compose`, shared by both: `fill` reads a question's
    values from `(field, text)` pairs by each field's kind (bounds,
    options and required fields stay the node's `Question::check`), and
    `with_text` gives a waiting prompt new words, its attachments kept;
    `NodeClient::answer_question`.
  Decided here (the contract did not say):
  - `vessel send` without a delivery or a file makes `session.send` as
    before, so it still works with a node before protocol 10; with one,
    `session.prompt`.
  - A file is read and sent a piece at a time (at most the node's
    `chunk`), its size fixed at `attachment.begin`; one that ends early is
    refused (`changed while it was sent`). Its type comes from its
    extension (the four inline image types, common audio, a few text and
    document types, else `application/octet-stream`); no dependency for
    guessing more.
  - An `edit` keeps the entry's attachments and, without `--revision` (the
    tool's `revision`), uses the entry's current revision, read from
    `session.conversation` just before; `vessel session queue` shows the
    revision so a person can pin it.
  - `vessel session answer` reads what waits first: the id prefix names a
    question or a permission request, a question's form says how to read
    each value (`yes`/`no` for a boolean, decimal numbers, one pair per
    chosen value), and a permission takes one option's id. `--decline`
    on a permission sends `cancelled`, what the agent tool
    `answer_permission` already did for "refuse". One no longer waiting
    is an error naming the prefix, not a call.
  - `read` marks seen what the session had logged when the CLI looked it
    up (`SessionInfo.next_seq`): a record landing after stays unread.
  - Not offered to agents: marking read or unread (the node forbids it),
    pinning (a person's own lists), deleting (for good; an agent archives
    instead), attachments (an agent names a file's path in the prompt and
    the other agent reads it with its own tools). An agent operates only
    on its own sessions and the node asks questions only in sessions a
    person or a device started, so the agent's `answer_question` has
    nothing to answer until profile grants (ADR 0013) exist; it is tested
    with the same tools over the owner's connection for the answer, and
    over the agent's for the refusal (`forbidden`).
  - Tests: `crates/vessel-cli/tests/cli/conversation.rs` (each command
    and its main refusal through the binary; the fixture gained `restart`
    and `owner`), `crates/vessel-mcp/tests/conversation.rs` (the MCP
    tests' support moved to `tests/support/mcp.rs`, shared by `#[path]`).
  Known limits: an upload that fails half way stays open on the node
  until its 10-minute idle drop (protocol 10 has no call to abandon one);
  a file's type is its extension's (the node still checks an image's
  first bytes); `vessel attach` keeps sending a typed line as a `start`
  prompt and shows questions without answering them (`vessel session
  answer` does); the CLI shows the agent's commands and context only as
  `vessel attach` and `vessel log` print them.
- Files this track pushed past about 400 lines, divided by
  responsibility (no behaviour change, the tests as they were):
  `vessel-session`'s `runtime` (`launch`, `agent`, `parts`), `handle`
  (`input`), `delivery` (`moves`); `vessel-agent`'s `driver`
  (`connection`); `vessel-node`'s `sessions` (`conversation`, `read`);
  `vessel-store`'s `store` (`conversation`, `attached`) and
  `attachments::files` (its two platforms' halves in `files/unix.rs` and
  `files/other.rs`); `vessel-proto`'s `conversation::question` (`check`).
  Inline tests moved to a file of their own: `vessel-node`'s
  `profiles::launch` and `attachments::uploads`, `vessel-session`'s
  `activity`, `vessel-cli`'s `render`. Left as they are: files already
  past the guideline before this track, which other tracks also edit
  (`vessel-node`'s `service.rs` and `node.rs`, `vessel-proto`'s `rpc.rs`
  and `options.rs`, `vessel-mcp`'s `tools.rs`, `vessel-cli`'s
  `args.rs`, `vessel-rpc`'s `client`, `vessel-store`'s `data_dir.rs`,
  `vessel-node`'s `envelope.rs`, `vessel-session`'s `board.rs`), where a
  split now would cost every other track a conflict; `vessel-agent`'s
  `mapper.rs` (402, its tests read the prompt side through the module);
  the fake agent and the long test files, where a split would only move
  helpers.

## Track B: progress

2026-11-11, on `backend/m11-workspace`. ADRs 0023 (git as a program)
and 0027 (projects, worktrees, the project's file and its trust; 0024 on
this track's branch); store schema 9 (7 on this track's branch); crate
`vessel-workspace`.

Built, each slice with its tests:

- The wire (protocol 10): every type and the 40 methods above, with
  `Call::since`, the forwardable list and `vessel-web`'s list.
- `vessel-workspace`: the hardened git runs and both pools, status,
  diffs, log, branches, stashes, every write, clones, worktrees, the
  inspection, snapshots, checked paths, files, the name index, `git
  grep`, `vessel.toml` and its digest, setup and teardown; 36 tests
  against temporary repositories with the real git.
- The node: the registry (schema 9, read back at start, a clone or a
  setup the node's stop interrupted told as failed), every call, the
  agents' list in `guard`, `session.create`'s place, `SessionInfo.
  workspace` on every answer; 16 tests (13 against temporary
  repositories) and 2 end to end over the wire (as the owner, and as an
  agent).
- `vessel mcp`: `list_projects`, `list_worktrees`, `create_worktree`
  (follows setup), `create_session`'s `project`, `dir` and `worktree`.
- The CLI: `vessel project list|add|clone|init|remove|commands|approve`,
  `vessel worktree list|new|output|setup|remove|snapshots`, `vessel run
  --project [--dir] | --project --new-worktree | --worktree`.

Changed from the contract while building, and why:

- The worktrees' folder is `<data>/workspace/worktrees/<project
  id>/<8 hex>`: the data folder's workspace folder also holds the setup
  logs, git's empty hooks folder and scratch files, all owner-only.
- The first session started in a worktree made for no session claims it
  (`created_for`): a worktree is made before its session (the three
  steps), so this is how tidying finds the session's own.
- `session.create` with both a `place` and a `cwd` is `invalid_request`;
  an imported session's `place` is dropped (it named the other
  machine's project; carrying worktrees is wave 2's).
- Tidying is `Workspaces::retire(session, live sessions)`; nothing calls
  it yet. The coordinator wires it to track A's archive and delete; the
  node's retention dropping a session's log does not tidy yet.

Measured (release build, the development machine; no local path is
named): the name index over 300,000 synthetic paths builds in about
1.0 s and answers a query in 21-25 ms; over a real repository of about
35,000 paths, 5-10 ms. A git process costs 2.0 ms (`rev-parse`) to
2.5 ms (`status`) each, over 200 runs.

What agents cannot do, so is not offered as an agent tool: approve
commands, add, clone, start or forget projects, run setup again, remove
worktrees, snapshots, and every git and file call (an agent uses its own
tools in its folder).

Only CI can confirm: Windows (setup through `cmd.exe /d /c` on a `.cmd`
file; `git~1` and trailing dots and spaces; drive names; Git Credential
Manager honouring `GCM_INTERACTIVE=never`; the symbolic-link tests are
Unix-only), macOS (canonical `/private` temporary paths), and git
2.25 to 2.42 (developed with 2.43).

Not verified: a successful clone from a real remote and its progress
(only the failing path is tested; the parser has unit tests); a `ssh` or
`gpg` prompt on a node started from a terminal; the MCP tools end to
end with a project (their parsing is unit-tested, the agent's rights end
to end through the node).

For the page track: `project.list` says which git the node has; a
clone is followed with `project.get { changed_from }`; a new worktree is
`worktree.create`, then `worktree.output` until `setup` is not
`running`, then `session.create { place: worktree }` (a session in a
worktree whose setup runs is refused); `needs_approval` carries the
commands and the digest to show and approve with `project.approve`;
`worktree.remove` answers `kept { reasons }` for a person to choose
`snapshot` or `discard`; every path is a `RelPath` from a listing;
`file.find` takes a `key` per input box so typing replaces the running
search; `SessionInfo.workspace` names a session's project, worktree and
branch.

## Track W: progress

Branch `backend/m11-page`. The contract above (reviewed) holds, with the
deviations listed below. ADR 0028 is taken (0025 on the branch,
renumbered at the integration).

### Built

- **Preferences kept by the bridge** (ADR 0028): `vessel_web::keep`
  (one task per file, bounded queues, a `watch` with a rising revision
  pushed to every tab as a `preferences` frame), the lenient codec that
  keeps unknown keys and unreadable values, `paint` (the theme, sizes
  and width written on the served index's `<html>` from memory), the
  ops and frames in `wire/prefs`. The page: `data/prefs.ts` (the store,
  `prefer` applying a change at once and putting back the bridge's
  answer), `prefs-change.ts` (the same change rules as the bridge),
  the theme on it with no flash at first paint.
- **Drafts**: per session and one for the new-session page, kept by the
  bridge (`ui-drafts.json`); the page saves 500 ms after typing stops
  and when the tab is hidden, never per keystroke; Up/Down recall
  (`data/recall.ts`); the send key chosen in Settings › General.
- **Shortcuts**: one registry and matcher (`data/shortcuts/`), one
  listener (`features/shortcuts/`), Settings › Shortcuts with search,
  the Modified filter, the recorder, Reassign, Restore and Reset all.
  `palette/shortcuts.ts` is gone.
- **Find and the outline** over the session's model (`data/find.ts`,
  `data/outline.ts`), through one seam the timeline exposes
  (`reveal.ts`: `Timeline` takes `reveal` and `scrollerRef`, and its
  items carry `data-entry`).
- **Notices**: `data/notices.ts` (a pure function of two polls, ADR
  0019's placement), the browser notification, a tone, the tab's title,
  Settings › Notifications.
- **Sidebar and Appearance**: Display (group by machine or status, sort
  by created or last activity, show ended), hiding, the phone sheet;
  interface and code sizes, content width, "Collapse tool calls"; a new
  session on a machine starts as its last one did (`data/defaults.ts`,
  kept per machine id).
- **Milestone 10's leftovers**: an untitled session without a folder is
  "Untitled session"; Settings is reachable below 768px (the sheet);
  `bridge/client.ts` divided (`socket.ts`, `failure.ts`) and
  `check-size.mjs`'s list is empty; branded ids (below).

### The coordinator's conditions

- `NodeSource::preferences` and its override in
  `crates/vessel-cli/src/ui/source.rs`: an error working out the place
  is returned, not folded into "nowhere" (`a_place_that_cannot_be_worked_out_is_an_error`),
  and the bridge keeps the choices for this launch and says so in both
  cases (`without_a_place_they_are_kept_for_this_launch`: no place, and
  an error). The file sits beside the book and is written as the book
  is: owner-only (0600, its directory 0700), a temporary file in that
  directory renamed over the name, so a link at the name is replaced,
  never followed; `--config-dir` moves it with the book
  (`the_pages_files_follow_the_book_and_the_data_directory`).
- Branded ids (`dbfa8e2`, after the page's `0d0e042`, both after the
  preferences work): the rule, the numeric handles, one table and its
  tests, secrets plain, guards only at the edges, as PROTOCOL.md's
  "TypeScript declarations" says.
  - Rule: a handle minted by a node, the bridge, a harness or the page's
    bridge client is branded (`SessionId`, `NodeId`, `NodeKey`,
    `DeviceId`, `ToolCallId`, `PermissionId`, `PermissionOptionId`,
    `OptionId`, `OptionValueId`, `TransferId`, `HarnessSessionId`,
    `BlobHash`, `NodePublicKey`, `LaunchId`, `Nonce`).
  - Numeric handles are branded (`CallId`, `StreamId`,
    `SubscriptionId`): the first two come from one counter in the
    client and are looked up in different maps, which is the mistake a
    brand stops; minting them costs one guard where the counter is read.
  - The classification is `vessel-proto/src/ts/aliases.rs` (ts.rs is 384
    lines); vessel-proto's test fails on an unclassified alias of its
    own, vessel-web's on one of the bridge's and on an entry neither
    file declares any more.
  - Page: `bridge/ids.ts` is the only place a brand is given (features
    may not import `bridge/`); a frame's handles come typed with the
    frame's guard (`isServerFrame` checks the frame's kind, not each id:
    the bridge is authenticated and generated from the same types);
    route segments and `?machine=`/`?node=` go through `nodeKeyOf` and
    `sessionIdOf`. Tests use `src/testing/ids.ts`'s `fixtureId`, which
    lint allows only in tests and `data/fixtures/`.
  - Typed-by-person values (`ProfileName`, `SecretName`, `SessionTitle`,
    `FilePath`, `EnvName`, `HttpUrl`, `BookName`) are not branded: see
    "Not built".

### Checked, and how

- Rust: `vessel-web`'s `tests/ui/prefs.rs` (ordering, two tabs, two
  bridges, a change from another process, no place, an error, an
  unreadable or newer file never written, drafts merged across two
  bridges and bounded, the index's first paint), the keep modules' unit
  tests, the CLI's two tests above, the alias tests.
- Web: vitest over every data module named above; Playwright's
  `e2e/page.spec.ts` against the test fleet's real `vessel ui` (the
  theme painted on the index before any script, a draft surviving a
  reload, Up recall, find's count, rebinding with a clash and Reset all,
  `Ctrl+Enter` mode, the sidebar's status grouping, Settings from the
  phone sheet, an address naming no session).
- Screens: `e2e/page-screens.ts` captures General, Appearance,
  Shortcuts (recording, reassign), Notifications, the find bar, the
  outline's card, collapsed tool calls, the Display menu and status
  grouping, and at 390px the sheet and Shortcuts, light and dark.
- Find, 5,000 entries (`VESSEL_FIND_TIMINGS=1 npx vitest run
  src/data/find.speed.test.ts`, three runs on the development machine):
  the first search 92.7–95.4 ms in all, given back to the page every 200
  entries, the longest stretch without a yield 6.9–7.6 ms; a repeated
  search (the session growing) median 4.2–4.4 ms; 20,000 matches.

### Not built, and why

- **Brands for values a person types.** A book name, a profile, a path,
  a title or a URL is text from a form; branding it would put a guard on
  every field and say nothing a validator does not. Each is checked where
  it is typed (the book's name rule on the page, the rest by the node).
- **A notifications inbox** (GOAT's list): the browser keeps its own
  list; a waiting session is on the board.
- **Editing a machine's remembered start in Settings**: the page
  remembers what the last session on a machine started with (Paseo's
  behaviour); `forget_defaults` is on the wire but has no button yet.
- **Firefox and Safari**: only Chromium runs the end-to-end tests;
  notification permission and the Custom Highlight API were not checked
  in other browsers (Safari cannot open `vessel ui` at all).

### Deviations from the references

DESIGN.md §12, 17–20: preferences kept by `vessel ui`; no inbox;
shortcuts as physical keys (`code`), shown as a US keyboard prints them;
three content widths instead of GOAT's slider. "Collapse tool calls"
folds a single call too (Paseo's overview does).

### For the page track (Track P)

- `timeline.tsx` maps `TimelineEntry` (from `folds.tsx`'s
  `FoldProvider`) and takes `reveal` and `scrollerRef`; a windowed
  timeline must keep `data-entry` on prompts and messages and scroll a
  revealed key into view even when it is not rendered (find and the
  outline hand it a key, never a node).
- `files.rs` splices the first paint's attributes after the index's one
  `<html lang="en"`; a change to how the index is served keeps that.
- Drafts are read once when the link opens and written by the `Drafts`
  owner in `data/drafts.ts`; a composer never writes per keystroke.
- A `preferences` frame may skip revisions (the bridge pushes the latest
  only); compare by revision, never count frames.

## Integration (node)

2026-11-12, on `backend/m11-int-node`: track A's branch
(`backend/m11-conversation`) with track B's (`backend/m11-workspace`)
merged in. Tracks P and W (the page) are joined on another branch; the
coordinator joins the two. Every track's commit is kept; the merge
commit builds and passes, and each follow-up is a commit of its own.

### What conflicted, and how each was resolved

The trial merge conflicted in 28 files. Where both tracks added to the
same list (ids, the method table and every list that names each method:
`Call::since`'s and `Reply::for_protocol`'s arms, the forwardable list,
the unary calls, the web bridge's list, the guard's and the envelope's
rows, the test handlers, the agent tools), both stay, track A's first.
Beyond that:

- `SessionInfo` takes both tracks' fields: `marks` and `abilities` (A),
  `workspace` (B).
- The store: track A's schemas 7 and 8 stay, track B's 7 is 9 (nothing
  of protocol 10 was released, so no database had it under the other
  number); the writer takes both tracks' writes; the migration tests
  take a database back to schema 6 and walk 7, 8 and 9 in order
  (`UNDO_SCHEMAS_7_TO_9`, and `marks.rs`'s test now checks the
  workspace's tables come back empty).
- The agent tools are listed session tools, conversation, workspace,
  language server; the server's instructions join both tracks' lines.
- Replies newer than every older peer pass `Reply::for_protocol`
  unchanged, track B's 40 included.
- In the merge commit, beyond resolving: the long exhaustive matches over
  every method (forwardable, unary, the web bridge's list, the older
  peers' rewrite) passed clippy's line bound only with a stated reason;
  `RelPath::name` became `file_name` (under the `ts` feature `name` is the
  TypeScript type's, and clippy refuses the clash, as W found for
  `BookName`, whose fix is cherry-picked here unchanged so the join is
  clean); track B's expectation on the CLI's dispatch length was dropped
  (track A had shortened it).
- Generated files were regenerated: `web/src/wire/proto.ts` after every
  wire change; `Cargo.lock` merged without a conflict and did not change
  when built.

### Numbers

- Track B's ADR 0024 is **0027** (file, title, every mention: each hit
  read by what its sentence is about; the queue, delivery and steering
  stay 0024). Track A keeps 0024 to 0026, track B 0023.
- Store schema: A's 7 and 8, B's **9**.
- Protocol 10 raised once.

### Where the two contracts met, and what was decided

1. **Tidying a session's worktree** (B's `retire`, which had no caller).
   It runs when the session the worktree was made for is deleted or
   retention drops it, never when it only ends or is archived:
   `session.resume` starts an archived session again in the same folder
   (the coordinator's rule; B's contract had said "archived or deleted",
   ADR 0027 is amended).
   - `session.delete` deletes the session, then removes the worktree by
     the rule `worktree.remove { if_lost: keep }` follows, and answers
     what became of it: `SessionDeleted { worktree?: { worktree,
     outcome: removed | kept { reasons } } }` (it answered `Ack`; the
     reply changed in place, protocol 10 being unreleased). A worktree
     with uncommitted work, commits no other branch has, an operation,
     submodules, flagged files or refs of its own, or another live
     session in it, is kept, its record saying why: a delete never loses
     work, never snapshots or discards unasked, and never keeps it
     silently. Rejected: refusing the delete while the worktree would
     lose work (two calls for a person, and retention needs the keeping
     rule anyway), and a snapshot taken unasked.
   - Retention prunes in the store's writer, so the store now says how
     many sessions it removed (`Store::pruned`, a watch). The node's
     `tidying` module sweeps the worktrees whose sessions are gone
     (neither the runtime nor the store has them; an ended session the
     runtime still holds counts, since retention prunes in the commit of
     its final record) at start and after each prune, and, for those
     kept because a session was in them, after each deletion. One kept
     for its work is left to a person.
   - One tidying at a time (a tokio mutex across the deletion and the
     removal), so a delete and a sweep never remove the same worktree
     and the delete's answer is right. A delete waits for an approved
     teardown (at most 5 minutes), as `worktree.remove` does.
   - Tested: `crates/vessel-node/tests/e2e/tidying.rs` (removed; kept with
     its reasons and its file intact; retention), and the workspace unit
     tests (`tidy_candidates`, a worktree kept for its work left alone).
2. **`session.resume` when the folder is gone anyway** (a person removed
   the worktree, or it went with the session it was made for while
   another session that ran in it had ended): `conflict`, naming the
   folder, before anything starts; nothing of the session changes (it
   failed at the agent's spawn before, with the system's words). So is
   one into a worktree a setup, teardown or removal holds
   (`Workspaces::busy`, which `place` already asked), since the removal
   could take the folder from under the agent.
3. **`SessionInfo`**: track B filled `workspace` at the service, on
   `session.list`, `.get`, `.create`, `.move_profile` and `.rename`;
   `session.subscribe`, `.resume` and `.mark` (A's) did not have it.
   They do now, through the same function, and a test asks each. A's
   `marks` and `abilities` come from the session's own record and were
   on every answer already.
4. **`session.create` with a place, and a first prompt with
   attachments**: an attachment needs a live session (`attachment.begin`),
   so a first prompt goes after the create (the MCP tool, the CLI); the
   place's notices are the log's first lines either way. An attachment
   is kept in the node's data folder, never in the worktree, so it never
   makes the worktree dirty, and goes with its session (tested).
5. **An agent's limits**: the guard runs the scope, then track B's
   workspace rule, then ownership; track A's rules (no read or unread
   marks; ownership on every `operate` call naming a session, delete,
   resume, mark and uploads included) and track B's (orchestration's
   calls only, 8 worktrees, `session.create` in its own worktrees) do
   not overlap. An agent deleting its own session tidies the worktree
   made for it by the same rule; a session it resumes counts as one of
   its live ones (A). Track B's rule was a `matches!`, so every other
   method was open to agents by default; it is an exhaustive `match`
   now. An agent's own `cwd` inside the node's data folder (where
   worktrees are made) is `forbidden`: the ownership checks are the
   place's, and a raw folder went around them. Creations at once count
   against the 8 together (`State::making`). Tested end to end through
   the agent tools (below).
6. **The board, and archived sessions that own a worktree**: an archived
   session is in the archive lane whatever its worktree; the worktree
   stays (archiving tidies nothing). `WorktreeInfo.sessions` lists live
   sessions only, so such a worktree lists none, and `created_for` still
   names its session; a person may remove it, after which a start again
   is refused (2).
7. **Export and import**: the importer drops the place (it named the
   other machine's project, B), keeps the title (protocol 9's rule), and
   leaves the marks and the queue on the source (A's known limit). The
   imported session runs in the folder the import names, or its own,
   which must exist when it starts; it was made for no worktree, so
   tidying never touches one for it. Checked by reading the importer; no
   test was added.

### The files the tracks left long

Divided by responsibility, code moved without a change of behaviour,
one commit per file, each building and passing: `vessel-node`'s
`service.rs` (967 lines, now 313, and `service/{sessions, profiles,
fleet, transfers}.rs`), `vessel-proto`'s `rpc.rs` (652, now 341, and
`rpc/methods.rs`, the table's macro, and `rpc/since.rs`),
`vessel-mcp`'s `tools.rs` (793, now 336, and `tools/{sessions, waiting,
args}.rs`), `vessel-cli`'s `args.rs` (1,043, now 331, and `args/{workspace,
node, fleet, nodes, tests}.rs`), `vessel-agent`'s `mapper.rs` (402, now
181, and `mapper/{prompt, tool}.rs`); and track B's
`workspace/worktrees.rs` (998, now 107, and `worktrees/{create, start,
removal, snapshots, place, branches}.rs`), `git/diff.rs` (630, now 83,
and `diff/{scope, summary, file, tests}.rs`), `vessel-proto`'s
`workspace/git.rs` (625, now 47, and `git/{state, outcomes,
requests}.rs`), `workspace/projects.rs` (594, now 217, and
`projects/{adding, commands}.rs`), `git/run.rs` (591, now 211, and
`run/{limits, pool, output, tests}.rs`). Only the visibility a move
needed changed (`pub(super)`, or `pub(crate)` one level deeper).

### The flaky test

`a_failed_probe_says_why_and_never_hangs` failed now and then on
branches without track A's probe change, its stderr lines empty.
Confirmed, not assumed: with the probe's read after the disconnect taken
out, 4 runs in 80 failed under load, each with the line missing (the
agent's stderr is read by a task of its own, apart from its ACP stream,
so its last line can arrive after the disconnect); with it, 160 of 160
in parallel and 50 in a row under load passed. The cause was fixed on
track A's branch in the code under test (the probe reads on until the
stderr pump has ended, bounded only against a stray process holding the
pipe); the test now says what it pins.

### What track B could not verify

- A clone that succeeds: a clone URL is never a local path or `file://`,
  not even for a test (the URL type refuses both and git runs with the
  file protocol off), so the test serves a bare repository over HTTP from
  loopback, ready for git's dumb transport (`update-server-info`), with a
  few lines of the test's own: the project answers `cloning`, becomes
  `ready` with the repository's files and a clean status, and leaves no
  temporary folder (`e2e/workspace.rs`).
- The agent tools with a project present (`vessel-mcp`'s
  `tests/workspace.rs`): an agent lists the projects, makes a worktree
  and follows its approved setup to the end, starts a session in it with
  a first prompt, and is refused another's worktree, a ninth worktree of
  its own, and status, a file, removing a worktree and approving
  commands, whatever its scopes.

### Found while reading both tracks, and fixed

- `cargo test -p vessel-node` alone did not build (tokio's `test-util`
  came only from other crates of a workspace build).
- The CLI showed a kept worktree's reasons as a debug dump; it says them
  in words now (`render::worktree`), for `vessel worktree remove` and
  `vessel session delete`.
- The `create_session` agent tool left out a `dir` given without its
  `project`, and a `project` given with a `worktree`, so the session
  started somewhere the agent had not asked for; both are refused.
- The node's end-to-end fixture ran its sessions in the node's data
  folder. Once an agent's `cwd` there was refused (review item 2,
  `4c765e3`), seven agent tests were refused too, which only the full
  run showed; `2b53a36` gives the fixture a folder of its own. The
  commits between those two fail those seven tests; the branch is not
  rewritten (no rebase), so it is said here.
- After the division of `git/diff.rs` its tests imported `ModeChange`,
  which only the Unix test uses, so a Windows test build warned
  (`28fca6a`). `vessel-workspace` was linted for Windows here; the rest
  of the workspace needs a Windows C compiler this machine lacks, so
  CI is its check.

### The independent review

One reviewer (Opus, read only, security first) read the branch at
`17e32ec` against both contracts and found 19 problems. Each one's fate,
most severe first:

1. A read ran programs from configuration an agent wrote (a worktree's
   `.git` rewritten to name a folder of the agent's, whose `config`
   names a filter). **Fixed for worktrees** (`b29ac30`): before git runs
   in one, its `.git` must be the file `git worktree add` wrote, naming
   the record in the project's `.git/worktrees/` that names it back
   (`git::gitfile`), else `conflict`; tidying's inspection asks the same
   and keeps such a worktree. **Left open for a project's own folder**,
   where an agent can edit `.git/config` itself: ADR 0023 trusts that
   configuration as the user's, and closing it needs git's help (an
   empty attribute source, `--attr-source` from git 2.40, or reads
   against a configuration of the node's). A decision of its own,
   recorded in ADRs 0023 and 0027.
2. An agent's raw `cwd` went around worktree ownership. **Fixed**
   (`4c765e3`): an agent's `cwd` inside the node's data folder is
   `forbidden`.
3. "Missing" and "outside" answered differently (not a folder, no
   permission, a link loop, a link to nowhere). **Fixed** (`ca4efa0`):
   every failure to follow a path is `not_found`, the detail in a debug
   log only; `PathError::Link` is gone; `vessel.toml` that leads out is
   as if there were none.
4. A worktree's folder swapped for a link was followed. **Fixed**
   (`b29ac30`): every call opens its root again and refuses one that is
   no longer the folder recorded.
5. A relative data folder made the empty hooks folder relative, so a
   repository could ship hooks there. **Fixed** (`e55df1d`): the node's
   folders are made absolute, and git's probe refuses a relative one.
6. Setup and teardown waited without end for a command that closed its
   output and ran on. **Fixed** (`6ba54d9`): the wait is held to the
   deadline and the cancel; the process group is killed at either.
7. A removal raced a session starting again in its worktree. **Fixed
   for `session.resume`** (`4e42255`: `conflict` while a setup, teardown
   or removal holds the worktree). **Left**: a session that passes
   `place`'s or `resume`'s check just before a removal claims the
   worktree can still start in it; closing that needs a "starting" count
   taken under the registry's lock and given back once the session is
   live, a change to both tracks' start paths, so it is reported, not
   made here.
8. `worktree.create`'s limit was checked and recorded awaits apart, so
   creations at once all passed. **Fixed** (`00e2651`; a test makes 12
   at once: 8 made, 4 refused; 12 made without the fix).
9. An upload finishing between `begin`'s two reads was counted in
   neither. **Fixed** (`024720a`): the open uploads are counted first,
   so one finishing meanwhile is counted twice at worst.
10. A cut through a URL left the end of its token. **Fixed**
    (`3528696`): redacted whole, then cut; a test cuts inside a token.
    The ADR's 64 KiB is what is kept of standard error, the 2 KiB what a
    reply carries; ADR 0023 now says both.
11. A git call waited for a pool slot outside its time. **Fixed**
    (`51d71de`): the wait counts against the call's time and ends with
    its cancel. **Left**: clones share the 4 network slots with fetch,
    push and pull; a slot or a cap of their own is a design change for
    track B's next wave.
12. Tidying retried worktrees git could not inspect, running their
    teardown each time, under the lock `session.delete` waits on.
    **Fixed** (`11c4752`): only those kept because a session was in
    them are retried. **Kept by design**: a delete still waits for the
    session's own worktree's teardown and for a sweep already running
    (one tidying at a time, `tidying.rs` says why).
13. `worktree.output`'s long poll lost its wake-up. **Fixed**
    (`6663040`).
14. `vessel.toml` followed links, had no bound, then could wait on a
    FIFO. **Fixed** (`e55e2b5`, `3826627`): read only as a bounded
    regular file inside its folder, opened without waiting and without
    following a last link, judged on the handle; `file.read`, a file's
    version and a gitfile are read the same way (`regular`).
15. A queue `edit` took empty content. **Fixed** (`7c959b9`):
    `invalid_request`, as an empty prompt.
16. `project.init` made folders before checking the root. **Fixed**
    (`493c45c`): the folder it would be is checked first.
17. Weaker root checks. **Fixed** for the home's parents (`1462f84`).
    **Left**: a project added at a `.git` folder or a bare repository
    is a `git: false` project whose `config` and `hooks/` the file calls
    can write; refusing those is a rule for track B's contract (which
    roots are repositories in disguise), reported rather than guessed.
18. On Windows two setups at once could share a script's name.
    **Fixed** (`6ba54d9`): the name has the process, the index and a
    counter, and the file is created new.
19. Rules broken:
    - `file.find` keyed callers by a `Debug` text. **Fixed** (`589a07a`).
    - The CLI showed `Debug` names. **Fixed** (`43d82fa`).
    - `file_in` answered a `String`. **Fixed** (`be03a30`).
    - A `JoinError` became an empty list, then `unreachable!`. **Fixed**
      (`a236b6b`).
    - A blocking `is_dir` on the runtime in `place`. **Fixed** (moved to
      the blocking pool with the resolve).
    - Network stderr kept 256 KiB against the contract's 64. **Fixed**
      (`006f7ef`). The name index's 64 MiB listing against 16: **kept**,
      for its 500,000 paths, and now said in ADR 0023 and the code.
    - `--end-of-options` missing in six commands. **Fixed** (`b861785`).
    - **Left**: `WorkspaceError::Io(String)` and its ten call sites
      (track B's error type, a refactor of its own); `Busy` decided by
      git's `index.lock` text (git gives no other sign: the exit code is
      128 for every fatal error); track A's Windows `is::<Refused>()`
      downcast in the store; `find.rs`'s refresh outside the task
      tracker, more than 8 indexes while an evicted build runs, and its
      walk outside git not counting folders (track B's index, for its
      next wave).

### New ids and names on the wire, for `aliases.rs`

Every newtype protocol 10 adds to the wire (W's aliases file):

- Track A, handles: `AttachmentId`, `QuestionId`, `QueueEntryId`,
  `UploadId`; named: `AttachmentName`, `CommandName`, `FieldName`,
  `MimeType`, `CostUnit`; a quantity: `FiniteNumber`.
- Track B, handles: `ProjectId`, `WorktreeId`, `SearchKey`; content
  hashes, best as handles: `CommandsDigest`, `ContentHash`; named:
  `BranchName`, `RefName`, `RemoteName`, `CommitId`, `CloneUrl`,
  `CommitMessage`, `FolderName`, `ProjectCommand`, `ProjectName`,
  `RelPath`, `FindQuery`, `GrepQuery`.

### The gate, on the final tree

- `cargo fmt --all --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo clippy -p vessel-proto -p vessel-web --features ts --all-targets
  -- -D warnings`: clean.
- `cargo test --workspace --no-fail-fast`: 1,287 passed, 0 failed, 8
  ignored, over 94 test binaries.
- The TypeScript regeneration (`VESSEL_WRITE_TS=1 cargo test -p
  vessel-proto -p vessel-web --features ts --lib typescript`): passes,
  and `web/src/wire/proto.ts` does not change.
- In `web/`: `npm ci`, `npm run lint`, `npm run typecheck`, `npm run
  test` (149 passed), `npm run build`, then the node's binaries built
  and `npm run e2e` (23 passed). Nothing under `web/` was restructured.

## Integration (web): tracks P and W

Branch `backend/m11-int-web`: track W's `backend/m11-page` with track
P's `backend/m11-perf` merged into it (`d58e7fd`), then the follow-ups
below, each its own commit. No crate outside `vessel-web` was touched
(nor `vessel-cli`'s one file of track W, nor `vessel-proto/src/ts*`).

### What conflicted, and how each was resolved

- `features/session/timeline.tsx`: P's windowed list and W's `reveal`,
  `scrollerRef` and folds. The merge kept P's window and drew W's
  `TimelineEntry` in each row; going to a key that is not drawn followed
  (below).
- `features/session/structured-session.tsx`: W's find bar, outline and
  folds around P's timeline. P's `key={session.id}` on the timeline was
  dropped: the session screen is keyed by its session already
  (`session-page.tsx`), so every state under it starts afresh.
- `main.tsx`: W's `startPreferences()` and P's gallery loaded on demand
  (`kit-gallery/path.ts`).
- `data/fleet.ts`: both comments; P's unchanged-list path with W's
  polling while notices are on and its typed record walks.
- `data/timeline.test.ts`: P's new test of the reducer's index with W's
  branded fixture ids (`fixtureId`).
- `Cargo.lock`: P's, which already listed `chrono` for `vessel-web` (as
  P's dev-dependency); `cargo metadata` changed nothing.
- `docs/ARCHITECTURE.md` and this plan: every track's lines kept; P's
  progress moved to the end, before W's, and "Left open" before the
  contracts, so the file ends contract W, progress P, progress W, this.
- Files that merged without a conflict but were wrong together:
  `crates/vessel-web/src/files.rs` (P's compressed forms and W's paint:
  the merged `respond` sent the painted index with a brotli
  `Content-Encoding` over plain bytes, and did not build); the index now
  has an answer of its own (below).

### Changed beyond resolving

- **Going to a key** (`c66d187`, `c4cdd73`, `4731047`, `1c1248a`). The
  window's rows are `data/rows.ts`'s: a closed run of tool calls is one
  row, its members are not rows at all; the reducer's indices stay in
  the reducer and only keys cross. `useWindow` takes a target: it
  scrolls to the row's estimated place so it is drawn in the same frame,
  then `window-pin.ts` holds it there after every change of the rows,
  their heights or the view, before paint, with the browser's anchoring
  off while held; a wheel, a touch, a press, a key or any scroll not its
  own lets go. The end is followed only within 48 px of it. Find's mark
  is put back on a row drawn again or grown; the outline's jump gives
  the prompt focus. `e2e/timeline.spec.ts`: 20,000 entries, a match far
  above, one far below, a prompt 20,000 entries up, the same with tool
  calls collapsed, each in place from its first frame to a second later.
- **The index** (`d58e7fd`, `59974c6`). Painted per request, so the one
  text file not compressed (`files::index`, `compress.mjs` skips it): no
  `Content-Encoding`, no `Vary`, no entity tag. The end-to-end test reads
  the first animation frame over the slow link and finds it dark. ADR
  0017's amendment and ADR 0028 say so.
- **The bundle**: W's Settings sections went into Settings' chunk, the
  find bar and the outline into the session's, the preferences, the
  shortcut listener, notices and the sidebar's display into the entry;
  every chunk is in the import map; the content policy tests pass. The
  race P fixed was re-checked with W's routes: Settings' redirect got
  the index's rule (`db68394`), and a test holds Settings' chunk while
  Mod+, then N are pressed (the redirect is not drawn again today, so
  the test passes with or without the rule; it guards the flow).
- **Find**: it moves to a match when the match changes, not when a
  refresh finds it again (`d11cbd6`); an underscore inside a word is the
  word's (`684d9d2`); it yields by a message, not a 4 ms timer
  (`0a52c3d`: 547 → 144 ms typed to the count over 20,000 entries).
- **ADR 0025 → 0028** (`7f993f0`), each mention read for what it is
  about. Small: `files.rs`' tests in `files/tests.rs` (`14bea1a`),
  `chrono` once in `vessel-web` (`e452991`), the committed
  `tsconfig.tsbuildinfo` removed and ignored (`94f0bd0`).
- **`npm run perf`** measures find and going far (`360a33f`);
  `docs/PERFORMANCE.md`, "Tracks P and W merged", has both trees' figures
  side by side.

### Review

An independent reviewer (Opus, read-only) read the merged diff of `web/`
and `crates/vessel-web`. Found, and what became of each:

1. Find never searched while a session streamed (every batch reset its
   timer and cancelled the search). Fixed: `use-search.ts` (`3e5f8b3`).
2. The outline read every message's Markdown on every batch and redrew
   every mark. Fixed: previews once per entry, an unchanged outline
   kept, memoised (`726b160`); measured, it was most of the timeline's
   regression.
3. The palette's shortcut lived in the sidebar, gone when it was hidden.
   Fixed: the shell's `PaletteHost` (`d627ea8`), with a test.
4. "Default" profile could not be chosen once another was remembered.
   Fixed: the draft's `chosen`, `startingPicks` tested (`d3deb68`).
5. The highlight matched by its own rule (case only). Fixed: `spansOf`
   in `data/find.ts`, the count's folding mapped back (`929f32f`).
6. A mark in a streaming message collapsed with its text node. Fixed:
   marked again when the entry changes, a collapsed range is gone
   (`1c1248a`).
7. Only the appearance is painted into the index (a hidden sidebar
   shows for a moment). Not taken here: what the index carries is ADR
   0028's contract; widening it is track W's or the coordinator's call.
8. Every preferences push replaced every section. Fixed:
   `keepSections` (`ae700ef`).
9. The sidebar's status grouping reads the whole fleet state. Not
   taken: it copies the machine grouping's existing pattern, the fleet
   state changes only when something did (track P), and the board at
   rest measured no long task.
10. Find's marks survived a session switch. Fixed (`726b160`).
11. A scroll the browser clamps at the end released the pin. Fixed: a
    clamp is the pin's own (`1c1248a`).
12. The row lookup ran on every render. Fixed: memoised (`1c1248a`);
    the composer's `entries.find` it also named is the pending
    permission's, base code, left.

Found while measuring, not by the review: an audio context opened on
every page's first click whatever the setting (`b22cef0`); the outline
read the layout on every scroll frame while not drawn (`5efa82c`).

### Measured

Track P's tree alone and this one, release builds, `npm run perf` back
to back (`docs/PERFORMANCE.md` has every row and the reasons):

| Figure | P alone | Merged |
| --- | --- | --- |
| board usable, cold, slow link | 2,573 ms | 2,664 ms |
| … on a phone | 3,238 ms | 3,274 ms |
| bytes for a cold board | 326,629 B | 335,114 B |
| 20,000 entries: last entry drawn | 2,645 ms | 2,845 ms |
| … a scroll step; heap; elements | 33 ms; 12.8 MiB; 501 | 33 ms; 13.1 MiB; 507 |
| 500 cards at rest: long tasks per 10 s | 0 | 0 |
| find, 20,000 entries: typed to the count | — | 138 ms |
| a match far away: typed to it shown; moved after | — | 154 ms; 0 px |

### Gate (final tree)

```
cargo fmt --all --check                                   fmt exit 0
cargo clippy --workspace --all-targets -- -D warnings     Finished; exit 0
cargo clippy -p vessel-proto -p vessel-web --features ts --all-targets -- -D warnings
                                                          Finished; exit 0
cargo test --workspace --no-fail-fast                     83 suites: 1,099 passed, 0 failed, 8 ignored
                                                          (a_failed_probe_says_why_and_never_hangs: ok this run)
cargo test -p vessel-proto -p vessel-web --features ts --lib typescript
                                                          2 passed (declarations current)
web: npm ci                                               exit 0
web: npm run lint                                         exit 0 (prettier, fonts: 132 stacks, controls, size: within budget)
web: npm run typecheck                                    exit 0
web: npm run test                                         52 files, 232 tests passed
web: npm run build                                        exit 0
web: npm run e2e                                          36 passed (1.7m)
web: npm run screens                                      5 passed (2.4m)
web: VESSEL_CARGO_PROFILE=release npm run perf            5 passed
```

One e2e run during the integration, beside a fat-LTO release build,
failed once on a tab sent to "Open Vessel from its address" (W's
shortcut test); it passed alone and in the full run since. Not
reproduced, not explained.

### Not verified

- Firefox and Safari (only Chromium runs the tests); the pin's hold in a
  browser whose scroll anchoring differs.
- A far row among entries of very different heights (the 20,000-entry
  log's are alike); the outline's jump with thousands of prompts.
- The first entry drawn about 50 ms later than track P alone, in every
  pair: the session screen mounts more; not separated further.

## Wave 1 joined (2026-11-12)

The node's integration (`backend/m11-int-node`) and the page's
(`backend/m11-int-web`) were merged into `backend/m11` by the
coordinator, in that order.

**What conflicted in text** (seven files): this plan, `README.md`,
`docs/ARCHITECTURE.md`, `docs/PROTOCOL.md` (each side had added to the
same place; every section kept, in the order contracts, progress,
integrations), `crates/vessel-session/Cargo.toml` (the benches' `serde_json`
was already a dependency for the queue's bound), and the timeline's
reducer and model (the conversation's state and the id indexes both
kept).

**What did not conflict in text and was still wrong together**, each
found by a check and not by reading:

- The generator's test listed 27 primitive aliases of protocol 10 in no
  class. Classified in `ts/aliases.rs` by its rule: handles are the ids
  the node mints, the two hashes the page only hands back, a question's
  field name and the page's search key; a commit id stays plain because
  a person may type an abbreviated one; a project's command line is
  content.
- Two types named `Conversation`: the protocol's (a session's queue,
  commands and questions) and the bridge's preference (how tool calls
  are shown). The bridge's is `ConversationDisplay`; its field and its
  change's tag keep their names, so a preferences file written before
  reads the same.
- The compiler found the speed track's load test and one agent test
  written against the wire as it was before protocol 10: a turn's
  `usage`, a cancel's `turn`, three new session events in an exhaustive
  match.
- The lint found the reducer one line over its budget once both sides
  were in it; joining text chunks moved to the model module, where what
  an entry is already lives.

**Found by looking at the captures**, fixed on the page's integration
branch before the merge: a tool call's card sat in a second box. The
timeline's row and the card shared the class name `tool`, so the row
drew the card's border again around it (since milestone 10). The card is
`tool-card`, and an end-to-end test holds the row to no border.

**The gate on the merged tree** (`79de0d3`):

| Step | Result |
| --- | --- |
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo clippy -p vessel-proto -p vessel-web --features ts --all-targets -- -D warnings` | clean |
| `cargo test --workspace --no-fail-fast` | 95 suites, 1,339 passed, 0 failed, 9 ignored |
| the wire's TypeScript, regenerated | no difference |
| `npm run lint`, `typecheck` | clean, no file over its budget |
| `npm run test` | 236 passed |
| `npm run build` | 13 files, 1,359 KiB, 338 KiB as brotli |
| `npm run e2e` | 36 passed |

**Open, carried into wave 2**:

- From the node integration's review (its section lists all nineteen
  findings): a project folder's own `.git/config` can name programs git
  runs; a `.git` directory or a bare repository is accepted as a
  project's root; a short gap between checking a place and starting a
  session in it; clones share the slots of other git runs. Track B2,
  with an ADR.
- Not verified on this machine: Windows and macOS (the node tracks list
  what only CI can confirm), Firefox and Safari, a real remote for a
  clone (a loopback HTTP server stands in), a real agent's turn (a turn
  costs money; the fakes are free).
- Protocol 10 has no screen yet beyond what wave 1's page track drew
  from protocol 9. Tracks A-page and B-page.
- The session list is still polled every two seconds; a subscription is
  a call of its own (the speed track's note), not scheduled.

## Wave 2: tracks (written 2026-11-12, after wave 1 was joined)

Wave 1 left the node able to do things no screen shows, three board lanes
that nothing fills, and a review's open points. Wave 2 is in two halves
because four workers at once is what there is room for. Each half is two
page tracks or two node tracks beside two of the other kind; a half's
tracks are joined (as wave 1's were: the node tracks together, the page
tracks together, then the two) before the next half starts from the
result.

The rules of wave 1 hold ("Rules for every feature track"), with these
added from what wave 1 taught:

- **Protocol 11 is wave 2's.** One number for the whole wave, raised once.
- **Numbers are given out here, not taken**: ADRs and store schemas are in
  each track's entry below. A track that needs one more asks.
- **Every commit passes the whole gate**, the end-to-end tests included,
  not only the crate it touches (wave 1's node integration has a stretch
  of commits where seven end-to-end tests fail, found only at the end).
- **A file that passes the budget is divided in the track that grew it**,
  not left for the join.
- **Every primitive alias a track adds to the wire is classified** in
  `crates/vessel-proto/src/ts/aliases.rs` in the commit that adds it; the
  generator's test fails otherwise.
- **A page track is judged by looking** (milestone 10's rule): captures
  beside the references' screens, light, dark and at 390px, before it
  reports.

### Half 1

**Track A-page: the conversation on the page** (`backend/m11-a-page`).
Everything protocol 10's conversation can do, in the session's page and
the new-session page, taken from the three references' composers:

1. The queue above the composer: each waiting prompt, edit, move, remove,
   send now; held, and why, with resume. The send button's choice while a
   turn runs (queue, steer, interrupt), offering steer only where
   `abilities.steer` says so, and the default kept in preferences.
2. Attachments: paste, drop and pick; a tray of what is attached with
   progress and failure per file; images shown in the prompt they went
   with; offered by `abilities` and bounded by `limits` before anything
   is uploaded.
3. `/` and `@`: one suggestion menu in the composer; `/` lists the
   agent's commands (`commands_changed`), `@` finds files of the
   session's project (`file.find`, one search key per field, the
   previous search replaced). Where the session has no project, `@` says
   so instead of offering nothing.
4. Questions the agent asks, as a form over the composer in the place
   the permission dock has, each field by its kind, declined or answered.
5. The context meter beside the model, with what it counts on hover; a
   turn's tokens in the timeline's turn line.
6. A session's own actions: pin, archive, bring back, start again
   (`session.resume`, and what it says when the agent cannot), delete
   (and what became of its worktree), unread marks in the sidebar and on
   the board; the board's archive showing archived sessions, and ended
   ones staying in their lane.
7. Left over from wave 1's captures: a machine's sessions in the sidebar
   are not set in from the machine's own row, so the two read as one
   list.

**Track B-page: code on the page** (`backend/m11-b-page`). Projects,
worktrees, git and files, from the references' workspace screens:

1. Projects: a list (the sidebar's group and a Settings section), add a
   folder, clone with progress, start new; a project's commands shown to
   be approved, with what changed since the last approval.
2. Where a session runs, on the new-session page: the folder as it is, a
   new worktree from a base branch, or an existing branch; setup's output
   as it runs; the session's branch on its card, its row and its page.
3. A panel beside the conversation (the inspector's place, as tabs with
   the summary): Changes (working, staged, everything since the base;
   per file, typed hunks, added and removed counts), Files (the tree,
   open by name, search in files, a viewer, an editor with the content
   hash as the guard against overwriting), and git's actions (stage,
   commit, push, pull, branch, stash, discard) with each typed failure
   said plainly.
4. A selection in a diff or a file sent to the composer as a reference.
5. Removing a worktree that would lose work: what would be lost, and the
   choice to snapshot or discard.

**Track B2: pull requests** (`backend/m11-forge`; ADRs 0029 and 0030,
schema 10). Rust, wire, CLI, agent tools.

1. A forge behind one trait; GitHub first, through the machine's own
   `gh` or a token the machine already holds (decided in ADR 0029 after
   reading how each reference authenticates; Vessel stores no forge
   token of its own unless the ADR finds it must).
2. Open a pull request from a session's branch; read one (draft, checks,
   reviews, mergeable, merged or closed); merge one (the method given,
   the head it was looked at given, so a push in between refuses).
3. The node watches the pull requests of its live sessions, bounded and
   backing off, and their facts become the session's board placement:
   the three lanes ADR 0019 left empty are filled by the reducer already
   ported for them.
4. Failing checks, requested changes and unresolved comments sent to the
   session that owns the branch, once per distinct event, off by
   default per session.
5. From wave 1's review, with ADR 0030: git never takes configuration a
   repository's own folder supplies as a reason to run a program
   (`core.fsmonitor`, `core.sshCommand`, external diff and the like in a
   project's `.git/config`), a folder that is itself a `.git` directory
   or a bare repository is refused as a project, the gap between
   checking a place and starting a session in it is closed, and clones
   have slots of their own.

**Track C: shells and scripts** (`backend/m11-shells`; ADR 0031, schema
11). Rust, wire, CLI, agent tools.

1. Shell terminals beside a session or in a project, with no agent:
   created, listed, renamed, ended, their output read back; they outlive
   the page and are ended with their session's deletion.
2. A project's named scripts (the file shaped in wave 1 gains them):
   run, stop, restart, their output; a script that is a service, kept
   running, restarted on failure within a bound.
3. The ports a session's or a script's processes listen on, found and
   reported; a port offered to the page only through the bridge, never
   by opening it to the network.
4. Agent tools for terminals: create, read, type, end, within what an
   agent may already do.
5. A script is a repository's command: it runs under the approval of ADR
   0027, the same digest covering it.

### Half 2 (after half 1 is joined)

- **C-page**: shell tabs, the script runner, a preview panel for a
  reported port. **B2-page**: the pull request on a session's page and
  card, a pull requests page.
- **D**: schedules, the recurring prompt, saved prompts, webhooks; the
  starter told when its session finishes. **E**: one inbox, push.
  **G**: any ACP agent, the registry, install and sign-in. Their pages
  with them or after, decided when half 1 is in.
- **A2**: rewind, fork, import from the agent's history, search across
  sessions, parking idle agents.

## Wave 2, half 1: where it stands (2026-11-12)

Three of the four tracks finished on their branches: B-page
(`backend/m11-b-page`), B2 (`backend/m11-forge`) and C
(`backend/m11-shells`). A-page's worker stalled for six hours with its
first slice uncommitted and was restarted; it is still building.

The node's two are joined first (`backend/m11-int2-node`: C into B2), as
in wave 1. Besides the merge, the join carries what the tracks found in
each other:

- From C: the short window where a terminal starts in a worktree being
  removed, closed inside the removal's own claim.
- From B-page, two gaps in the wire: `ProjectCommands` gains the set last
  approved, so the page can say what changed; and a file's write is
  guarded by something a page can carry exactly (the content hash), not a
  nanosecond time past what a JavaScript number holds.
- C's condition on the page (the approval dialog shows a set's scripts, or
  refuses a set that has any) is the page's join.

The page's join (A-page with B-page, then with the node's) waits for
A-page.

### Started before half 1 is joined: tracks D and G

Two node tracks of half 2 that build modules of their own, started now so
workers are not idle while A-page finishes. They branch from
`backend/m11` and are joined with the rest; the rules of "Wave 2: tracks"
hold (protocol 11, numbers given out here, every commit passing the whole
gate, files divided where they grow, aliases classified).

**Track G: any agent that speaks ACP** (`backend/m11-agents`; ADR 0032,
schema 12 if one is needed). Rust, wire, CLI.

1. An agent by its command: a name, a command and arguments, an
   environment, kept in the node's configuration, started over ACP like
   the four Vessel knows, and offered everywhere a known agent is (a
   session's agent, the options probe, profiles). `HarnessKind` is a
   closed set on the wire today; how an agent outside it is named without
   breaking an older peer is the ADR's first question.
2. The ACP registry as a catalogue: read from the registry's published
   list, cached and bounded, never run from it without a person adding
   the agent; what is run is what the person saw.
3. Whether each agent is installed and signed in, as typed facts with the
   reason, checked without starting a turn; the four known agents first.
4. Installing and signing in: what the node can do for a person, and what
   it only explains. Nothing is installed from the network without the
   person asking for that agent on that machine.

**Track D: schedules and saved prompts** (`backend/m11-schedules`; ADRs
0033 and 0034, schema 13, which became 12 at the join: track G needed
none). Rust, wire, CLI, agent tools.

1. Schedules: every so often, or at a time (cron with a time zone),
   starting a new session or sending a prompt to an existing one through
   the queue; paused, resumed, run once now, ended after a count or a
   date; their runs kept as a bounded history. They run in the node, and
   what was missed while it was down is caught up within a stated bound,
   never all at once.
2. A recurring prompt a session sets for itself (Paseo's heartbeat), as
   a schedule whose target is its own session, ending with it.
3. Saved prompts: named, per project or for the machine, with what a
   session started from one runs with.
4. The starter told when a session it started finishes or needs it,
   through the queue, once.
5. A webhook that starts a session (ADR 0034): a listener the node opens
   only when a person configures one, on loopback unless told otherwise,
   each hook with its own secret checked in constant time, deliveries
   deduplicated, bounded and logged without their bodies. If the ADR
   finds this cannot be made safe within the track, it is described and
   left.

## Track B2: contract

Protocol 11 (wave 2's; track C raises the same number, once).
ADR 0029 (the forge, and how the node signs in to it), ADR 0030 (git
runs independent of what a repository's folder supplies), store schema
10. Everything below is new except where it says which shared type it
touches.

### What the references do (read 2026-11-12)

agent-orchestrator (AO), GOAT and Paseo at the revisions the feature
tables were read from; their paths are in the inventories.

- **Signing in.** None keeps a forge token of its own for pull
  requests. AO reads `AO_GITHUB_TOKEN`, then `GITHUB_TOKEN`, then runs
  `gh auth token`; the token is kept five minutes and dropped on a 401;
  requests are its own HTTP (REST and GraphQL). GOAT: a token saved in
  its settings (a file, mode 0600), else `GH_TOKEN` or `GITHUB_TOKEN`
  for `github.com` and `*.ghe.com` only, else `gh auth token --hostname
  <host>` with `GH_DEBUG` blanked (it would print the token) and
  `GH_PROMPT_DISABLED=1`, ten seconds, four `gh` at once; hits kept five
  minutes, misses ten seconds; its own HTTP, and a redirect to another
  origin drops the header. Paseo runs `gh` for every call (`gh api`,
  `gh pr …`), with `GH_HOST` set to the repository's host. Signed in as
  someone else: AO reads `GET /user` and follows only pull requests that
  login authored; GOAT falls back to `gh`'s default account; Paseo does
  not look. `gh` missing or signed out: each reports it (`unavailable`,
  `missing-tool`, `cli_missing`) and tries again later. A host is
  GitHub when it is `github.com` (or `*.ghe.com`), or when `gh` is
  signed in to it; none sends a credential to a host it is not signed in
  to.
- **The abstraction.** AO: a read port (parse a repository, a
  conditional list of open pull requests, a conditional check of a
  commit's checks, read pull requests in a batch, failed-check logs,
  review threads) and write ports (merge, request review, resolve).
  GOAT: a provider (list by head, get, create, check out) and a pull
  request API (detail with head, checks and merge capabilities,
  actions with an optional pinned head). Paseo: list, get, the current
  pull request of a branch, create, merge, auto-merge. GitLab differs in
  the same places for all three: the number is the merge request's
  `iid`, checks are the head pipeline's, there is no "changes
  requested" (approvals only), mergeability is `detailed_merge_status`,
  and a pinned head is honoured for merge and squash but not rebase.
  Gitea and Forgejo: no pinned head, states named differently.
- **Which pull request is the session's.** AO lists a repository's open
  pull requests and matches their head branch to a session (and `claim-pr`
  by URL); GOAT links them explicitly (created from its page, parsed
  from an agent's `gh pr create`, or by hand); Paseo asks for the
  branch's (`head` and the repository's owner), preferring an open one,
  then a closed one whose head is the local `HEAD`, then the most
  recently updated. Forks: AO and Paseo require the head owner.
- **Reading.** All three read with GraphQL: draft, state, mergeable and
  `mergeStateStatus`, review decision, head and base, updated at, the
  head commit's check rollup; AO and GOAT also review threads. AO maps
  GitHub's values to the facts its board reducer (Vessel's, ported in
  milestone 9) reads; that table is taken whole below.
- **Watching.** AO every 30 s, review threads at most every 2 minutes,
  a full read of any pull request older than 5 minutes (its two ETag
  checks, the open list and a commit's check runs fetched as a full
  page, notice neither a merge nor a close). GOAT every 2 minutes
  through a cheap fingerprint, a full read every 30; 4 at once, 8
  requests at once; stops background reads below 10 % of the quota.
  Paseo only while someone looks, 20 s with checks pending, else 120 s,
  errors doubling to 300 s; pauses a host at 40 % of its quota. Rate
  limits: AO treats a 429, or a 403 with no requests left, a
  `retry-after` or "rate limit / abuse / secondary" in the body, as a
  limit; waits `retry-after`, else until the reset, else about 60 s,
  clamped to 30 s to 10 minutes. GOAT doubles from 30 s to 15 minutes.
  Every request 30 s at most. AO stops at a merge, a close, or the
  session's end.
- **Opening.** AO: none (its agents run `gh pr create`). GOAT: the branch
  must be pushed; an open pull request for it is answered as it is.
  Paseo pushes first (`git push -u origin <branch>`). Neither offers a
  draft; both generate a title and body.
- **Merging.** AO: squash only, with the head the caller saw; a fresh
  read must still show that head; REST `PUT .../merge {sha,
  merge_method}`; 409 is "head changed", 405 and 422 "not mergeable".
  GOAT: merge, squash or rebase, GraphQL with `expectedHeadOid`. Paseo:
  `gh pr merge` without a pinned head. None deletes the branch with it.
- **Feedback.** AO sends failing checks, each counting unresolved
  comment and each changes-requested review, and a conflict, to the
  session; identity: `ci:<url>` with each failed check's name, commit,
  status and URL; `comment:<url>:<comment id>`; `review:<url>:<review
  id>`; on by default; refused (not marked sent) while the session waits
  on its user. GOAT wakes the agent per event kind (newly failed checks,
  checks passed, others' comments, a conflict), after the running turn
  ends, never steering it; on by default; bots ignored. Paseo: none.
- **Agents.** AO's agents run `gh` themselves and are told not to merge
  (a prompt, no check). GOAT's agents open and merge through its tools
  unless read-only. Paseo's agents have no pull request tool.

### Where it lives

- **`vessel-forge`**, a new crate (depends on `vessel-proto` and
  `vessel-proc`; `reqwest` and `rustls` as `vessel-llm` has them): the
  `Forge` trait, a remote's URL read into a repository, GitHub over
  REST and GraphQL with conditional requests and rate limits, signing in
  (ADR 0029), and a test double (`fake`, behind a feature only tests
  enable). No session, store or principal. ADR 0001's map gains the
  edge `vessel-node -> vessel-forge`.
- **`vessel-node`'s `forge` module**: which sessions are watched, the
  watcher task, the calls, feedback through the queue, and the store's
  schema 10.
- **`vessel-session`**: one watch per session carrying its pull
  request, read by its actor; the facts live in its activity tracker,
  so the reducer that already places it (`board::derive`) gets them. No
  second place decides a lane.
- **`vessel-workspace`**: ADR 0030 (how every git run is made), a
  third pool for clones, and the branch, remote and push `pr.open`
  needs.

### Types (`vessel_proto::forge`)

Parsed on decode (a bad one does not decode). Every one of them that
comes from a forge (a title, a check's name, a login, a URL) is made by
the node with a lossy constructor that cuts and replaces what the rule
refuses (or drops an optional one), so a forge's text can never make a
reply fail to decode (`session.list` carries the summary):

- `PullNumber`: 1 to 2^53 − 1 (what JavaScript holds exactly).
- `ForgeHost`: a host name as a remote names it, lower case, 1 to 253
  bytes of `a-z 0-9 . -`, labels not starting or ending with `-`, an
  optional `:port`.
- `RepoPath`: `owner/name`, 2 to 8 segments (GitLab's nested groups),
  each 1 to 100 bytes of `A-Z a-z 0-9 . _ -`, not `.` or `..`, no
  leading `-`; `.git` taken off the end by whoever parses a remote.
- `ForgeLogin`: 1 to 100 bytes of `A-Z a-z 0-9 - _ .` and a trailing
  `[bot]`.
- `PullTitle`: one line, 1 to 256 characters, no control character.
- `PullBody`: at most 64 KiB, no NUL.
- `ForgeUrl`: an `https` URL (`http` only for a loopback host, for the
  tests' fake), at most 2 KiB, with no user information.
- `CheckName`: 1 to 256 characters, one line.
- `ConfigKey` (ADR 0030): a git configuration key named in a refusal,
  1 to 256 characters, one line; never its value.

Every reply enum that will grow has `unknown`, as protocol 10's do.

- `ForgeKind`: `github`, `unknown`. GitLab and Gitea are later
  implementations of the same trait.
- `RepoRef { kind, host, path }`.
- `PullState`: `open`, `merged`, `closed`, `unknown`.
- `CheckState`: `pending`, `passed`, `failed`, `cancelled`, `skipped`,
  `unknown` (AO's table, below).
- `ChecksState`: `none` (no checks reported), `pending`, `passing`,
  `failing`, `cancelled` (cancelled with nothing running), `unknown`.
- `CheckRun { name, state, url? }`.
- `Checks { state, passed, failed, pending, truncated, failing:
  Vec<CheckRun> }`, counts `u32`, `failing` at most 20, `truncated` when
  more checks were reported than one read takes (100).
- `ReviewDecision`: `approved`, `changes_requested`, `review_required`,
  `none`, `unknown`.
- `Mergeable`: `mergeable`, `conflicting`, `blocked`, `unstable`,
  `behind`, `unknown`.
- `Reviews { approved, changes_requested, commented, unresolved }`:
  people's reviews on the current head, the latest per person, bots left
  out (AO's `ExternalReviewFacts`); `unresolved` the count of unresolved
  review threads.
- `MergeMethod`: `merge`, `squash`, `rebase`.
- `PullSummary { repo, number, url, title, state, draft, head_commit,
  checks: ChecksState, review, mergeable, updated_at }`: what a card
  needs, carried in `SessionInfo`.
- `PullRequest { summary, author?, head_branch: BranchName, base:
  BranchName, checks: Checks, reviews, created_at, merged_at?,
  closed_at?, merge_methods: Vec<MergeMethod>, read_at }`: everything
  `pr.get` answers; `merge_methods` what the repository allows, `read_at`
  when this node last read it.
- `RefusedReason`: `unauthorized` (401), `no_access` (404 or 410 for a
  repository the login cannot see, 451), `sso { url? }` (403 asking for
  single sign-on), `forbidden` (another 403), `unknown`.
- `GhState`: `missing`, `signed_out`, `failed`, `unknown`.
- `ForgeAccess`, why the node can or cannot see a session's pull
  request: `ready { login? }`; `not_placed` (no project or worktree, so no
  repository); `no_branch` (the session's folder is on no branch);
  `on_base` (it is on the project's default branch, where no pull request
  starts); `no_remote` (the branch's remote, or `origin`, does not
  exist); `not_supported { host }` (no provider for that host on this
  node); `signed_out { host, gh }`; `refused { host, reason, login? }`;
  `rate_limited { host, until }`; `unreachable { host, retry_at }`;
  `limit` (the node watches as many as it allows); `ended` (the session is
  not live, so nothing is watched); `unknown`.
- `SessionPullRequest { access, summary?, feedback }`: what
  `SessionInfo.pull_request` holds; `summary` the last one read (kept
  while access fails later, so the card says what was last known),
  `feedback` whether feedback is on.
- `PullRequestView { access, pull_request?, feedback }`: `pr.get`'s
  answer, the whole pull request.
- `OpenOutcome`: `opened { pull_request }`; `exists { pull_request }`
  (an open one has that head already; nothing new was made); `refused {
  refusal: OpenRefusal }`; `unknown`. `OpenRefusal`: `not_pushed` (an
  agent's call, whose branch is not on the remote at the folder's
  `HEAD`), `no_commits`, `push { refusal: GitRefusal }`, `access {
  access: ForgeAccess }`, `rejected { message }` (the forge's own words,
  cut to 2 KiB), `unknown`.
- `MergeOutcome`: `merged { commit }`; `refused { refusal: MergeRefusal
  }`; `unknown`. `MergeRefusal`: `no_pull_request`, `other_pull_request
  { number }` (the session's pull request is not the one named),
  `not_open { state }`, `draft`, `head_moved { head }` (the head now;
  nothing was merged), `not_mergeable { message? }`,
  `method_not_allowed`, `merge_queue` (the repository merges through a
  queue, which this node does not drive), `cannot_pin` (this forge cannot
  pin the head for that method), `access { access }`, `rejected { message
  }`, `unknown`.
- `ForgeStatus { hosts: Vec<HostStatus>, watched, limit }`, `HostStatus
  { host, kind, access: ForgeAccess, source?: TokenSource }`,
  `TokenSource`: `environment`, `gh`, `unknown`. At most 16 hosts
  (remotes an agent controls could add hosts); never a token.
- `PullRequestList { entries: Vec<SessionPullEntry { session,
  pull_request: SessionPullRequest }> }`: watched (live) sessions only.

On shared types (the smallest additions):

- `SessionInfo.pull_request?: Box<SessionPullRequest>`: set by the
  session's actor from what the node gives it, for a live session whose
  place is a project or a worktree; absent otherwise, and cleared when
  the session ends (the store keeps none, so a stored session and the
  node's last view of it agree).
- `GitRefusal::repository_config { key?: ConfigKey, reason }` (ADR
  0030), `reason` one of `program`, `layout`, `credential`, `unnamable`,
  `too_many`, `too_large`, `unknown`.

### Calls

All protocol 11, forwardable through `fleet.forward`, and in
`vessel-web`'s list. "Agents": whether an agent principal (ADR 0013)
may call it; `own` means only on a session it may operate (its own, or
one it started), by the guard's ownership rule.

| Method | Scope | Agents | Request | Reply |
|---|---|---|---|---|
| `forge.status` | read | no | `GetForgeStatus {}` | `ForgeStatus` |
| `pr.list` | read | no | `ListPullRequests {}` | `PullRequestList` |
| `pr.get` | read | yes; `refresh` own | `GetPullRequest { session, refresh }` | `PullRequestView` |
| `pr.open` | operate | own, no push | `OpenPullRequest { session, title, body?, base?, draft }` | `OpenOutcome` |
| `pr.merge` | operate | no | `MergePullRequest { session, number, head }` | `MergeOutcome` |
| `pr.feedback` | operate | no | `SetFeedback { session, enabled }` | `SessionPullRequest` |

Every call can also answer `forbidden`, `not_found` (no such session),
`invalid_request` (a head that is not a full commit id), `unsupported`
(feedback for a terminal session), `unavailable` (the watcher's call
channel is full) and `internal`.

- `forge.status`: every host the node has watched or been asked about
  (at most 16), `github.com` always, each with what signing in found
  (`GET /user` once per token, kept five minutes). Asks nothing of a
  host it has no token for.
- `pr.list`: every watched session's pull request (at most 64).
- `pr.get`: the session's pull request as the watcher holds it, whole.
  An agent may read any session's, as it reads every session's info
  (ADR 0013); `refresh` (read now, conditional requests, so cheap when
  nothing changed; at most once per 10 s per session, else answered from
  memory) only on one it may operate, and an agent's refresh is
  background traffic (below). A session that is not live answers
  `access: ended`.
- `pr.open`: a live session only (`access: ended` otherwise). The branch
  is the one checked out in the session's folder now; the remote is the
  branch's upstream remote, else `origin`; the base is `base`, else the
  project's default branch (without its remote's name).
  - A person's call pushes first, `git push --set-upstream <remote>
    HEAD:refs/heads/<branch>` (never forced, whatever the branch's name),
    the repository's hooks running as for `git.push`, ADR 0030's
    refusals applying.
  - An agent's call never pushes (the node would run the repository's
    hooks and the user's credentials for it, which `git.push` refuses
    agents): the branch must be on the remote at the folder's `HEAD`, or
    the answer is `not_pushed`. The agent pushes with its own tools.
  - Then, if an open pull request has that head, `exists`; else it is
    created (`draft` as asked) and watched. The forge's "no commits
    between" is `no_commits`; its other refusals `rejected` with its
    message. Forks (a head in another repository) are not offered.
- `pr.merge`: a person only, a live session only. `head` must be a full
  commit id (40 or 64 digits: GitHub matches it exactly). Reads the pull
  request fresh; refuses `other_pull_request` when the session's is not
  `number`, `not_open`, `draft`, or `head_moved` when its head is not
  `head`; then asks the forge to merge with `method` and the head pinned
  (`sha`), so a push between the read and the merge is refused by the
  forge too (409 is `head_moved`, with the head read again). A 405 is
  `merge_queue` when the forge says a merge queue is required,
  `method_not_allowed` when it says the method is not allowed, else
  `not_mergeable`. Never deletes the branch, never retries, never waits
  for checks (GitHub's own branch rules decide), never automatic:
  nothing in the node calls it.
- `pr.feedback`: turns feedback on or off for a structured session
  (`unsupported` for a terminal session, which has no queue); kept in
  the store; off by default.

### Signing in (ADR 0029)

- A token for `github.com` is, in order: `GH_TOKEN`, then `GITHUB_TOKEN`
  from the node's environment (read by the binary, handed to the node in
  its configuration; a library reads no environment); else `gh auth
  token --hostname github.com`. For another host: `GH_ENTERPRISE_TOKEN`
  or `GITHUB_ENTERPRISE_TOKEN` only when `GH_HOST` names that host (a
  token is never sent to a host taken from a remote's URL by default),
  else `gh auth token --hostname <host>`; a host `gh` has no token for is
  `not_supported`, and nothing is sent to it.
- `gh` is the one on the node's search path, run with no shell, its
  working folder the node's data folder (outside any repository), its
  environment `GH_DEBUG=` (it would print the token), `GH_PROMPT_DISABLED
  =1`, `GH_NO_UPDATE_NOTIFIER=1`, `NO_COLOR=1`; ten seconds; two at once.
  Its standard output (the token) is never logged; its standard error is
  reduced to `missing`, `signed_out` or `failed`.
- A token is held in memory in a type whose `Debug` says only that it is
  one, kept five minutes (a miss ten seconds), dropped on a 401 (asked
  again at most once a minute per host); it travels only in an
  `Authorization` header marked sensitive, never in a URL, an argument,
  an error, a log or an event. Nothing of it is stored.
- Signed in as someone else: `GET /user` names the login, shown in
  `ready { login }` and `forge.status`; a repository that login cannot
  see answers `refused { no_access, login }`.

### How the node watches (bounded)

- **Per repository and branch.** A watch is keyed by `(repository,
  branch)`; every live session on that branch (in any folder) is given
  its facts. A session is watched when it is live, has a place, is on a
  branch that is not the project's default branch, and the branch's
  remote is on a host the node has a provider and a token for. At most
  64 watches per node; sessions a person started are given watches
  before sessions an agent started; past that, `access: limit`.
- **One task** owns every watch. It is told (a bounded channel, 64) when
  a session starts, is resumed, ends, is archived or is deleted, and
  drops a session at once on the last three; calls reach it through the
  same channel (`unavailable` when it is full).
- **The folder**: each cycle, one git read per session folder
  (`rev-parse HEAD --symbolic-full-name HEAD`, so the commit and the
  branch, plus ADR 0030's listing), at most 2 of the watcher's git reads
  at once; the remote's URL read again when the branch changes and every
  10 minutes. A branch that changed is found again.
- **Finding the pull request**: `GET /repos/{owner}/{repo}/pulls?head=
  {owner}:{branch}&state=all&sort=updated&direction=desc&per_page=10`,
  conditional (`If-None-Match`), every 120 s until one is found. The
  pick (Paseo's): the most recently updated open one; else the most
  recently updated closed or merged one whose head is the folder's
  `HEAD` now (so an old pull request of a reused branch name is not
  shown for new work).
- **Watching it**: every 60 s three conditional requests, the pull
  request itself (`GET .../pulls/{n}`) and its head commit's check runs
  (`.../commits/{sha}/check-runs?per_page=100`, the full page, AO's
  trap) and statuses (`.../commits/{sha}/status`); a 304 costs nothing
  against GitHub's limit. When any of them changed, or the last full
  read is older than 5 minutes (neither ETag notices everything), one
  GraphQL read gets the rest (review decision, review threads, the latest
  reviews, mergeability, the repository's merge methods). After 30
  minutes without a change the interval is 300 s; any change brings it
  back to 60 s. Every interval is jittered by up to 10 %. A merged or
  closed pull request is read once more, then the branch is found again
  every 300 s (a new pull request may be opened on it).
- **Requests**: at most 4 at once per node; a person's call goes before
  background reads (the watcher's, and an agent's refresh); per host a
  token bucket of 5 requests a second (10 at once) for background reads,
  under GitHub's secondary limit; 30 s each (10 s to connect); bodies at
  most 4 MiB; a redirect followed only to the same origin, at most 3 (a
  renamed repository); the token only in its header.
- **Rate limits** (AO's detection, GOAT's floor): `x-ratelimit-*` read
  on every answer, per resource (`x-ratelimit-resource`: `core`,
  `graphql`); background reads of a resource stop while less than 10 %
  of its quota remains, until its reset. A 429, a 403 with nothing
  remaining, a `retry-after`, "rate limit" or "abuse" in a body, or a
  GraphQL error of type `RATE_LIMITED`, puts the host in a cooldown:
  `retry-after`, else until the reset, else 60 s doubling to 15 minutes.
  Watches wait by being scheduled after it (nothing queues); a person's
  call answers `access: rate_limited { until }` at once.
- **Errors**: a read that fails (a network error, a 5xx) backs that
  watch off 60 s doubling to 30 minutes, its last facts kept, `access:
  unreachable`. A 401 drops the cached token and asks for it again.
- **What the node keeps**: in memory, per watch, the repository, the
  pull request's number, the last facts, and an ETag per request (a few
  hundred bytes); nothing of a forge is stored but feedback (below).

### The facts, and the board (ADR 0019)

The node gives each watched session's actor its `SessionPullRequest`
through a watch (latest wins; no queue). The actor keeps the facts in
its activity tracker, puts the summary in `SessionInfo.pull_request`,
and derives the board again; watchers wake when the lane, the phrase,
or the summary (not `read_at`) changes. The reducer gets
`PullRequests::Seen` with the one pull request (or none) once the node
has read the session's repository at least once; `Unseen` before that,
and for a session the node cannot see (not placed, no branch, on the
base, no remote, no provider, signed out). So a session with no pull
request, or on a node with no forge access, is placed as today; a
watched one with none yet is `awaiting_pr` between turns, as in
agent-orchestrator.

The fleet-policy facts follow the feedback switch, as in
agent-orchestrator, where one flag drives both the routing and the lane:
with feedback on, `auto_inject_ci` and `auto_inject_review` are set, so a
session whose agent is being sent its failing checks is "fixing CI" in
Validating, not "CI failing" in In review. `auto_review` stays off
(Vessel has no reviewing agent).

An ended session is not watched: the tracker drops the pull request's
facts at the final status, before that record's board is derived, and
the actor clears `SessionInfo.pull_request`, so an ended session is
`exited` in building, exactly as the store places a stored one (ADR
0026), and the test that holds the two equal keeps holding. (agent-
orchestrator keeps an exited worker in its pull request's lane; a Vessel
session ends with its process, and nothing watches it then.) ADR 0019 is
amended: the board is no longer only a projection of the session's
records; the node's pull request facts are the one other input.

The mapping, AO's (`provider.go`, `observer_provider.go`), into the
reducer's facts:

- A check: `SUCCESS`, `NEUTRAL` passed; `FAILURE`, `TIMED_OUT`,
  `ACTION_REQUIRED`, `STARTUP_FAILURE` failed; `CANCELLED` cancelled;
  `SKIPPED`, `STALE` skipped; `QUEUED`, `PENDING`, `REQUESTED`,
  `WAITING`, `IN_PROGRESS` pending; a status context's `SUCCESS`
  passed, `FAILURE` and `ERROR` failed, `PENDING` and `EXPECTED`
  pending. Only the newest run of each check name counts (a superseded
  run does not fail the head).
- Checks: any failed is failing; else any pending is pending; else a
  cancelled one is `cancelled`; else none at all is `none`; else
  passing; truncated is unknown to the reducer. The reducer's `CiState`
  takes `unknown` for `none`, `cancelled` and truncated.
- Review decision: as GitHub says; none is `none`.
- Mergeable, first rule that holds: `DIRTY` or `CONFLICTING` is
  conflicting; `BLOCKED` blocked; `UNSTABLE` unstable; a draft, failing
  checks, changes requested or a review required is blocked; `MERGEABLE`
  with `CLEAN`, `HAS_HOOKS` or nothing is mergeable; else unknown.
  `BEHIND` alone is `behind` on the wire and unknown to the reducer (its
  fact has no "behind").
- People's reviews: approved and changes requested on the current head,
  the latest per person, bots left out; `commented` when an unresolved
  thread has a person's comment.

### Feedback to the session

Off by default per session (`pr.feedback`); AO and GOAT have theirs on
by default, the owner asked for off.

- **Events**, and what makes two the same (each event's signature,
  prefixed with the repository, `<host>/<owner>/<name>#<number>:`):
  - a failing check: `check:<head commit>:<check name>`; the same check
    failing again on the same head is the same event, on a new head a
    new one;
  - changes requested: `review:<review id>`;
  - an unresolved comment: `comment:<comment id>`, for each comment of
    an unresolved thread;
  - a conflict with the base: `conflict:<head commit>`, dropped when the
    pull request is mergeable again (so a later conflict is sent, AO's
    re-arming).
- **Who is quoted**: a review's or a comment's text only when its
  author has write access to the repository (`admin`, `maintain` or
  `write`, asked of `GET /repos/{owner}/{repo}/collaborators/{login}/
  permission` and kept an hour per repository and login); others' and
  bots' are counted ("2 comments by others, not quoted") and linked,
  never quoted: on a public repository anyone can review or comment, and
  the text becomes the agent's prompt. Quoted text has control and
  format characters (Unicode `Cc` and `Cf`: bidirectional and zero-width
  ones) removed, and is set apart as quoted data, each quote at most
  1 KiB.
- **How**: after a read finds events whose signatures were not sent,
  the node sends one prompt listing them (at most 20, the whole at most
  16 KiB, then "and N more" with the pull request's address), through
  track A's queue (`Delivery::Queue`, as the owner: the person who
  turned it on; it begins "Vessel: pull request feedback"). At most one
  such prompt waits in a session's queue: while it waits, newer events
  are added to it (`QueueChange::Edit`, within its 16 KiB) rather than
  queued again, so feedback never fills the person's queue. Never typed
  into a running turn (it waits in the queue for the turn's end), never
  started while the session waits on a permission or a question (the
  node does not send then; the next read tries again), and held with the
  queue when a person holds it.
- **Kept**: the signatures sent, per session, in the store (schema 10),
  at most 1,000 per session: past that, those of events the last read no
  longer saw go first, then the oldest; deleted with the session. A
  signature is recorded once the prompt was accepted (started, queued, or
  added to the waiting one); a refusal (the queue full) records nothing,
  so the next read tries again.

### ADR 0030: git independent of the repository's folder

From wave 1's review (plan, "Integration (node)", findings 1, 7, 11,
17). Measured on git 2.43 before deciding: a `-c <key>=` given on the
command line wins over the repository's configuration and every file it
includes; an empty filter command turns a filter off, and an empty
merge driver makes git's merge of that file fail; an empty diff driver
command is run as an empty program and fails (so diffs pass
`--no-ext-diff --no-textconv` instead); `GIT_ATTR_SOURCE` (git 2.40+)
does not cover `.git/info/attributes`; aliases never shadow git's own
commands; a pager runs only on a terminal; `GIT_ALLOW_PROTOCOL` stops
`ext::`, a remote's `vcs` helper and a repository's `protocol.allow`.

- **Before every git run** in a repository the node lists the
  repository's own configuration (`git config --local --list -z
  --includes`, and `--worktree` when `extensions.worktreeConfig` is on):
  its `.git/config`, a worktree's `config.worktree`, and every file they
  include. The user's global and system configuration are the user's
  and are left alone.
- **Turned off on the command line**: every
  `filter.<driver>.{clean,smudge,process}` (and `required=false`) and
  `merge.<driver>.driver` the repository's configuration defines; and,
  on every run whatever it sets, `core.fsmonitor`, `core.hooksPath` (but
  for commit and push), `diff.external`, `submodule.recurse`,
  `fetch.recurseSubmodules`, `merge.verifySignatures`, `tag.gpgSign`,
  `push.gpgSign`, `commit.gpgSign` (but for commit), `--no-pager`,
  `GIT_EDITOR`, `GIT_SEQUENCE_EDITOR`, `GIT_ASKPASS` (empty) and
  `GIT_ALLOW_PROTOCOL=file:git:http:https:ssh` (a clone keeps its
  narrower `https:http:ssh:git`). Diffs pass `--no-ext-diff
  --no-textconv`; status, diffs and the removal's inspection never go
  into a submodule's working tree (`--ignore-submodules=dirty`; a gitlink
  keeps a worktree anyway).
- **Refused** when the repository's own configuration sets one of these,
  for the runs that would use it, because turning it off would lose the
  user's own global value: on network runs (fetch, pull, push, a
  person's `pr.open` push) `core.sshCommand`, `core.gitProxy`,
  `core.alternateRefsCommand`, `credential.helper` and
  `credential.<url>.helper`, `remote.<name>.uploadpack`,
  `.receivepack`, `.vcs` (a program), and `http.proxy`, `http.sslVerify`,
  `http.sslCAInfo`, `http.sslCAPath`, `http.sslCert`, `http.sslKey`,
  `http.sslBackend`, `http.curloptResolve`, `http.extraHeader`,
  `http.cookieFile` (each also per URL) and `remote.<name>.proxy` (where
  the credential would go); on commit and push `gpg.program`,
  `gpg.<format>.program`, `gpg.ssh.defaultKeyCommand`; on every run
  `core.worktree` (it moves the working tree out of the project's
  folder), a driver whose name holds `=` (git's `-c` would split it),
  more than 64 drivers, a configuration over 256 KiB. A typed refusal
  names the key (`GitRefusal::repository_config`, or `conflict` on a
  read), never the value. The user moves such a setting to their global
  configuration (`includeIf "gitdir:…"`) if they want Vessel to use it.
- **Kept, and said**: commit and push run the repository's hooks (its
  `.git/hooks`, or the `core.hooksPath` its configuration names), as the
  user's own `git commit` would; those two calls are a person's only
  (agents are refused them, and an agent's `pr.open` does not push). A
  key git adds later that names a program is not on the list until it is
  added with a test that plants it. A process of the user's that rewrites
  the configuration between the listing and the run is not stopped
  (ADR 0027: checks inside the node, not a sandbox).
- **A project's root**: refused when any part of its path names `.git`
  by git's own rules (any case, trailing dots or spaces, `git~1`, ignored
  zero-width characters), when it holds `HEAD`, `objects` and `refs` (a
  bare repository, told apart without git), or when git says it is inside
  a git directory or bare, for `project.add`, `project.init` and a
  clone's target.
- **Between checking a place and starting in it**: `place` (and
  `session.resume`'s check) takes a "starting" count on the worktree in
  the same lock as its check, given back once the session is live or
  failed to start; a removal claims the worktree first, then counts the
  starting ones and reads the live sessions, so a session is either seen
  or refused.
- **Clones**: a pool of their own, 2 at once and 16 waiting, apart
  from fetch, push and pull's 4.

### Agents, the CLI

- Agent tools (`vessel mcp`): `get_pull_request` (any session; refresh
  its own) and `open_pull_request` (its own; no push). No merge, no
  feedback switch: a merge publishes to a shared branch and the owner
  asked for merging to be explicit; ADR 0029 says why the node does not
  merge or push for an agent.
- CLI: `vessel pr list`, `vessel pr show <session> [--refresh]`,
  `vessel pr open <session> --title <t> [--body <b> | --body-file <f>]
  [--base <b>] [--draft]`, `vessel pr merge <session> --number <n>
  --method merge|squash|rebase --head <commit>` (the number and the full
  head are required: the ones `show` printed), `vessel pr feedback
  <session> on|off`, `vessel forge status`.

### What an older client and an older node see

- A protocol 8 to 10 client gets `SessionInfo` with a field it does not
  know (`pull_request`), which it ignores, and boards whose lanes and
  phrases are protocol 8's enums, all of which it already draws: a
  watched session can now be in Validating, In review or Ready, the
  lanes those enums were made for (ADR 0019). `GitRefusal::
  repository_config` reaches it as `unknown` (`serde(other)`).
- A client asks no protocol 11 method of a node that speaks 10 or less:
  `NotInProtocol` before sending.
- No event is added.

### Bounded, and by how much

| What | Bound | Beyond it |
|---|---|---|
| Watches | 64 per node, a person's sessions first | `access: limit` |
| Forge requests | 4 at once, a person's first; 5 a second per host in the background; 30 s each (10 s to connect); 4 MiB a body; 3 same-origin redirects | waits its turn; `unreachable` |
| Finding a pull request | every 120 s, conditional; 300 s after a merge or close | — |
| Watching one | every 60 s (300 s after 30 quiet minutes), ±10 %, three conditional requests, a full read on change or every 5 minutes | — |
| The folder | one git read per session per cycle, 2 at once; the remote's URL every 10 minutes or on a new branch | — |
| A refresh | once per 10 s per session | answered from memory |
| Rate limit | background stops under 10 % of a resource; cooldown 60 s doubling to 15 min, or as the forge says | `rate_limited` |
| Errors | 60 s doubling to 30 min per watch | `unreachable` |
| Token | kept 5 min, a miss 10 s, dropped on 401 (asked again at most once a minute) | `signed_out` |
| `gh` | 2 at once, 10 s each | `signed_out { gh: failed }` |
| Write access asked | once an hour per repository and login | counted, not quoted |
| Feedback | one waiting prompt per session, 16 KiB, 20 events, 1 KiB a quote; 1,000 signatures per session | "and N more"; unseen, then oldest dropped |
| Failing checks named | 20 per pull request; 100 checks read | counted; `truncated` |
| Hosts in `forge.status` | 16 | left out |
| The watcher's channel | 64 | `unavailable` |
| A repository's configuration listing | 256 KiB, inside the run's time and slot | the run refused |
| Clones | 2 at once, 16 waiting | `unavailable` |

### Store (schema 10)

- `forge_feedback_on (session_id PRIMARY KEY REFERENCES sessions ON
  DELETE CASCADE)`: the sessions with feedback on.
- `forge_feedback_sent (session_id REFERENCES sessions ON DELETE
  CASCADE, signature, sent_at, PRIMARY KEY (session_id, signature))`.

Nothing else of a forge is stored: the pull request is found again by
its branch after a restart.

### Review (2026-11-12)

An independent reviewer (Opus, read only) read the first version of this
section against the references, the existing wire and code, and found
twelve problems. Taken, as written above:

1. `pr.open` let an agent make the node push, running the repository's
   hooks and the user's credentials: an agent's call never pushes now
   (`not_pushed`), a person's pushes `HEAD:refs/heads/<branch>` (a name
   starting with `+` cannot force); the repository's proxy, TLS,
   resolver, header and cookie keys refuse a network run (`credential`).
2. Nested repositories: status, diffs and the removal's inspection no
   longer go into a submodule's working tree, where its own configuration
   would run.
3. Signature checks and signing on runs other than a commit are turned
   off; the protocols list is the code's (`file` kept, its `uploadpack`
   refused); the race between the listing and the run is stated; the
   root's `.git` test is git's own rule.
4. Feedback: reviews quoted by the same rule as comments; write access
   asked of the forge (an association does not say it); format
   characters removed and quotes set apart; one waiting prompt per
   session, edited, so the queue never fills; signatures carry the
   repository and are dropped unseen-first; conflicts are an event.
5. A forge's text is made lossy by the node, so a title or a check name
   never breaks `session.list`.
6. The policy facts follow the feedback switch (AO's one flag); the
   facts live in the tracker and are dropped at the end, before the
   final board, and `pull_request` is cleared then, so the runtime and
   the store agree; ended sessions answer `ended`; ADR 0019 amended.
7. The removal claims the worktree before it reads the live sessions.
8. A watch per repository and branch, sessions on the base branch not
   watched, a person's sessions first for the 64 watches.
9. Per-resource rate limits, GraphQL's own limit, typed 403s (single
   sign-on), same-origin redirects, cooldowns that reschedule rather than
   queue, a person's calls first, a token bucket per host, jitter.
10. The folder's git read per cycle bounded and counted; a changed
    branch found again; finding again after a merge.
11. `pr.merge` names the number and takes a full head; `unknown` on the
    outcomes; `merge_queue` and `cannot_pin`; `cancelled` and `truncated`
    checks; an optional login; the failure codes; a summary in
    `SessionInfo`, the whole in `pr.get`; times and merge methods.
12. The token rules stated here, not only in the references paragraph;
    `gh` run outside any repository; `forge.status` bounded.

Not taken, and why:

- *Feedback to the branch's owner session only.* Feedback is turned on
  per session by a person; two sessions on one branch both get it only
  if a person turned both on, which is their choice, and "the owner" of
  a branch in a project's own folder has no record to come from.
- *A principal of the node's own for feedback prompts.* `Creator` is
  protocol 5's shared enum, matched by every client; the prompt is sent
  on behalf of the person who turned feedback on, and says it comes from
  Vessel's pull request feedback.
- *Keeping an ended session's pull request in the store.* It is cleared
  at the end instead: nothing watches an ended session, and a summary
  kept without a watch would go stale.
- *Driving merge queues.* Refused as `merge_queue`; a later track.
- *Measuring the empty-driver behaviour on git 2.25.* Not available on
  this machine; CI runs the runners' git, and the planted tests fail if
  a driver runs.

## Track C: contract

Protocol 11 (shared with track B2, raised once). ADR 0031 holds the
decisions that could have gone another way. Store schema 11 was set
aside for this track and is **not used**: nothing below is kept on disk
(why under "What is kept, and for how long").

### What the references do (read 2026-11-12)

- **Shell terminals.** agent-orchestrator: shell tabs per session or
  standalone, surviving its daemon (a terminal host of their own); the
  shell is the user's login shell (`--login -i`), on Windows `pwsh.exe`,
  then `powershell.exe` (both `-NoLogo`), then `%ComSpec%`, then
  `cmd.exe`; titles at most 80 characters. GOAT: terminals per thread,
  `$SHELL` (else `bash`; `-l` for bash on macOS, `-o nopromptsp` for
  zsh), `pwsh.exe -NoLogo` on Windows; history kept per terminal, 5,000
  lines and 8 MiB; its own variables and `VITE_*` removed from the
  shell's environment and `COLORTERM=truecolor` added; 128 ended terminals
  kept. Paseo: terminals per working folder, `$SHELL` (else `/bin/sh`),
  `%ComSpec%` on Windows; a headless emulator per terminal with 10,000
  lines of scrollback; its own control variables removed.
- **Scripts and services.** Paseo: `paseo.json` `scripts.<name> {
  command, type?: "service", port? }`; a service gets a port Paseo picks
  (`PASEO_PORT`, `HOST`, its peers' ports as `PASEO_SERVICE_<NAME>_PORT`),
  a route on Paseo's own proxy by host name (`<script>--<branch>--<project>
  .localhost`), and a TCP health probe every 3 s (two failures: stopped);
  no restart. agent-orchestrator: post-create and pre-remove steps with a
  time limit, and `ao preview` running `.ao/launch.json` with a readiness
  probe (30 s, at most 55) and no restart. GOAT: `goat.json` scripts, run
  in terminals.
- **Ports.** GOAT scans every 3 s while a page holds the list: `lsof
  -iTCP -sTCP:LISTEN -P -n -F pcn` (5 s limit), a walk of `/proc` (at most
  50,000 descriptors) where `lsof` is missing, `netstat.exe -ano` on
  Windows (16 MiB of output at most). Paseo knows its services' ports
  because it chose them. agent-orchestrator waits for the port a preview
  was told to use.
- **Agent tools.** Paseo: `list_terminals`, `create_terminal` (any
  folder), `capture_terminal` (lines, escape sequences removed by
  default), `send_terminal_keys` (tmux key names), `kill_terminal`, and
  the workspace scripts' list, start and stop; no ownership: an agent
  can type into any terminal. GOAT: its agent reads at most 32 KiB and
  writes at most 16 KiB a call, lists at most 100.

### The decision this track turns on: what a terminal is

A shell terminal is a terminal-mode session without an agent, so it
could be a kind of session. It is **a thing of its own beside sessions,
carried by the sessions' terminal transport** (ADR 0031):

- As a session it would be on the board (a card with no turns), in every
  session list (a protocol 10 client cannot decode a spec without a
  harness, so its whole `session.list` would fail unless the node hid
  shells from it), counted in an agent's eight live sessions, offered to
  export, import, archive, resume and marks, given an agent credential and
  `vessel mcp`, and written to the store: every byte a shell prints
  (`env`, `cat .env`) kept on disk under the sessions' retention.
- Beside sessions, none of that happens, and what the session's terminal
  already does is reused, not copied: `vessel-pty` runs the shell; the
  session runtime's ring keeps the scrollback and broadcasts it; a
  terminal's log is made of the same `EventRecord`s a terminal session's
  is (`terminal` output, `status`, `vessel:` notices as `harness_log`),
  numbered gap-free from zero; it is followed through the same
  subscription frames (`ServerFrame::Event`, `SubscriptionEnded`,
  `lagged` and `resume_from`) and paged with the same `EventPage`; input
  is the same `TerminalInput`. The page's terminal view therefore draws a
  shell exactly as it draws a terminal session.
- The wire is shared; the code that follows a log is not yet. The rpc
  client's driver opens only `session.subscribe`'s routes, and
  `vessel-client`'s streams page with `session.read`; both are taught a
  terminal (a subscription answered with `TerminalSubscribed`, catching
  up with `terminal.read`), and the node's forwarder for a terminal reads
  its ring only (there is no store behind it). Listed as work, not
  assumed.

### Types (`vessel_proto::terminals`, `vessel_proto::scripts`, `vessel_proto::ports`; new modules)

Ids (`id.rs`): `TerminalId`, node-minted UUIDv4 (a handle in
`ts/aliases.rs`).

Parsed on decode:

- `ScriptName`: a script's or a service's name in `vessel.toml`: 1 to 64
  bytes of ASCII letters, digits, `-`, `_` and `.`, starting with a letter
  or a digit (a plain name in `aliases.rs`: a person types it).
- `ProcessName`: what the system calls a listening process (`comm` on
  Linux, `lsof`'s command on macOS, the image name on Windows), at most 64
  characters, control characters removed (plain: content).
- Titles are `SessionTitle` (one line, at most 120 characters).

Terminals:

- `TerminalPlace`: `session { session }` (the session's folder: its
  `cwd`, the node's home when it has none) or `project { project, dir?:
  RelPath }` (the project's folder, or a folder in it, checked as every
  `RelPath` is: resolved, inside the root, not under `.git`).
- `TerminalKind`: `shell { program }` (the program started), `script {
  script, command }` or `service { script, command }` (a run of a project
  script; `command` the `ProjectCommand` that runs), `unknown`.
- `TerminalInfo { id, place, folder, kind, title?, created_by,
  created_at, ended_at?, status, service?, size?, oldest_seq, next_seq,
  revision }`: `folder` the absolute folder it runs in; `created_by` ADR
  0013's `Creator`; `status` the process state, the session's own
  `SessionStatus` (`running`, then `exited { code }`, `failed { message
  }` or `interrupted`); `service` a service's `ServiceState`; `size` a
  shell's window; `oldest_seq` the first record still kept (older output
  has been dropped, below) and `next_seq` the next one; `revision` changes
  with any field but the two sequence numbers, for waiting.
- `ServiceState`: `starting { attempt }`,
  `running { attempt, since }`, `backing_off { attempt, code?, restart_at
  }`, `exited { code }` (it ended by itself with 0: not restarted),
  `failed { reason }` with `ServiceFailure` one of `gave_up { restarts }`,
  `not_approved` (the set it came from is no longer approved), `spawn`,
  `unknown`; `stopped` (a person, its session's deletion, its project's
  removal, or the node stopped it); `unknown`. `attempt` counts from 1.
- `TerminalList { terminals }` (at most 96: 64 live and 32 ended).

Scripts:

- `ScriptKind`: `script` or `service`.
- `ScriptEntry { name, kind, command, dir? }`.
- `ProjectCommands::Listed` gains `scripts: Vec<ScriptEntry>` (left out
  when empty).
- `ScriptPlace`: `session { session }` (its project's checkout: the
  worktree it runs in, or its project's folder) or `project { project }`.
- `ScriptList { root, commands, runs }`: `root` the folder the file was
  read in; `commands` what `project.commands` would answer for it, as it
  is now; `runs` the live and kept runs in that root, as `TerminalInfo`.
- `ScriptRunOutcome`: `started { terminal }`, `needs_approval { commands
  }` (nothing ran), `refused { reason }` with `ScriptRefusal` one of
  `no_project` (a session with no project), `no_such_script`,
  `no_such_dir` (its `dir` is not a folder in the root), `invalid_file {
  message }`, `already_running { terminal }`, `unknown`.

Ports:

- `ListeningPort { port, address, pid, process?, owner }`: a TCP socket
  in the listening state; `address` the local address as text
  (`127.0.0.1`, `::`, `0.0.0.0`), so a client tells loopback from every
  interface; `owner` a `PortOwner`: `session { session }` or `terminal {
  terminal }`, the root of the process tree the listener is in.
- `PortList { ports, scan, revision, more }`; `PortScan`: `pending` (none
  yet), `scanned { at }`, `unavailable { reason }` (the system's tool is
  missing or failed), `unknown`.

No event is added. A service's state is typed on its `TerminalInfo`
(`service`), every change bumps `revision`, and `terminal.get {
changed_from }` and `terminal.list` deliver the changes, as `project.get`
delivers a clone's progress; the run's log gets a `vessel:` line at each
change (`vessel: service exited with 1; restarting`), for a person
reading the output. A `SessionEvent` variant was considered and dropped
(review item 14): only protocol 11 calls serve a terminal's log, so its
stand-in could never be sent, and it would add an arm to about ten
exhaustive matches in other tracks' code.

### Calls

All protocol 11. Every one but `terminal.subscribe` is forwardable
through `fleet.forward` and in `vessel-web`'s list of calls a page may
make; `terminal.subscribe` is a stream, which is neither (as
`session.subscribe` is not): the bridge follows a terminal for the page
(`follow_terminal`, below), and another node's terminal is read with
`terminal.read`. "Agents": what an agent principal (ADR 0013, local or delivered)
may do; the guard refuses the rest with `forbidden` before the call
runs.

| Method | Scope | Agents | Request | Reply |
|---|---|---|---|---|
| `terminal.create` | operate | when allowed (below) | `CreateTerminal { place, size?, title? }` | `TerminalInfo` |
| `terminal.list` | read | its own | `ListTerminals { place? }` | `TerminalList` |
| `terminal.get` | read | its own | `GetTerminal { terminal, changed_from? }` | `TerminalInfo` |
| `terminal.read` | read | its own | `ReadTerminal { terminal, from, limit, wait? }` | `EventPage` |
| `terminal.subscribe` | read | its own | `SubscribeTerminal { terminal, from }` | `TerminalSubscribed { subscription, terminal }` |
| `terminal.send` | operate | its own | `SendTerminal { terminal, input }` | `Ack` |
| `terminal.rename` | operate | its own | `RenameTerminal { terminal, title }` | `TerminalInfo` |
| `terminal.kill` | operate | its own | `KillTerminal { terminal }` | `Ack` |
| `terminal.close` | operate | its own | `CloseTerminal { terminal }` | `Ack` |
| `script.list` | read | no | `ListScripts { place }` | `ScriptList` |
| `script.run` | operate | no | `RunScript { place, script }` | `ScriptRunOutcome` |
| `script.restart` | operate | no | `RestartScript { terminal }` | `ScriptRunOutcome` |
| `port.list` | read | its own | `ListPorts { changed_from? }` | `PortList` |

"Its own" means terminals the agent created, and ports owned by its own
session or by those terminals. A subscription is ended with
`session.unsubscribe`, which already takes any subscription of the
connection.

Every call can answer `forbidden`, `not_found` (no such terminal,
session, project or folder, and a terminal an agent did not create is
`forbidden`, not `not_found`), `unavailable` (a limit below, or the
node's wait shares) and `internal`. Beyond that:

- `terminal.create`: starts the shell in a PTY of `size` (80×24 when
  absent) and answers once it runs. `not_found` for a place whose
  session, project or folder is gone; `conflict` while the session's
  worktree is being set up, torn down or removed (`Workspaces::busy`);
  `spawn_failed` when the shell does not start. A session that has ended
  but is not deleted still has a folder, and a terminal can be opened in
  it.
- `terminal.list`: live terminals first, then kept ones, each newest
  first; with `place`, those of that session or project only.
- `terminal.get`: with `changed_from`, waits (at most 30 s, in the node's
  wait shares, at once when forwarded) until `revision` differs.
- `terminal.read`: a page from `from`, at most `limit` records (capped at
  512) and 1 MiB of output; with `from` older than `oldest_seq`, the page
  starts at `oldest_seq` (the scrollback that was dropped is gone, as a
  pruned session log is); `wait` (at most 30 s, against `session.read`'s
  60: a terminal is followed by subscription, a wait is for a tool's
  poll; in the wait shares, and answered at once when forwarded) waits
  for a record at `from`. `invalid_request` for `from` past `next_seq`.
- `terminal.subscribe`: the records from `from` (from `oldest_seq` when
  older), then live ones, then `subscription_ended { session_ended }` once
  the terminal ended and its last record was sent (also when it was
  closed: its final status comes first); `lagged { resume_from }` for a
  follower further behind than the live buffer (64 records), as for
  sessions, after which the client pages with `terminal.read`.
- `terminal.send`: `data` written to the shell (at most 64 KiB a call),
  or a `resize`. `unsupported` for a script or a service (they have no
  terminal input; standard input is closed); `conflict` once it ended.
- `terminal.kill`: a shell gets `SIGHUP` and, two seconds later, its
  group `SIGKILL` (as a terminal session); a script or a service its
  group's `SIGTERM` and, five seconds later, `SIGKILL`; on Windows the
  process (a shell) or its job (a script) is terminated at once. A
  service is not restarted after it. The terminal is kept, ended, with
  its output. Nothing for one already ended.
- `terminal.close`: kills it if it runs, waits for its end (at most the
  grace and 5 s more; a process stuck in the kernel is forgotten anyway,
  and logged), and forgets it; its subscriptions end after its final
  status.
- `script.list`: the place's root (a session's worktree, or its or the
  given project's folder), its `vessel.toml` as it is now, its approval,
  and its runs. `invalid_request` for a session without a project.
- `script.run`: reads the root's `vessel.toml` as it is now (a bounded
  regular file inside the root, as ADR 0027 reads it), and runs the named
  script only when that file's set is approved for the project
  (`needs_approval` otherwise, carrying it); the command that runs is the
  one parsed from the file whose digest matched, never a second read. It
  runs in `<root>/<dir>`, through the platform's shell as setup commands
  do (`/bin/sh -c`; `cmd.exe /d /c` on a temporary `.cmd` file on
  Windows), piped, standard input closed, its standard output and error
  into the run's log as `terminal` records, as one process group
  (`vessel-proc`: a group on Unix, a job object on Windows). One live run
  per root and script (`already_running`).
- `script.restart`: a live run is stopped and run again in the same
  terminal (its log continues, a `vessel: restarted` notice between, a
  service's count starts again); an ended run's log is closed, so it runs
  again in a new terminal, as `script.run` would (the old one stays kept
  until closed or evicted). Either way with the command it ran, and only
  if the set it came from is still approved (`needs_approval`
  otherwise).
- `port.list`: the last scan's listeners whose owner the caller may see;
  each call counts as interest (below). With `changed_from`, waits (at
  most 30 s, in the wait shares) for another revision.

### A shell: what runs, where, and with what

- **The program** is found once, by the binary at start (library code
  never reads the process's environment, AGENTS.md section 8), and given
  to the node in its `Config`: on Unix `$SHELL` when it is an absolute
  path to an executable file, else the account's shell (the password
  database), else `/bin/sh`; started with `-l` on macOS (as its Terminal
  starts every shell, so `~/.zprofile` and Homebrew's `PATH` apply) and
  with no argument elsewhere (a Linux terminal emulator starts an
  interactive shell that is not a login one). On Windows `pwsh.exe -NoLogo`
  when found on `PATH`, else `powershell.exe -NoLogo`, else `%ComSpec%`,
  else `cmd.exe` (agent-orchestrator's order). A node whose `Config` has
  no shell answers `terminal.create` with `unsupported`.
- **The folder** is the place's, resolved when the terminal starts; the
  shell may `cd` anywhere afterwards: a folder is where it starts, not a
  sandbox (ADR 0013's limit for everything run as the node's user).
- **The environment** is the node's own, as a terminal the user opens has
  it, with `TERM=xterm-256color` and `COLORTERM=truecolor` (what the
  page's terminal draws), less every credential of Vessel's: any
  `VESSEL_*` variable whose name ends in `TOKEN` or `SECRET`
  (`VESSEL_TOKEN`, `VESSEL_AGENT_SECRET`, `VESSEL_MODEL_SECRET`,
  `VESSEL_MCP_<n>_<m>_SECRET`) and the two that name an agent's
  connection (`VESSEL_AGENT_SESSION`, `VESSEL_NODE_SOCKET`); and, when
  any of those is there, the variables a model route sets for a harness
  (`ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN`, ADR 0015). The node never
  puts a credential in its own environment, so these are there only when
  it was started from inside a session's process tree, and then they
  belong to that other session; a node started normally leaves the
  user's own `ANTHROPIC_*` alone, as the user's terminal has them. The
  binary works out the list from its own environment at start (library
  code reads no process environment); a node in a test removes the fixed
  names. A
  session's agent credential (ADR 0013) and a routed session's model
  token (ADR 0015) are given to that session's harness only, never to a
  terminal beside it. Scripts get the same, plus setup's
  `VESSEL_PROJECT_ROOT`, `VESSEL_WORKTREE` and `VESSEL_BRANCH`, and
  `VESSEL_SCRIPT` (its name).
- **The process tree.** A shell is a session leader in its PTY
  (`vessel-pty`), stopped as a group on Unix. An interactive shell puts
  each job in a process group of its own, which a signal to the shell's
  group misses: the polite `SIGHUP` is resent to every job by bash and
  zsh, and the kernel sends `SIGHUP` to the foreground job when the
  session's leader exits; on Linux, once the shell has exited, every
  process still in its session is killed too (the session id is the
  shell's pid, which the system does not hand out again while the
  session has members), so `npm run dev &` ends with its terminal. On
  macOS a background job that ignores `SIGHUP` can outlive it; on Windows
  the pseudo console's closing ends what is attached to it (the gap ADR
  0003's terminal sessions have). A script or a service runs through
  `vessel_proc::spawn`: a process group on Unix, a kill-on-close job
  object on Windows, so its children end with it, except, on Unix, one
  that left the group on purpose (`setsid`, a daemon). If the node itself
  crashes, nothing stops a Unix group: a service keeps its port, and the
  same service started again fails to bind it (Windows closes the job
  with the node's handle).
- **The window.** Every client's resize is applied; the last one wins
  (Paseo arbitrates between attached clients; Vessel's terminal sessions
  never have, and a shell is one more of them).
- **Piped, not a terminal.** A script or a service has no PTY: a program
  that checks for a terminal prints without colour, and one that buffers
  when piped (Python without `-u`) shows its output late. Paseo and GOAT
  run scripts in a PTY; Vessel cannot put a PTY's program in a job object
  before it starts (`vessel-pty`'s Decision Log), and a service's
  children must end with it on Windows too.

### A service is kept running

- Exit with a code other than 0, or by a signal Vessel did not send: a
  restart after 1, 2, 4, 8, then 16 s; a run that lasted 60 s resets
  the count; the sixth failure without such a run gives up (`failed {
  gave_up { restarts: 5 } }`). Exit with 0: `exited`, not restarted (a
  service that ends cleanly is done, or misconfigured; restarting it
  forever would hide that).
- Before every restart the node checks that the set the command came
  from is still approved for its project (`failed { not_approved }`
  otherwise): withdrawing an approval stops what it allowed, at the next
  start, without killing what runs.
- Each change is a `service` record in the run's log and on its
  `TerminalInfo`; the log's `status` ends only when the service is given
  up, exits, or is stopped.
- Its output is the run's log, bounded as every terminal's (below), across
  restarts.
- Stopped by `terminal.kill`, `terminal.close`, its session's deletion
  or removal by retention, and the node's stop. Not by its session's end:
  a session that ended can be started again (ADR 0026) in the same
  folder, and a dev server is often what a person is still looking at.
- `running` means the process started, nothing more: no readiness or
  health probe (agent-orchestrator waits for its preview's URL, Paseo
  probes its services' ports every 3 s). The ports below say whether it
  listens.
- No port is assigned (Paseo gives each service `PASEO_PORT`): the same
  service run in two worktrees binds the same port, and the second fails
  until the first stops. Assigning one would make every command depend
  on a variable Vessel invents; left for the preview track.
- `node.stop { when: idle }` waits for sessions only: terminals and
  services are not work an agent is in the middle of, and end with the
  node as everything else does.

### Ports

- Found without a privileged call, for the process trees of the node's
  live sessions (their PTY program or agent) and terminals:
  - **Linux**: `/proc/<pid>/stat` for parents, `/proc/<pid>/fd` for the
    socket inodes of each process in a tree, `/proc/net/tcp` and
    `/proc/net/tcp6` for those in the listening state. Every process of
    the node's user can be read, which is every process Vessel started.
  - **macOS**: `ps -A -o pid=,ppid=` for parents and `lsof -nP -iTCP
    -sTCP:LISTEN -a -p <pids> -F pcn` for the listeners of the trees'
    processes; `lsof` sees the user's own processes without privilege.
  - **Windows**: `netstat -ano -p TCP` and `-p TCPv6` for listeners and
    their process ids (no privilege needed for the ids), and one
    PowerShell `Get-CimInstance Win32_Process` for parents and image
    names. Where PowerShell is not allowed to run, the scan is
    `unavailable`.
- Bounded: at most one scan every 3 s (GOAT's period), and only while
  someone asked `port.list` in the last 60 s; a scan stops at 4,096
  processes in the trees and 50,000 descriptors, the tools at 5 s and 4
  MiB of output; at most 256 ports are reported (`more`). A scan runs on
  the blocking pool, one at a time.
- Reported as facts only: nothing is opened, forwarded or proxied, and no
  listener is opened to the network. How a page reaches a port is the
  preview track's ADR, with one rule fixed now (review item 12): a
  previewed app is repository code, so it is served on an origin other
  than the page's (another port or host, as Paseo's `*.localhost`), never
  under the page's own origin, and it never holds a credential of the
  bridge.
- Only TCP listeners. A port is the owner whose tree holds the listening
  process at the scan; a process that moved out of the tree (a daemon that
  detached) is not reported, and is not Vessel's to stop either.

### The project's file

`vessel.toml` gains two tables beside `[worktree]`, which is unchanged:

```toml
[scripts.test]
command = "cargo test --workspace"

[services.web]
command = "npm run dev"
dir = "web"            # optional: where it runs, inside the root
```

- `command`: one `ProjectCommand`; `dir`: a `RelPath`. A name in both
  tables, an entry without a `command`, a name that is not a
  `ScriptName`, or more than 32 scripts and services together makes the
  file invalid (`invalid { message }`): nothing in it runs, setup
  included. An unknown key inside an entry is ignored, as ADR 0027
  ignores every key it does not read (a later node's key must not stop
  this one's setup); the digest covers what runs. `[env]` stays unread.
- **The digest grows only where the file does.** A file that names no
  script and no service keeps its version 1 digest exactly, so every
  approval given in protocol 10 still covers what it showed. A file that
  names one is digested as version 2: `vessel.toml commands 2\n`, setup
  and teardown as in version 1, then the count of scripts and services
  and, for each in name order, its name, its kind (`script` or
  `service`), its command and its folder (empty when none), every count
  and length 8 bytes little-endian. No version 1 approval can match it,
  so adding a first script asks again, and so does any later change to a
  script's name, kind, command or folder, and any script added or
  removed (entries are digested in name order, so their order in the
  file does not matter). Setup and teardown are
  approved with the scripts, as one set: one approval per set, as ADR
  0027 says.
- **An old client never approves what it could not show.** A protocol 10
  connection reading a listed set that has scripts gets `invalid {
  message: "this project's file names scripts or services, which this
  client cannot show; approve it from a newer client" }` instead
  (`Reply::for_protocol`, in `project.commands`, `project.approve` and
  `worktree.create`'s `needs_approval`), so it never sees the digest of
  a set it cannot display.
- **And the page must not either** (review item 2). The page speaks
  protocol 11, and its approval view is track B-page's, built beside this
  track. **A condition of the join**: B-page's approval shows `scripts`
  (name, kind, command, folder) when there are any, or refuses to approve
  a set that has them. Track C draws nothing, so it cannot close this
  itself; the report says so to the coordinator.

### Agents (ADR 0013), and what stops one leaving what it was given

- **Off unless a person turned it on** (review item 1). An agent's own
  shell tool may be sandboxed (Codex's sandbox, Claude Code's sandbox
  and command allowlists); a terminal the node starts is not, so it would
  give a sandboxed agent more than it has. An agent may open a terminal
  only beside a session whose profile says so (`agent_terminals = true`
  in its `[profiles.<name>]`, `profiles.toml`, ADR 0011): its own
  session, or one it may operate (ADR 0013's ownership, so a session it
  started on another node works through `fleet.forward`). The default
  profile of every harness has it off. `forbidden` otherwise.
- At most 4 of its terminals live at once (`forbidden` past that, as its
  session limit is). Every other terminal call works only on terminals it
  created; `port.list` shows only its own session's and terminals' ports.
  Its terminals end when its own session ends (its credential dies then,
  and nothing it started should outlive what it may still answer for).
  `script.*` are refused: a repository's commands run where a person
  approved them and a person asked; an agent runs commands with its own
  tools.
- Typing into a terminal is running a program, so the rule is that a
  terminal gives an agent nothing its own command tool does not:
  - a person turned it on for that profile, knowing what the agent's own
    sandbox was;
  - it starts in the session's folder, as the same user;
  - the shell holds no credential: not the node's token, not the agent's
    own credential, not a model token, so it cannot act through the node
    as the agent or the node;
  - the agent reaches only terminals it made, never a person's (which may
    hold an `ssh` login, a `sudo` timestamp, another folder) or another
    agent's;
  - every `write_terminal` is a tool call of the agent's harness, held to
    the session's permission mode as its own shell tool is (Vessel does
    not pre-approve its own MCP tools), and is marked destructive and
    open-world in its MCP annotations, so a harness that asks before such
    tools asks;
  - its terminals are counted, named as its own (`created_by`), and end
    with its session.
  What it cannot stop, stated as ADR 0013 states it: a process of the
  node's user can read the node's files, its token included. A terminal
  is attribution and least privilege, not containment.

### What an older client and an older node see

- A client asks no protocol 11 method of a node that speaks 10 or less
  (`NotInProtocol` before sending); the CLI and the agent tools say the
  node is too old.
- A protocol 10 client of a protocol 11 node sees no terminal (they are
  not sessions), no new event (a `service` record exists only in a
  service's log, which only `terminal.*` serve), and a project file with
  scripts as `invalid` (above).
- `PROTOCOL_VERSION` goes to 11; nothing else in that line.

### What is kept, and for how long

Nothing on disk. A terminal, its scrollback and its runs live in the
node's memory: they outlive the page and every connection, and end with
the node (its stop marks them `interrupted`; a new node has none). Kept
on disk, a shell's output would be a second copy of whatever a person
printed in it, read back by anyone who reads the data folder later; and a
terminal reopened after a restart would be a new shell anyway. So the
store's schema is not raised.

### Bounded, and by how much

| What | Bound | Beyond it |
|---|---|---|
| Live terminals (shells and runs) | 64 a node, 8 a session, 16 a project's folder, 4 an agent's | `unavailable` (`forbidden` for an agent) |
| Ended terminals kept | 32 a node | the oldest ended one is forgotten |
| Scrollback | 1 MiB of output and 4,096 records a terminal | the oldest records are dropped; `oldest_seq` moves; a reader from before gets from `oldest_seq` |
| Live followers | 64 records behind (the broadcast's buffer: at most 1 MiB of 16 KiB chunks held for a stalled follower) | `lagged { resume_from }`, then pages |
| Input | 64 KiB a `terminal.send` | `invalid_request` |
| A page | 512 records, 1 MiB of output | `next` |
| Long polls | 30 s, in the node's wait shares | answered as it is |
| A service | restarts after 1 to 16 s; 5 without a run of 60 s | `failed { gave_up }` |
| Stopping | 2 s (shell), 5 s (script) from the polite signal to `SIGKILL` | killed as a group |
| `vessel.toml` | 32 scripts and services; names of 64 bytes | `invalid` |
| Port scans | one per 3 s, only within 60 s of a `port.list`; 4,096 processes, 50,000 descriptors, 5 s and 4 MiB per tool; 256 ports | `more`; `unavailable` |

### The CLI, the agent tools, the page

- `vessel shell [--session S | --project P [--dir D]] [--title T]` opens
  a shell and attaches to it (the attach of a terminal session: raw mode,
  resize, the detach key); `vessel shell list | attach | kill | close |
  rename`. `vessel script list | run | restart | stop | output` with
  `--session` or `--project`; `vessel ports`.
- Agent tools (`vessel mcp`): `create_terminal`, `list_terminals`,
  `read_terminal` (the text of whole records from a cursor, a record's
  sequence number, escape sequences removed (one split across two records
  may leave a stray character), at most 32 KiB a call, with an optional
  wait, and the cursor for the next),
  `write_terminal` (text, at most 16 KiB, and optional named keys:
  `enter`, `tab`, `escape`, `backspace`, `up`, `down`, `left`, `right`,
  `ctrl_c`, `ctrl_d`, `ctrl_z`), `close_terminal`, and `list_ports`. Each
  takes `node`, as every session tool does. No tool renders a screen
  (Vessel has no terminal emulator on the node): `read_terminal` gives the
  output, not the picture a full-screen program draws.
- The bridge (`vessel ui`) follows a terminal as it follows a session:
  `follow_terminal { stream, node, terminal, from }`, answered with the
  same `event` and `stream` frames, reconnecting as a session's stream
  does. The page's data layer gets the generated types and nothing drawn
  (track C-page draws).

### Where terminals meet the other tracks

- **Deleting a session** (`session.delete`) ends its terminals and runs
  before its worktree is tidied, so they never hold it; **retention**
  dropping a session does the same, from the node's tidying sweep.
- **Removing a worktree** (`worktree.remove`, and tidying) counts a live
  terminal whose folder is inside it as `in_use`, as a live session is: a
  person's shell or a service there keeps it (and teardown, `docker
  compose down`, never runs under a live service).
- **Removing a project** (`project.remove`) answers `conflict` while a
  live terminal is placed in it or runs inside its folder, as it does for
  a live session; ended ones are forgotten with it.
- **A session or project going away while a terminal starts for it**:
  the terminal is registered, then its place is looked up again; gone, it
  is killed and the call answers `not_found`. Deletion ends what is
  registered, so one of the two always ends it.

### Review (2026-11-12)

An independent reviewer (Opus, read only) read the first version of this
contract against the references, the code it claims to reuse and the
other tracks' contracts, and found fifteen problems. Each one's fate:

1. Agent terminals would give a sandboxed agent an unsandboxed shell.
   **Taken**: off unless a person turns it on in the session's profile
   (`agent_terminals`); `write_terminal` annotated destructive and
   open-world.
2. The protocol 11 page could approve scripts it never shows. **Taken as
   a join condition** for B-page's approval view (this track draws
   nothing).
3. Removing a worktree or a project ignored live terminals; retention did
   not end them. **Taken** ("Where terminals meet the other tracks").
4. The reuse held on the wire only, and streams are not forwardable.
   **Taken**: `terminal.subscribe` is neither forwardable nor a page call;
   the client's and the bridge's following are listed as work. **Not
   taken**: dropping the subscription for `terminal.read { wait }` polls,
   which would hold one of the bridge's eight waiting reads (the node's
   wait shares per origin) for every terminal a page shows, where a
   subscription holds none.
5. The memory bound was by record count only. **Taken**: the ring is
   bounded by bytes and records, the live buffer is 64 records.
6. Restarting "in the same terminal" had no mechanism once its log was
   closed. **Taken**: in place while it runs, a new terminal once ended.
7. A shell's jobs survive a signal to its group. **Taken** for Linux (the
   shell's session is swept); stated for macOS, Windows and a node that
   crashes.
8. `node.stop { when: idle }` did not say. **Taken**: terminals do not
   hold it.
9. The agent tools' `node` could never work, and agent terminals outlived
   their agent. **Taken**: beside sessions the agent may operate; its
   terminals end with its session.
10. A routed session's model token was not scrubbed. **Taken**, for a
    node running inside a session's tree.
11. Behaviour of the references dropped without saying. **Taken**: piped
    output, no port assignment, no readiness or health probe, the last
    resize wins, each stated.
12. A preview's origin. **Taken** as a rule for the preview track.
13. Races and an unbounded wait. **Taken**: looked up again after
    registering; `terminal.close` waits at most the grace and 5 s; the
    subscription's end is named.
14. The `service` event was overreach. **Taken**: no event; the typed
    state is on `TerminalInfo`, read by revision.
15. Smaller points. **Taken**: `IpAddr` for addresses; unknown keys inside
    an entry ignored; `terminal.read`'s wait stated; `read_terminal`'s
    cursor defined; "order" removed from the digest's description.

## Track A-page: contract

Written 2026-11-12 on `backend/m11-a-page`, before any screen. A page
track adds no call: everything below is protocol 10 as track A built it,
one preference of the bridge's document (ADR 0028, extended by its own
rules), and what the fake agent must do for the tests. What follows is
what the page promises: which call each screen makes, what it shows for
each answer, and what it bounds.

### What is offered, and from what

Nothing is offered that the node would refuse, nothing hidden that it
would do. Every rule is a pure function in `data/` with a unit test.

| Thing | Shown when | Read from |
| --- | --- | --- |
| Anything of protocol 10 | the machine's connection speaks protocol 10 or later | the bridge's status for the machine (`protocol`) |
| Queue (send while a turn runs) | a live structured session | always: the node queues for every agent |
| Steer | the agent says `abilities.steer` | `ready.abilities` in the log, else `SessionInfo.abilities` |
| Interrupt and send | any live structured session in a turn | the node does it for every agent |
| Attach (pick, paste, drop) | a live structured session, or the new-session page for a structured agent | files: always; images: `abilities.image`; audio: `abilities.audio` (the probe's `HarnessOptions.abilities` on the new-session page) |
| An attachment's size | before `attachment.begin` | `session.conversation`'s `limits` (`image`, `audio`, `file`, `per_prompt`, `inline_per_prompt`, `per_session`, `chunk`) |
| `/` | a structured session | the last `commands_changed`; "No commands from <agent> yet" before one |
| `@` | a session with a project (`SessionInfo.workspace`) | `file.find` on its worktree, else its project; "This session has no project" otherwise |
| A question | while the agent asks one | `question_asked` without its `question_resolved` |
| The context meter | after the first `usage_changed` | `usage.used`, `usage.size`, `usage.cost` |
| A turn's tokens | a `turn_ended` with `usage` | that record |
| Pin, unpin | protocol 10, any session | `marks.pinned` |
| Archive | protocol 10, a session not archived (a live one is stopped by it, and the dialog says so) | `marks.archived_at` |
| Bring back (unarchive) | an archived session | `marks.archived_at` |
| Start again | an ended structured session whose recorded `abilities.resume` is not false | `SessionInfo.abilities`, `status` |
| Delete | an ended session | `status` |
| Unread | `marks.unread` | the session list |

A session stored before protocol 10 has no recorded abilities: start
again is offered and the node's answer decides (track A's review item
14). Kill stays for a live session (it ends it without archiving).

### The one preference

`composer.delivery`: what Send does while a turn runs, `queue` (the
default), `steer` or `interrupt`, changed with `{ "set": "delivery",
"value": … }`. Settings › General shows it as Paseo's "Default send"
beside the send key. The other way is one modifier away, as in all three
references: ⌘/Ctrl with the click, or ⌘/Ctrl+Enter (⌘/Ctrl+Shift+Enter
where ⌘/Ctrl+Enter is the send key), and the button's icon turns while
the modifier is held. The other way of `queue` is `steer` where the
agent can, `interrupt` where it cannot; of `steer` and `interrupt` it is
`queue`. Where the chosen delivery is `steer` and the agent cannot
steer, Send queues and its tooltip says so; the choice is not changed.
A file written before this field reads `queue`; an older bridge keeps
the field it does not know (ADR 0028's "lenient by keeping").

Touches, outside `web/`: `crates/vessel-web/src/wire/prefs.rs` (the
type, the field, the change, the default test),
`crates/vessel-web/src/keep/prefs_doc*` (the section's field and the
change's one arm: a preference the document does not apply is not a
preference) and the generated `web/src/wire/bridge.ts`.

### Calls, and what each answer shows

All through `data/` functions that answer an `Outcome` (no throw).

- **`session.prompt`** `{ session, content, delivery }`: Send. The
  answer's `delivered` decides nothing on the page (the log and
  `queue_changed` show what happened); the draft and the tray are
  cleared on any success. `invalid_request` (over a limit, a kind the
  agent does not take): said under the box, the draft and tray kept.
  `conflict` (the queue full, the session ended): the same.
  `unsupported` for `steer` (an agent that answered "method not found"
  anyway): said, and the page stops offering steer for this session.
  `unavailable` for `interrupt` (the turn did not end in 10 s): said; the
  text stays.
- **`session.queue`** `{ session, change }`: the queue's controls. Every
  change that names an entry sends the revision the page drew it with
  (`edit`). The page draws the queue only from `queue_changed` (and the
  reply's `queue`, which is the same state a moment earlier): nothing is
  moved or removed on the page before the node says. `conflict` on an
  edit (someone else changed the entry): "Changed elsewhere; showing the
  new text", the edit's text kept in the field for the person to apply
  again; `not_found` (it was sent or removed meanwhile): "Already sent or
  removed", the editor closed.
- **`session.conversation`** `{ session }`: read once when a structured
  session's page opens and again after `session.resume`, for `limits`
  (the log has everything else). A failure leaves attachments unoffered
  with the reason in the attach button's tooltip.
- **`attachment.begin` / `.write` / `.finish`**: a file is checked when
  it is attached (kind, size, count, against the agent's abilities and
  the limits) and uploaded when Send is pressed, so a file removed before
  sending never takes the session's room (the node has no call to give an
  upload or an attachment back). Pieces of `Upload.chunk` bytes, read with
  `Blob.slice` and `arrayBuffer()` one at a time (never the whole file),
  each next offset the node's `received`; at most two files at once from
  one composer. A failed piece stops that file: its chip says why. Retry
  writes the same upload again from the node's `received`; only
  `not_found` (the node dropped it) begins a new one; `invalid_request`,
  `unsupported` and `conflict` offer Remove only (the reason stands);
  `unavailable` (the node's upload places are taken) offers Retry. The
  prompt goes once every file is in; a file uploaded before a failure is
  kept for the next Send. An image's first bytes are checked on the page
  before it is sent (the node refuses and drops one that does not match).
- **`attachment.read`** `{ session, attachment, offset }`: an image in a
  prompt of the timeline, fetched only when its row is drawn and within
  one screen of the view (an `IntersectionObserver`; the timeline is
  windowed), pieces joined into one `Blob`, shown from an object URL. A
  file attached in this tab is shown from the local file (its object URL
  kept under the id `finish` answered), never fetched again. The page
  keeps at most 48 images or 64 MiB of them, oldest dropped and revoked.
  Audio and files are chips with name and size, not fetched. One
  observer for the timeline, at most two reads at once, and a tile of
  fixed size until its image is drawn, so the windowed rows keep their
  measured height.
- **`file.find`**: through `data/workspace/find.ts` only
  (`newSearchKey` once per composer, `findFiles`, an answer marked
  `superseded` dropped); asked once the word after `@` has a character,
  80 ms after typing stops (before that the menu says "Type to search
  files"). The root is the session's worktree, else its project
  (`SessionInfo.workspace`); a session started in a bare folder has none
  until track B-page's place picker starts sessions in projects. A chosen
  file is written into the prompt as text, `@path` relative to that root
  (in quotes when it holds a space).
- **`session.send` `answer_question`** `{ id, answer }`: the question
  dock's Submit (`accept` with every field that has a value) and Decline.
  `invalid_request` is shown under the field it names when the message
  names one, under the form otherwise; `conflict` (answered elsewhere
  first) and `not_found` close the dock with a toast.
- **`session.send` `cancel_turn` `{ turn }`**: Stop names the turn it
  saw start (`turn_started`'s seq, kept on the timeline), so a stop that
  lands after the queue started the next turn holds the queue and cancels
  nothing; the queue's strip then says it is paused and offers Resume.
  To a node before protocol 10 `turn` is left out.
- **`session.mark`** `{ session, mark }`: pin, unpin, archive (a live
  session: a dialog first, "Archiving stops the agent"), unarchive, and
  `read { through }`. `read` is sent when the session's page is in front
  (the tab visible and focused) and it has shown a record that asks for
  attention at or after the last `through` sent, or the session list
  says it is unread; `through` is the last record shown plus one. Not
  per record: at most once a second. After "Mark unread" no `read` is
  sent for that session until a record arrives after it. Every mark's
  answer replaces the session in the page's list at once (the list is
  polled every two seconds). The page cannot know a device was given
  only `read` (no scope reaches it): the first `forbidden` from a mark
  stops `read` marks to that machine for the page's life.
- **`session.resume`**: "Start again". The button waits while the agent
  starts (up to the handshake's bound); `unsupported` "This agent cannot
  pick up where it left off", `conflict` (still being written, already
  live, another start under way, or the folder is gone: the node's
  message), `spawn_failed` (the log says why), `invalid_request` (the
  profile was removed), `harness_not_installed`, `unavailable` (the
  machine is stopping) each said plainly. Not offered for a session that
  never had an agent session to load (`harness_session` absent). The
  page follows the log again from where it stopped once the session is
  live again (the answer, or the list, says so), so another device's
  start again shows here too.
- **`session.delete`**: a dialog first; it may wait while the worktree's
  teardown runs (up to 5 minutes), the dialog's button saying so;
  `conflict` (its log still being written, or live) said. Its answer's
  worktree: removed, or kept and why (`KeptReason`, each in words), in a
  toast (removing a kept one is track B-page's worktree screen). The
  session's draft, tray and kept images are dropped; the page goes back
  to the board if it was showing the session.

### Questions, drawn

agent-orchestrator's question dock, over the composer in the permission
dock's place (the permission first when both wait). One question at a
time, oldest first; within it one step per field, "Question · 1 of 3".
Fields in the node's order (by key), except Claude's: `question_N` are
ordered by N, and `question_N_custom` is drawn with `question_N` as its
"Other" row, in the agent's own words (its title and description). For a
multi-select the custom text is added to the picks; for a single choice
it is a note to the pick, or the answer when nothing is picked
(`claude-agent-acp`'s reading), so typing in it never clears a pick.
Each kind by a kit control: text a field, number and integer a field
taking a sign and (for number) a point, boolean a checkbox row, a single
choice option rows with their descriptions, several the same rows
checked; defaults filled in; `unknown` a line saying it cannot be
answered here, with Skip only. The page checks what the node checks
(required, lengths, bounds, item counts) before sending, so a refusal is
rare; one that comes is shown at the field it names.

### The fake agent (`crates/vessel-testkit`, additive)

- `ask claude`: Claude's `AskUserQuestion` form (track A's fixture: a
  single choice with descriptions, a multi-select, each with its
  `_custom` text), answered like `ask`.
- Whatever else the tests need is added the same way, each prompt
  documented in `fake_acp.rs`'s list.

### The contract's review (2026-11-12)

An independent reviewer read this section against the node, the page and
the references and found fifteen problems. Amended above: uploads at
Send and retry from the node's `received` (1, 2), the read-only device
(3), following again after a start again and its other refusals (4), the
queue drawn only from the log, the revision on `edit` alone, an edit
keeping its files, `send_now`'s delivery (6), `@`'s empty word, its root
and what it inserts (7), Mark unread, mark answers applied at once (8),
Stop's `turn` and an older node (9), Claude's "Other" (10), names and
types cleaned as the node reads them and images checked (11), the image
fetches bounded (12), delete's wait and what it drops (13), question
order, signs and defaults, checked before sending (14).

Rejected, and why:

- **Between turns, Send always `queue`, labelled "Add to held queue"
  while held** (5). A person who stopped a turn and types a new message
  means it now: `start` sends it, and what was held stays held until
  they resume or send it (agent-orchestrator: "an empty queue is never
  held, so messages typed after a stop still start"; GOAT's Resume is a
  button of the queue, not of the next message). `start` races nothing
  while the queue is held (it moves only on resume).
- **Two tabs' search keys** (7): every tab's first key is 1, so two
  tabs searching at once on one machine replace each other's searches.
  `find.ts` is the other page track's file; the composer asks again when
  its own answer comes back `superseded` with no newer search of its
  own, and the coordinator is told.
- **Pinned and unread ended sessions hidden** (8): the sidebar's "show
  ended" is track W's preference. With protocol 10 an ended session that
  is not archived is still on the board; the sidebar keeps the
  preference, and pinned sessions are listed first, ended or not.

Built differently from a reference, and why (15): a large paste stays
text (GOAT makes it a file; the queue's 128 KiB and the draft's bound
already say so); no "Compact context" button by the meter (the agent's
own `/compact` is in the `/` menu where it offers one); no Undo after
archive (the archive's Restore is one click).

### Bounds, on the page

| What | Bound |
| --- | --- |
| A file in memory while uploading | one piece (`Upload.chunk`, 1 MiB) per uploading file |
| Files uploading at once, per composer | 2 |
| Attachments in the tray | `limits.per_prompt` (16) |
| Images kept for the timeline | 48 or 64 MiB, least recently drawn dropped; two read at once |
| `@` searches | one in flight per composer (the node replaces the older), 80 ms after typing stops |
| `/` and `@` rows shown | 50 |
| `read` marks | at most one a second per session |

### An older node, an older page

A machine speaking protocol 9 or older gets the page of wave 1: Send and
Stop, no attach, no `/` or `@` menu, no meter, Kill and Copy id. A
newer page against a protocol 10 node shows all of it. The preference
is the bridge's, not the node's.

## Track B2: progress

2026-11-12, on `backend/m11-forge`. ADRs 0029 (the forge and signing in)
and 0030 (git apart from a repository's own configuration); ADRs 0001,
0019, 0023 and 0027 amended; store schema 10; protocol 11; crate
`vessel-forge`.

Built, each slice with its tests, each commit through the whole gate:

- ADR 0030: every git run judges the repository's own configuration
  (`.git/config`, `config.worktree` and what they include, read once per
  run, at most 256 KiB): drivers turned off with `-c`, programs,
  credential-moving settings and `core.worktree` refused with the key
  named; git's own folders refused as projects; a worktree held from
  removal while a session starts in it; clones in a pool of their own.
  Planted tests per vector, against the real git.
- The wire (protocol 11): `vessel_proto::forge`, the six calls in every
  table a method passes, `SessionInfo.pull_request`; the page's types
  regenerated, nothing drawn.
- `vessel-forge`: the trait, GitHub (REST conditional reads, one GraphQL
  read, writes; a merge pinned to its head), signing in, the bounded
  HTTP client with quotas, cooldowns and a token bucket, remotes read
  into repositories, and the in-memory forge (feature `fake`). 23 tests,
  GitHub's against a stand-in on loopback.
- `vessel-session`: the pull request watch, the tracker's facts and
  policy, the info's summary, both dropped at the end.
- `vessel-store`: schema 10 (the feedback switch and the signatures
  sent, deleted with the session); the rollback tests undo it too.
- `vessel-workspace`: the checkout in one run, a branch's remote and its
  address (kept out of `Debug`: it may hold credentials), its
  remote-tracking commit.
- The node (`forge`): the watcher, hosts and sign-in, folder reads,
  looks, the views, feedback through the queue, `pr.open` and
  `pr.merge`, the hooks on create, start again, archive and delete; the
  binaries hand it the environment's tokens and the machine's `gh`.
  End-to-end tests against the in-memory forge: a watched session's card
  and board, an exact merge, a person's open pushing to a bare repository
  on disk (the remote's fetch address names the forge, its push address
  the disk), an agent's open refused `not_pushed`, an agent refused
  merge, feedback and status, a forge that is down holding nothing, an
  end clearing the pull request, feedback sent once.
- `vessel pr` (list, show, open, merge, feedback, status) and the
  `vessel` tools `get_pull_request` and `open_pull_request`.

Not offered to agents, and why: merging (it publishes to a shared
branch: a person's decision), the feedback switch (what is sent into a
session is a person's to decide), the node's sign-in and the whole list
(a person's view), and a push by the node for an agent (the node would
run the repository's hooks and the user's credentials for it).

Deviations from the contract, each deliberate:

- An end is not told to the watcher: the session's actor clears its pull
  request itself at the final status, and the watcher drops the session
  at its next turn (within a minute).
- A person's request is never conditional (the `ETag`s are shared with
  the watcher's looks, and a 304 would answer a person with nothing): a
  person's refresh, open or merge costs its requests; an agent's refresh
  stays background traffic.
- A conflict is re-armed when the forge knows the pull request does not
  conflict (mergeable, blocked, unstable or behind), not while it is
  still computing it (unknown), which would send the same conflict twice.

Found while building, and fixed:

- A feedback switch turned on while the session's first folder read was
  loading the store's copy was undone by that load (seen once under
  load): the switch is unknown until either answers, and a person's
  stands.
- A waiting feedback prompt took an "and N more" line as an addition
  when no note fitted; it now takes nothing then, and its old count is
  replaced, not repeated.
- Checked against the contract after the node slice: the bound of 1,000
  signatures now drops those a read no longer has before the store drops
  the oldest; `unreachable.retry_at` is the next look's time, not a
  guess; a watch with nothing to look with is put off instead of being
  due at once.

For the page track (B2-page):

- `SessionInfo.pull_request` is set only for a live session placed in a
  project or a worktree, a moment after it starts (the watcher's first
  folder read), and cleared when it ends; draw nothing from its absence.
  `access` says why there is no summary; `summary` stays while a later
  look fails (`unreachable`, `rate_limited`), so draw the reason beside
  the last facts, not instead of them.
- `pr.get { refresh: true }` reads now, at most once per 10 s per
  session (more often answers from memory); a person's refresh costs
  requests, so call it on a person's action, not on a timer.
- `pr.merge` needs the full `head_commit` the page showed and the method
  from `merge_methods`; `head_moved { head }` carries the head now, to
  show and ask again. Nothing merges on its own.
- `pr.feedback` answers `unsupported` for a terminal session; the switch
  also changes the board (failing checks with feedback on are "fixing
  CI" in Validating while the agent works).
- Feedback prompts begin `Vessel: pull request feedback` and are queued
  as the owner's: the queue shows them like any other entry.

Not verified here:

- A real GitHub: every forge request in the tests goes to the in-memory
  forge or to a stand-in on loopback (the brief's rule). The REST and
  GraphQL shapes are GitHub's documented ones, as the references read
  them; GitHub Enterprise Server's API addresses are GOAT's.
- `gh` signed in for a real account: the tests run a stand-in `gh`
  script.
- The page e2e suite is flaky under this machine's load: each run of the
  slices' gates had 35 of 36 passing, the one failing different each time
  and always in `web/e2e/page.spec.ts` (a theme kept by the bridge, the
  Mod+Enter and shortcut-rebinding tests), code this track did not
  touch. The theme test failed 2 of 6 times alone: after clicking System
  it fetches the index before the bridge has written the change.

## Track C: progress

2026-11-12, on `backend/m11-shells`. ADR 0031 (and ADR 0001's two new
edges, `vessel-session -> proc` and `vessel-node -> proc`); protocol 11;
store schema 11 not used (terminals are kept in memory only).

Built, each with its tests:

- The wire: `vessel_proto::{terminals, scripts, ports}`, the thirteen
  methods with `Call::since`, the forwardable list (all but
  `terminal.subscribe`), the stand-ins (`project.commands` and
  `project.approve` answer `invalid` to a protocol 10 client for a set
  with scripts, `worktree.create`'s `needs_approval` the same), the
  aliases classified (`TerminalId` a handle, `ScriptName` named,
  `ProcessName` content), the TypeScript regenerated.
- `vessel-workspace`: `[scripts]` and `[services]` read and checked, the
  version 2 digest only for a file that names one (a test pins version
  1's digest unchanged), `shell_command` shared by setup and scripts.
- `vessel-proc`: `listening::scan` (Linux `/proc`; macOS `ps` and
  `lsof`; Windows a CIM query and `netstat`; bounded in processes, file
  descriptors, time and output), `kill_session` (Linux), `account_shell`.
- `vessel-session::terminals`: the shell actor in a PTY and the script
  actor piped in its own process group, the ring bounded by bytes as well
  as records (`Ring::bounded`), a service's backoff and its approval asked
  again before every restart, a restart in place, `SessionHandle::pid`
  for the ports; 9 tests against real processes (a script's grandchild
  ends with it; a shell's job in a group of its own ends with it on
  Linux; the scrollback drops its oldest output; the node's stop
  interrupts both kinds).
- `vessel-rpc` and `vessel-client`: `Connection::subscribe_terminal` (the
  driver and the server's writer announce a terminal's subscription as a
  session's), `Subscription<I>` over what it follows, `LogStream<L: Log>`
  with `SessionStream` and `TerminalStream`, `ResumingStream<C, L>` with
  `follow_terminal_resuming`.
- The node: `terminals` (registry with reservations against the limits,
  the environment without Vessel's credentials, `TerminalsConfig::detect`
  for the binaries, the scripts' plans, the port scanner), the guard's
  rows, `script.*` refused to agents in the workspace policy, the agents'
  rule in `service::terminals`, `Occupant` (a live terminal keeps a
  worktree, `kept { in_use }`, and a project, `conflict`), tidying ending
  a deleted or dropped session's terminals first; 6 tests end to end over
  the wire (a shell typed into, read, renamed, killed and closed; a node
  without a shell; scripts approved, run, refused while running, a
  service restarted in place and holding its project; an agent refused
  without the profile, limited to 4 and to its own, its terminals ending
  with its session; a port found and owned by its terminal; a deleted
  session's terminal forgotten).
- `vessel-web`: the call list (and the pinned test), `follow_terminal`
  frames sharing the session streams' code; no `web/src/data` change was
  needed (the page typechecks against the regenerated types).
- `vessel mcp`: `create_terminal`, `list_terminals`, `read_terminal`,
  `write_terminal` (destructive, open world), `close_terminal`,
  `list_ports`; the agent's own session from its environment; a test
  through a node (refused without the profile, then opened, typed into,
  read, closed).
- The CLI: `vessel shell [--session | --project [--dir]] [--title]
  [--detached]`, `vessel shell list | attach | rename | kill | close`,
  `vessel script list | run | restart | stop | output`, `vessel ports
  [--watch]`; the terminal attach serves a session and a terminal alike;
  `vessel project commands` shows the scripts; 2 tests against the
  binary.

Changed from the contract while building, and why:

- `ShellStart.size` is the wire's `TerminalSize`, so the node needs no
  `vessel-pty` edge.
- `TerminalsConfig::detect` takes the binary's environment as closures
  (the library reads none), and removes the known credential names as
  well as those found.
- Removal checks take what runs where as `Occupant` (a session or a
  terminal) instead of session ids alone; a worktree's `sessions` still
  lists sessions only.

### Review of the code (2026-11-12)

An independent reviewer (Opus, read only) read the commits against ADR
0031 and the contract, and found seven defects; areas 3 (the
environment), 5 (the wire) and 6 (path checks) clean. All seven taken:

1. The switch was read on the target session's profile only: an agent
   without it could start a session with a profile that has it and open
   a shell beside that. Now its own session's profile must allow it too
   (its own node checks before forwarding).
2. An agent's terminals could outlive it: another node's agent's always,
   a local one's when the call was cancelled before the watcher started.
   Now one watcher a session, started right after registering and before
   anything else is awaited, ends an agent's terminals when its own
   session or the session they are beside ends; a create beside an ended
   session is refused.
3. A script could start in a worktree being removed. Now `script.*`
   refuse a busy worktree, a start in progress counts as occupying its
   folder, and a terminal that started as a removal began is ended.
4. Two `script.run` at once could both start. The check is now in the
   reservation, under the registry's one lock.
5. A terminal beside a session in a project did not count against the
   project's 16 once started. The counted owner is now kept with it.
6. Approval was not asked again at the spawn itself. The run's actor now
   asks before every start, the first and a person's restart included.
7. Linux's socket tables were read unbounded. Now at most 4 MiB each,
   cut at a whole line, the scan marked truncated.

Measured (`docs/PERFORMANCE.md`, "Track C"): the benchmarks of the
event frames, the session's fan-out and the board's reducer, before and
after alternated twice, are within the spread of one tree's runs; the
load test's paced and flood workloads are the same at the same load,
the burst moves between runs of one build as before; `vesseld` holds
0.5–1.4 MiB more. Nothing on the record path got slower that the
machine's own variation could not explain.

What remains: a removal whose look at what runs in a worktree precedes a
terminal's reservation by a hair, and whose claim follows the terminal's
last check, can still run its teardown under that terminal; closing it
needs track B's removal to read the occupants under its own claim.

What agents cannot do through Vessel: run, restart or list a project's
scripts and services; open a terminal unless both its own session's
profile and that of the session it is beside set `agent_terminals = true`
(every default profile does not), or beside a session it may not operate;
hold more than 4 live terminals; see, read, type into, kill or close a
terminal it did not open; see ports of terminals it did not open or of
sessions other than its own; keep a terminal past the end of its own
session or of the session it is beside.

Only CI can confirm: Windows (`pwsh.exe`/`powershell.exe`/`%ComSpec%`
detection, the ConPTY shell, scripts through `cmd.exe /d /c` in a job
object, the CIM and `netstat` port scan, the `.cmd` scratch file's
removal) and macOS (`ps`/`lsof` port scan, `-l` login shells, a job
ignoring `SIGHUP` outliving its shell). The terminal tests are Unix-only
(`#[cfg(unix)]`); the parsers of every platform's tool output are
unit-tested on every platform.

Not verified: a real interactive attach from a terminal emulator (the
tests attach with piped input); a service crash-looping to `gave_up`
end to end (the backoff policy is unit-tested, a restart in place and a
refusal to restart unapproved are tested with real processes); the page
drawing any of it (track C-page).

For the page track:

- A terminal is followed with `follow_terminal { stream, node, terminal,
  from }` and drawn as a terminal session is; `TerminalInfo.revision`
  moves on every change (`terminal.get { changed_from }` waits for it),
  `service` carries a service's typed state and `oldest_seq` where the
  kept scrollback starts.
- `script.run` answers `needs_approval { commands }`: show the scripts
  (name, kind, command, folder) with setup and teardown before
  `project.approve`. **A condition of the join**: B-page's approval view
  shows `ProjectCommands::Listed.scripts` when there are any, or refuses
  to approve such a set.
- `port.list { changed_from }` is a long poll; the first answer may be
  `pending` until the first scan. A port is reported, never opened:
  previews are the preview track's, on an origin other than the page's.

## Track A-page: progress

Written 2026-11-12 on `backend/m11-a-page`, at the end of the track.

### Built

Every screen of the contract above, on protocol 10's calls as track A
built them; nothing was added to the wire.

- **The node's queue over the composer** (`queue-dock.tsx`,
  `data/queue-view.ts`): edit in the box (with the row's revision, so an
  edit that crossed another is refused, not overwritten), move by drag
  or by Up and Down on the grip, delete, Steer or Send now, a held
  queue's reason and Resume. The send button's three ways while a turn
  runs, steer only where `abilities.steer` says so (and not again after
  the agent refused it once), the other way one modifier away
  (`data/delivery.ts`, `send-button.tsx`). The default is the bridge's
  `composer.delivery` (ADR 0028), Settings › General's "Default send".
- **Files** (`data/attach.ts`, `upload.ts`, `tray.ts`, `trays.ts`,
  `images.ts`, `thumbnail.ts`): paste, drop and pick; a chip each with
  its progress, failure, Retry (going on from what the node has) and
  Remove; checked as the node checks them, uploaded when the message is
  sent, two at a time; the new-session page's files go with the first
  message once the agent has said what it takes. A sent prompt shows its
  files; an image is read with `attachment.read` only near the view.
- **`/` and `@`** in one kit list (`ui/kit/suggestions.tsx`,
  `data/suggest.ts`, `use-suggest.tsx`, `use-file-search.ts`): the
  agent's commands from `commands_changed`; files through
  `data/workspace/find.ts` only (a search key per field from
  `newSearchKey`, `superseded` dropped, asked again once when this
  field's own answer was superseded with no newer search of its own);
  nothing else under `data/workspace/`. A session with no project says
  so.
- **Questions** (`question-dock.tsx`, `question-field.tsx`,
  `data/question-form.ts`, `question-answers.ts`): one step per field,
  kit controls by kind, Claude's `question_N_custom` as its question's
  "Other" row, the node's checks first and its `invalid_request` at the
  field; the answered card in the timeline.
- **The context meter** beside the model and a turn's tokens in its
  ended line (`context-ring.tsx`, `data/context-meter.ts`).
- **A session's marks and actions** (`data/marks.ts`, `read-hold.ts`,
  `use-mark-actions.tsx`, `use-read-marks.ts`, `ended-banner.tsx`, the
  board's `card-actions.tsx` and `card-marks.tsx`): pin, unread, start
  again, archive, unarchive or restore, delete and what became of the
  worktree; `read` sent as the contract says (`through` the last record
  shown plus one, when in front and seen, at most once a second); the
  board keeps an ended session in its lane until it is archived; the
  sidebar shows pinned first and unread.
- **The sidebar's sessions set in** from their machine's row (22px, the
  mark under the machine's name).
- The fake agent's `ask claude` (Claude's `AskUserQuestion` form) in
  `crates/vessel-testkit`.

Found while looking at the captures, and fixed:

- The page's content policy (`img-src 'self' data:`, ADR 0017) refuses
  `blob:` images, so every image preview was empty. Images are drawn
  from `data:` thumbnails made on a canvas; the full image is read into
  a `data:` address only when opened.
- A dock opening over the composer (a question, the queue), or a phone's
  keyboard, shortened the timeline's view, and the window read that as a
  scroll up past `NEAR_END`: the last row stayed under the dock.
  `use-window.ts` keeps a reader at the end there on a resize.
- Claude's free text was a line of its own in the answered card; it
  joins its question's line.
- The composer took the whole timeline and was drawn again, with its
  docks, controls and tooltips, for every frame of a long log being
  read. It is drawn again only when something it shows changed
  (`composer-props.ts`); measured below.

### Checked

- Unit tests for every module under `data/` named above; end-to-end
  against real nodes and the fake agent: `e2e/conversation.spec.ts`
  (the queue, steer by key, Default send, files and their previews, a
  new session's files, `/` and `@`, a question and Skip, a question's
  dock at 390px, the meter and tokens) and `e2e/marks.spec.ts` (pin,
  unread, start again, archive, restore, delete).
- Captures: `e2e/conversation-screens.ts` (`npm run screens`), light,
  dark and 390px, looked at beside agent-orchestrator's and GOAT's.

### Measured (`npm run perf`, release binaries)

This branch against `backend/m11` (its own release build, run with
`VESSEL_TARGET_DIR`), one after the other on the same machine; medians
(minimum–maximum) of the suite's runs. Two pairs of the whole suite
(the first with this branch first, the second with the base first),
then two more pairs of the timeline after the composer stopped drawing
for every frame.

| Figure | Base | This branch, before | This branch, after |
| --- | --- | --- | --- |
| Board load, script bytes | 183,990 | 189,418, then 189,374 | not measured again |
| Board load, stylesheet bytes | 24,255 | 26,178, then 26,197 | not measured again |
| Session screen's script (gzip) | 133.6 KB | 144.2 KB | 144.4 KB |
| 5,000 entries, first entry drawn | 144, 160, 128, 145 ms | 178, 175 ms | 176, 129 ms |
| 5,000 entries, last entry drawn | 759, 727, 729, 759 ms | 840, 808 ms | 795, 742 ms |
| 20,000 entries, first entry drawn | 129, 128, 128, 129 ms | 161, 161 ms | 146, 146 ms |
| 20,000 entries, last entry drawn | 2,711, 2,728, 2,478, 2,459 ms | 2,628, 2,926 ms | 2,492, 2,529 ms |

- Worse, and still worse: a long session's first entry is drawn about
  17 ms later (it was 32 ms later before the composer change), and the
  page loads 7 KB more (the conversation's screens). A CPU profile of
  opening a 5,000-entry session shows no single function behind it: the
  page's JavaScript time is about 5% more, spread over the session
  screen, the shell and the preferences module; the timeline's fold
  itself is the same as the base's (a unit measurement of `reduce` over
  the same records: 5,000 in about 4 ms, 20,000 in about 45 ms, both
  trees).
- Within the runs' spread, not counted: the loads' paint and usable
  times (each pair disagreed in sign), find, the terminal, the board of
  500 cards.

### Gate (final tree)

```
cargo fmt --all --check                                   fmt exit 0
cargo clippy --workspace --all-targets -- -D warnings     Finished; exit 0
cargo clippy -p vessel-proto -p vessel-web --features ts --all-targets -- -D warnings
                                                          Finished; exit 0
cargo test --workspace --no-fail-fast                     95 suites: 1,340 passed, 0 failed, 9 ignored
cargo test -p vessel-proto -p vessel-web --features ts --lib typescript
                                                          2 passed (declarations current)
web: npm run lint                                         exit 0 (prettier, fonts: 132 stacks, controls, size: within budget)
web: npm run typecheck                                    exit 0
web: npm run test                                         66 files, 302 tests passed
web: npm run build                                        exit 0
web: npm run e2e                                          47 passed (2.1m)
web: npm run screens                                      6 passed (3.1m)
web: VESSEL_CARGO_PROFILE=release npm run perf            5 passed (each run)
```

### Not built, and why

- No "Compact context" button, no Undo after archive, a long paste kept
  as text, files uploaded on Send (DESIGN.md §12, 21–24).
- No pointer test of dragging a queued row: the end-to-end test moves it
  by key; the drag shares the same `move` call.

### Not verified

- A real agent: steering, images and Claude's own question form were
  run against the fake agent only (no prompt was sent to a real agent).
- Paste and drop: the tests pick files through the file chooser; paste
  and drop go through the same `Tray.add`.
- A protocol 9 node in the browser: the page's older path (no queue,
  attach, menu, meter or marks) is decided by `hasConversation` and
  unit-tested, not run against an older node.
- The worktree's fate after delete with a worktree to keep: the toast's
  words are unit-tested, the fleet's sessions have no worktree.

### For the join

- `ui/kit/icon.tsx`: this track added `file` and `trash` (with
  paperclip, pin, grip, steer, queue, restore, enter, circle, help);
  B-page added `file` and `trash` too. Keep one of each.
- `data/workspace/find.ts`: every tab's first search key is 1, so two
  tabs searching one machine at once replace each other's searches; the
  file's owner may want a per-tab start.
- Under `crates/`, besides `vessel-testkit` and `wire/prefs*`, the new
  preference needed `keep/prefs_doc*` (the document that keeps it) and
  the re-export in `wire.rs`.
- `features/session/use-window.ts`: this track changed the resize
  observer (`watch`); B-page changed the `Reading` type and the scroll
  handler's condition. Different lines; with B-page's `follow: false`
  nothing here takes effect.
- `docs/design/web-ui/DESIGN.md`: §2.2, §6.5, §6.7, §6.12, §6.15, §7.4,
  §9 and §12 (21–26) are this track's.
- `e2e/kit.ts`: `startSession` waits for a structured session's timeline
  before anything is typed into it.
- Commit `ac9243b` does not pass the type check (a test's type, fixed in
  the next, `600f9d5`), and from `c53d44f` to `b9289a3` image previews
  are empty under the content policy; the branch was not rewritten.
- Around this branch since it was cut (not merged here): the node's
  join changed `file.write` (`expect: { kind: "content", hash }`) and
  `ProjectCommands::Listed` (`last_approved`), and track G opened the
  set of agents (`HarnessRef`, compared by key; an agent a person added
  shown by its label). This track's code names an agent through
  `data/agents.ts`'s `agentName(harness)` in the composer (which hands
  the name to the refusal lines and the tray) and in the new-session
  page's controls (moved there from `new-session.tsx`, where the base
  calls it once); it compares no harness by kind. The join should run
  those calls through G's rule.
- Flaky once each in full runs, passing alone: `timeline.spec.ts` (a
  1.09px drift against 1), `reconnect.spec.ts`, and the first test of a
  run waiting on the agent menu's radio item that kept being redrawn
  (`kit.ts`'s `chooseAgent`).

## Track B-page: progress

2026-11-12, on `backend/m11-b-page`. Page only: nothing under `crates/`
changed, no wire change, no ADR, no store schema. The screens are
taken from agent-orchestrator first, then GOAT, then Paseo, as
DESIGN.md §6.18 and §12 (21–27) say value by value.

### Built

- **Every call of protocol 10's projects, worktrees, git and files** as
  a typed function giving one `Outcome` (`web/src/data/workspace/`: one
  module per subject; `find.ts`, written before the track, is the one
  wrapper of `file.find`), and the pure parts the screens draw, each with
  unit tests: git's refusals and a worktree's states and kept reasons in
  words; setup's output as lines (escape sequences scanned out, carriage
  returns applied, the last 2,000 lines kept); a diff as numbered rows
  keyed so pages add without moving them; the Changes list; the file
  tree; the git button's action and GOAT's reasons for an action that is
  off; a branch name checked with git's rules; where a session runs and
  the spec it makes; the commands to approve compared with the folder's
  approved set; a selection as agent-orchestrator's captured-code
  reference; a lexer for syntax colour.
- **The inspector as tabs** (agent-orchestrator's): Summary, Changes,
  Files, the kit's new `PanelTabs`. Changes and Files are a chunk of their
  own (`features/workspace/panel.tsx`, loaded with `lazy()`; the import
  map pins it as it pins every chunk, and `security.spec.ts` still
  passes). Summary keeps 340px; code takes 500px, at most 45% of the
  window. Below 1280px the branch's chip opens the panel as a sheet from
  the right (the kit's `Sheet` gained a side).
- **Changes**: GOAT's git bar and split button (the kit's new
  `SplitButton`), agent-orchestrator's comparison picker (Changes,
  Uncommitted, Staged, Unstaged, the latest twenty commits), whitespace,
  fold all, refresh; every changed file's diff in one list windowed by
  the timeline's `useWindow` (reused with one added option, `follow:
  false`, for a list read from its top), a file's diff and each next
  page asked for when its row is drawn; stage, unstage, discard (with
  Paseo's confirmation, deleting an untracked file said as such), commit
  (GOAT's dialog), push, pull, fetch, switch, create and rename a branch,
  stash save, apply, pop and drop; each refusal said in words above the
  list or in its dialog, never a toast.
- **Files**: the tree (a folder listed when opened), a file by name, lines
  by content, a viewer windowed and paged, an editor (the kit's
  `TextArea` gained `fill`) that saves with the version it read and,
  when the file changed on disk since, keeps the edit and shows both
  sides with Reload from disk and Overwrite.
- **A selection sent to the composer**: "Add to chat" beside a selection
  in a diff or a file appends the reference to the session's draft
  (`fleet.drafts`), the composer's own; nothing of the composer was
  touched.
- **Where a session runs** on the new-session page: the project, then
  Local, New worktree (with its base) or Existing branch; the node's
  order for a worktree (create; approve when asked; follow setup's
  output to its end; a failed setup waits for the person; then the
  session). `new-session.tsx` kept its 296 lines: the tray and the
  start are `where-tray.tsx`, `where-place.tsx`, `use-starter.tsx` and
  `start-worktree.ts`.
- **Projects**: Settings › Projects (add a folder, clone with progress,
  start one; the commands' approval and withdrawal; worktrees, removed by
  the node's rule and, when work would be lost, Paseo's warning with a
  snapshot or a discard that says what it deletes; snapshots dropped),
  and a Projects group in the sidebar (each machine's projects, open to
  their live sessions, a "+" that starts a session there).
- **A session's branch** on its title bar (a chip that opens Changes),
  its board card and its sidebar row (`SidebarSession.branch`, one added
  field).
- **The end-to-end fleet** makes real repositories in its temporary
  directory (`e2e/repos.ts`: `shop`, with commits, a branch, a
  `vessel.toml` and uncommitted, staged and untracked work; `notes`,
  without git) and adds them as laptop's projects with the CLI.

### Checked, and how

- Unit tests (vitest) for every pure module above, 68 of the suite's 304
  tests.
- `e2e/code.spec.ts` against the real fleet: a session started in a new
  worktree only after its commands were approved, with setup's output
  shown as it ran and the branch on its bar; the Changes tab listing the
  work, staging a file, and a branch renamed to one that exists refused
  in words; an edit saved after the file changed on disk shows both
  sides and the disk's text is still there; lines selected in a diff
  land in the draft as the reference; a folder added from Settings and a
  failed clone said plainly. Two older tests followed the inspector's new
  name ("Session panel").
- `e2e/code-screens.ts` captures the new-session page's places and modes,
  the approval (light and dark), setup running, a session in its
  worktree, Changes with the git menu and the commit dialog, a file and
  its editor, find, Changes dark, the sidebar's projects (light and
  dark), Settings › Projects, the clone dialog, and Changes as a sheet at
  390px (light and dark).

- `npm run perf` on this tree and on its base, back to back in release
  (`docs/PERFORMANCE.md`, "Track B-page"): a new measurement of the code
  panel (a diff of 4,000 hunks beside 1,500 changed files: 764 ms from
  the tab to its first line, 33 ms a scroll step, no long task; a folder
  of 5,000: 390 ms). **Worse**, explained there and not fixed: the
  board's load over the slow link by 40 ms until usable (70 ms on the
  phone) and 5,164 bytes (the code panel's styles in the one stylesheet
  ADR 0017 allows, five more chunks pinned, the sidebar's projects and
  the branch chip in the entry, `project.list` at start); opening a long
  session by 16–27 ms to its first entry and 59–132 ms to its last, with
  one long task of about 50 ms (not profiled apart). Find, the terminal
  and the 500-card board stayed within their spread.

### A review, and what it changed

A second reading of the whole page side (code, not this plan) found
these, each fixed with a test where one can hold it:

- The panel's root was a new object on every drawing of the session's
  page, so while an agent streamed, the Changes tab's reads, a file's
  pages and a search's settle started again on each record. The root is
  now one object while it names the same place, and the tabs are keyed
  by it.
- A save sent the newest version read, not the one the edit started
  from, so a re-read during the edit let a save overwrite an agent's
  change; pages of two versions could also be joined. The edit now keeps
  its own version, and a page of another version reads the file again.
- Every guarded save of an untouched file was refused (the wire gap
  below); the page now writes it when the disk's size and content hash
  are the ones the edit started from.
- Leaving the new-session page while setup ran still started the
  session, sent the prompt and opened it. The start is aborted on
  leaving (and on another place chosen).
- The follow of setup's output and of a clone asked back to back when
  the node answered without waiting (a forwarded call, its waits taken);
  a read that brings nothing new now waits out a second
  (`data/workspace/pace.ts`). A failed read is tried again three times.
- A poll answered after a stage or a discard could put back the older
  status; a diff asked before its file changed could be shown under the
  new counts. Only the latest read is taken, and a diff only if its file
  did not change since it was asked.
- The sidebar's "+" did not choose its project; discarding a file added
  since the last commit said it "goes back" (it is deleted); a stash and
  a snapshot were dropped on one press (now Keep or "Drop for good");
  Settings › Projects said a refused removal in a toast (now a banner);
  a line far down a file was not gone to; a first page inside one long
  line never showed; `ESC ( B` left a "B" and `ESC [ K` left a progress
  bar's tail in setup's output; below 1280px the inspector stayed
  mounted beside the sheet, reading twice; the whole setup log was a
  live region; "Go to file" could show an older query's answer; the
  tree never listed a folder again (Refresh); a clone URL in scp form
  was refused on Enter; a hunk the node cut was not marked.

Found while checking the screens, outside the code panel: the tray's
folder field was squeezed to nothing at 390px beside the project select
(it now wraps to its own line); the fleet asked laptop to add projects
before its listener was bound (it now waits for each node's listener);
and track W's tests looked for an echoed reply anywhere on the page,
where the timeline's live region repeats it (now inside the timeline).

### Not built, and why

- **A guarded write of an untouched file.** `FileVersion.modified` is
  nanoseconds since 1970 (about 1.7e18), past what a JavaScript number
  holds exactly (2^53): the bridge's JSON rounds it, the page sends the
  rounded value back, and the node's exact comparison
  (`vessel-workspace`'s `write`) refuses nearly every guarded write on a
  file system with nanosecond times. **Wire gap**: the node should
  compare size and hash when the version has a hash (or carry the time
  as text). Until then the page writes with `any` when the disk's size
  and hash are the ones the edit started from, which leaves one round
  trip in which a change would be overwritten
  (`features/workspace/use-file.ts`'s Decision Log).

- **What changed since the last approval, exactly.** The node keeps only
  the digests it approved (store schema 9's `approvals`), not their
  commands, so the page compares the set asked for with the folder's set
  when that one is approved and differs (the common case: a branch or a
  pull changed `vessel.toml`), and otherwise says it has nothing to
  compare with. **Wire gap**: `ProjectCommands` (or `project.commands`)
  should carry the last approved set (its setup and teardown lists and
  when) for the dialog to compare with that.
- A per-line comment, "viewed" marks, a split view, word-level marks, image
  and rich previews (agent-orchestrator's review pane): a selection sent
  to the draft is what is offered; DESIGN.md §12.
- A quick-open shortcut (`⌘P`) and "Add to chat"'s `⌘L`: the shortcut
  registry is track W's, and a browser keeps Ctrl+L.
- Resizing the panel.

### What the next tracks need to know

- `useWindow(items, target, { follow: false })` is a list read from its
  top; the timeline's own call is unchanged.
- `data/workspace/project-store.ts` holds each machine's projects for
  every screen; `tookProject` after a change made from the page,
  `loadProjects` to read again. `wantedPlace` starts the next new session
  in a project.
- The panel finds a session's project from `SessionInfo.workspace`, or,
  for a session started in a typed folder, from the project whose root
  holds its `cwd` (compared as text).
- B2's pull request on a session's page belongs beside the git bar; the
  card's branch chip is where its facts can go.

## Integration 2 (node): tracks B2 and C

2026-11-12, on `backend/m11-int2-node`: track B2's branch
(`backend/m11-forge`) with track C's (`backend/m11-shells`) merged in.
The page's tracks (A-page, B-page) are joined apart; the coordinator
joins the two. Every track's commit is kept; the merge commit builds
and passes, and each follow-up is a commit of its own.

### What conflicted, and how each was resolved

The merge conflicted in 33 files: the wire (`vessel-proto`: `rpc.rs`,
`rpc/older.rs`, `rpc/since.rs`, `rpc/tests.rs`, `forward.rs`,
`ts/aliases.rs`), the node (`service.rs`, `service/worktrees.rs`,
`guard.rs`, `envelope.rs`, `node.rs`, `config.rs`, `tidying.rs`,
`workspace/{mod,policy,tests}.rs`, `workspace/worktrees/removal.rs`),
the session runtime (`handle.rs`, `runtime/parts.rs`), the agent tools
(`vessel-mcp`'s `tools.rs` and its test), the CLI (`args.rs`,
`commands.rs`, `main.rs`), `vessel-rpc` (the client's unary list, its
test handler), `vessel-client`'s test handler, `vessel-web/src/calls.rs`,
ADR 0001, `docs/ARCHITECTURE.md`, `docs/PROTOCOL.md`, this plan and the
generated `web/src/wire/proto.ts`.

- Where both tracks added to the same list (the method table and every
  list naming each method: `Call::since`, `Reply::for_protocol`, the
  forwardable list, the unary calls, the web bridge's list, the guard's
  rows, the envelope's dispatch, the test handlers, the agents' list,
  the agent tools, the CLI's commands, `Config`, the node's parts, the
  session's shared state), both stay, B2's first. The agent tools are
  session, conversation, workspace, pull request, terminal, then the
  language server's (34).
- A worktree's removal: B2 read the live sessions after its claim
  (ADR 0030); C passed what runs where (`Occupant`: sessions and
  terminals) read before. The merge keeps B2's order with C's occupants:
  `remove_worktree` and `retire` take a closure called after the claim,
  returning live sessions, live terminals and terminals starting.
- Code written against a type the other side changed, with no textual
  conflict, found by `cargo check --workspace --all-targets` right after
  the merge: C's terminals and scripts called `Workspaces::busy`, which
  B2 had replaced with the "starting" count. Resolved by giving terminals
  the same count (below, meeting point 2).
- `ProtocolVersion` 11 raised once; schema 10 (B2) kept. Track C was
  given schema 11 and needed none; the join's set last approved took it
  (below), and the store's tests walk 9, 10, 11 in order
  (`tests/approvals.rs`; the rollback SQL of the older tests is
  `UNDO_SCHEMAS_7_TO_11`).
- The plan keeps every section: contracts B2, C; progress B2, C; then
  this one. `web/src/wire/proto.ts` regenerated, never hand-merged;
  `Cargo.lock` merged clean and did not change when built.
- In the merge commit, beyond resolving: B2's protocol 11 reads sat in a
  test list that only ever saw protocol 10's prefixes, so their scope was
  never checked; protocol 11's nineteen methods have a test of their own.
  C's refusal of `script.*` to agents had a broken line continuation (18
  spaces in the message); the agents' refusal now says why by family.

### Where the two tracks met, and what was decided

1. **How git runs, and what C starts (ADR 0030 with ADR 0031).** A
   terminal, a script, a service, setup and teardown are not git runs of
   the node's: they run only where a person asked (a terminal; an
   agent's only where a person set `agent_terminals` on both profiles) or
   approved (the digest, which covers scripts), and git inside them runs
   under the repository's configuration as in the user's own terminal.
   ADR 0030 is amended to say so. Checked that nothing C starts runs
   unapproved: `script.run` runs the command of the very parse whose
   digest is approved, a service asks again before every restart, a
   restart of a run asks again; no git is run to build a script's
   environment. The environment: besides Vessel's credentials (C's list),
   the variables the forge's token is read from (`GH_TOKEN`,
   `GITHUB_TOKEN`, `GH_ENTERPRISE_TOKEN`, `GITHUB_ENTERPRISE_TOKEN`, now
   `vessel_forge::SIGN_IN_VARIABLES`) are removed from every terminal,
   script and service, and from setup and teardown, which removed
   nothing before. Tested: the removal list (`env.rs`), setup's command
   without a removed variable (`commands.rs`). Not closed, and said in
   ADRs 0029 and 0031: an agent's own harness inherits the node's
   environment (ADR 0013), and `gh`'s stored login is readable by any
   process of the user.
2. **A terminal starting in a worktree being removed (C's open
   window).** Closed inside the removal's own claim: a terminal or a
   script's run whose folder is in a worktree takes ADR 0030's "starting"
   count (`Workspaces::hold_folder`) before its reservation and its
   spawn, and gives it back once registered, when it is one of the live
   terminals the removal reads after its claim. Either the removal sees
   it (`kept { in_use }`) or the start finds the claim and is refused
   (`conflict`) before anything runs. C's check after the start, which
   killed a terminal that started under a removal, is gone. Tests:
   `git_apart.rs` (a held worktree is kept; inside the removal's reading
   a start is refused) and the e2e `meeting` race (six rounds of a
   terminal opened beside a session in a worktree while the worktree is
   removed; with the hold taken out, it fails in the first round on
   every run, three of three).
3. **B2's watcher and feedback beside C's terminals and services.**
   Decided, for a session with a pull request and a service and a shell
   beside it:
   - its end: the watch and `SessionInfo.pull_request` end (B2); the
     service and the shell run on (C: `session.resume` starts it again in
     the same folder), so its worktree is kept;
   - archived: the watch ends (B2) and now the terminals and runs beside
     it end too, kept with their output (a service beside a session put
     away would hold its port and its worktree unseen); the worktree can
     then go;
   - deleted: terminals and runs end and are forgotten, then its
     worktree is tidied (C), its watch dropped and its feedback rows
     deleted with it (B2);
   - its worktree removed: never while anything live is in it; a removal
     ends nothing.
   Tested end to end in `meeting` (archive; end then delete), against the
   in-memory forge and `/bin/sh`. The meeting test found a bug of C's:
   a script beside a session and a terminal's count against a project
   read the session's place from the runtime's info, which never carries
   it, so `script.run { place: session }` answered `no_project` for
   every session; both take it from the workspace registry now.
4. **The agent's limits in one place.** `workspace/policy.rs`'s
   exhaustive `refused` is the one list of what an agent may call through
   the node: refused `forge.status`, `pr.list`, `pr.merge`,
   `pr.feedback`, `script.list`, `script.run`, `script.restart` (and
   protocol 10's workspace calls); open here and held to their own rule
   `pr.get` (refresh only on a session it may operate), `pr.open` (same,
   never pushing), every `terminal.*` and `port.list` (both profiles,
   four at once, only its own). No method is undecided (a new one does
   not compile); the test names every protocol 11 method.
5. **`SessionInfo` and the board.** Every reply that carries a session
   carries its pull request while it is watched (`session.list`,
   `session.get`, a mark's reply: checked in `meeting`); an ended
   session's is cleared, as the store has none. A session's terminals are
   not put on `SessionInfo`: a page lists `terminal.list {}` (at most 96)
   once and groups by `place`, since a terminal's change moves its own
   `revision`, not the session's, and a count on the session would be
   stale between reads. Older peers: a protocol 8 to 10 client ignores
   `pull_request` and draws the board's lanes it knows; it sees no
   terminal.

### The two gaps the page found, fixed in protocol 11

- **`ProjectCommands::Listed.last_approved`** (`ApprovedCommands {
  commands: CommandSet { setup, teardown, scripts }, digest, approved_at
  }`), while the listed set is not approved. The node remembers the last
  16 sets it described by digest; an approval of one stores the set with
  it (schema 11: `approvals.commands`; approved again, the newest time);
  the four newest per project are held in memory. An approval from
  before, or of a digest the node never described, has no set and is
  never shown. Bounded as the set it mirrors. A protocol 10 client gets
  the set without it. A page shows `last_approved.commands` beside the
  listed set.
- **A write guarded by the content hash.** A page sends `file.write {
  root, path, text, expect: { kind: "content", hash } }` with
  `FileVersion.hash` of what it read or last wrote. `conflict`: nothing
  is there, or the content is not that (another write landed, or the
  file grew past 8 MiB and has no hash); nothing is written. `absent`:
  `conflict` when something is there. `version`, which compared
  nanoseconds no page can hold, is gone (protocol 10 was never
  released); `modified` is milliseconds, shown only. B-page's
  workaround (`any` after a check of its own) can be removed in the
  page's join: `{ kind: "version", version }` no longer typechecks.
- ADR 0027 amended for both; tests: `approvals.rs` (node), the store's
  schema walk and bound, `a_write_is_guarded_by_the_content_it_read`
  (workspace), `Call::since` (a `content` write needs 11), the stand-in
  for protocol 10, and `page_wire` through a node for both.

### New aliases on the wire

All twelve were classified by their tracks; the join added none
(`CommandSet` and `ApprovedCommands` are structs of existing ones):

- B2: `CheckName`, `ConfigKey`, `ForgeHost`, `ForgeLogin`, `ForgeUrl`,
  `PullNumber`, `PullTitle`, `RepoPath` named; `PullBody` content.
- C: `TerminalId` a handle; `ScriptName` named; `ProcessName` content.

### The independent review

One reviewer (Opus, read only, security first) read `git diff
backend/m11` at `35ab1fc` and found ten problems. Each one's fate, most
severe first:

1. **High: `gh` handed the node's enterprise token to any host.** `gh
   auth token --hostname <host>` answers from `GH_ENTERPRISE_TOKEN` (and
   `GH_TOKEN` for `*.ghe.com`) before its stored logins, and the node
   asks it for every host a remote names, which an agent can set: the
   token went to the agent's host in a request's header. **Fixed**: `gh`
   runs without the token variables and `GH_HOST` (`Gh::token_spec`,
   tested); the node reads them itself for their own hosts.
2. **High (design): an agent without `agent_terminals` can start a
   session under a profile that has it, and prompt that agent.**
   **Not taken as a change here**: the child's terminal is held to the
   child's profile and permission mode, which is what a person allowed
   that profile; which profiles an agent may start sessions under is ADR
   0013's question. ADR 0031's amendment now says so instead of claiming
   more.
3. **Medium: commit and push hooks run with the forge's token
   variables.** **Not taken**: they run with the user's environment as
   the user's own `git push` does (a person's calls only); removing the
   variables would break pushes through `gh auth git-credential`, and a
   hook could still ask the credential helper. Said in ADR 0030.
4. **Medium: another node's agent's terminal outlives the agent's
   session.** **Left, and said** (ADR 0031): this node sees only the end
   of the session the terminal is beside; the agent's credential ends
   with it, so it cannot reach the terminal, which a person sees in
   `terminal.list`. Closing it needs the agent's node to close what it
   opened elsewhere, a fleet change of its own.
5. **Low: a resumed session's new agent terminal may go unwatched**
   while the old watch is still ending the previous terminals (the set
   of watched sessions is by id). **Left, and said here**: it needs the
   watch keyed by the runtime's handle, not the session's id; the window
   is the ten seconds the old terminals take to end.
6. **Low: a terminal or a script could start beside an archived
   session.** **Fixed**: `conflict` until it is brought back, and a start
   racing the archiving ends itself (`meeting` checks both calls).
7. **Low: turning `agent_terminals` off leaves open terminals running.**
   **Not taken**: the switch is asked at each start, as an approval is
   for a service; said in ADR 0031.
8. **Low: an agent's `port.list` was filtered after the node-wide bound
   of 256**, so its own ports could be cut by others'. **Fixed**: a scan
   keeps up to 4,096, and each caller's are picked out before the reply's
   bound (`ports.rs` test).
9. **Low: unbounded keeping.** `gh`'s answers and the hosts refused a
   moment ago were kept per host a remote names: **fixed**, at most 64
   each (tested). Every approval kept its set forever: **fixed**, only
   the newest four per project keep theirs, every approval keeps its
   digest (store test).
10. **Low: `SessionInfo.pull_request` reaches agents.** **Not taken**: an
    agent reads any session's pull request by B2's contract (`pr.get`);
    said in ADR 0029.

Areas the reviewer checked and found clean: scripts' approval (the
approved parse runs, every restart asks again, agents refused), the
environments, terminal ownership on every call, the races of removal and
start and of deletion and start, git's overrides and listing, the forge's
token handling over HTTP, the bounds of the forge, terminals and port
scans, and the wire for older peers. Not read deeply: `vessel mcp`'s
terminal tools, GraphQL decoding, `own_config`'s tests, Windows paths.

### Tests that fail only under load

The machine ran at load 10 to 18 for most of the join (other workers
building). The page's end-to-end tests are fixed in the page's join, so
they are reported here, not changed:

- On the merge commit: `page.spec.ts:255` ("with Mod+Enter chosen, Enter
  starts a line…") failed once, a strict-mode violation: `getByText("echo:
  plain", { exact: true })` matched the Timeline's paragraph and the live
  region's visually hidden copy (`aria-live="polite"`). It races on
  whether the live region still holds the announcement when the
  assertion runs; the locator wants scoping to the Timeline region, as
  B-page did for another.
- On the first follow-up, at load 15: three failed in one cascade.
  `page.spec.ts:154` ("with the sidebar hidden, Mod+K still opens the
  command palette") pressed Control+B to show the sidebar again and did
  not see it within 15 s; one `vessel ui` keeps preferences for the whole
  run, so the sidebar stayed hidden for the tests after it,
  `page.spec.ts:279` (the Display button detached while the sidebar went
  away) and `reconnect.spec.ts:10` (the sidebar's "Connected" mark not
  found). Its `finally` presses Control+B right after Escape closed the
  palette: under load the keypress most likely lands while the closing
  palette still holds focus, and is lost; a page that kept the preference hidden
  then hides the sidebar for every later test. Waiting for focus to come
  back (or setting the preference through the bridge in the `finally`)
  would close it. Run again at load 2 on the same binaries: 36 of 36
  passed.
- On the second follow-up, at load 15: `app.spec.ts:61` ("a terminal
  session runs and echoes keys") timed out clicking the agent menu's
  "Claude Code" item, which re-rendered under the pointer ("not stable",
  then "detached"); `page.spec.ts:122` ("a screen asked for while
  Settings loads…") waits for Settings' chunk with `waitForResponse`
  registered after `release()`, so a chunk that arrives in between is
  never seen. Run again at load 1.5: those two passed and
  `timeline.spec.ts:101` failed instead, a row held 1.28 px from where it
  was (the bound is 1 px) while the window settled.
- None touches terminals, pull requests or files; no node code was
  involved. `vessel-remote`'s connection test passed in every run here.

### Commits

- `207fce6` the merge (33 files resolved; the removal's reading after its
  claim with C's occupants; terminals and scripts on B2's "starting"
  count; protocol 11's scope test; the agents' refusal by family).
- `1532826` where the tracks meet: the window closed and raced, archive
  ends what runs beside a session, no forge sign-in in a repository's
  processes, a script beside a session finds its project, the agents'
  list tested; two files divided.
- `35ab1fc` the two wire gaps: a write guarded by the content hash, the
  set last approved (schema 11).
- The review's fixes: `gh` without the environment's tokens, nothing new
  beside an archived session, an agent's ports picked out before the
  bound, bounded token and approval keeping; the ADRs and this section.

### The gate, on the final tree

Run on the final tree under load 7 to 12, binaries and bundle rebuilt
first:

- `cargo fmt --all --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo clippy -p vessel-proto -p vessel-web --features ts --all-targets
  -- -D warnings`: clean.
- `cargo test --workspace --no-fail-fast`: 1,486 passed, 0 failed, 9
  ignored, over 102 test binaries.
- The TypeScript regeneration (`VESSEL_WRITE_TS=1 cargo test -p
  vessel-proto -p vessel-web --features ts --lib typescript`): passes;
  `web/src/wire/proto.ts` committed with the wire's change.
- In `web/`: `npm ci`, `npm run lint`, `npm run typecheck`, `npm run
  test` (238 passed, 54 files), `npm run build`, then
  `VESSEL_WEB_REQUIRE_BUNDLE=1 cargo build -p vessel-cli -p vessel-node
  -p vessel-testkit --bins` and `npm run e2e` (36 passed). Nothing under
  `web/` changed but the generated declarations.

### For the page's join

- B-page's editor saves with `{ kind: "content", hash }` (its `version`
  guard no longer typechecks), and drops its check-then-`any` workaround;
  on `conflict` it reads again and shows both.
- B-page's approval dialog shows `last_approved.commands` beside the
  listed set, and shows `scripts` (C's join condition) or refuses a set
  that has them.
- A session's terminals: `terminal.list {}` once, grouped by `place`.
- Archiving a session ends its terminals and services; say so where a
  person archives.

## Track G: contract

Protocol 11, wave 2's number: this track raises `PROTOCOL_VERSION` to
11 and changes nothing else on that line. Written 2026-11-12 from the
three references and from what ACP itself carries; ADR 0032 records the
choices that could have gone another way. No store schema is taken: an
added agent is configuration (`agents.toml`), and a session's spec
already keeps its agent inside the JSON the store holds.

### What the references do (read 2026-11-12)

- **Agents outside a fixed set.** agent-orchestrator has none: 36
  adapters compiled in. GOAT adds any agent from the ACP registry or a
  local ACP command as an instance of its ACP driver in its settings
  file (id `[a-zA-Z][a-zA-Z0-9_-]`, up to 64), launched without a shell,
  its environment the server's with the instance's on top (sensitive
  values as references to its secret store). Paseo adds
  `agents.providers.<id>` with `extends: "acp"`, `command` (the whole
  argument vector) and `env`, in its config file (written owner-only and
  atomically); the command gets the daemon's environment minus four of
  Claude Code's own variables; a built-in id overrides, never collides.
- **What such an agent gets.** In both: models, modes and options from
  ACP; resuming where the agent offers `loadSession`; the orchestrator's
  MCP tools in `session/new`. In neither: rewinding, the agent's own
  conversation files, or the orchestrator's text generation (titles).
- **Who may add one.** GOAT: `providers:manage`; an agent inside GOAT
  only through an approval card, or from a full-access thread. Paseo:
  `daemon.manage`; the local owner has every permission.
- **The ACP registry.** Published at
  `https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json`:
  `{ version, agents: [ { id, name, version, description, authors?,
  license?, website?, repository?, icon?, distribution: { npx?: {
  package, args?, env? }, uvx?: { ... }, binary?: { "<os>-<arch>": {
  archive, cmd, sha256?, args?, env? } } } } ] }`, packages pinned to an
  exact version. GOAT fetches it only when asked (search, add), within
  30 s and 1 MiB, https only; the envelope is strict, an entry it cannot
  read is dropped and counted; a disk copy without expiry stands in when
  the network fails, and its age is never shown. It installs on "Add"
  (npm or uv into a folder of its own; a binary downloaded, its SHA-256
  checked when the registry gives one, extracted and kept inside its
  folder) without showing the command first. Paseo does not fetch the
  registry: it ships about 35 entries pinned by version (`npx -y
  pkg@x.y.z --acp`), adding one writes the command, and `npx` downloads
  it at the first session.
- **Installed and signed in.** agent-orchestrator keeps typed readiness:
  installation `installed | not_installed | unknown`, authentication
  `authorized | unauthorized | unknown | not_applicable | configured`,
  each with a reason (`install_check_timeout`, `auth_check_unsupported`,
  ...) and its time, checked without a model call (a status command, 3 to
  10 s), cached five minutes for display, refreshed on request, four
  checks at once; minimum versions are enforced at launch from
  `--version` or ACP's `agentInfo.version`. It reads `claude auth status`'s
  JSON `loggedIn` (true is "configured": credentials present, not proven
  valid; false is signed out) and `codex login status`'s words ("not
  logged in", "logged in"), never the exit status, and keeps only the
  verdict (the output can hold a key fragment). GOAT reports `installed`,
  `version`, `auth.status authenticated | unauthenticated | unknown` from
  a probe of the agent: `auth_required` means signed out, and a probe
  that succeeds is not taken as proof of being signed in. Paseo prints a
  diagnostic per provider (command, resolved path, version, each ACP
  step), 20 s a step.
- **Installing.** agent-orchestrator: a fixed list of each vendor's
  installers per agent (npm, the vendor's script, brew); the client picks
  one, never a command; a job runs at most 15 minutes, keeps the last
  4,000 bytes of output, is polled, and is refused while the agent has
  live sessions. GOAT installs registry agents on add. Paseo links to the
  agent's page.
- **Signing in.** agent-orchestrator: a terminal running the agent's own
  login (`claude auth login`, `codex login`, `opencode auth login`), and
  for Claude Code and Codex a login run in the background whose URL and
  device code (hosts from an allowlist) are relayed to the page, a code
  typed back into it, a key typed into the page piped to the login's
  standard input (16 KiB, never logged). GOAT: ACP's methods (`terminal`:
  the agent's command with the method's arguments in a terminal;
  `agent`: ACP `authenticate`, then a check), keys as secret references.
  Paseo: none ("does not manage their authentication").

### Naming an agent outside the four (`vessel_proto::agents`)

`HarnessKind` stays the four, closed and `Copy`: what Vessel knows of
Claude Code, Codex, Gemini CLI and OpenCode (homes, logins, session
files, fleet sync, routing) stays matched on it. A new type names any
agent a node can run:

```rust
pub enum HarnessRef {
    Known(HarnessKind),
    Added(AgentName),
}
```

- **On the wire** a known agent is the string it always was
  (`"claude-code"`, `"codex"`, `"gemini"`, `"open-code"`); an added one
  is an object, `{"added": "<name>"}`. TypeScript: `HarnessKind | {
  added: AgentName }`. An object rather than another string: a client
  tells the two apart by shape, and a stored spec keeps its meaning if
  Vessel later knows a fifth agent by a name someone had used for an
  added one. Being an object, it is never a map key on the wire: a
  per-agent map is a list of pairs.
- **`AgentName`**: the profile name rule (`[A-Za-z0-9._-]`, 1 to 64),
  refused when it is, ignoring case, any wire name or alias of the four
  (`claude-code`, `claude`, `codex`, `gemini`, `opencode`, `open-code`).
  An added agent's default profile has its name, as the four's have
  theirs (ADR 0011), so agent and profile names are one space, unique
  ignoring case: checked by `agent.add` and whenever the two files are
  read. Where a person types one word (the CLI, an agent tool's
  argument, `harness = "..."` in `profiles.toml`), it is read as one of
  the four first, then as an added agent.
- **Where `HarnessRef` replaces `HarnessKind`**: `SessionSpec.harness`,
  `Harness.kind` (`harness.list`), `ProfileInfo.harness`,
  `ListProfiles.harness`, `GetHarnessOptions.harness`. A session's
  `SessionInfo` (and so `TransferDetail::Session.info`) carries it with
  the spec. It stays `HarnessKind` where only the four can be:
  `TransferDetail::Memory` and the page's per-agent defaults
  (`vessel-web` preferences, which do not remember added agents).
- `Harness` gains `terminal: TerminalSupport` (`available`, or
  `unavailable { reason }`), left out when available and absent from
  older nodes, whose every harness runs in a terminal.

### What an added agent has, and what it does not

| | The four | An added agent |
|---|---|---|
| A structured session over ACP | yes | yes, its only mode |
| A terminal session | yes | no: `terminal: unavailable`; `session.create` in terminal mode is `unsupported` |
| `harness.options` (the probe, ADR 0022) | yes | yes |
| Profiles (ADR 0011) | `env`, `unset`, `args`, `model` | `env` and `unset`; `args` (terminal only) and `model` (Vessel does not know how to point it at the proxy) refused when the file is read |
| The `vessel` MCP server and the agent credential (ADR 0013) | in `session/new` | the same, the same way |
| The fleet's MCP servers (ADR 0012) | every one, its secrets filled in | only those its definition names (`mcp = [...]`): a server's definition in `session/new` holds its secret values, and an added agent is often a package fetched from the network |
| `session.resume` | where the agent offers `loadSession` | the same |
| `session.move_profile`, `session.export` | where Vessel knows the home and the session files | `unsupported`: Vessel does not know where it keeps conversations; an import of one is refused too |
| Fleet sync of instructions, skills, memory (ADR 0012) | into each profile's home | none: no home is known |
| Installed | found on the search path, `--version` | its program found (on the search path, or the absolute path written) or missing; never run with `--version` (an unknown program may take it as a prompt); its version from the handshake's `agentInfo` once probed. For a package runner (`npx`, `uvx`) the program found is the runner: the agent itself is fetched at its first start, inside the handshake's time limit |
| Signed in | the agent's own status command, on request; or the handshake | the handshake only |
| Installed by Vessel | the vendor's documented command | no |

Nothing about an added agent is guessed: what ACP gives (the
handshake, its options, abilities and sign-in methods) is what Vessel
has, and a feature that needs more is not offered and says why.

### Where an added agent is kept, and who may change it

`<data_dir>/agents.toml`, at most 256 KiB, written by the node
owner-only (0600) and atomically through `toml_edit`, so comments stay.
On Unix the node refuses a file, or a data directory, that others can
write or that another user owns (a file of commands run as the node's
user is dangerous when someone else can change it; one others can only
read gets a warning, as `profiles.toml` does); on Windows the data
directory's ACL is relied on, as for every file there. Read at start (a
file that does not parse stops the node) and with `profiles.toml` on
`profile.reload`: profiles may name an added agent, so the two are read
together and a reload keeps both or neither. `agent.add` and
`agent.remove` read the file as it is now, refuse if it does not parse,
change only their own table, and answer with what the reload put in
effect (a hand edit made meanwhile included).

```toml
[agents.goose]
label = "Goose"
program = "goose"         # a name found on the node's search path, or an absolute path
args = ["acp"]
env = { GOOSE_MODE = "auto", ANTHROPIC_API_KEY = { secret = "anthropic" } }
inherit = ["GOOSE_PROVIDER"]  # passed on from the node's own environment, by name
mcp = ["docs"]            # fleet MCP servers it gets, by name (default none)
source = { registry = "goose", version = "1.9.0" }   # when added from the registry
```

- **Who.** `agent.add` and `agent.remove` need `admin` and are refused
  (`forbidden`) to an agent principal whatever its scopes (agents are
  held to `read` and `operate` today; the refusal keeps a later grant
  from widening it: choosing a command to run is not something an agent
  may give itself), and to a call delivered from another node: an added
  agent is this machine's, set by its owner or its own devices. Editing
  the file and `profile.reload` stays the owner's other way.
- **What it is given.** Not the node's environment. The command starts
  from an empty environment, then gets:
  - `PATH` (the node's search path, on which its program was found) and
    the user's home (`HOME` on Unix, `USERPROFILE` on Windows, from the
    node's `HarnessEnv`);
  - the system's own non-credential variables the node has. Unix:
    `USER`, `LOGNAME`, `SHELL`, `LANG`, `LANGUAGE`, `LC_ALL`, `LC_CTYPE`,
    `LC_MESSAGES`, `TERM`, `TZ`, `TMPDIR`, `XDG_CONFIG_HOME`,
    `XDG_DATA_HOME`, `XDG_STATE_HOME`, `XDG_CACHE_HOME`,
    `XDG_RUNTIME_DIR`, and on macOS `__CF_USER_TEXT_ENCODING`. Windows:
    `USERNAME`, `USERDOMAIN`, `COMPUTERNAME`, `HOMEDRIVE`, `HOMEPATH`,
    `SystemRoot`, `SystemDrive`, `windir`, `ComSpec`, `PATHEXT`, `TEMP`,
    `TMP`, `APPDATA`, `LOCALAPPDATA`, `ProgramData`, `ALLUSERSPROFILE`,
    `ProgramFiles`, `ProgramFiles(x86)`, `ProgramW6432`,
    `CommonProgramFiles`, `CommonProgramFiles(x86)`,
    `CommonProgramW6432`, `PSModulePath`, `NUMBER_OF_PROCESSORS`,
    `PROCESSOR_ARCHITECTURE`, `PROCESSOR_IDENTIFIER`, `PROCESSOR_LEVEL`,
    `PROCESSOR_REVISION`, `OS`. Both: `SSL_CERT_FILE`, `SSL_CERT_DIR`,
    `NODE_EXTRA_CA_CERTS`, and the proxy variables (`HTTP_PROXY`,
    `HTTPS_PROXY`, `NO_PROXY`, `ALL_PROXY`, either case) unless the value
    holds a user and password (`user:pass@`);
  - the variables its `inherit` names, as the node has them; its `env`
    (text, or a secret of the node's `secrets.toml` by name, ADR 0012;
    one not set there fails the launch, naming it); then its profile's
    `env` and `unset`.
  Left out on purpose, though the four inherit them: the keyring's bus
  (`DBUS_SESSION_BUS_ADDRESS`) and the SSH agent's socket
  (`SSH_AUTH_SOCK`), each a way to other credentials; `inherit` passes
  them when the owner wants. So no provider key, token or Vessel
  variable of the node reaches it unless its definition names one; the
  agent credential reaches it only inside the `vessel` MCP server's
  definition in `session/new`, as it reaches the four.
- **The command** runs without a shell: the program and its arguments as
  written, `~` and a leading `~/` expanded as in profiles. A program
  written with a path separator must be absolute. It is looked up at
  each start, and `agent.status` reports the path it found. On Windows a
  program that resolves to a `.cmd` or `.bat` (`npx.cmd`) runs through
  `cmd.exe`; Rust's standard library refuses arguments it cannot quote
  safely there, and that refusal is the start's error. Arguments are
  shown to every client and visible in the process table, so a secret
  belongs in `env` as `{ secret = "..." }` (the CLI's help says so).
- **The fleet copies none.** A definition names programs, paths and
  secrets of the machine it was written for: copied to another it would
  run something else, or nothing, there. Adding the same agent on
  another machine is one call there.

### Types (`vessel_proto::agents`)

```rust
/// One agent as `agent.list` reports it.
pub struct AgentInfo { pub agent: HarnessRef, pub detail: AgentDetail }

pub enum AgentDetail {                       // tag "kind"
    /// One of the four: how it is installed (each command shown before
    /// it runs) and how a person signs in.
    Known { installs: Vec<InstallOffer>, sign_in: SignInHelp },
    /// Added on this node.
    Added { added: AddedAgent },
}

/// An added agent as clients see it: its command whole, its environment
/// by name (values never leave the node, as for profiles).
pub struct AddedAgent {
    pub label: Option<AgentLabel>,           // one line, at most 80 characters
    pub program: AgentProgram,               // as written, at most 1 KiB
    pub args: Vec<String>,                   // 64, 1 KiB each
    pub env: Vec<EnvName>,                   // set as text
    pub secrets: Vec<SecretEnv>,             // { variable: EnvName, secret: SecretName }
    pub inherit: Vec<EnvName>,
    pub mcp: Vec<McpServerName>,             // the fleet's servers it gets
    pub source: AgentSource,                 // by_command | registry { id, version }
}

/// What `agent.add` writes when a person gives the command.
pub struct AgentDefinition {
    pub label: Option<AgentLabel>,
    pub program: AgentProgram,
    pub args: Vec<String>,
    pub env: Vec<(EnvName, ConfigValue)>,    // text, or { secret = NAME }
    pub inherit: Vec<EnvName>,
    pub mcp: Vec<McpServerName>,
}

pub enum AgentToAdd {                        // tag "from"
    /// The command a person wrote.
    Command { definition: AgentDefinition },
    /// An entry of the node's copy of the registry, at this version:
    /// the node builds the definition from its own copy, so what runs is
    /// what the catalogue showed.
    Registry { id: RegistryId, version: RegistryVersion, mcp: Vec<McpServerName> },
}

pub struct InstallOffer { pub part: InstallPart, pub command: String }
pub enum InstallPart { Harness, Adapter }

/// Facts about one agent under one profile (`agent.status`).
pub struct AgentReadiness {
    pub agent: HarnessRef,
    pub profile: ProfileName,
    pub installed: Installed,
    pub signed_in: SignedIn,
    pub checked_at: DateTime<Utc>,           // when the older of the two was found
}

pub enum Installed {                         // tag "state"
    Found { path: PathBuf, version: Option<AgentVersion>, structured: StructuredSupport },
    Missing { program: AgentProgram },
    /// Its ACP version (from `initialize`) is older than Vessel speaks.
    TooOld { path: PathBuf, version: Option<AgentVersion>, speaks: u16, needs: u16 },
    /// Newer than Vessel speaks.
    TooNew { path: PathBuf, version: Option<AgentVersion>, speaks: u16, needs: u16 },
    Unknown { reason: UnknownReason },
}

pub enum SignedIn {                          // tag "state"
    /// The agent's own status command says it holds credentials
    /// (Claude Code's `loggedIn: true`, Codex's "logged in"). Whether they
    /// are still valid is known at the first turn.
    Yes,
    /// Its status command, or `auth_required` in its handshake, says not.
    No { help: SignInHelp },
    /// No sign-in is needed: the profile runs on a model route (ADR 0015).
    NotRequired,
    Unknown { reason: UnknownReason },
}

pub enum UnknownReason {                     // tag "reason"
    NotInstalled,
    /// Not asked yet: a status command and a probe run only on `refresh`.
    NotChecked,
    /// A session of this profile is running; its own check could race it
    /// for the credentials (refreshing a token), so it is not run now.
    InUse,
    /// The handshake completed without asking for a sign-in: many agents
    /// ask only at the first prompt.
    NotSaid,
    /// The profile's home variable is relative: whose login is unknown.
    NoHome,
    /// The check gave an answer Vessel does not read (another version's
    /// words).
    Unread,
    TimedOut { after: DurationMs },
    /// The check or the probe failed: at which step, never its output.
    Failed { step: CheckStep },              // spawn | initialize | session_new | exited
}

pub struct SignInHelp {
    /// The agent's own command, to run on that machine as the profile's
    /// account; none when it signs in inside its own interface.
    pub command: Option<String>,
    /// The variable that chooses the account (`CLAUDE_CONFIG_DIR`), by
    /// name: its value is the profile's, in `profiles.toml` (values never
    /// leave the node).
    pub home: Option<EnvName>,
    /// The methods its last handshake offered (ACP `authMethods` of type
    /// `agent`; 16 at most, names 128 and descriptions 512 characters).
    /// Vessel does not advertise ACP's terminal sign-in, so none of that
    /// type is sent to it.
    pub methods: Vec<AuthMethod>,            // { id, name, description? }
}

pub struct Catalogue {
    pub fetched_at: Option<DateTime<Utc>>,   // None: not fetched since the node started
    pub version: Option<RegistryVersion>,    // the registry's own
    pub entries: Vec<CatalogueEntry>,
    pub skipped: u32,                        // entries Vessel could not read or that pin no version
}

pub struct CatalogueEntry {
    pub id: RegistryId, pub name: String, pub version: RegistryVersion,
    pub description: String, pub website: Option<HttpUrl>,
    pub repository: Option<HttpUrl>, pub license: Option<String>,
    pub runs: CatalogueRun,
}

pub enum CatalogueRun {                      // tag "kind"
    /// One of the four, by its documented package: install and use it as
    /// that agent (`agent.install`), not as an added one.
    Known { harness: HarnessKind },
    /// Started by a package runner (`npx`, `uvx`) that fetches the package
    /// at its first session, pinned to the exact version (what it depends
    /// on is resolved by the runner then): exactly this would be added.
    Command { program: AgentProgram, args: Vec<String>, env: Vec<(EnvName, String)>,
              runner_found: bool },
    /// Only as an archive for this platform, which Vessel does not
    /// download: a person installs it, then adds it by command.
    Download { archive: HttpUrl, sha256: Option<Sha256Hex> },
    /// Nothing for this platform.
    Unavailable,
}

pub struct InstallJob { pub id: InstallId, pub agent: HarnessKind,
                        pub part: InstallPart, pub command: String,
                        pub started_at: DateTime<Utc> }

/// A page of an install's output: the shape of `worktree.output`.
pub struct InstallOutput {
    pub from: u64,
    pub bytes: TerminalBytes,                // standard output and error, as printed
    pub next: u64,
    pub truncated: bool,                     // the beginning is no longer kept
    pub state: InstallState,                 // running | succeeded | failed { code } | timed_out
}
```

New aliases for `ts/aliases.rs`: `AgentName`, `AgentLabel`,
`AgentProgram`, `AgentVersion`, `RegistryId`, `RegistryVersion`,
`Sha256Hex` are `Named` (a person types or reads them); `InstallId` is a
`Handle` (the node mints it), and so is `AuthMethodId`, which the agent
names and a client only hands back (as built; "Track G: progress").

### Calls

All protocol 11. "Not an agent": refused with `forbidden` to an agent
principal (local, or delegated from another node) and to any call
delivered from another node.

| Method | Scope | Request | Reply | Failures |
|---|---|---|---|---|
| `agent.list` | read | `ListAgents {}` | `Vec<AgentInfo>` | – |
| `agent.status` | read; `refresh` not an agent | `GetAgentStatus { agent?: HarnessRef, profile?: ProfileName, refresh: bool }` | `Vec<AgentReadiness>` | `not_found` (no such agent or profile), `invalid_request` (a profile of another agent; `refresh` without `agent`), `forbidden` (`refresh` from an agent) |
| `agent.add` | admin, not an agent | `AddAgent { name: AgentName, agent: AgentToAdd, replace: bool }` | `AgentInfo` | `conflict` (the name is taken, ignoring case, and `replace` is false; a profile has it), `not_found` (the registry entry is not in the node's copy at that version), `invalid_request` (a relative program with a separator, an empty one, a variable twice or both set and inherited, past a bound; an MCP server the fleet does not define now is not refused: the agent gets it once the fleet defines it, as a profile's servers follow the fleet), `internal` (the file cannot be written; the current file does not parse) |
| `agent.remove` | admin, not an agent | `RemoveAgent { name: AgentName }` | `Ack` | `not_found`; `conflict` while a profile of `profiles.toml` extends it (the file would no longer read) |
| `agent.catalogue` | read; `refresh` not an agent | `GetCatalogue { refresh: bool }` | `Catalogue` | `forbidden` (`refresh` from an agent: a fetch is a person's ask), `unavailable` (the registry could not be reached, answered late or too much, or was not the registry's shape; the previous copy stays) |
| `agent.install` | admin, not an agent | `InstallAgent { agent: HarnessKind, part: InstallPart }` | `InstallJob` | `invalid_request` (that agent has no such part), `harness_not_installed` (the installer's own program, `npm`, is missing), `conflict` (an install is running, or the agent has live sessions) |
| `agent.install_output` | admin | `ReadInstall { job: InstallId, from: u64, wait?: u32 }` | `InstallOutput` | `not_found` (another install replaced it, or the node restarted) |

- `agent.list` starts no process: the four, then the added agents by
  name.
- `agent.status` without `agent` answers for every agent's default
  profile from what the node knows, starting nothing but the lookup and
  `--version` of the four (as `harness.list` does). With `agent` it
  answers for its `profile` (its default when absent), and with
  `refresh` it also asks: Claude Code's and Codex's status commands
  (`claude auth status`, `codex login status`) under the profile's
  environment, their output read for the verdict only (Claude Code's
  JSON `loggedIn`, Codex's words), never logged, kept or sent, and not
  while a session of that profile runs (`unknown { in_use }`: a second
  process refreshing the same credentials has signed people out before);
  for Gemini CLI, OpenCode and added agents a probe (ADR 0022's, sharing
  its cache and its ten minutes): `auth_required` (ACP's `-32000` to
  `session/new`) is `signed_in: no` with the methods the agent listed,
  a completed handshake is `unknown { not_said }`. A routed profile is
  `not_required`.
- `agent.add` checks the agent, writes it, reads both files again, and
  answers with what is now in effect. A registry entry is taken from the
  node's own copy (it must be there, at that version, with a command);
  its definition gets the entry's `env` as text and `source = {
  registry, version }`. Running sessions keep what they started with;
  `replace` changes what the next session starts, and drops the probes
  kept for that agent. Nothing is started or fetched.
- `agent.remove` leaves running sessions running; an ended session of
  it cannot be started again (`session.resume` answers that the agent is
  not defined here).
- `agent.catalogue` without `refresh` answers the copy (empty, with no
  `fetched_at`, before the first fetch); with it, fetches first, unless
  the copy is younger than a minute. Nothing is run or installed by
  listing.
- `agent.install` runs the vendor's documented command for that agent
  or its adapter (the one `vessel init` offers, shown in `agent.list`),
  found through the node's search path, as the node's user, in the
  user's home, with the environment an added agent gets plus `npm_config_*`
  (so an install script in the package's dependencies sees no provider
  key), standard input closed. On Windows `npm` is `npm.cmd` and runs
  through `cmd.exe`; its arguments are Vessel's constants. While it runs,
  `session.create`, `session.resume` and `harness.options` for that agent
  answer `conflict`. Its output is read with `agent.install_output`;
  `wait` holds the read (30 s at most, inside the node's wait shares)
  until there is more or the job ends.

Forwarding: the bridge passes all seven (the node checks scopes);
through the fleet's envelope only `agent.list` and `agent.status`
(without `refresh`) pass, as `harness.list` does; the rest are a node's
own administration.

The CLI: `vessel agents` (each agent and what is known of it),
`vessel agents status <agent> [--profile p] [--refresh]`,
`vessel agents add <name> [--label l] [--env K=V] [--secret K=NAME]
[--inherit K] [--mcp NAME] [--replace] -- <program> [args...]`,
`vessel agents add <name> --from-registry <id> [--yes]` (shows the
command the node will write, asks), `vessel agents remove <name>`,
`vessel agents catalogue [--refresh]`,
`vessel agents install <agent> [--adapter] [--yes]` (shows the command,
asks, prints the output as it runs). `vessel run`, `vessel profiles
--harness` and the agent tool that starts a session take an added
agent's name too. No new agent tool: adding, removing, installing and
checking are a person's.

### Older clients and older nodes

- **A client of protocol 2 to 10 is never handed an added agent.**
  `harness.list`, `profile.list`, `profile.reload` and `session.list`
  leave them out, in replies nested in the fleet's envelope too. A reply
  that is about one (`session.get`, `session.create`,
  `session.move_profile`, `session.rename`, `session.resume`,
  `session.mark`, `session.subscribe`, `session.import`'s session)
  becomes `unsupported` ("this session runs an agent added on this
  machine; a client of protocol 11 or later sees it"); inside an
  envelope, the whole envelope does. The node records each connection's
  protocol, and refuses, with the same error and before they run, the
  calls that act on such a session (the guard's ownership targets:
  send, kill, move, export, options, rename, prompt, queue, resume, mark,
  delete, upload) and `session.subscribe`, so nothing happens that the
  client could not see. `session.read` of one still answers: events name
  no agent.
- **Through the fleet**, a delivered call carries no client protocol: a
  member answers its home as a protocol 11 peer, and the home withholds
  for its own older client. A protocol 10 client that names an added
  agent's session by its id (from another session's `created_by`) can
  act on it through a home; recorded in the review below.
- **A newer client asks an older node nothing it cannot read.**
  `Call::since` makes a call naming an added agent (`session.create`,
  `harness.options`, `profile.list` filtered by one) need 11; the seven
  methods are 11.
- **No event is added.** An install's output is read by its call; an
  added agent's session logs what any structured session logs.

### Bounded, and by how much

| What | Bound | Beyond it |
|---|---|---|
| The registry's fetch | https only, no redirect, no proxy; 10 s to connect, 30 s in all; 2 MiB | `unavailable`; the previous copy stays |
| Fetches | one at a time; a `refresh` within a minute of the last fetch answers the copy | the copy, with its time |
| Catalogue entries | 1,000; each string bounded (id 128, name 160, version 64, description 1,024 characters, a package 256, 64 arguments of 1 KiB, 64 variables) | the entry skipped and counted; past 1,000 not read |
| The copy | in memory until the node stops | – |
| `agents.toml` | 256 KiB; 256 agents; per agent 64 arguments of 1 KiB, 64 `env`, 64 `inherit`, 64 `mcp` | the file refused when read, naming the agent and key; `agent.add` refused |
| A reply | `agent.list` is bounded by the file (well under the 16 MiB frame); `agent.status` lists at most the agents' default profiles | – |
| From the agent | 16 sign-in methods (name 128, description 512 characters); a version of 64 characters | cut |
| Installed checks | lookup and `--version` within 5 s, 4 KiB read (as `harness.list`); kept 60 s | `version: None` |
| Sign-in checks | on `refresh` only; a status command within 10 s, 64 KiB of its output read; a probe within the handshake timeout; kept five minutes (a probe's, ten) | `unknown { timed_out }` |
| `refresh` | one agent and profile per call; at most once per 10 s each | the kept answer |
| An install | one at a time on a node; 15 minutes; 1 MiB of output kept from the start (`truncated` past it), 64 KiB a page; a waiting read 30 s | killed as a group, `timed_out` |
| Install jobs | the last one, until the next or the node stops | `not_found` |

### Tests

- The registry is never fetched by a test: its parser is tested against
  a recorded fixture in the shape GOAT's decoder and the registry's
  format give (no reference vendors a copy of the real file), and the
  fetch against a loopback HTTP server of the test's, the URL being the
  node's `Config` value (the published one otherwise): plain http is
  accepted only to a loopback address, so no test needs a certificate of
  its own (see "Track G: progress").
- Installs run nothing real: the installer's program (`npm`) is found
  through the node's `Lookup`, which tests point at a testkit fake.
- Added agents are the testkit's fake ACP agent, started by absolute
  path, with its behaviour chosen by the definition's `env` (it inherits
  nothing else, which a test checks by having the fake report the names
  of its environment).
- Status commands are fakes under the four's names, as the session tests
  already place them.

### For the page track

- An added agent is shown by the name the node gives it (its label when
  `agent.list` has one); its mark is that name's first letter.
- Terminal mode is offered only where `terminal` is absent or
  `available`.
- Two `HarnessRef`s are compared by a key (`"claude-code"`, or
  `"added:<name>"`), never as objects; an address's `?agent=` takes only
  the four.
- The page's per-agent defaults keep the four only; an added agent is
  started with the agent's own defaults.
- This track draws nothing: the page's agent lists filter added agents
  out until the page track draws them (`agent.list`, `agent.status`,
  `agent.catalogue`, `agent.add`, `agent.install` are there for it).

### Not in this track, and why

- **Downloading a registry agent's archive** (GOAT downloads it, checks
  its SHA-256 when given, extracts it, keeps it inside its folder):
  extracting archives from the network is an attack surface of its own
  (paths that leave the folder, links, sizes) that wants its own
  review. The catalogue shows such an entry with its archive, for a
  person to install and add by command.
- **Signing in from the node** (agent-orchestrator's relayed logins,
  GOAT's ACP flows). A login acts as the account, and its output carries
  one-time codes; Claude Code's `--mcp-config` takes every word after
  it (`claude --mcp-config <json> auth login --help`, run in an empty
  home, prints the top-level help), so a terminal session running
  `claude auth login` would need a launch without MCP servers, which the
  session runtime does not have. The node says how instead: the command,
  the profile's home variable by name, the agent's ACP methods. ADR 0032
  describes the next step.
- **Minimum versions of the four**: none is recorded that Vessel has
  verified. The one version Vessel checks is ACP's own.
- **Copying added agents between machines** (above).

### Store and numbers

ADR 0032. No schema (12 stays free). A new file: `agents.toml` in the
data directory.

### Review (2026-11-12)

A reviewer that did not write the contract read it against the
references, the wire and the code, and found sixteen problems. Taken:

1. Older peers: the node records each connection's protocol and refuses
   the calls that act on an added agent's session before they run; every
   reply that can carry one is listed, and a withheld reply inside an
   envelope withholds the envelope.
2. The fleet's MCP servers no longer reach an added agent unless its
   definition names them (`mcp`): a server's definition carries its
   secret values in `session/new`.
3. Signed in: a completed handshake is `unknown { not_said }`, not
   `yes`; `not_required` for routed profiles; the status commands' output
   is read as agent-orchestrator reads it (the exit status was never
   checked), run only on an explicit `refresh` of one agent and profile,
   never while a session of that profile runs.
4. `refresh` names one agent and is refused to agent principals.
5. Probes get typed outcomes (`auth_required` with the methods, the
   agent's version and ACP version); a sign-in refusal is kept like a
   success; an added agent is never run with `--version`; replacing an
   agent drops its probes; the windows are the cache's real ones.
6. A registry entry is added by id and version and built by the node
   from its own copy; entries not pinned to an exact version are skipped
   and counted; entries that are one of the four are marked so.
7. The environment list grows (XDG, macOS, Windows, proxies without
   credentials); the keyring's bus and the SSH agent's socket stay out,
   by name, with the reason.
8. Bounds that fit a frame (`agents.toml` 256 KiB), bounds on what the
   agent sends, and no stderr in a status (a step instead).
9. Installs: the clean environment, the home as working directory,
   sessions and probes of that agent held while one runs, Windows'
   `npm.cmd` said, and the output in `worktree.output`'s shape.
10. A page section; both forwarding lists decided.
11. A tests section; the fetch's URL and roots are `Config` values.
12. Every wire name and alias of the four is reserved; agent and profile
    names are unique ignoring case across both.
13. `TransferDetail` described as it is; export and import of an added
    agent's session refused.
14. `agents.toml` refused when others can write it or another user owns
    it; adds start from the file as it is.
15. Windows `.cmd` programs, lookups at each start, and what "found"
    means for a package runner, said.
16. Types: `AgentProgram`, `Sha256Hex`, `AgentVersion`; ACP's terminal
    sign-in dropped (Vessel does not advertise it); `TooNew`; the alias
    classes' real names; `AgentReadiness` (the page has an `AgentStatus`
    of its own); the fleet reason argued from what a definition names;
    added agents are never map keys.

Not taken, and why:

- Carrying the client's protocol in a delivered call (`Origin`), so a
  member could refuse an older client's act on an added agent's session
  through the fleet. It changes the fleet's envelope for a case that
  needs an older client to know such a session's id, and what it can do
  then is what its person asked of that session by id; the home still
  withholds every reply that would carry it.
- `SignInHelp.home` with the variable's value: a profile's values never
  leave the node (ADR 0011), and the person who signs in is the one who
  can read `profiles.toml`.
- Reusing `vessel-llm`'s client for the registry: it is built for model
  calls (endpoints, keys, retries, redaction); the fetch is one bounded
  GET with the same TLS settings, written in the node's registry module.

## Track G: progress

Built in slices on `backend/m11-agents`, each commit passing the whole
gate (fmt, clippy with and without `ts`, every Rust test, the page's
lint, typecheck, unit tests and build, and its end-to-end run against
binaries built from the tree).

### Decided while building

- **An MCP server a definition names that the fleet does not define now
  is not refused** (the contract's first text refused it): the agent gets
  it once the fleet defines it, as a profile's servers follow the fleet,
  and a node with no fleet would otherwise refuse every name.
- **A secret an added agent names that the node lacks** makes
  `session.create`, `session.resume` and `harness.options` a `conflict`
  naming it (`vessel fleet secret set NAME`): it would start without
  what its definition says it needs. A secret set by `secret.set` is
  taken up at the fleet's next look at `secrets.toml` (5 s by default).
- **`vessel run <added agent>`** starts a conversation without
  `--structured`, and refuses arguments after `--` (they are the
  definition's).
- **An added agent's memory** is not exported (`memory.export` answers
  `unsupported`), as its conversation is not moved or copied.
- **`vessel init`** reads `agents.toml` with `profiles.toml`, so a profile
  extending an added agent does not stop it; it checks the sign-in of the
  four only.
- **The registry's fetch** accepts plain http to a loopback address
  besides https: a test serves its fixture that way, so no test needs a
  TLS server or a certificate of its own.
- **`AuthMethodId` is a handle**, not a plain name: the agent names it
  and a client only hands it back, as a question's `FieldName`.
- **A catalogue entry's variables** are `{ name, value }` objects
  (`CatalogueVariable`), not pairs, as the rest of the wire.
- **`in_use` applies to status commands**, as the contract says; a
  probe's handshake (the others, and added agents) is the one
  `harness.options` runs anyway.
- **Another ACP version** (`too_old`, `too_new`) is tested at the node's
  reading of a refused handshake, not end to end: the ACP crate's agent
  side, which the testkit's fake uses, always answers version 1.
- **An install runs its program directly**: `npm` as the node's search
  path finds it (`npm.cmd` on Windows, which the standard library runs
  through `cmd.exe` itself, its arguments being Vessel's constants).
- **An install's waiting read** is held 30 s at most, but is not counted
  in the node's wait shares (`waits`): there is one install at a time and
  only `admin` reads it. Recorded here as a difference from the contract.
- **Not tested end to end**: that `session.create`, `session.resume` and
  `harness.options` answer `conflict` while an install of that agent
  runs (the fake installer ends at once); the three calls share one
  check (`refuse_installing`).

### Slices

1. Protocol 11 names agents beyond the four (`HarnessRef`, what older
   peers are handed), no agent addable yet.
2. Agents added by their command: `agents.toml`, `agent.list`,
   `agent.add`, `agent.remove`, the clean environment, the launcher's
   secrets and MCP servers, `vessel agents`, `vessel run <name>`.
3. `agent.status` (status commands, a handshake's `auth_required`, typed
   facts kept per agent and profile) and `agent.catalogue` (the
   registry's copy, an entry added from it).
4. `agent.install` and `agent.install_output`, and `vessel agents
   install`.

### Not offered, because an added agent cannot do it

An added agent runs conversations only: no terminal (Vessel knows its ACP
command, not a terminal of its own); its conversation cannot move to
another profile or be exported or imported, and its memory is not
exported (Vessel does not know where it keeps either); fleet sync writes
no instructions or skills into its home, and no model route is applied to
it (both need knowledge of the agent's own configuration); Vessel does
not install it (a person installs it, or adds a registry entry whose
runner fetches it) and does not sign it in.

## Track D: contract

Protocol 11, this track's part of it (track G adds its own; "Wave 2:
tracks" says the number is raised once). Written 2026-11-12 from the
three references' source (agent-orchestrator's `service/automation`,
`observe/automations` and `service/cue`; GOAT's
`apps/server/src/scheduledTasks` and `mcp/launchWatch.ts`; Paseo's
`server/schedule` and `agent/finish-subscriptions`) and from Vessel's wire
as joined in wave 1, then revised after an independent review (the last
part of this section). ADRs: 0033 (schedules, time, who a run acts as,
the starter told), 0034 (webhooks). Store: schema 12 (13 on the track's
branch; "Integration 3 (node)").

### What the references do (read 2026-11-12)

| | agent-orchestrator | GOAT | Paseo |
| --- | --- | --- | --- |
| Cadence | RRULE, or a cron narrowed to daily and weekly times, translated to RRULE (`rrule-go`); a time zone required | an interval (at least 60 s), a time of day with weekdays in the server's own zone, or a webhook | five-field cron written by hand, a time zone (UTC when left out); an interval kept for older files |
| How it wakes | a 15 s ticker | a 5 s loop with a due hint | a 1 s `setInterval` reading every schedule's file |
| Missed while down | up to 3 runs, the oldest three, the newer dropped | one run; a time of day more than 10 min late is skipped | skipped at start; one run when a tick finds one overdue |
| A run still going when due again | the next waits behind it: one live session per automation | skipped | skipped; a busy target agent fails the run |
| A new session after its run | left running (it blocks the next run) | left | archived (`archiveOnFinish`, on by default) |
| Nobody to answer | not handled | not handled | runs are unattended; a run waiting on a permission fails |
| Count, expiry | none (`COUNT`, `UNTIL` refused) | none | `maxRuns`, `expiresAt` |
| Run now | none | yes | yes |
| History | every run, unpruned | the last firing | every run, unbounded |
| Target | a new session in a project | a new thread, or a thread it posts into (queued) | a new agent, or an existing one (a heartbeat is a schedule targeting the caller) |
| Who it acts as | nothing stored | the creator and a ceiling stored | nothing stored; a permission to manage |
| Starter told | not found | launch-watch: queued after the active run, once per run, a reverse watch's own run ignored, 7-day expiry | finish subscriptions per prompt: steered into the active turn, batched 2 s, stored, across hosts; no loop guard |
| Saved prompts | Cues: per project, in its database, 16 KiB, sent to a session or a new one | none | none |
| Webhooks in | none | a token in the path, an optional HMAC over the body (no signed time), filters, dedupe by template, 60 a minute, a delivery log that keeps bodies | none |

### Types (`vessel_proto::automation`, new module)

Ids (`id.rs`, node-minted UUIDv4): `ScheduleId`, `SavedPromptId`,
`HookId`. `RunNumber`: a schedule's run, counted from 1.

Bounded values, parsed where a frame is decoded:

- `ScheduleName` (a schedule's or a hook's): trimmed, 1 to 80
  characters, no control character.
- `PromptName`: trimmed, 1 to 64 characters, no control character.
- `PromptText`: 1 to 16 KiB of UTF-8, not blank, no control character but
  tab, line feed and carriage return (so its JSON is at most twice its
  size). A schedule's, a saved prompt's and a hook's text.
- `TimeZoneName`: an IANA name's shape: 1 to 64 bytes of ASCII letters,
  digits and `/ _ - +`, no empty part. Whether the zone exists is the
  node's to say (`invalid_request`, "unknown time zone").
- `CronExpr`: five fields, minute (0-59), hour (0-23), day of the month
  (1-31), month (1-12 or `JAN`-`DEC`), day of the week (0-7, 0 and 7
  Sunday, or `SUN`-`SAT`); each a list of `*`, `n`, `a-b`, `*/s` or
  `a-b/s` (no `a/s`, which crons read differently; no wrapping range); or
  one of `@hourly`, `@daily`, `@midnight`, `@weekly`, `@monthly`,
  `@yearly`, `@annually`. No seconds, no `L`, `W`, `#` or `?`. The days
  follow Vixie cron: when both day fields are restricted (neither starts
  with `*`), a day matching either runs; otherwise both must match. At
  most 256 bytes; kept as written, with runs of white space made one.
- `EverySeconds`: 60 to 31,622,400 (366 days). `MaxRuns`: 1 to 1,000,000.
- `DeliveryKey`: a hook request's `webhook-id`, 1 to 128 visible ASCII
  characters.

Every enum a newer node may grow has an `unknown` variant a client shows
as such (protocol 9's rule); one sent to a node is `invalid_request`.

A schedule:

- `Cadence` (tagged `kind`): `every { seconds, anchor? }` (elapsed time:
  due at `anchor + k × seconds`, `k ≥ 0`, always after the cadence was
  set; the node fills `anchor` with that moment when it is left out, so
  the phase survives restarts and a resume keeps it; a day is 86,400 s
  whatever the wall clock does), `cron { expr, zone }` (wall-clock times
  in `zone`), `unknown`.
- `ScheduleTarget` (tagged `kind`):
  - `new_session { spec, after }`: a structured `SessionSpec`, as
    `session.create` takes it (harness, profile, title, place, model and
    options; terminal, or with `resume`, is `invalid_request`), and
    `AfterRun`: `archive` (the default: once its first turn ends and
    nothing waits in its queue, the session is archived, ADR 0026: it
    stops, its log stays and it can start again; Paseo's
    `archiveOnFinish`) or `keep` (it stays live, and the schedule's next
    run waits for it to end: agent-orchestrator's one live session per
    automation); `unknown`.
  - `session { session }`: an existing structured session, reached
    through its queue.
  - `unknown`.
- `CatchUp`: `once` (the default), `skip`, `unknown`.
- `ScheduleDefinition { name?, cadence, target, prompt, catch_up,
  max_runs?, ends_at? }`: what a person writes; `schedule.create` takes
  it, `schedule.update` replaces it whole.
- `ScheduleState` (tagged `state`): `active { next_due }`, `paused {
  since }`, `ended { at, reason }`, `unknown`; `EndReason`: `max_runs`,
  `expired`, `creator_gone` (the device that defined it was revoked),
  `target_gone` (the session, project or worktree it names was deleted),
  `unknown`.
- `ScheduleInfo { id, definition, state, runs, last_due?, last_run?,
  defined_by, created_at, updated_at, revision }`: `runs` counts the runs
  that sent their prompt, which `max_runs` bounds; `last_due` the due
  time the last run was for; `defined_by` who last wrote the definition
  (ADR 0013's `Creator`), whom a run acts as; `revision` changes when the
  definition or a person's pause or resume changes it, not at a run.

A run:

- `ScheduleRun { number, due_at, at, trigger, outcome }`; `due_at` the
  due time it ran for (the latest, after a catch-up; the moment asked,
  for a run now).
- `RunTrigger` (tagged `trigger`): `due`, `caught_up { missed }` (`missed`
  the due times it stood for, counted up to 1,000), `asked { by }`,
  `unknown`.
- `RunOutcome` (tagged `outcome`):
  - `making`: recorded before the run is made, so a node stopped while
    making it never makes it again (`interrupted` at the next start).
  - `started { session }`: a new session, its prompt sent; becomes
    `waiting { session }` while its first turn waits on a permission
    (nobody is asked a question: below) and `finished { session, end }`
    when that turn ends or the session does; `RunEnd` (tagged `end`):
    `turn { reason }` (a `StopReason`), `failed { message }`, `session {
    status }`, `unknown`.
  - `prompted { session, delivered }`: the prompt went into an existing
    session's queue (`Delivered`: `started` or `queued { entry }`).
  - `skipped { reason }`, `SkipReason` (tagged `reason`):
    `previous_running` (the previous run is being made, or its session is
    still live under `keep`, or still in its first turn under
    `archive`), `previous_waiting` (the previous run's prompt still waits
    in the queue), `stopped` (the session's last turn was stopped and
    nothing was sent since: the board's `interrupted`), `missed { count
    }` (due times passed while the node could not run them, not caught
    up), `unknown`.
  - `failed { error, session? }`: the call the run made, refused;
    `RunError { code, message }`, `code` the `ErrorCode` it answered (a
    `RunErrorCode` with every `ErrorCode` and `unknown`, so an older
    client still reads a list holding a newer code), the message cut at
    512 bytes.
  - `interrupted`: the node stopped while it was being made.
  - `unknown`.

Saved prompts:

- `SavedPrompt { id, name, text, project?, session?, defined_by,
  created_at, updated_at, revision }`: `project` absent is the machine's;
  `session` a structured `SessionSpec` (what a session started from it
  runs with; its place, if any, in the prompt's project). A name is
  unique within its project (or the machine's).

The starter told (on Track A's request, the one shared addition):

- `PromptSession.notify: bool` (protocol 11, left out when false).

Webhooks (ADR 0034):

- `HookDefinition { name?, target, prompt, enabled }` (`target` a
  `ScheduleTarget`).
- `HookInfo { id, definition, path, defined_by, created_at, revision }`:
  `path` is `/hooks/<id>` on the node's hook listener.
- `HookSecret`: 32 random bytes, written `whsec_<base64>` (Standard
  Webhooks' form: the key is the decoded bytes); a secret type
  (redacted in `Debug`), returned once.
- `HookCreated { hook, secret }`.
- `HookListener` (tagged `state`): `off`, `listening { address }`,
  `failed { message }`, `unknown`.
- `HookDelivery { at, key?, bytes, outcome }`, `DeliveryOutcome` (tagged
  `outcome`): `ran { run }` (the run's `RunOutcome`, followed as a
  schedule's run is), `duplicate`, `busy` (at its bound of live sessions or the
  target's queue is full), `rate_limited`, `rejected { why }` (`why`:
  `signature`, `stale`, `too_large`, `unknown`), `unknown`. No body and
  no header is kept.

### Calls

Every method is protocol 11 and in `vessel-web`'s list. All are
forwardable through `fleet.forward`; the rules under "Who may" say what a
delegated caller may do.

| Method | Scope | Request | Reply |
| --- | --- | --- | --- |
| `schedule.list` | read | `ListSchedules {}` | `ScheduleList { schedules }` |
| `schedule.get` | read | `GetSchedule { schedule }` | `ScheduleInfo` |
| `schedule.create` | operate | `CreateSchedule { definition, paused }` | `ScheduleInfo` |
| `schedule.update` | operate | `UpdateSchedule { schedule, revision, definition }` | `ScheduleInfo` |
| `schedule.pause` | operate | `PauseSchedule { schedule }` | `ScheduleInfo` |
| `schedule.resume` | operate | `ResumeSchedule { schedule }` | `ScheduleInfo` |
| `schedule.run` | operate | `RunSchedule { schedule }` | `ScheduleRun` |
| `schedule.delete` | operate | `DeleteSchedule { schedule }` | `Ack` |
| `schedule.runs` | read | `ListRuns { schedule }` | `RunList { runs }` |
| `prompt.list` | read | `ListPrompts { project? }` | `PromptList { prompts }` |
| `prompt.create` | operate | `CreatePrompt { name, text, project?, session? }` | `SavedPrompt` |
| `prompt.update` | operate | `UpdatePrompt { prompt, revision, name, text, session? }` | `SavedPrompt` |
| `prompt.delete` | operate | `DeletePrompt { prompt }` | `Ack` |
| `hook.list` | read | `ListHooks {}` | `HookList { listener, hooks }` |
| `hook.create` | admin | `CreateHook { definition }` | `HookCreated` |
| `hook.update` | admin | `UpdateHook { hook, revision, definition }` | `HookInfo` |
| `hook.rotate` | admin | `RotateHook { hook }` | `HookCreated` |
| `hook.delete` | admin | `DeleteHook { hook }` | `Ack` |
| `hook.deliveries` | read | `ListDeliveries { hook }` | `DeliveryList { deliveries }` |

`not_found` is every call's answer for an unknown schedule, saved prompt
or hook.

**`schedule.create`**: the node checks the zone exists, that a cron has a
due time within the next 8 years (`0 0 30 2 *` never does), that
`ends_at` is after the first due time, and the target as it would at a
run: a `new_session` spec is structured without `resume`, its profile
and its place exist; a `session` target exists and is structured.
`paused` makes it paused from the start. Failures: `invalid_request` (the
above; an `unknown` anywhere), `not_found` (a profile, project, worktree
or session named), `unsupported` (a terminal session as the target),
`forbidden` (below), `conflict` (the node's bound of schedules).

**`schedule.update`**: replaces the definition and makes the caller its
`defined_by`; `conflict` when `revision` is not the current one or the
schedule has ended; `create`'s checks. A changed cadence computes the
next due time from the moment of the update (a passed due time is not
run: agent-orchestrator, Paseo); anything else keeps it. A run already
being made finishes with what it started with. `runs` is kept, so
lowering `max_runs` to or below it ends the schedule.

**`schedule.pause`** stops its runs; **`schedule.resume`** computes the
next due time from now (an `every` keeps its anchor's phase; a paused
stretch is not caught up, as in all three). Both idempotent; `conflict`
for an ended schedule.

**`schedule.run`** runs it once now, outside its cadence, as its
`defined_by` (below), and answers once the run has its outcome
(`started`, `prompted`, `skipped`, `failed`); a run waits at most 30 s
for one of the node's run slots (`unavailable` after). Its next due time
does not move; it counts toward `max_runs` if it sent its prompt.
Allowed while paused; `conflict` while one of its runs is being made and
for an ended schedule; `forbidden` to agents, who would otherwise pump
their own heartbeat past its cadence.

**`schedule.delete`**: gone with its runs; a session it started is not
touched. **`schedule.runs`**: the newest 50, newest first.

**`prompt.create`** / **`prompt.update`**: `conflict` for a name its
project (or the machine) already has, an old `revision`, or 256 saved
prompts; `not_found` for an unknown project; `invalid_request` for a
terminal spec, one with `resume`, or one whose place is in another
project. `update` replaces name, text and `session` (absent clears it);
the project is fixed at creation. **`prompt.list`**: the machine's and
every project's, or with `project` that project's and the machine's
(`not_found` for an unknown project), by name. **`prompt.delete`**: gone.
Removing a project does not remove its prompts (they name a project that
is gone; a person deletes them).

**`session.prompt { notify: true }`** (the starter told; ADR 0033):

- Only a session's agent asks for it: an agent of this node, through its
  credential (`invalid_request` for the owner and devices, who are told
  by the page; `forbidden` for an agent of another node, whose session
  this node cannot reach). The session told is always the caller's own,
  which must be structured (`unsupported`). The prompted session is one
  the caller may operate (Track A's ownership check, as before).
- It is about this prompt's turn, exactly: the session's actor says
  which turn the prompt started (at once for `started` and `steered`,
  when its entry leaves the queue for `queued`; never, if the entry is
  removed). Then it tells, each as a prompt queued into the caller's
  session (`Delivery::Queue`: never steered into a running turn, never
  interrupting one), once per event:
  - `needs_you`, for each permission or question that turn asks, at most
    8;
  - then `finished` (that turn's end or failure) or `ended` (the
    session's final status), after which nothing more.
- Notices only go from a session to one it descends from, and the
  creation tree has no cycle, so notices alone never wake two sessions
  in turn; a loop needs an agent to prompt again each time, as any agent
  loop does. Bounded besides: 32 waiting per told session, 256 on a node
  (`conflict` beyond), and at most 60 notices an hour into one session
  (the rest dropped and logged on the node).
- A notice is not sent into a session a person stopped (its board says
  `interrupted`: its last turn was stopped and nothing was sent since),
  nor into one whose queue is full: it is dropped and logged on the node.
- The text, from the node, bounded:
  `vessel: session "<title>" (<id>) finished its turn (end_turn).` and
  the last 1,500 characters of that turn's reply, then how to read the
  rest (`read_events` from the turn's first record); `... is waiting for
  an answer: <the permission's title or the question's message>`;
  `... ended (<status>).` Nothing is stored: no session outlives the
  node.

### Who a run acts as, and who may change what

- A run acts as its schedule's `defined_by` **as it is when it runs**:
  the owner; a device with its scopes now (revoked: the schedule ends
  `creator_gone`); an agent whose session is still live, with its grant.
  It goes through the checks a call of theirs would: the scope
  (`operate`), ADR 0013's ownership and limits (8 live sessions, depth 3,
  scopes that only shrink), ADR 0027's place rules. A refused run is
  `failed`; the schedule goes on.
- Whoever updates a schedule becomes its `defined_by`: a device that
  rewrites the owner's schedule makes it act as that device from then
  on. Pausing, resuming, running now and deleting do not change it.
- A scheduled or hook session's agent is never offered questions (nobody
  is there to answer; ADR 0013's "unwatched", as for an agent's
  sessions); a permission its first turn asks makes the run `waiting`
  until a person answers or archives it, and the schedule's later runs
  are `skipped { previous_running }` meanwhile (no timeout: stopping an
  agent mid-work on a timer loses its work; a person who wants nothing
  asked chooses a mode that does not ask).
- Agents: at most 8 schedules per agent session; a `new_session` target
  (its child) or a `session` target it owns, or its own session (a
  heartbeat: the one exception to "an agent does not own its own
  session", for a prompt through its own queue). Every schedule an agent
  made is deleted, with its runs, when its session ends, and at the
  node's start (no session survives a restart): a session cannot use a
  schedule to outlive or outgrow what it was given. Agents may manage
  (update, pause, resume, delete) only their own schedules, never run one
  now, and may list saved prompts but not write them (a saved prompt is
  a person's text, later sent by a person who may not read it again).
- A delegated caller (`fleet.forward`) may read, pause, resume and delete
  schedules and read hooks, but not create or update a schedule, run one
  now, write a saved prompt or touch a hook (`forbidden`), nor ask for
  `notify`: what it is allowed depends on the home's grant and on an
  agent on another node, which this node cannot rebuild when the run
  comes. The page and the CLI reach each node directly.

### How a run goes

1. Due, or asked. A schedule has one run at a time: a due time while its
   run is being made is `skipped { previous_running }`. The node records
   the run as `making` together with the schedule's next due time, in one
   write, before anything else.
2. The definer is rebuilt (above). A `session` target that was deleted,
   or a `new_session` spec whose project or worktree was removed, ends the
   schedule `target_gone`.
3. Overlap: `new_session` under `keep`, its previous run's session still
   live, or under `archive`, still in its first turn: `skipped {
   previous_running }`. `session`: the previous prompt still waiting in
   that queue, `skipped { previous_waiting }`; the session stopped by a
   person (its board `interrupted`), `skipped { stopped }`: nothing
   automatic starts a session a person stopped (ADR 0024's rule, kept for
   prompts that arrive after the stop).
4. `new_session`: `session.create`'s path with the spec, as the definer,
   with no questions offered; if the spec has no title the session takes
   the schedule's name, else its prompt's first line; its log starts with
   `vessel: started by schedule "<name>" (run <n>)`. The prompt goes once
   the agent is ready. `started`, then followed: `waiting` while a
   permission waits, `finished` at the end of its first turn; under
   `archive`, archived then if nothing waits in its queue.
5. `session`: `session.prompt { delivery: queue }` as the definer: a turn
   starts on an idle session with nothing waiting; otherwise it waits its
   turn (held if the queue is held). It never steers, interrupts or
   answers a permission. `prompted`. An ended target session is `failed
   { conflict }` (it may be started again).
6. The run's slot (one of 4) is freed as soon as it has its outcome
   (`started`, `prompted`, `skipped`, `failed`), not when it finishes.
   `runs` counts it if it sent its prompt; at `max_runs` the schedule
   ends; a next due time past `ends_at` ends it `expired`.
7. At the node's start: a `making` run becomes `interrupted` (never made
   again); a `started` or `waiting` run whose session is no longer live
   becomes `finished { end: session }`; every agent's schedule is
   deleted.

### Time (ADR 0033)

- **Zones**: the IANA database bundled with the node (`jiff`), the same
  on every platform and in every test.
- **A wall-clock time that does not exist** (clocks go forward over it)
  runs once, at the instant of the jump: `30 2 * * *` in New York on the
  spring night runs at 03:00 EDT; several times in one gap, and 03:00
  itself, make one run. **A time that happens twice** (clocks go back)
  runs once, the first time, for every expression (Cronie runs a
  `*`-hour expression in both passes; here an expression is about the
  wall clock, and `every` is for elapsed rhythm). `every` knows no zone.
- **Due times are instants.** A schedule keeps its next due time and the
  due time of its last run. The clock stepped back under 24 hours never
  runs a due time twice (the next waits for the clock to reach it); a
  last due time more than 24 hours ahead of the clock means the clock
  was wrong when it ran, and the next due time is computed from now. The
  clock stepped forward over due times counts them as missed.
- **Missed** (the node stopped, the machine slept, the clock jumped): one
  run at most per schedule, however many. With `once`, if the latest
  missed time is less than 24 hours old, one run (`caught_up { missed
  }`); otherwise, or with `skip`, one record `skipped { missed }`. A due
  time noticed less than 60 s late is simply `due`. Lateness is judged
  when the scheduler picks the due time, not when a run slot frees. The
  next due time is then the first after now; `ends_at` bounds catch-ups
  as it bounds the rest.
- **Two due at once** both run, in due order, then by id. At most 4 runs
  are made at once on a node; the rest wait, oldest due first, one per
  schedule.
- **The scheduler sleeps** until the earliest due time, at most 10
  minutes at a time (a clock stepped forward is noticed within 10
  minutes; a suspended machine's monotonic clock may stop too), and is
  woken by every change. It keeps due times in an ordered index: a wake
  looks at the first entry, never at every schedule. With no active
  schedule it has no timer at all.

### Webhooks (ADR 0034)

- **Off unless configured**: the node opens a hook listener only when
  started with one (`vesseld --hooks [ADDR]`, `vessel node start --hooks
  [ADDR]`), on `127.0.0.1:7080` (7078 is the relay's: corrected in ADR
  0034) unless an address is given; another
  address logs a warning, as `--bind` does. Hooks can be made without it;
  `hook.list` says the listener is `off`.
- **A request** follows Standard Webhooks: `POST /hooks/<id>` with
  `webhook-id` (the `DeliveryKey`), `webhook-timestamp` (Unix seconds) and
  `webhook-signature: v1,<base64>` (one or more, space-separated): HMAC-
  SHA256 under the secret's decoded bytes over `<id>.<timestamp>.<body>`.
  The id is signed, so a captured request cannot be sent again under a
  new id.
- **Checked cheapest first**: the method (`405`); the path and the hook,
  looked up in memory (unknown or disabled: `404`, the same answer;
  unknown ids counted in one bucket, never one per id); the headers;
  `Content-Length` and then a capped read (64 KiB: `413`), with 10 s to
  send the headers and 10 s for the body, and at most 32 connections at
  once; the timestamp (within 5 minutes: else `401`); the signature in
  constant time (`401`); the id seen in the last 24 hours (at most the
  last 1,024 per hook: `200` with nothing run); the accepted budget (30
  a minute per hook: `429`); the target's room (`429` with
  `retry-after`, the id not remembered: a `new_session` hook has at most
  4 live sessions, a `session` hook's queue may be full). Then `202`: the
  run is made after the answer, so a sender's 10 s timeout is never the
  agent's handshake. A run whose call is refused forgets the id, so the
  sender's retry can run.
- **Rejected requests** cost a map lookup and an HMAC at most; they are
  logged at most 10 a minute per hook and never stop a correctly signed
  one.
- **The secret** is made by the node, returned by `hook.create` and
  `hook.rotate` only, kept in an owner-only file under the node's data
  folder (never in the database, never logged), compared in constant
  time; rotating refuses the old one at once.
- **What reaches the agent**: the hook's prompt, then a `vessel:` line
  saying the request came from outside this machine and is data, not
  instructions, then the body (UTF-8, cut at 16 KiB; otherwise only its
  size) between `<vessel-hook-data-<random>>` tags whose random part the
  sender cannot know, so the body cannot close them. That fence is advice
  to the model, not a control, so the session's agent gets `read` only
  (it can start no session and make no schedule through Vessel) and no
  questions; the permission mode is the hook's definer's choice, and
  ADR 0034 says what a mode that does not ask means here.
- **Not built** (ADR 0034 says why): GitHub's, Stripe's and Slack's own
  signature schemes (a relay can turn one into Standard Webhooks), GOAT's
  filters, coalescing and prompt templates over the body's fields.
- If ADR 0034 finds a part of this cannot be made safe in the track, the
  listener is not built and the ADR says what is missing.

### Events

None. A scheduled prompt and a notice are prompts: they appear as
`queue_changed` and `turn_started`, as any prompt does. A queue entry's
`by` is the schedule's or hook's `defined_by`, or for a notice the
session that finished (`Creator::Agent` of that session: what the notice
is about). A page reads `schedule.list`
and `schedule.runs` again when it wants the newest (no subscription in
this track).

### Old clients, old nodes

- `PROTOCOL_VERSION` is 11; nothing else on that line changes. A client
  asks no protocol 11 method of a node that speaks 10 or less
  (`NotInProtocol`, before sending), and a node refuses one on an older
  connection. A `session.prompt` with `notify` needs 11 (`Call::since`):
  an older node would drop the field and never tell.
- A protocol 2-10 client reading a protocol 11 node sees a scheduled
  session as a session its definer made, a scheduled prompt and a notice
  as prompts in the queue. No event or reply it reads gains anything, so
  no stand-in is needed.

### Bounds

| What | Bound | Beyond it |
| --- | --- | --- |
| Schedules people made (any state) | 256 per node | `conflict` |
| Schedules agents made | 8 per agent session, 64 per node; deleted when it ends | `forbidden`; `conflict` |
| How often | `every` at least 60 s | `invalid_request` |
| A cron with no due time | none within 8 years | `invalid_request` |
| Runs being made at once | 4 per node, one per schedule; `schedule.run` waits 30 s for a slot | wait, oldest due first; `unavailable` |
| Live sessions a schedule keeps | 1 (`keep`), none after its first turn (`archive`) | the next run `skipped` |
| Runs kept | 50 per schedule | the oldest dropped |
| Catch-up | one run per schedule, for a time missed by less than 24 h | `skipped { missed }` |
| A prompt (schedule, saved, hook) | 16 KiB, no control character but tab and line ends | `invalid_request` |
| Saved prompts | 256 per node; a name 64 characters | `conflict`; `invalid_request` |
| The starter told | 32 waiting per told session, 256 per node; 8 `needs_you` per turn; 60 notices an hour into one session | `conflict`; dropped, logged |
| A notice's excerpt | the last 1,500 characters of the reply | cut |
| A failed run's message | 512 bytes | cut |
| Hooks | 64 per node | `conflict` |
| A hook's request | 64 KiB read, 16 KiB given to the agent; 10 s for headers, 10 s for the body; 32 connections | `413`; cut; closed |
| A hook's deliveries | 30 run a minute; 4 live sessions; rejected ones logged 10 a minute; 100 kept | `429`; `429`; not logged; the oldest dropped |
| Delivery ids remembered | 24 hours, at most the last 1,024 per hook | forgotten |
| The scheduler's sleep | until the earliest due time, at most 10 minutes | wakes and looks again |

### Store and numbers

- Schema 12 (13 as given out in "Wave 2: tracks"; 10 and 11 are the
  first half's join's, and track G took none, so the join made it 12):
  `schedules` (id,
  created_at, the info as JSON), `schedule_runs` (schedule, number, the
  run as JSON; deleted with their schedule), `saved_prompts` (id, scope:
  the project or `''`, name, unique per scope), `hooks`,
  `hook_deliveries`, `hook_keys`. Hook secrets are files
  (`hooks/<id>.key`, owner-only), not rows.
- ADRs 0033 and 0034.

### The CLI and the agent tools

- `vessel schedule list|show|runs|create|edit|pause|resume|run|delete`:
  `create --every 30m | --cron "0 9 * * 1-5" [--tz ZONE]` (the zone this
  machine is in when left out), the target `--session ID` or a new
  session's `--harness`, `--profile`, `--project`/`--worktree`,
  `--model`, `--keep`, the prompt `--prompt TEXT | --prompt-file FILE`,
  and `--name`, `--max-runs`, `--ends-at`, `--catch-up`, `--paused`;
  `edit` reads the schedule, changes what it is told and sends it back
  with its revision.
- `vessel prompt list|show|save|delete|run`: `save NAME (--text | --file)
  [--project]` creates or replaces; `run NAME --session ID` queues its
  text, `run NAME --new` starts a session with its `session` (in its
  project when the spec names no place) and sends it.
- `vessel hook list|create|edit|rotate|delete|deliveries`; `create` and
  `rotate` print the secret once, to standard output only.
- Agent tools (`vessel mcp`), on the agent's own node only: `send_input`
  and `create_session`'s first prompt gain `notify` (default false: an
  agent that will call `wait_turn` does not want a second report);
  `create_schedule`, `create_heartbeat` (the target is the caller's own
  session), `list_schedules`, `schedule_runs`, `pause_schedule`,
  `resume_schedule`, `delete_schedule`; `list_saved_prompts`. No
  `run_schedule` and no hook tool (above).

### Not in this track, and why

- **RRULE** (agent-orchestrator only; ADR 0033): cron covers what its
  templates do, and `every` the intervals cron cannot say.
- **A new worktree per run** (Paseo's `isolation: worktree`): a
  `new_session` spec names a project folder or an existing worktree;
  making one per run needs worktree tidying per run. Later, as a
  `place` kind.
- **The starter told across nodes** (Paseo's remote finish
  subscriptions): the told session is on the caller's own node; another
  node cannot reach it without a link of its own.
- **Sessions shown under their starter**: the page's (D-page); the wire
  already says who started whom (`created_by`).
- **Where a queued prompt came from** beyond its `by` (a source naming the
  schedule): the run record names the session and its queue entry, which
  is enough to join them on a page; a field on Track A's `QueuedPrompt`
  would need the actor to carry it through every path that queues.

### Track D: the contract's review (2026-11-12)

An independent reviewer read the first version against the references,
the agents' wire and the node's code and found eighteen problems (four
it called blockers). The section above is the version after them; what
changed:

1. **Live sessions piled up**: a `new_session` run's session stayed live
   after its turn, one more each run. Fixed with `after` (`archive` by
   default, Paseo's; `keep` waits for the session, agent-orchestrator's)
   and hooks' bound of 4 live sessions.
2. **Nobody to answer**: a scheduled session created as the owner would
   have been asked questions at night. Scheduled and hook sessions are
   offered none; a permission makes the run `waiting`, and the schedule
   skips meanwhile.
3. **Ended agent schedules filled the bound**: agent schedules are deleted
   when they end and have a bound of their own.
4. **A watch could report the wrong turn**: `session.watch { from }`
   could not tell its prompt's turn from one already running. Replaced by
   `notify` on the prompt itself, the actor saying which turn it started.
5. **A crash could make a run twice**: a `making` record goes with the
   next due time before the run is made, and becomes `interrupted` at the
   next start.
6. **A list could pass the 16 MiB frame**: prompts take no control
   characters but tab and line ends, and saved prompts are 256.
7. **The delivery key was not signed**: Standard Webhooks' signature
   covers the id.
8. **The listener's checks were in the wrong order**: the hook is looked
   up before the body is read, rejected requests spend no budget a signed
   one needs, and timeouts and a connection bound are stated.
9. **A hook's agent could do anything its definer could**: it gets `read`
   only and no questions; ADR 0034 states the permission mode's risk.
10. **The hook protocol was underspecified**: the key's form, `202`
    before the run, the target's room, a refused run forgetting its id,
    and what is not built.
11. **A device could plant runs acting as the owner**: a run acts as the
    last definer.
12. **The loop bound was overstated**: the claim is now what holds;
    agents cannot run a schedule now; notices into one session are
    bounded by the hour.
13. **Automatic prompts could restart a session a person stopped**: they
    skip (or, notices, are dropped) while its board says `interrupted`.
14. **Time**: the fold rule is stated for every expression; a clock set
    wrong into the future is bounded (24 hours); the anchor is filled at
    creation; lateness is judged when picked; `ends_at` before the first
    due time is refused.
15. **Run slots**: freed at the outcome, one per schedule, 30 s for
    `schedule.run`.
16. **Wire**: `unknown` on every enum that grows; a run's error is a
    `RunError` whose code has `unknown`; a hook's run is its delivery's
    `ran { run }`; specs with `resume` and places in another project
    refused; `revision` does not move at a run; every failure listed.
17. **Webhooks through `fleet.forward`**: hook writes and `notify` are
    refused to delegated callers.
18. **The brief's items and the table**: the repository question is
    answered (ADR 0033: a prompt in a repository's file would be someone
    else's instructions; saved prompts are the node's); what is not built
    is listed; the table's catch-up and finish-subscription rows were
    corrected.

Not taken:

- **A `source` on `QueuedPrompt`** (finding 16): above, under "Not in
  this track".
- **Running a `*`-hour cron in both passes of a fold** (finding 14):
  Cronie does; Vessel keeps one rule for every expression (above), and
  `every` is what keeps an elapsed rhythm.
- **A timeout on a run waiting for a permission** (finding 2): stopping an
  agent on a timer loses its work; the run says it waits, and the board
  shows the session blocked.

## Track D: progress

Built on `backend/m11-schedules`, slice by slice; each commit passed the
whole gate (the end-to-end tests of the node and of the page included).

### Built

- **Slice 1, the wire's shapes and schema 13** (`vessel_proto::automation`,
  `vessel-store`'s `automation`): cron expressions parsed into bit sets,
  the bounded names and prompt text, cadences, targets, runs with typed
  outcomes, saved prompts and hooks; the tables, with a run about to be
  made written in the same commit as the next due time it took. Schema
  versions 10 to 12 were empty on this branch, for tracks B2, C and G;
  the join made this one 12 ("Integration 3 (node)").
- **Slice 2, schedules** (protocol 11's first nine methods): the time
  engine (`automation::time`: the next wall-clock minute in a zone from
  `jiff`'s bundled database, gaps and folds, missed due times), the
  scheduler (one task, an index of due times, a sleep of at most 10
  minutes and none with nothing active, four run slots, one run at a time
  per schedule), runs made as the definer through the service's checks
  (`service/scheduled.rs`), a new session's first turn followed and the
  session archived, the calls (`service/schedules.rs`), the client's
  methods. `Sessions` now tells who listens when a session's log is
  complete (`Sessions::ends`), which is what an agent's schedules end
  with.
- **Slice 3, saved prompts** (`prompt.list`, `.create`, `.update`,
  `.delete`): `automation::prompts` keeps a name unique within its
  project or the machine's, the node's 256 and the revision under one
  lock, so two writers meet `conflict`, never the store's index;
  `service/prompts.rs` refuses agents and delegated callers and checks a
  prompt's `session` as `session.create` would (the check schedules use,
  now shared: `check_new_spec`), its place in the prompt's project.
- **Slice 4, the starter told** (`session.prompt { notify }`): the
  session's actor gives a prompt that asked a one-shot ticket with the
  seq of the `turn_started` of the turn it started or joined, or, queued,
  of the turn its entry starts later; an entry removed, or the session
  ended, drops the ticket (`vessel-session`'s `delivery/tickets.rs`).
  The node follows that turn (`service/notices.rs`) and queues into the
  caller's own session a `needs_you` per ask (at most 8), then
  `finished` or `ended`, in words of `automation::notices`; bounded at 32
  turns followed per told session and 256 on the node (`conflict`), 60
  notices an hour into one session (the rest dropped and logged), none
  into a session a person stopped. The agent tools' `send_input` and
  `create_session` take `notify`.
- **Slice 5, the CLI and the agent tools**: `vessel schedule
  list|show|runs|create|edit|pause|resume|run|delete` (`--cron` without
  `--tz` takes the zone this machine is in, from `jiff`'s `tz-system`;
  `edit` reads the schedule and sends back what changed with its
  revision) and `vessel prompt list|show|save|delete|run`
  (`crates/vessel-cli/src/automation/`); schedules and saved prompts are
  named by id, a unique prefix or their name. The agent tools
  `create_schedule`, `create_heartbeat`, `list_schedules`,
  `schedule_runs`, `pause_schedule`, `resume_schedule`,
  `delete_schedule` and `list_saved_prompts`, on the agent's own node
  (`vessel-mcp`'s `tools/automation.rs`); `create_session`'s arguments
  now make a spec in one place (`spec`), which `create_schedule` uses.
- **The code review's findings** (an independent reviewer read slices 1
  to 4; fixed in one commit after slice 5):
  1. *A skip hid the run still running*: the overlap check and the
     restart's settling read `last_run`, which a skip recorded above a
     running run replaces, so the run after a skip started beside it.
     The scheduler now keeps each schedule's newest run that did
     something (`Entry::current`) for both.
  2. *A failed first turn left its session live*: a `new_session` run
     whose turn failed (the agent alive) was not archived; it is now, as
     one that ended is.
  3. *`ends_at` did not bound a catch-up*: due times past it, missed
     while the node was stopped, could be caught up; they are now cut at
     `ends_at` (how old the latest is still counts from now).
  4. *An agent's schedule that ended by `max_runs` or `ends_at` stayed*,
     counting toward the node's 64; an agent's schedule that ends for any
     reason now goes.
  5. *A told session that ended kept its places* until the turns it
     waited on moved: each follower now also listens for that end; the
     hourly count forgets sessions told nothing for an hour.
  6. `schedule.run` without a slot in 30 s answers `unavailable` (the run
     is still recorded `failed`); a store that cannot be read fails the
     run alone instead of ending the schedule `target_gone`; the plan's
     line on a notice's `by` now says what the node does (the session
     that finished).

  Not taken: *runs waiting for a slot are not started oldest first*
  (tasks race for the semaphore; the four slots are a bound on load, not
  a queue, and a run's due time is recorded either way); *a schedule
  made while the clock was days ahead keeps that far due time* until the
  clock gets there (the correction applies to a run made in that state;
  one made by a person at a wrong clock is theirs to edit).
  `actor.rs` was divided by responsibility on the way: its calls
  (`calls.rs`), a run's task (`task.rs`), the state's pure rules
  (`state.rs`).

### Checked, and how

- Time, on a clock the test gives (`automation::time::tests`, 12 tests):
  New York's spring gap (a time in it runs once at the jump; four times in
  it and 03:00 make one run) and autumn fold (the first occurrence only,
  also asked from inside the second pass), Lord Howe's thirty-minute
  shift, London, 29 February across 2100, a cron that never matches,
  `every` across a daylight-saving change and with a fractional anchor,
  missed due times (once, skip, a day old, more than the count holds).
- The scheduler, on a clock moved by hand with a recording runner
  (`automation::schedules::tests`, 12 tests, no sleep): due once and moved
  on; two due at once; missed while the node was stopped (one run, or one
  record); the clock stepped back (no second run), forward (counted as
  missed), and corrected after a run made days ahead; an edit while due;
  a due time while a run is being made; run now, `max_runs`, `ends_at`,
  pause and resume keeping the phase; a definer gone; an agent's schedule
  gone with its session; `making` interrupted at start; at most four runs
  at once.
- Through a real node with the fake agent (`tests/e2e/schedules.rs`, 6
  tests): a run that starts a session, follows its turn, archives it and
  names it; a run through a busy session's queue that never piles up,
  skips a session a person stopped and reaches it again once someone
  sends it something; a target ended (the run fails) and deleted (the
  schedule ends); what is refused at creation; an agent's heartbeat, what
  it may not touch, the owner running it as the agent, and its end with
  the session; schedules kept across a restart.
- Saved prompts (`automation::prompts`, 4 tests; `tests/e2e/prompts.rs`,
  2 tests): a name twice in one scope and once in another, a rename onto
  a taken name, an old revision, the bound of 256, eight creates of one
  name at once making one; through a node: a machine's prompt with a
  place, an unknown project, a resuming spec, the lists by project, an
  agent that reads them and is refused every write.
- The starter told: the actor's tickets (`delivery::tickets`, 3 tests:
  a record seen before or after the prompt's answer, a steer joining the
  running turn, a queued entry started and a removed one never), through
  the runtime with the fake agent (`vessel-session`'s `tests/tickets.rs`:
  started, queued then started after a steer ended the running turn,
  removed, steered); the words and bounds (`automation::notices`, 5
  tests: the reply's last 1,500 characters and where to read, a
  permission named by its tool call, at most 8 asks, a session's end,
  32 and 256 turns followed, 60 an hour); through a real node
  (`tests/e2e/notices.rs`): a child's turn finished, waiting on a
  permission, failed when the child is stopped, and a queued prompt that
  never started told as the session's end, each queued into the agent's
  own session; the owner refused.
- The CLI (`vessel-cli`'s `tests/cli/automation.rs`, 2 tests, the real
  binary against a node with the fake agent): a schedule made with
  `--every`, listed, run now, its runs, edited, shown, paused and
  deleted by its name, and a zone the node does not know refused; a
  saved prompt saved, saved again under its name (the same prompt),
  shown, sent into a session's queue, refused `--new` without a session
  spec, deleted. The lengths `90s` to `1d` (`automation::schedules`'
  unit test), the selector (a name before an id prefix, two matches
  refused).
- The agent tools (`vessel-mcp`'s `tests/automation.rs`, through a node
  with the fake agent): a heartbeat on the agent's own session, a cron
  schedule in a zone that starts sessions, `every_secs` and `cron` at
  once refused, the list, pause, resume, runs and delete of its own, a
  person's schedule refused (`forbidden`), the saved prompts read.
- The review's findings (`automation::schedules::tests::after_review`, 4
  tests, on the hand-moved clock): the run after a skip is given the run
  that started a session as `previous`; at start a `making` run under a
  skip becomes `interrupted` and the skip stays the newest record; a
  node stopped from 08:00 to 12:00 catches up the 10:00 due time of a
  schedule ending at 10:00 (five missed) and ends it `expired`; an
  agent's schedule with `max_runs: 1` goes after its run. The hourly
  count forgetting quiet sessions (`automation::notices` tests).

### Not built, and why

- **The hook listener** (ADR 0034): the wire's shapes and schema 12's
  tables are in place and the design is recorded; nothing listens. The
  node cannot tell whether a hook session's agent asks before it acts
  (permission modes are each harness's own values, and a profile may
  approve tools in advance), so a sender's relayed text could run
  commands with the definer's access; and the time went to the rest.
  ADR 0034's "Where it stands" lists what remains and what would close
  the first reason.
- **`vessel hook`** and the six `hook.*` methods: they come with the
  listener; no client can make a hook meanwhile.
- **Agents**: no `run_schedule` tool (an agent would pump its own
  heartbeat past its cadence), no saved-prompt writes (a person's text,
  sent later by a person), no hook tool, `notify` on the agent's own node
  only, and schedules on its own node only (another node cannot rebuild
  the agent when the run comes).

### What the page track must know

- Protocol 11's schedule and saved-prompt calls are in `vessel-web`'s
  list and in `web/src/wire/proto.ts`; nothing in the page draws them yet,
  and `web/src/data` gained nothing.
- `ScheduleInfo.state` is tagged `state` (`active { next_due }`, `paused
  { since }`, `ended { at, reason }`, `unknown`); `last_run` is the
  newest run; `schedule.update` takes the whole definition under its
  `revision`, and whoever updates becomes `defined_by` (the runs act as
  them: say so beside the save button).
- `Cadence` and `ScheduleTarget` are tagged `kind`; a run's `trigger`,
  `outcome` and `end` are tagged by those names; `failed.error.code` is a
  `RunErrorCode` that reads codes it does not know as `unknown`.
- `PromptSession.notify` is an agent's: the node answers the page
  `invalid_request` for it, so the page never sets it.
- `Cadence.cron.zone` is an IANA name the node checks; the page should
  default it to the browser's zone (`Intl.DateTimeFormat().resolvedOptions().timeZone`).
- A queued prompt's `by` is the schedule's `defined_by`, or, for a
  notice, the session that finished (`Creator::Agent`); a scheduled new
  session's log starts with `vessel: started by schedule "<name>" (run
  <n>)`.
- Saved prompts: unique per scope, 256 per node; `prompt.list { project
  }` gives that project's and the machine's; agents read only.
- Webhooks: when the listener is built, show a hook's secret once and
  never keep it; say beside a hook session's permission mode what ADR
  0034's first consequence says.

## Integration 2 (page): tracks B-page and A-page on the node of protocol 11

2026-11-13, on `backend/m11-int2-web`, branched from `backend/m11`
(`9655cea`: wave 1, the node join of tracks B2 and C, track G). Track
B-page's branch was merged first and brought onto the node as it is
now (part 1); track A-page's, finished meanwhile, was merged after
(part 2). Every track's commit is kept; each merge commit builds and
passes, and each follow-up is a commit of its own.

### Part 1: track B-page on the node

**The merge** (`ac38bcd`) conflicted in the plan (every section kept;
B-page's progress after track C's), `docs/PERFORMANCE.md` (track C's
section, then B-page's) and, where the trial merge had not, the
new-session page (B-page's starter beside track G's known-agent filter).
Beyond resolving, what protocol 11 no longer compiled against:
`file.write`'s `version` guard, and `GitRefusal::repository_config`
(ADR 0030) said in words (`git-words.ts`, the key named, never a value).

**What changed in the page for the node**, read module by module
against the wire as it is now, not only where the compiler pointed:

- *A file's save* (`d0ae053`). `file.write` is guarded by the content's
  SHA-256 the edit read (`expect: { kind: "content", hash }`); the node
  compares it and refuses with `conflict`, writing nothing. The editor's
  own check of the disk followed by an `any` write is gone. The decision
  is `data/workspace/save.ts`, tested against a node double: an untouched
  file is written; a file changed meanwhile is one refused write, read
  back for the banner (both states: what was read, what is there now,
  how many lines the edit differs by), never a second write; only
  Overwrite sends `any`; a read without a hash writes nothing and says
  why. `FileVersion.modified` is milliseconds now: the banner's times
  were read as nanoseconds and are fixed; pages of one file are compared
  by their hash. `code.spec.ts` checks both states and the editor kept.
- *The approval* (`899170d`). The dialog draws every script and service
  of the set (by name, kind, folder and command: the digest covers them,
  so none is summarised) beside setup and teardown, and says what changed
  since the set approved last (`last_approved`, with its time): setup and
  teardown compared line by line, a removed command followed by an added
  one read as one changed command with what it was, scripts by name and
  what about them changed. A node that does not say (protocol 10, or an
  approval from before the node kept sets) is compared with the folder's
  approved set, read only then, and the sentence says so; with neither,
  that nothing is known to compare with. Settings › Projects counts the
  scripts. The test repository gained a branch whose `vessel.toml`
  changed (a setup command edited, a script and a service added) for
  `code.spec.ts` and the captures (42d to 42f).
- *Agents beyond the four* (`36fcfb8`). Shown wherever agents are chosen
  or shown: the new-session page's agent menu (after the four,
  "added"), Settings › Agents (found, or not found naming the program),
  a session's bar, card, details and the machine's session rows, by the
  label `agent.list` gives, else the machine's name for it. Compared by
  key everywhere (a profile's move list compared objects). No terminal
  is offered where the machine offers none; an added agent is never kept
  as the machine's default; a `conflict` from `harness.options` is said
  in the page's words (an added agent's secrets named, one of the four's
  install or a route's key) and Send stays off. `agent.list` is read
  only for a machine that shows an added agent, and when Settings ›
  Agents opens. Adding, removing, the registry and installing are not
  drawn. The e2e fleet adds three agents to laptop (the fake ACP agent,
  one needing a secret the node lacks, one whose program is gone):
  `agents.spec.ts`, `agents-screens.ts`.
- *A worktree kept in use* (`2013e5e`): from protocol 11 a terminal or a
  service beside a session keeps it too; the reason says so.
- Read and left as they are: `SessionInfo.pull_request` (track B2-page's
  to draw), a session's terminals (`terminal.list`, track C-page's),
  `project.add`'s refusal of git's own folders (said by the node's
  message, as every refusal of a path is).

**The tests that failed under load**, each cause found and fixed where
it was:

| Test | What it raced on | Fixed in |
| --- | --- | --- |
| `page.spec.ts` "with the sidebar hidden, Mod+K…" (and the three after it) | the page: a preference changed while the page was still connecting was sent at once, refused as not connected and taken back, and a state arriving while a change was on its way showed the document without it; Ctrl+B pressed as the page loaded hid the sidebar a moment, the next one hid it for good | the page (`e6107c8`): a change waits for the link to open, unanswered changes are applied over every state that arrives; unit tests with a link a test opens |
| `timeline.spec.ts:101` (1.28 px against 1 px) | the page: heights are measured after layout and before paint, and the held row was corrected only after React drew them, a frame later | the page (`ef8bceb`): the observer puts the held row back before paint; `use-window.ts` divided (`window-watch.ts`) |
| `app.spec.ts` "every New…", `page.spec.ts` "a shortcut is rebound…" | the test: `openUi` returned on the sidebar, before the bridge welcomed the page; a test navigating at once reloaded with no session and saw "Open Vessel from its address" (a bridge whose answers are held back reproduces it every time) | `e2e/open.ts` waits for the session the page keeps for reloads (`59000e2`) |
| `page.spec.ts` Mod+Enter | the test: a strict locator matched the timeline and its live region | looked for in the timeline |
| `page.spec.ts:122` (Settings' chunk) | the test: its wait was set after the release | set before it |
| the theme test | the test: the index was read once, before the bridge kept the change | asked until written |
| `app.spec.ts:61` (a menu item "not stable", then "detached") | not found in the page: slowing its processor ten and twelve times, holding the bridge's answers back, and answering the agent's options while the submenu was open did not reproduce it | `chooseAgent` chooses with Enter, the menu's keyboard path, which needs no still box |
| `app.spec.ts:37` (the old `/fleet` address not sent on to Settings in 15 s, seen by the schedules' join) | the test: the same as "every New", a navigation before the bridge welcomed the page | the same wait in `openUi` (`59000e2`) |
| `timeline.spec.ts:101` again, at the join's end (1.19 px) | the page: sampled every frame over five runs, the held match sat anywhere from 339.19 to 340.48 px, because a scroll offset is whole pixels and the hold moved only from a whole pixel off, so the row could rest up to a pixel either side of its place | the page (`6b2b398`): put back to the nearest whole pixel (`holdStep`, tested), within half a pixel of its place |
| `marks.spec.ts:16` (one run in about twelve, at load 3: "Mark as unread" detached, the test's time ran out) | the test: recorded with focus and mutation events in the page, the menu drew its items (the test's check passed) before it took the focus, so the Escape went to the button and was lost, the menu stayed open, and the next press on the button closed it | `closeMenu` waits for the focus in the menu before Escape, then for the menu to go (`9bab5d3`); fifteen runs of the test passed after |

**What B-page slowed down.** A CPU profile of opening a 20,000-entry
session (three runs, sourcemaps from a separate build of the same tree,
the machine at load 20 to 29): 39 to 51 ms in `when`, the session's
details writing its start time on every batch of the replay with a
formatter made per call (`toLocaleTimeString`/`toLocaleDateString`):
the formatters are made once (`3b5bcd1`). The rest of the replay's time
is the window's follow of the end (a forced layout per commit while the
log replays, `use-window.ts`), which the base has too, and which costs
more per commit by what B-page added to the document (the inspector's
tabs, the title bar's chip, the sidebar's projects). The projects'
`project.list` at start: on a phone the sidebar beside the page is
hidden, so its group, and the read, wait for the sheet (`cca8f84`); on a
wider window the group is on screen from the start and needs the list.

### Part 2: track A-page joined

Track A-page's progress section and gate were on its branch and its
working tree was clean (`eccbd24`), so it was merged (`fba9997`).
Conflicts, each kept from both sides: the README's board paragraph,
DESIGN.md's differences from the references (A-page's renumbered 28 to
33), the plan (A-page's contract after track C's, its progress before
B-page's), the kit's exports, `data/calls.ts`'s imports, the session
card, the "+", the new-session page, the session's composer and the
window. Beyond resolving: A-page's `AgentControls` name the agent by
`HarnessRef`; `data/fleet.ts` passed its budget with both tracks'
additions, so a machine's session list's pure updates are
`data/fleet-sessions.ts`, with tests. The generated declarations were
regenerated and did not change.

The join's conditions:

- One module per responsibility: the new-session page's controls are
  A-page's `NewSessionControls`, carrying B-page's place and track G's
  agents; the window's watch on its scroller is one module
  (`window-watch.ts`) with A-page's "a reader at the end stays there when
  the view shortens" and this join's hold before paint; a machine's
  session list is updated in one place for A-page's marks.
- `@` in the composer (`use-file-search.ts`, `use-suggest.tsx`) and
  open-by-name in the Files panel (`file-search.tsx`) both call
  `data/workspace/find.ts`, each with its own search key.
- A selection from the Changes or Files panel ("Add to chat") writes the
  session's draft (`fleet.drafts`), which A-page's composer shows.
- Archiving says that the terminals and services beside the session end
  (`637c820`), on a node of protocol 11.
- The board keeps an ended session in its lane until archived, the
  archive shows the archived ones, and the cards carry A-page's marks
  and actions beside B-page's branch chip (capture a65).

### The independent review

One reviewer (Opus, read only, the save guard and the approval first)
read `git diff backend/m11` at `2013e5e` and found twelve problems. Each
one's fate, most severe first:

1. **High: `@` asked `file.find` again after every answer** while its
   word stayed open: the composer made the session's root a new object
   on every drawing, which the search's effect read as a new root.
   **Fixed** (`ce40218`): the composer and the code panel read one hook,
   `useSessionRoot`, which keeps the root one object while it names the
   same place.
2. **Medium: keys typed while a save was on its way were lost** when the
   save closed the editor. **Fixed** (`ff0c839`): the editor is read-only
   until the save answers (the kit's `TextArea` gains `readOnly`).
3. **Medium: a preference could stay shown over another tab's newer
   value**: the bridge sends pushes and replies from two tasks, so a
   newer push can come before this tab's reply, which was then dropped
   as older with the values never worked out again. **Fixed**
   (`6e8d1c3`): once a change is answered the values are worked out
   from what the bridge confirmed and what is still on its way; tested.
4. **Medium-low: a conflict's two sides could be wrong or look alike**:
   a file over one page was compared by its first page against the whole
   edit, and two versions of one minute and like size read the same.
   **Fixed** (`ff0c839`): sides are compared only when the edit read the
   whole file, and each names the start of its content hash.
5. **Medium-low: Settings' agent card asked an added agent twice**, its
   effect keyed by an object the machine's lists replace, and said a
   refusal in the machine's words. **Fixed** (`591211b`): keyed by the
   card's agent, refusals in the page's words.
6. **Medium-low: a refusal of an agent's options was never asked again**,
   so Send stayed off after the secret was set. **Fixed** (`591211b`):
   "Ask again" beside the refusal; `agents.spec.ts` presses it.
7. **Low-medium: choosing another machine during a worktree's start did
   not stop it**, and Approve then approved the first machine's project
   under the second's name. **Fixed** (`5cb92e2`): the start stops, as
   it does when another place is chosen.
8. **Low: on a node before protocol 11 every save would say the file is
   too large to be compared.** **Not taken**: `file.read` has given the
   content's hash for every file up to 8 MiB since the file calls came
   (protocol 10), and Edit reads at most 1 MiB, so a read without a hash
   does not reach the editor on any node that has the file calls.
9. **Low: a page read for the view could land after Edit read the whole
   file** and put a part back. **Fixed** (`ff0c839`): dropped when the
   file was read past it.
10. **Low: two functions worked out a session's folder and disagreed**
    (the panel takes a folder inside a project as that project, `@` did
    not). **Fixed** with 1 (`ce40218`).
11. **Low: an added agent was named by its machine name** in Settings ›
    Models and the new-session page's notes and option names. **Fixed**
    (`591211b`).
12. **Low (efficiency): the sidebar's media query subscribed again on
    every drawing**: **fixed** (`2cb9285`). The time formatters' zone is
    fixed when the module loads: **not taken**, a zone changed while the
    page is open is read again on reload, as everywhere else the page
    writes a time.

Found clean by the reviewer: the guard (`any` sent only by Overwrite, a
conflict read back once and never followed by a write, the base set only
when Edit starts, no draft crossing files or roots), the approval's
comparison (line pairing, scripts by every field) and its digest, the
agents compared by key everywhere, the window's hold against its own
scrolls, and the A-page merge's resolutions.

### Five runs of the end-to-end suite

Each run is the whole suite (`npm run e2e`, 56 tests) on the binaries
and bundle of the tree named, the machine shared with other workers:

| Tree | Run | Load (1 min) | Result |
| --- | --- | --- | --- |
| `3868d68` (the review's fixes) | 1 | 26.6 | 56 passed |
| | 2 | 22.1 | 56 passed |
| | 3 | 17.1 | 56 passed |
| | 4 | 12.1 | 55 passed, `timeline.spec.ts:101` failed (1.19 px): fixed in `6b2b398` |
| `6b2b398` | 1 | 4.0 | 56 passed |
| | 2 | 2.7 | 56 passed |
| | 3 | 4.3 | 56 passed |
| | 4 | 3.1 | 55 passed, `marks.spec.ts:16` failed (the menu's Escape): fixed in `9bab5d3` |
| | 5 | 3.5 | 55 passed, the same |

The last run of the fourth tree's was not made: the cause was found
first (the test repeated alone, twelve runs, one failure; after the fix,
fifteen of fifteen). The suite on the final tree is in the gate below.

### Merged with the node's third join

`backend/m11` at `7be51d9` (the schedules join: track D on pull
requests, terminals and added agents; the store's schema 12; ADR 0033
to 0036; the milestone 12 plan; the brand's files) was merged last
(`0065591`), so that the preview is deployed from one tree. One
conflict: the fake ACP agent's list of prompts, where this branch's
`ask claude` (track A-page's) and the node's `fail` are both kept.
`cargo check --workspace --all-targets` passed before anything else;
the TypeScript regenerated from the merged wire is byte for byte the
file the node's join committed, whose new aliases it classified; the
page compiles and its tests pass against it. No page draws the
schedules yet (their screens are not this pass's).

### Commits

- Part 1: `ac38bcd` the merge of B-page; `d0ae053` the save's guard;
  `899170d` the approval's scripts and changes; `36fcfb8` agents beyond
  the four; `e6107c8` a preference before the link opens; `ef8bceb` the
  held row before paint; `59000e2` four tests' waits; `3b5bcd1` the
  time's formatters; `cca8f84` the phone's projects read on opening.
- Part 2: `fba9997` the merge of A-page; `637c820` archiving says what
  ends; `2013e5e` a worktree kept in use by terminals and services.
- The review: `ce40218` one session root for `@` and the panel;
  `ff0c839` the editor during a save, the conflict's sides, a page read
  past; `6e8d1c3` a preference answered after a newer state; `591211b`
  Ask again, one probe per card, labels; `5cb92e2` another machine stops
  a worktree's start; `2cb9285` one media subscription; `0a36a59`,
  `3868d68` the README and DESIGN.md.
- Under load, later: `6b2b398` the hold to the nearest pixel; `9bab5d3`
  a menu closed once it holds the focus.
- `0065591` the merge of `backend/m11` at `7be51d9`; this section.

### The gate, on the final tree

On the merge with `backend/m11` (`0065591`), page and binaries rebuilt
first, the machine shared with other workers (load 9 to 40 while the
Rust tests ran):

- `cargo fmt --all --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo clippy -p vessel-proto -p vessel-web --features ts --all-targets
  -- -D warnings`: clean.
- The TypeScript regeneration (`VESSEL_WRITE_TS=1 cargo test -p
  vessel-proto -p vessel-web --features ts --lib typescript`): passes;
  nothing to commit.
- `cargo test --workspace --no-fail-fast`, run twice. The first: 1,619
  passed, 6 failed, all six in `vessel-web`'s `tests/ui` `prefs::` (the
  bridge closed its socket, 4401 "the UI was restarted or
  authentication failed"), at a time an end-to-end test of the page on
  the same machine was also starting `vessel ui`; that binary alone then
  passed 29 of 29 twice. The second, with nothing of this work running
  beside it: 1,624 passed, 1 failed, 9 ignored, over 105 test binaries;
  the failure `vessel-node`'s `scheduled::a_scheduled_session_is_watched_and_put_away_with_what_runs_beside_it`
  (a terminal beside the session still running when looked at, at load
  16). The node's `tests/e2e` binary alone at the same load: 142 of 142,
  then 141 with `grants::a_reinstalled_node_grants_again_once_the_home_forgets_it`
  failed (its 20 s wait for a report), which had also failed once in a
  run before the merge (as had `forge_rules::a_forge_that_is_down_holds_nothing_and_an_end_clears_it`);
  at load 3 that binary passed twice. These are the node's own timed
  tests, untouched here (no code under `crates/vessel-node` differs from
  `backend/m11`): they are named for the node's track, not changed.
- In `web/`: `npm run lint` (fonts, controls, size: within budget),
  `npm run typecheck`, `npm run test` (401 passed, 77 files), `npm run
  build`, then `VESSEL_WEB_REQUIRE_BUNDLE=1 cargo build -p vessel-cli -p
  vessel-node -p vessel-testkit --bins`, `npm run e2e` (56 passed),
  `npm run screens` (8 passed, 200 captures, looked at), and
  `VESSEL_CARGO_PROFILE=release npm run perf` (`docs/PERFORMANCE.md`).

## The first chat (page), 2026-11-13

The owner, on starting a chat: "type and send, and it should just work,
as in GOAT or the orchestrator; now something is off". Measured against
the written comparison of the three (GOAT, agent-orchestrator and the
page, read from their code the same day), whose thirteen differences
are taken most harmful first. "Like X" is reproduced closely; each
deliberate difference is named. The native app (ADR 0036) must end with
the same flow: the decisions are in `web/src/data` with their tests,
which are the fixtures ADR 0036 names, and the list at the end of this
section is what the app's new-chat screen does the same way.

### 1. Send no longer waits on the agent's start

Before: Send created the session, then sent the prompt and waited for
the node's answer, which comes once the agent has finished starting (its
handshake, up to a minute); only then did the page open the session.
Nothing showed the prompt meanwhile but a busy button.

Now, as agent-orchestrator (`NewChatPage.openSession`, its local echo):
the session's page opens in place of the new-session page as soon as the
node has created the session (`replace`, and not for someone who has
left the page meanwhile); the prompt is sent without waiting on it
(`data/openings.ts`) and drawn at the end of the session's log, marked
"sending", from the first frame, until the log has the prompt; the
start shows there as the log tells it (starting, the agent's own lines,
ready); a message typed meanwhile is queued behind the first ("Queue a
follow-up while Claude Code gets ready", agent-orchestrator's words).
Files for the first message keep the path they had (the session's
composer sends them with the prompt once the agent can take them).

Deliberate differences: GOAT reserves the thread's id before Send and
creates the thread and its first turn in one call; agent-orchestrator's
daemon takes the brief with the create. The node takes neither (below),
so the page makes two calls and shows the first prompt from its own
memory: a reload while it is on its way loses the echo, not the prompt.

For the node (not changed here, the wire is the node's): `session.create`
taking the first prompt (`SessionSpec.prompt: Vec<ContentBlock>`,
delivered as soon as the agent is ready and logged at once as the
session's first record), so every device shows it from the start and a
reload keeps it. (The title at creation needs nothing of the node: the
spec's `title` already exists, and the page now sends it, 8 below.)

Timed against the fake agent made to take four seconds to start ("Slow
starter", `fake-acp --slow-start 4000`, added on laptop by the e2e
fleet), from Send's click to the prompt in the chat
(`e2e/new-chat.spec.ts`): before, 4,480 ms (the page stayed on
the new-session page, nothing shown but the busy button, capture n0);
after, 195 ms (168 and 170 in two later runs), the agent's answer at
4,120 ms. Meanwhile an empty box offers no Stop (`6f7281a`). Captures
n0 to n2, before and after, taken by the test with
`VESSEL_SCREEN_DIR`, looked at: half a second after Send, before, the
new-session page with nothing but the busy button; after, the chat with
the prompt marked "sending" and the agent starting above it).

Commit: `828292d`; its fake agent's argument failed the test kit's
clippy (index arithmetic), fixed in `0f65110`.

### 2. Opening the page leads to a composer, in the project worked in

Before: the index was the board; every New and `N` opened the
new-session page at "No project", in the machine's home folder (the
sidebar's "+" on a project's row, held in memory, was the only way to
start in a project).

Now, as GOAT (`IndexDraftLanding`, "a draft in the most recently updated
project"): the index opens the new-session page once a machine is
connected (the welcome while none is, as before), the cursor in the box;
the place starts in the project of the machine's most recently active
session that is still one of its projects (`data/workspace/last-project.ts`),
in its folder ("Local"); New or `N` from a session starts in that
session's project. The board stays one click away (the sidebar's
Board).

Deliberate difference: GOAT and agent-orchestrator remember the last
project on the device or server; the page reads it from the node's
sessions, the same on every device, because the bridge's preferences
keep a machine's last agent and choices but not its place (adding one
is a change to the bridge's keeping, outside the page: a
`default_place { node, project, mode }` change kept beside
`default_agent`).

### 3. The remembered model, effort and mode are always applied

Before: the choices were sent only when the agent had already said what
it offers (`specOf` with options `ready`), so a Send during "Loading
models…" started on the agent's own defaults; the start then kept the
empty list, replacing what was remembered; and a choice equal to the
agent's default today was neither sent nor kept.

Now, as agent-orchestrator (a send while the page loads waits up to
10 s): Send while the agent's options are read waits for them, at most
10 s, then starts with the remembered and chosen choices applied; what
is kept is every choice the agent offers, a value it already has
included (`keptChoices`); a start that could not check them (the agent
did not answer in time, or failed to) keeps the remembered ones as they
were, never fewer. Commit: `0d16b53`; `new-chat-choices.spec.ts` (a
Send made as soon as the page opens starts with build-server's kept
effort; one put back to the default is kept). With the fake agents the
options come in a fraction of a second, so the test does not always
catch Send inside the wait; the wait itself is `use-options-wait.ts`.

### 4. A first prompt the machine refuses is not lost

Before: a toast ("The session started; the prompt did not: …"), the
new-session page's draft cleared, and the page went to the session: the
text was gone.

Now, as GOAT and agent-orchestrator (the brief returned to the composer
with the reason): the refused prompt is put in the session's box (before
anything typed there meanwhile), the echo goes, and the box says why
("The first message was not sent: …") until the next message goes.
Commit: `c7e3504`; `new-chat-refused.spec.ts` answers the first prompt
with a refusal in the bridge's place.

### The rest of the comparison

- **8. The title, at once**: taken (`bb74b40`). The spec carries the
  title from the prompt's first line by the node's own rule
  (`titleFromPrompt`), which the node keeps; the sidebar and the board
  name the chat from its creation. An AI title is not made (neither the
  node nor the page has one).
- **5. A new-chat shortcut that works while typing** (GOAT's Mod+Shift+O,
  Mod+N in the app): not taken in this pass; the shortcut registry's
  defaults and their Settings page change with it, and the app's keys
  are being made in the same registry (ADR 0036); next with the app.
- **6. A scratch folder for "No project"**: the node's (it would make a
  per-session folder for a session with no `cwd`); not changed here.
- **7. The worktree made on the session's page**: needs the session to
  exist before its worktree, which the node does not do (a session is
  created in a place that exists); the approval of unapproved commands
  would still be asked first. Not taken.
- **9. Model, effort and permission shown before the agent answers**:
  the controls are the agent's own options, known only once it answers;
  drawing the kept ones meanwhile needs their names, which the bridge
  does not keep (only ids and values). Not taken; Send's wait (3) covers
  the harm of starting without them.
- **10. `/` and `@` on the new-session page**: `@` needs a root before
  the session exists (the chosen project's folder) and `/` the agent's
  commands, which only a session reports. Not taken in this pass.
- **11. Mod+Enter sends in the background and opens a fresh draft**: now
  possible after 1; not taken in this pass (it changes what Mod+Enter
  does on the new-session page, a key the person may have rebound).
- **12. The follow-up default**: kept (queue); agent-orchestrator agrees,
  GOAT steers. A choice of the person's in Settings › General already.
- **13. Smaller**: the machine kept in the address (the page already
  takes `?machine=`); one draft for the new-session page (kept, the
  bridge keeps it); the agent's own permission mode (kept: the agent's
  default is the person's to change, GOAT's full access is not assumed).

### Found on the preview, fixed

Opened by its address, the new-session page said "This machine has no
agent installed." for about a second, until the machine's agents were
listed: an agent not found was taken for one missing. It says "Loading
the machine's agents…" until the list has come, and that there is none
only once the list holds none (`cf4ad9c`; `new-session-loading.spec.ts`
holds the bridge's answers to the list back for three seconds).

### The owner's mark

The sidebar's head and the welcome draw the owner's mark
(`docs/design/brand/`, the corrected file) in the text's colour (18px
beside "Vessel", 28px on the welcome), and the page's icon is the mark
in white on a rounded tile of the brand colour, as the brand's README
asks of the interface and of icons. The build reads the brand's file
(`web/vite-mark.ts`: the paths as data for React, the icon as an asset
the index links), so no generated file is kept and no copy of the paths
can drift (`e1a7eca`; `brand.spec.ts`). The look of the page is
otherwise unchanged in this pass.

### What the native app does the same way (ADR 0036)

The app's new-chat screen ends with the same flow; the page's tests in
`web/src/data` are the fixtures:

- Send creates the session and shows it at once, in place of the
  new-chat screen, unless the person has left it; the first prompt is
  sent without waiting on it and drawn at the end of the session's log,
  marked "sending", until the log has a prompt or the machine refused it
  (`openings.test.ts`); a message typed meanwhile is queued, and the box
  says "Queue a follow-up while <agent> gets ready"
  (`composer-words.test.ts`).
- A refused first prompt goes back into the session's box, before what
  was typed there, and the box says "The first message was not sent:
  <why>" until the next message goes (`openings.test.ts`).
- The first screen with a machine connected is the new-chat screen; its
  place starts in the project of the machine's most recently active
  session still among its projects, else none (`last-project.test.ts`);
  New from a session starts in its project.
- Send while the agent's options are read waits for them, at most 10 s;
  what is kept for the machine is every choice the agent offers, or the
  remembered ones unchanged when they could not be checked
  (`picks.test.ts`).

## Track A2: contract

Wave 2's last node track (`backend/m11-a2`, from `backend/m11`). Protocol
12, raised once in this track's first slice that changes the wire (every
later track of the wave finds it raised); store schema 13; ADRs 0037 and
0038. Five features, each its own slice, each with its contract here
before its code: search every session's messages; park an idle agent and
wake it on the next prompt; rewind and fork where the agent can; import
from the agent's own history; change the agent with a handoff. What the
references do was read in their source on 2026-11-13 (agent-orchestrator
and GOAT at `main`, Paseo 0.15.2) and in the installed adapters
(`claude-agent-acp` 0.100, `codex-acp` 2.6); no prompt was sent to a real
agent.

### 1. Search every session's messages

**What the references do.**

| | agent-orchestrator | GOAT | Paseo |
| --- | --- | --- | --- |
| Over | finished chat messages (external-content FTS5, `porter unicode61`) | finished messages (contentless FTS5, `trigram remove_diacritics 1`) | titles, branches, project names only ("never an incomplete subset of transcripts"); in-session find apart |
| Words | all must match, the last by prefix; if none matches, any may | all, as substrings; one two-character CJK word sends the whole query to a scan of the newest 20,000 messages; Latin words under three characters only narrow; no word of three characters (and no CJK) finds nothing | typo-tolerant over names |
| Order | bm25, by recency past 2,000 matches | its own ranking over at most 2,000 hits | by score |
| Page | 8 hits, no paging | 50 matches, 2,000 hits read, 10,000 candidates | all |
| Snippet | 24 tokens, matches wrapped in control characters | 240 characters, the match 24 in | none |
| A message still being written | not indexed until it ends | not indexed until it ends | – |
| Left out | cancelled and rolled-back turns, other branches | generated notices, attachment placeholders, subagent threads, archived threads | – |

Vessel takes GOAT's tokenizer (substrings, so a Korean word with its
particle and an identifier inside a longer one are both found), both
references' rule that a message is indexed once it ends, and pages by
time so a client can go on past the first page.

**What is a message.** The node's log has no messages, only records; the
index makes them:

- a prompt: the text blocks of a `turn_started`, a `steered`, or a run of
  `user_message_chunk` records (a resumed or imported conversation's
  replay);
- a reply: a run of `message_chunk` records' text.

A run ends at the next timeline record that is not one of its chunks
(a tool call, a thought, a plan, a permission, a question, a turn's end
or failure, a status); records outside the timeline (`harness_log`,
`usage_changed`, `commands_changed`, `queue_changed`, `model_call`,
renames, option and profile changes) do not end it. A run longer than
8 KiB of text, or than 512 records, is indexed as consecutive pieces,
each pointing at the record where it starts, so one long reply is many
hits rather than one cut short. Thoughts, tool calls and their output,
plans, permissions, questions, terminal output and harness logs are not
indexed: they are what the agent did, not what was said, and a tool's
output would drown every word in file contents.

**The index** (schema 13, `vessel-store`, ADR 0037; amended by the
review below):

- `search_messages (id INTEGER PRIMARY KEY, session_id, seq, role, at,
  text, indexed)`: one row a message piece; `id` is the message's time in
  nanoseconds, moved forward past one already taken, so the index's own
  order is newest first and a page is a walk of it, never a sort. No
  foreign key: what a removed session or pruned records leave is hidden
  by a join and deleted later.
- `search_index`: FTS5 over `text`, external content, `tokenize =
  'trigram'` (case-insensitive; diacritics kept).
- `sessions.search_seq` (the next record the index has not read) and
  `sessions.search_open` (where the run held in memory starts).
  Every session stored before schema 13 starts at 0.
- An append's records are read into plain rows inside its savepoint;
  every FTS5 write and every deletion happens in the writer's idle
  steps, bounded; sessions stored before schema 13 are caught up there.

**Types** (`vessel_proto::search`, new module):

- `SearchQuery`: 1 to 256 characters once trimmed, at most 16 words
  (split on whitespace); a person's text, plain in TypeScript.
- `SearchLimit`: 1 to 100; absent means 20.
- `SearchCursor`: where the next page starts, the node's handle (a
  string on the wire, branded in TypeScript); only ever handed back.
- `MessageRole`: `prompt`, `reply`.
- `SearchHit { session: SessionId, seq: EventSeq, at, role, snippet }`:
  `seq` is the record where the message (or its piece) starts, so a
  client opens the session there (`session.read` from `seq`).
- `Snippet { parts: SnippetPart[], cut_before, cut_after }`,
  `SnippetPart { text, matched }`: at most 240 characters in all,
  whitespace runs as one space, starting up to 24 characters before the
  first match, every match inside it marked. Typed parts, not marker
  characters inside a string.
- `SearchMessages { query, session?: SessionId, before?: SearchCursor,
  limit?: SearchLimit }` and `SearchPage { hits: SearchHit[], next?:
  SearchCursor }`.

**The call.**

| Method | Scope | Request | Reply |
| --- | --- | --- | --- |
| `session.search` | read | `SearchMessages` | `SearchPage` |

- Forwardable, like every read. Not ownership-checked: an agent reads
  the fleet's sessions (ADR 0013), so it may search them.
- Every word must be in the message (the piece), in any order, as a
  substring, without case: every candidate is checked by one Rust fold,
  whichever walk found it.
- Newest first, by the message's time. `next` is present while older
  messages may still match; a page can hold fewer than `limit` hits and
  still have `next` when a bound below stopped it.
- `session` keeps the search to one session (`not_found` when unknown).
- Failures: `invalid_request` (the query or the cursor does not parse),
  `not_found` (the session), `unavailable` (two searches already run),
  `internal` (the store), `unsupported` on a connection older than 12
  ("needs protocol 12").

**Bounds.**

| What | Bound | Beyond it |
| --- | --- | --- |
| A query | 256 characters, 16 words | `invalid_request` |
| A page | 100 hits (20 by default) | refused |
| Candidates read when some word is under three characters (the index finds the longer words, the short ones filter) | 10,000 a page | the page ends, `next` set |
| A query whose every word is under three characters (no index can serve it) | 20,000 newest messages scanned a page | the page ends, `next` set |
| An indexed piece | 8 KiB of text or 512 records | the next piece |
| An idle step | 32 KiB of text and 128 rows taken in or deleted; or 256 records (256 KiB) of a log read; only while the writer's queue is empty | the next step |
| Searches at once on a node | 2 | `unavailable` |

**Older peers.** A client sends `session.search` only to a node of
protocol 12 or later (`Call::since`); an older node is a typed "needs
protocol 12" for that machine, and its sessions are left out of a
fleet-wide search with that said. A node refuses the call on an older
connection. No event and no existing type changes.

**CLI and agents.** `vessel search <query> [--session <id>] [--limit
<n>] [--before <cursor>]` (with the usual `--node`), one line a hit:
session, record, time, role, the snippet with its matches marked, and
the cursor to go on with (no other command prints JSON, so this one does
not either). The agent tool
`search_sessions { query, session?, limit?, before? }` returns the page.

**What it costs** is measured on a store of tens of thousands of
messages (in "Track A2: progress"): the index's size beside the log's,
an append's time with and without it, a query's time for a rare word, a
common word, a two-character Hangul word, and catching up.

### 1. Search: the contract's review (2026-11-13)

An independent reviewer (read-only, Opus) read the contract against the
store, the wire and the references, and probed SQLite (the system's
3.45.1, the contract's schema, one commit per message). Two blockers,
eight should-fix, three minor. What changed (the code is the amended
design; ADR 0037 records it):

1. **Blocker: deleting a session's index rows stalled the writer** (6.1 s
   for a session of 2,000 messages through the cascade and trigger). Now
   nothing of the index is deleted in the deleting transaction: a query
   joins `sessions` and its `first_seq`, so rows of a removed session or
   of pruned records are hidden at once; triggers on `sessions` note what
   is gone (`search_gone`) and the writer's idle steps delete it, 32 KiB
   and 128 rows a step. `search_messages` has no foreign key.
2. **Blocker: indexing in the append** cost 14 ms a commit on average and
   up to a second with FTS5's merges. Now the append writes plain rows
   only; every FTS5 write (taking rows in, deleting them, the merges that
   come with them) happens in idle steps of 32 KiB of text. Measured:
   the appends' median and tails are the same with the index as without
   ("Track A2: progress").
3. **Catch-up**: the cursor is two columns (`search_seq`, the next record
   not read; `search_open`, the start of the run held in memory), so a
   session mid-reply is never "behind". A step that fails or finds
   nothing stops the steps until the next commit (no spinning). A step
   is bounded by bytes as well as records (256 records or 256 KiB). An
   append to a session still behind indexes nothing; the steps reach its
   end and hand the open run over to the appends.
4. **An indexing failure fails the append**: indexing now runs in a
   nested savepoint of its own; a failure is logged and left to the idle
   steps. A run cached for an append whose savepoint rolled back is
   dropped (taken out before, given back only after).
5. **The FTS5 query**: every word goes in as a quoted string; a word
   under three characters never goes in (it would match everything).
   Every candidate is checked again in Rust, so the index only narrows.
6. **Three foldings**: one now. The tokenizer folds case only
   (`remove_diacritics` dropped), and Rust's per-character fold (lower
   case when that is one character, never a change of length) checks
   every candidate and marks every snippet, so a hit always has its words
   marked. Diacritics and Unicode normalisation are not folded (ADR 0037
   says why: a table of Vessel's own would be needed to agree with the
   tokenizer).
7. **Unbounded work**: a search kept to one session walks that session's
   rows (its index), bounded like a scan; every walk counts what it reads
   and stops at its bound; at most two searches run at once on a node
   (`unavailable`).
8. **The cursor**: `next` is the last piece read, so a page that ran out
   moves on; `reached` (new) is that piece's time, so a client merging
   machines knows how far a short page looked. Rows indexed later above a
   cursor are seen only by a new search (said in `PROTOCOL.md`).
9. **Pieces**: cut after white space near 8 KiB; matching is per piece
   (said); the 512-record bound counts every record in a run's span, not
   only its chunks.
10. **The references**: GOAT's row corrected, and what each leaves out
    added. Vessel indexes every prompt, including the node's own (a pull
    request's feedback, a schedule's prompt): they are what the agent was
    told. Messages a later rewind removes are the rewind slice's to hide.
11. **Disk**: measured; the index does not count toward retention's caps.
12. **Time as the id**: replayed and imported messages carry the time
    they were logged; a clock stepping back sorts new messages lower
    (said in ADR 0037).
13. **Prune**: a run whose start was pruned now starts after it (the
    cache no longer matches the row and is read again); recovery's
    `interrupted` records are left to the idle steps; titles are not
    searched (the client has them).

Rejected, and why:

- **A `unicode61` side index for two-syllable Hangul** (review item 12).
  Measured, a common two-character word is found in under a millisecond
  by the scan (it stops at its page); only a word that matches nothing
  reads the whole bound (about 130 ms). A second index would double the
  index's size for that case.
- **A progress-handler time budget on the reader** (item 7). FTS5's work
  between two rows is not interruptible by it, and every walk already
  counts its rows; the two-at-once bound caps what a caller can hold.
- **Rebuilding the index on demand** (item 4's repair path). The index is
  derived: deleting `search_messages` and resetting the cursors makes the
  idle steps build it again, which is what the measurement does.


## Track A2: progress

### 1. Search every session's messages (built 2026-11-13)

Protocol 12 raised (`PROTOCOL_VERSION` 12, range 2..=12), schema 13, ADR
0037. The design is the contract as its review amended it (above).

- **Wire** (`vessel_proto::search`): `session.search` (read, `since 12`,
  forwardable, allowed through `vessel-web`'s bridge and to agents),
  `SearchQuery`, `SearchLimit`, `SearchCursor` (aliases classified:
  handle, plain, quantity), `SearchHit`, `Snippet` as typed parts,
  `SearchPage { hits, next?, reached? }`.
- **Store** (`vessel-store/src/search/`): `said` (what a record is to the
  index), `index` (the append's side: runs, pieces, the cursor, a nested
  savepoint), `idle` (the writer's idle steps: take in, delete what is
  gone, catch up), `query` (the walks and their bounds; the index's walk
  merged with the rows it has not taken in yet, so a message is found the
  moment it is written), `snippet` (one fold, the window, the marks).
  `Store::search`, `Store::indexed`.
- **Node**: `service/search.rs` (a named session must exist; two
  searches at once).
- **CLI**: `vessel search <words> [--session] [--limit] [--before]`.
- **Agents**: `search_sessions`.

**Checked**: unit tests (record classes, pieces cut after white space,
snippets with Hangul inside words, the quoting of a person's words, every
walk an index walk without a sort); the store's (a reply found once it
ends and split over two appends, short and Hangul words, paging newest
first without a gap or a repeat over 26 hits in pages of 10, pieces of a
long message, deletion and retention hiding and then deleting rows with
the FTS5 integrity check passing, a reply cut by a restart indexed whole,
a schema-12 database caught up while idle; the older-schema migration
tests now walk through 13); the node's end to end (two sessions asked
through the fake agent, found after a restart with their records and
roles, a Hangul word, one session's, an unknown session `not_found`, an
agent searching, a protocol-11 connection refused `unsupported`); the
CLI's line and the tool's arguments.

**Measured** (`cargo test -p vessel-store --release --test search_measure --
--ignored --nocapture`; the shared 16-core machine, load average 2 to 16
during the runs; two stores fed the same 220,000 appends one at a time,
100 sessions of 100 turns of a prompt, 20 reply chunks and a turn's end,
20,000 messages; in one the replies are `thought_chunk`, which the index
passes over):

| What | With the index | Without |
| --- | --- | --- |
| An append: median, p99, p99.9, max | 107 µs, 1.6 ms, 41 ms, 271 ms | 104 µs, 1.2 ms, 47 ms, 251 ms |
| Idle work left after the last append | none (1 µs) | – |
| The database | 76.1 MiB | 49.7 MiB |

The appends are the same within the machine's noise (three runs: the
"without" store was the slower in two). Queries on the 20,000 messages
(20 runs each, median and max): a rare word 1.0 ms, 3.8 ms; a common one
1.2 ms, 2.2 ms; two words 1.5 ms, 3.4 ms; a two-character Hangul word
(the plain walk, stopping at its page) 0.57 ms, 0.65 ms; the same with a
second word 0.62 ms, 0.74 ms; a two-letter word that matches nothing
(its whole bound, 20,000 messages) 126 ms, 143 ms (740 ms in a run under
load); one session's 0.57 ms, 0.82 ms. Catching up the whole store from
nothing (as after an upgrade) took 13.6 s while idle; appends to another
session meanwhile waited a median of 1 to 4 ms, p99 50 to 360 ms, at
most 450 ms (each waits for at most one step; the first steps were
longer). The bounds of an idle step were halved twice from 2,048 records
and 256 KiB to these after the first measurement showed appends waiting
up to 100 ms a step during a catch-up.

**Not built, and why**: diacritics and Unicode normalisation are not
folded (ADR 0037); titles are not searched (clients have them); hits are
per piece, not merged per message.

**For the page track**: a hit opens its session at `seq` (`session.read`
from there); `next` with fewer hits than asked means "the node looked
this far" (`reached`), not "no more"; a machine older than 12 answers
"needs protocol 12", so search is offered per machine; `unavailable`
means ask again.

### Tests that fail only under load (the coordinator's condition, 2026-11-13)

Slice 1's whole-workspace run (load average 20 to 30) failed seven tests
that pass alone. Neither is this track's code; their owners make them
load-proof. What waits, and the fix:

- **`forge_rules::a_forge_that_is_down_holds_nothing_and_an_end_clears_it`**
  (`crates/vessel-node/tests/e2e/forge_rules.rs`, the first `refresh`).
  The test starts the session, then puts the pull request in the fake
  forge, then asks `pr.get { refresh: true }`. The node's watcher looks
  at a new session's branch on its own; when that look lands before the
  `forge.put` (more likely the busier the machine), it finds nothing, and
  its `refreshed` mark makes the test's refresh "recent"
  (`forge/watcher.rs`, `REFRESH_EVERY` = 10 s), so the reply is the empty
  view instead of a new look. Fix: put the pull request in the fake forge
  before `running_in` starts the session (the branch is known from the
  worktree already), so the watcher's first look finds it whichever comes
  first.
- **Six of `vessel-web`'s `ui/prefs` tests** (4401 "the UI was restarted
  or authentication failed"). `tests/ui/support.rs` starts every test UI
  with `hello_deadline` = 300 ms (`Ui::configured`); the bridge closes a
  socket whose challenge and hello do not finish within it
  (`socket.rs`). Under load the test tab's answer takes longer than 300
  ms. Fix: the support's default to the product's 5 s (or the 30 s
  `server.rs`'s tests already use), and the one test that wants the
  deadline to pass (`server.rs`, "Nothing at all, past the deadline")
  sets 300 ms itself.

## Track A2: contract, 2. Park an idle agent and wake it on the next prompt

**What the references do.**

| | agent-orchestrator | Paseo | GOAT |
| --- | --- | --- | --- |
| After | 10 min without activity, swept every 30 s | 60 min, swept every 60 s | 30 min, at most 4 h held by background work |
| On by default | off (a developer setting, off at each start) | on (`agents.idleRuntimeCleanup`) | on, no setting |
| Not while | a turn runs or is being dispatched, anything is queued, an interaction waits, a person views the chat (a 30 s lease), the agent reports background work (a private ACP extension `_ao/session/can_hibernate`, which the stock Claude adapter lacks, so Claude never parks) | the agent is busy, a permission waits, a delivery is queued, children work, the agent vetoes (`canStopRuntime`) | a turn is busy, the adapter reports live work |
| Stopped how | the process ended, without ACP `session/close` (which may delete what resume needs) | `session.close()` | the driver released |
| Woken by | a send, a person opening the chat, a provider switch; reads do not wake | a prompt or opening the agent | the next turn |
| Woken with | `session/load` (or `session/resume`) on the same conversation, 60 s | the provider's own resume | `resumeThread` |
| Shown | `hibernated`; the composer still sends | `closed` | `stopped` |
| A failed wake | queued turns fail, "could not wake the agent" | stays closed, retryable | – |

Vessel parks like Paseo (on by default, an hour) and stops like
agent-orchestrator (the process ends, no `session/close`), and offers it
only where the agent can resume: an agent whose handshake says
`loadSession` (`abilities.resume`).

**What Vessel can and cannot know.** ACP has no call that asks an agent
whether it has background work (agent-orchestrator's is its own
extension; neither installed adapter has it). Vessel kills the agent's
process group when it parks it, so a shell the agent left running in the
background goes with it. That is why the default wait is long (Paseo's
hour) and why a person can turn it off for a node.

**When a session is parked.** A structured session that is live and
ready, whose agent can resume and has a harness session id, when for
`park_after` nothing happened in it: no turn runs, no permission or
question waits, the queue is empty or held, no prompt, option change or
agent output arrived, and no shell beside it (protocol 11's terminals,
`session` scope) is running. Reads and subscriptions do not count (an
agent's `wait_turn` or a page left open would otherwise keep every
session awake). Each session's actor keeps its own deadline, re-armed on
each event; there is no sweep.

**Parking.** The actor stops the agent as a move to another profile
does (`SIGTERM`, then `SIGKILL` after 5 s; no `session/close`), keeps its
last options, commands, usage and abilities, and publishes `status:
parked { reason }`. The session stays live: listed, on the board as an
idle running session, its log readable, its queue kept.

**Waking.** Whatever needs the agent wakes it first: a prompt (any
delivery, `session.send`'s too), a queue change that starts a prompt
(`send_now`, `resume` with entries), `session.set_option`, and the new
`session.wake`. The actor publishes `status: starting`, starts the agent
under the session's current profile with `session/load` (its replay
dropped, as for a move), and once it is ready publishes `status: running`
and goes on with what woke it. A `session.move_profile` on a parked
session moves it (it starts under the new profile). `session.kill` and
archiving end a parked session without starting it. A wake that fails
(the agent does not start or `session/load` fails, bounded by the
handshake timeout) leaves the session parked with `reason: wake_failed {
message }`, and the call that woke it answers `spawn_failed`; the next
one tries again.

**Types and calls** (protocol 12):

- `SessionStatus::Parked { reason: ParkReason }`, live. `ParkReason`
  (tagged `reason`): `idle` (after `park_after`), `asked`
  (`session.park`), `wake_failed { message }`, `unknown`.
- `session.park { session }` (operate, ownership-checked): park now.
  `conflict` when it is not idle (a turn runs, something waits, the queue
  moves) or not running; `unsupported` for a terminal session or an agent
  that cannot resume. Already parked: answered with the session.
- `session.wake { session }` (operate, ownership-checked): wake now,
  answered once the agent is ready. Not parked: answered at once.
  `spawn_failed` as above.
- Both answer `SessionInfo`; forwardable; refused on a connection older
  than 12.
- The node's setting: `vesseld --park-after <minutes>` (default 60; 0
  never parks), in the node's `Config`.

**Older peers.** A client of protocol 2 to 11 gets `Running` where the
node has `Parked` (in `SessionInfo` and in a status record's stand-in);
its prompt wakes the session as any prompt does. An older node never
parks; a client asks it `session.park`/`wake` only at 12 (`Call::since`).

**Bounds.** A wake waits for the handshake (its timeout); a stop waits 5
s before `SIGKILL`; a parked session holds no process, only its actor
(its last options, commands and usage in memory). A node restart
interrupts parked sessions as it does running ones (`session.resume`
starts them again); keeping them parked across a restart would need the
node to rebuild their actors at start, which no session does today.

**CLI and agents.** `vessel session park ID`, `vessel session wake ID`;
`vessel sessions` shows `parked`. The agent tools: none new (an agent's
`send_input` wakes a session as a person's prompt does).

ADR 0038 records the choice.

### 2. Parking: the contract's review (2026-11-13)

An independent reviewer (read-only, Opus) found four blockers, six
should-fix and a batch of minor points. What changed; where this
section and the contract above disagree, this one holds:

1. **Prompts could not reach a parked session**: a move's way of
   holding input (`route` empty, `SessionError::Moving`) refuses every
   order. The route becomes typed (`Live(input)`, `Moving`, `Parked`):
   orders pass while parked and the actor wakes for what needs the
   agent; `session.options` answers the last reported options;
   `turn_running` answers false; an answer to a permission or question
   is refused (nothing waits on a parked session). Park is a control
   message of the actor's, so a prompt that arrives while the agent is
   being stopped waits its turn instead of being refused.
2. **A wake would restore old options** (the launch keeps the choices
   from creation or the last move): a wake starts with the session's
   current choices and its harness session id, as a move does.
3. **Older peers and a new status**: `SessionStatus::Parked` would reach
   protocol 10 and 11 peers through replies `Reply::for_protocol` passes
   on unchanged (a mark's answer, an export's manifest, an import) and
   through status records (`since` 2). Taken: the status stays `running`
   throughout; `SessionInfo.parked?: Parked { since, reason }` says it
   (absent otherwise, ignored by older peers), and two records,
   `parked { reason }` and `woken`, both `since 12`, reach older peers as
   `vessel:` log lines. `is_live`, the page's composer, terminals' status
   and the board are untouched; the board shows a parked session where it
   shows an idle one.
4. **The actor's loop**: while parked the agent's exit and outputs are
   not waited on; a kill ends the session at once (`exited`, no code), a
   node stop `interrupted`; park and wake run with the move's flag set,
   so the old agent's disconnect does not end the session; the published
   pid is cleared while parked.
5. **Statuses the compiler does not catch** no longer apply (item 3).
   A node draining for an update does not wait for parked sessions (they
   hold no process; its restart interrupts them as any live session).
6. **A failed wake and automatic prompts**: after `wake_failed`, a
   prompt delivered `queue` (pull request feedback, a schedule, a
   starter's notice) goes into the queue, held `failed`, instead of
   trying again; a person's prompt of another delivery, `session.wake`
   or the queue's `resume` tries again.
7. **"No shell beside it running"** cannot be seen by the actor and
   would keep any session with an open shell awake forever. Replaced by
   what the brief asks ("never while ... a terminal is attached"),
   read as agent-orchestrator's view lease: not while anyone follows the
   session live (a page that has it open, `vessel attach`, an agent's
   `wait_turn`). A port the agent's processes listen on (a dev server it
   started) is a real sign of background work; the node's port scan is
   not wired to sessions' actors yet, so it is left as a known limit.
8. **The wake's launch**: the session's own (its command, environment
   and MCP servers as at its start or last move); a profile or agent
   removed since makes the wake fail (`wake_failed`), and `session.resume`
   after ending it re-resolves them as it does for an ended session.
9. **A move while parked** wakes first (under the current profile), then
   moves; a failed wake fails the move.
10. **The wake's sequence**: the agent's replay and its `ready` are
    dropped, as for a move; the actor publishes `woken` once it is ready.
    A cancel on a parked session holds the queue and answers at once
    (nothing runs). A kill during a wake ends the session; the call that
    woke it answers `ended`.
11. **What restarts the idle clock**: a delivery, a queue change, a
    cancel, an option change, and the agent's timeline records; not its
    stderr, usage or commands reports, model calls, forge views or
    renames (a chatty adapter would never park). The timer is the last
    arm of the actor's biased `select`.
12. **Minor**: `Config.park_after` is a duration (`--park-after`
    minutes on the command line; tests use milliseconds); a wake's
    failure message is cut at 1 KiB; a parked session keeps its record
    ring as a running one does; parked sessions an agent started count
    toward its eight live sessions (ADR 0013); `session.resume` on a
    parked session is `conflict` (it is live); a parent session is parked
    while sessions it started work, unlike Paseo, and a starter's notice
    wakes it (deliberate: the notice is the reason to wake).

Rejected: telling the woken agent in a prompt that its background
processes were stopped (a line in its conversation the person did not
write); a veto from the agent (ACP has none).

### 2. Park an idle agent and wake it (built 2026-11-13)

Protocol 12 (no new version: slice 1 raised it), no schema change, ADR
0038. The design is the contract as its review amended it (above).

- **Wire** (`vessel_proto::parking`): `session.park` and `session.wake`
  (operate, `since 12`, ownership-checked, forwardable, allowed through
  `vessel-web`'s bridge and to agents for their own sessions),
  `ParkSession`, `WakeSession`, `SessionInfo.parked?: Parked { since,
  reason }`, `ParkReason` (`idle`, `asked`, `wake_failed { message }`,
  `unknown`), the records `parked { reason }` and `woken` with their
  `vessel:` log lines for versions 2 to 11.
- **Session runtime** (`vessel-session/src/parking.rs`): `Route`
  (`Live`, `Moving`, `Parked`) replaces the move's empty route; the
  actor's `Parking` (its timer, the last activity, the parked state, the
  harness session id and the current choices, both kept from the agent's
  output); park and wake as control messages, reusing the move's
  `stop_agent` and `start_agent`; a delivery, a queue change, an option
  change or a move wakes first; after a failed wake a `queue` delivery is
  queued and the queue held `failed`; reads answer from what was kept.
  `RuntimeConfig.park_after`.
- **Node**: `service/parking.rs`; the guard's targets; `--park-after`
  (`DEFAULT_PARK_AFTER`, 60 minutes); the drain for an update skips
  parked sessions.
- **CLI**: `vessel session park|wake ID`; `vessel sessions` shows
  `(parked)`; `vessel log` and attach show both records.
- **Agents**: no new tool; `session.park`/`wake` are in the agents'
  policy for their own sessions (the MCP tools stay as they were).

**Checked**: the proto's (a failure message cut on a character, the
reason's round trip and a newer reason read as `unknown`, older peers'
log lines); the session runtime's with the fake agent (an idle agent
parked by its timer with its process gone and its status `running`,
reads not waking it, the next prompt waking it with `session/load` and
the mode a person chose, only two status records; by hand: refused
before the handshake and while a turn runs, a wake of a running agent
doing nothing, parked once quiet and parked twice still parked, a cancel
waking nothing, a kill ending a parked session at once with no exit
code; a followed session never parked over several periods); the
node's end to end
(parked by hand and woken by a prompt over the wire, `parked` and
`woken` in the log, the conversation going on; an agent that cannot
load again: `session.wake` answering `spawn_failed`, the session still
parked saying why, and a `queue` delivery waiting in the queue held
`failed`).

**Not built, and why**: no port or shell check for the agent's
background work (the node's port scan is not wired to sessions' actors);
parked state is not kept across a node restart (a restart interrupts
live sessions anyway); a family of sessions is not kept awake together
(a starter's notice wakes the parent).

**For the page track**: `SessionInfo.parked` shows "parked" beside a
running session (the composer stays enabled: a prompt wakes it); the
`parked` and `woken` records are timeline marks; `session.wake` is a
button for a session whose wake failed (`reason: wake_failed` carries
the message).

## Track A2: contract, 3. Rewind and fork, only where the agent can

**What the references do** (read 2026-11-13 in their sources and in the
two installed adapters, `claude-agent-acp` 0.100.0 and `codex-acp`
2.6.0).

| | agent-orchestrator | GOAT | Paseo |
| --- | --- | --- | --- |
| Fork, Claude and other ACP agents | the current head only, ACP `session/fork` (offered when the handshake advertises `sessionCapabilities.fork`); a fork at a turn is refused ("ACP has no turn-anchored fork") and replaced by a fresh session with up to 24 KiB of transcript as context | Claude through its SDK (`forkSession` with `upToMessageId`); generic ACP: the head only | Claude through its SDK (`forkSession` with `upToMessageId`); a "fork into a new tab" carries the history as a text attachment, not a fork |
| Fork, Codex | its native app-server: `thread/fork { lastTurnId }` | `thread/fork { lastTurnId }`, then a rollback | `thread/fork { beforeTurnId }` |
| Rewind | Codex only, native `thread/rollback { numTurns }`; destructive | in place: Claude's SDK `resumeSessionAt`, Codex `thread/rollback`, OpenCode `session.revert`; generic ACP: a reset | Claude: a fork at the message, then the session swapped to it (the old one kept on disk); Codex: a fork then a rollback; OpenCode: `session.revert` |
| Files | never restored | its own git checkpoints, only in an isolated worktree | Claude's `rewindFiles`, OpenCode's revert |
| The old branch | kept and switchable (edit) | kept | kept on disk |

What reaches Vessel is ACP, and over ACP:

- **No adapter rewinds in place.** Neither `claude-agent-acp` nor
  `codex-acp` has a rollback, revert or truncation method; the only way
  back is a fork.
- **Both fork at a point.** Both advertise `sessionCapabilities.fork` and
  read an anchor in the request's `_meta`: `jetbrains.air.fork { version:
  1, messageId }`, `messageId` being the ACP id of an agent message (the
  one `agent_message_chunk` carries). Claude forks through its SDK up to
  that message; Codex forks after the turn that holds it. An id neither
  finds is an error (`invalid_params`), not a silent head fork. The anchor
  is advertised nowhere: an agent that does not read it forks the whole
  conversation without saying so.
- **Files cannot be restored over ACP** (Claude's `rewindFiles` is used
  only for a dry run). Vessel does not restore them either; ADR 0052
  says why.

**What Vessel offers.** Both only for a structured session whose agent
advertised `sessionCapabilities.fork` (a new ability, `fork`) and has a
harness session id:

- **Fork**: a new session that continues the conversation as it was at a
  point, the original untouched.
- **Rewind**: the same session goes back to a point; what came after
  stays in its log, marked as left behind, and its agent continues from
  the point. The agent's own copy of the longer conversation stays in its
  history (as with Paseo's Claude rewind).

**The point.** A point is the end of a turn: `after: EventSeq`, any record
of a turn that ended (its `turn_ended` record, typically). Vessel sends
the id of that turn's last agent message as the anchor. A point inside a
turn is not offered: Claude would cut between a tool call and its result,
and Codex keeps the whole turn anyway. Without `after`, a fork is of the
whole conversation (ACP's head fork; refused while a turn runs). A
rewind always has a point. A turn whose last agent message is followed
by a tool call (the agent ended on a tool) is not a point either: the
fork would cut the call from its message.

- To have message ids, `AgentEvent::MessageChunk` gains `message?:
  AgentMessageId` (the chunk's ACP `messageId`, absent when the agent
  sends none, and in records written before 12). A turn with no agent
  message id (an older record, an agent that sends none, a reply of only
  tool calls) is not a point: `unsupported`.
- "Edit from here" on a prompt is a rewind to the turn before it, and
  the client puts the prompt back in the composer. A rewind to before the
  first prompt is not offered (that is a new session).

**Checked, not trusted.** Because an agent may ignore the anchor, Vessel
checks every anchored fork: after `session/fork` answers, it starts the
agent again on the new conversation with `session/load` and reads the
replay; the last agent message id of the replay must be the anchor. If it
is not (or the replay has no ids), the fork fails `unsupported` ("the
agent did not fork at the point") and nothing changes: a new session is
not created; a rewound session keeps its old conversation. The check
costs one more agent start (a few seconds) on an operation a person asks
for by hand. A head fork is not checked (any fork of the head is right).

**Fork** (`session.fork { session, after?, place? }`, operate,
ownership-checked on the source; answers the new `SessionInfo`):

- The new session is the caller's, as a `session.create` of theirs
  would be (creator, an agent's limits and credential), with the
  source's harness, current profile (whose home holds the transcript),
  working directory (or `place`, resolved as `session.create` resolves
  it, a new worktree for the fork), and current choices, titled "<source
  title> (fork)". Its agent is started with ACP `session/fork` on the
  source's harness session id instead of `session/new`, then checked as
  above.
- Its log starts with a `forked { from: SessionId, after?: EventSeq }`
  record. The earlier conversation is not copied: a client shows the
  source's records up to `after` above the fork's own (the source may be
  read for as long as it exists); its agent has it whatever happens to
  the source. Search finds the earlier messages in the source.
- The source may be live, parked, ended or archived; it is never woken
  or started (the fork runs in the new session's own process). A fork on
  another node or under another profile is not offered (a copy, then a
  fork, does that).
- An agent creating a fork counts toward its eight live sessions.

**Rewind** (`session.rewind { session, after }`, operate,
ownership-checked; answers `SessionInfo`):

- Only between turns: `conflict` while a turn runs, something waits on
  an answer, the queue moves, or the session is starting or moving. A
  parked session is rewound without being woken first (the fork and the
  check start its agent).
- The actor holds input as for a move, stops the agent, starts it with
  `session/fork` at the point, checks it, and continues on the fork's
  harness session id. On any failure it starts the agent again on the
  old conversation (as a failed move does) and answers the error; the
  log then has no `rewound` record.
- The log gets `rewound { after: EventSeq }`: records after `after` and
  before this one are no longer the conversation. They stay readable and
  searchable. The queue is kept as it was.
- Files are not restored; the reply and the record say nothing about
  them, and the docs say so.

**Types and calls** (protocol 12, no new version):

- `AgentAbilities.fork` (from `sessionCapabilities.fork`; ACP's unstable
  `session/fork`: the crate's `unstable_session_fork` feature gives the
  v1 `ForkSessionRequest` a request impl, so the v1 connection sends it
  as it sends `session/load`).
- `AgentEvent::MessageChunk.message?`, `AgentMessageId` (a handle alias,
  at most 256 bytes; a longer one is dropped).
- `session.fork` (`ForkSession { session, after?, place? }`) and
  `session.rewind` (`RewindSession { session, after }`), `since 12`,
  forwardable, allowed through `vessel-web`'s bridge and to agents on
  sessions they may operate.
- Records `forked { from, after? }` and `rewound { after }`, `since 12`;
  a version 2 to 11 client gets a `vessel:` log line for each.
- Errors: `unsupported` (no `fork` ability, no harness session id, no
  point there, the check failed), `conflict` (as above), `not_found`
  (`after` is not a record of the session), `spawn_failed` (the agent did
  not start or `session/fork` failed).

**Older peers.** Fields and records as above; an older node is never
asked (`Call::since`). A source session's records from before 12 have no
message ids, so they offer head forks only.

**Bounds.** A fork or rewind waits for at most two agent handshakes
(each bounded by the handshake timeout). One fork or rewind at a time per
session (the actor's control channel). No copy of records.

**CLI and agents.** `vessel session fork ID [--after SEQ]` with the
place flags `vessel run` takes (prints the new session), `vessel session
rewind ID --after SEQ`;
`vessel log` shows both records (a fork's says which session and record
its history is in). Agent tools: `fork_session` and
`rewind_session`.

**Not offered, and why**: a point inside a turn (above); restoring
files (no ACP surface; Vessel's own checkpoints would be a feature of
the worktree track); a rewind that deletes what came after (the agent
keeps it; Vessel keeps its records); switching back to a left branch
(a fork from the session's old point does that); forks of terminal
sessions.

ADR 0052 records the choice.

## Track A2: contract, 2b. Parking as a node setting (the coordinator, 2026-11-13)

Parking is not usable from the app while its only control is `vesseld
--park-after`. Paseo has a card in a host's settings
(`packages/app/src/screens/settings/idle-runtime-cleanup-card.tsx`): one
switch, "host-wide, persisted, and applied live by the daemon", read and
changed through the daemon's config (`patchConfig({ idleRuntimeCleanup
})`, `server/daemon-config-store.ts`), the card hidden when the host does
not offer the setting.

**What Vessel adds** (protocol 12, ADR 0038's terms, no schema change):

- `NodeSettings { park_after: ParkAfter }`, `ParkAfter` (tagged `kind`):
  `never`, `idle { minutes }` (1 to 10,080, a week), `unknown` (a newer
  node's; never accepted in a change). One type for the wire and the
  file, so a later setting is one more field.
- `node.settings {}` (read, `since 12`, forwardable as `node.info` is):
  the settings in force. A read-only device sees them.
- `node.set_settings { park_after? }` (admin, `since 12`, not
  forwardable: the node's own administration, as `profile.reload` and
  `node.stop` are): changes what it names, answers the settings in
  force. `invalid` for minutes out of range or `unknown`.
- **Kept** in the data directory as `settings.json` (owner-only, replaced
  atomically, as `agents.toml` is), written before the change is applied,
  so a failed write changes nothing (`internal`) and the value survives a
  restart. At start a saved value wins over `--park-after` (the flag is
  the value of a node that has none saved; a service started with the
  flag would otherwise undo every change at its next restart); a file
  that does not parse is logged and set aside, and the flag's value used.
- **Applied live**: the session runtime holds the value in a watch; each
  structured session's actor reads it whenever it arms its idle timer
  and re-arms when it changes (a session idle for longer than a new,
  shorter time is parked at once). Off: no timer is armed; parked
  sessions stay parked until something wakes them.
- **CLI**: `vessel node settings` prints them; `vessel node settings
  --park-after MINUTES|never` changes them (admin).
- **Older peers**: a client older than 12 never asks; a client asking a
  node older than 12 gets `unsupported`, and hides the card (as Paseo
  hides it when the host lacks the feature). Agents: neither call (agents
  hold no `admin`; the read is not in their policy either).

ADR 0038 gains a paragraph; `PROTOCOL.md` and `README.md` say the same.

### 2b. The setting: the contract's review (2026-11-13)

An independent reviewer (read-only, Opus) found no blocker, five
should-fix and five minor points. Taken (the code and ADR 0038's "The
setting" are the amended design):

1. **One lock over write and apply** (two changes at once would leave the
   file and the sessions disagreeing), the write in a blocking task, as
   `agent.add` holds its lock.
2. **The flag is bounded** (`0..=10080` when parsed), so it never runs a
   time `node.settings` cannot say.
3. **The file keeps only what a person changed** (every field optional,
   the wire's `SetNodeSettings`), laid over the flags; a saved `unknown`
   (a newer build's, read after a downgrade) means the flag's value; a
   file that cannot be read or parsed is logged and left for the next
   change to replace.
4. **The flag is an `Option`**, so the node knows it was given and logs
   when a saved value overrides it.
5. **The setting's receiver is the actor's own field**, beside `parking`
   (the `select` borrows both), its arm before the timer's and not gated
   on `ready`; a park already under way finishes.
6. **Neither call is forwardable** (no caller needs another node's; the
   web bridge connects to each node). An agent may read the setting, as
   it reads `node.info`; the change needs `admin`, which no agent holds.
   `GetNodeSettings {}` with braces, as `ListAgents {}`. The range of
   `idle` is fixed; a longer time would be a new `kind`. A change that
   changes nothing writes nothing.

Not taken: telling connected clients when the setting changes (Paseo's
`daemon_config_changed`); a client reads it when it shows it.

### 2b. Parking as a node setting (built 2026-11-13)

- **Wire**: `node.settings` (read) and `node.set_settings` (admin), `since
  12`, not forwardable, through `vessel-web`'s bridge; `NodeSettings`,
  `SetNodeSettings`, `ParkAfter`, `IdleMinutes`.
- **Store**: `settings.json` (`DataDir::read_settings`/`write_settings`).
- **Session runtime**: `SessionRuntime::park_after`/`set_park_after`, a
  watch every structured actor follows (`Parking::set_after`).
- **Node**: `settings` (in force = saved over the flag; one lock; write
  then apply), `service/settings`; `vesseld --park-after` bounded and
  optional (`Config::park_after_given`).
- **CLI**: `vessel node settings [--park-after MINUTES|never]`.

**Checked**: the proto's (the forms of `park_after`, out of range
refused, a newer kind read as `unknown`); the store's (read back, a bad
file says why); the session runtime's (a runtime that never parked starts
parking a live idle session when the setting changes); the node's end to
end (the flag's value with nothing saved; a change answered, read back,
kept across a restart with the same flag and winning over it; `unknown`
refused; an empty change; an agent reading it and refused the change,
which changes nothing).

**For the settings screen** (E2): Paseo's
`packages/app/src/screens/settings/idle-runtime-cleanup-card.tsx` (a
switch, hidden when the host lacks the setting). Vessel: read
`node.settings` when the card shows; on/off maps to `idle { minutes: 60
}` / `never` (a minutes field is optional beyond Paseo); hide the card
on `unsupported` or `invalid_request` (a node older than 12); a device
without `admin` gets `forbidden` on a change.


### 3. Rewind and fork: the contract's review (2026-11-13)

An independent reviewer (read-only, Opus) read the contract against
Vessel's code and both installed adapters. Every claim about the
adapters held (both advertise `fork`, both read the same anchor, an
unknown id is `invalid_params`; Claude cuts at the last entry of that API
message id, Codex keeps the whole turn; live and replayed ids agree:
Claude's API message id, Codex's item id). Three blockers, seven
should-fix, a few minor points. What changed; where this section and the
contract disagree, this one holds:

1. **Blocker: a rewind was lost by the next move, resume, export or
   fork**, which read the harness session id from the last `ready`
   (`SessionInfo.harness_session` and the store's column), and a
   restarted agent's `ready` is dropped with its replay. `rewound`
   carries the new `harness_session`; the store's projection and the
   session's info take it from there as from `ready` (the column exists:
   no schema change), and the actor's launch resumes it.
2. **Blocker: "no new session if the check fails"** contradicted
   creation, which answers once the actor starts. `session.fork` answers
   at creation, as `session.create` does; a failed check ends the fork
   `failed` ("the agent did not fork at the point"). The fork's actor
   drops the load's replay and publishes its first `ready`, as a session
   started again does.
3. **Blocker: `place`** would put a Claude fork's transcript in the
   source's project folder (the SDK writes it there) where a later move
   or export of the fork, which looks under the fork's own folder, would
   not find it. Dropped: a fork runs in the source's folder.
4. **The check can pass a wrong fork**: (a) when the turns after the
   point have no agent message (cancelled, failed, tool-only), a head
   fork's last agent message is the anchor too: the check also requires
   that no user message follows the anchor's in the replay; (b) Claude's
   TodoWrite and Task tools arrive as `plan`, not as a tool call, and a
   final message of only thinking: a point is a turn whose last agent
   message is followed by nothing but usage and the turn's end (a thought,
   a plan, a tool call or its update disqualify it). The check takes the
   last id seen, skipping chunks without one (Codex's notices).
5. **Simpler, taken**: the point is the turn's, not every chunk's:
   `TurnEnded.point?: AgentMessageId`, set by `vessel-agent`'s driver
   when the rule above holds; chunks carry no id. One rule, one place;
   records stay the size they were.
6. **A parked session is woken first**, then rewound, as a move does.
7. **A failed check in a rewind** stops the fork's agent (it is alive and
   ready) before the old conversation starts again, with the session's
   current choices.
8. **Branch switching**: a fork or rewind at a point uses the harness
   session id in force when that turn ended (the last `ready` or
   `rewound` before it), so a fork from a point a rewind left behind
   takes the old conversation, which still holds it.
9. **Errors**: the agent's `invalid_params` on an anchored fork is
   `unsupported`; a profile removed since is `not_found` naming it; an
   agent whose fork handshake does not advertise `fork` is `unsupported`
   (sessions started before 12 stored no `fork` ability: the fork's own
   handshake decides).
10. **Retention** prunes a session's oldest records first, the fork's
    `forked` record with them (only past 64 MiB of a session's log);
    said in the docs, not kept elsewhere.
11. **Not offered, listed**: merging a fork back (GOAT); a fork that
    carries the transcript as context for agents without `fork`
    (agent-orchestrator, Paseo); a failed turn as a point (its last
    message may be cut); restoring files. `rewound.files_changed` (new):
    how many tool calls left behind edited, deleted or moved files, so a
    client can warn that the files were not put back.
12. **The fork's creation** goes through `session.create`'s own path
    (draining and installing refusals, the caller's profile grant, an
    agent's limits and its `vessel` credential, a routed profile's model
    token, the forge's watch); its MCP servers are resolved for it, never
    copied from the source's launch.

**One start for the check**: `session/fork` then `session/load` of the
fork on the same connection, as the reviewer suggested: both adapters
load a session that is already open (`claude-agent-acp`'s `loadSession`
reads the transcript again and replays it; `codex-acp` reattaches and
replays), so a fork costs one agent start, not two.

### 3. Rewind and fork (built 2026-11-13)

Protocol 12 (no new version), no schema change, ADR 0052. The design is
the contract as its review amended it (above).

- **Wire**: `session.fork` and `session.rewind` (operate, `since 12`,
  forwardable, through `vessel-web`'s bridge and to agents),
  `ForkSession`, `RewindSession`, `AgentAbilities.fork`, `AgentMessageId`
  (a handle alias), `TurnEnded.point?: TurnPoint { message, conversation
  }`, the records `forked { from, after? }` and `rewound { after,
  harness_session, files_changed }` with `vessel:` log lines for versions
  2 to 11.
- **Agent** (`vessel-agent`): the crate's `unstable_session_fork`;
  `AgentSetup.fork` (`ForkFrom`); the handshake's `session/fork` with the
  anchor in `_meta.jetbrains.air.fork`, then, for a point, `session/load`
  of the fork on the same connection; the driver's `tail` (the last agent
  message, what followed it): a turn's point at `TurnDone`, and the check
  of a fork's replay at `Started` (`ForkRefusal::NotAtPoint`).
- **Session runtime**: `LaunchOptions.fork` (`ForkStart`): the fork's
  `forked` record first, its check's replay dropped, the fork cleared at
  `ready`; `forking` (the actor's rewind: input held, the agent stopped
  and started on the fork, the old conversation again on any failure,
  `rewound` with the new id); `SessionHandle::rewind`; `RewindRefusal`,
  `ForkRefused`, `RewindFailed`.
- **Store**: the projection takes `rewound.harness_session` as it takes
  `ready`'s.
- **Node**: `service/forking` (the point from the `turn_ended` record;
  the fork through `create_starting`, `session.create`'s path; the
  rewind's count of file changes left behind from the stored records).
- **CLI**: `vessel session fork ID [--after SEQ]`, `vessel session rewind
  ID --after SEQ`; `vessel log` says `fork or rewind --after N` on a turn's
  end that has a point, and shows both records.
- **Agents**: `fork_session`, `rewind_session` (45 tools).
- **`fake-acp`**: message ids on `plain`'s reply, `session/fork` reading
  the anchor as the real adapters do (`FAKE_ACP_NO_FORK`,
  `FAKE_ACP_FORK_IGNORES_POINT`), a fork's replay ending where it was
  made.

**Checked**: the driver's tail (a point only when nothing followed the
message; an id too long; a replay that a user message follows); the
node's end to end with the fake agent: a fork after the first of two
turns runs on `fork~<conversation>~<message>`, its log starts with
`forked`, the check's replay is not in it, the source untouched, and a
fork of the whole conversation; a rewind to the first turn answers the
new conversation, logs `rewound`, and goes on there, and a fork from the
second turn, which the rewind left behind, still takes the old
conversation; an agent that ignores the point: the rewind refused
`unsupported` with the old conversation going on and no `rewound`, and a
fork of it ending `failed` "did not fork at the point"; a record that is
not a turn's end (`unsupported`), one that does not exist (`not_found`),
a running turn (`conflict` for a rewind and a whole fork), an agent
without `session/fork` (`unsupported`, and the session goes on).

**Not built, and why**: the reviewer's list (merging back, a transcript
carried as context, a failed turn as a point, files); checking a real
`codex-acp` fork end to end (the adapter keeps item ids in `thread/fork`
per its source, not run here: no prompt to a real agent tonight).

**For the conversation track** (C): Paseo's rewind is
`packages/app/src/components/rewind/rewind-menu.tsx` and
`composer-restore.tsx`, from a message's actions (`components/message.tsx`;
`agent.rewind.request`, server `agent/rewind/rewind.ts`), and GOAT's "Edit from here"
(`apps/web/src/components/chat/useTimelineRewind.ts`). Vessel: a turn's
end with `point` offers "Fork from here" (`session.fork { after: seq }`)
and "Rewind to here" (`session.rewind`); "Edit from here" on a prompt is
a rewind to the turn before it with the prompt back in the composer; a
fork's view shows its source's records up to `after` above its own
(`forked`); a `rewound` record folds the records between `after` and it,
and says `files_changed` when it is not 0.
