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

## Left open

- The desktop and phone shells (ADR 0020, still proposed). All three
  references ship a phone app; Vessel's answer until the decision is the
  page on a phone plus push (track E).

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
