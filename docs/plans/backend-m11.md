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
  repeat or a gap is `invalid_request`), the data at most `chunk`
  bytes, never past `size`.
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

## Left open

- The desktop and phone shells (ADR 0020, still proposed). All three
  references ship a phone app; Vessel's answer until the decision is the
  page on a phone plus push (track E).

## Track A: progress

Built in slices, each committed with its tests; a call whose slice is
not built yet answers `unsupported` ("not built yet") so a client sees
the same refusal an older node gives.

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
- Next: questions; marks, the board and delete; start again;
  attachments; the CLI and the agent tools.
