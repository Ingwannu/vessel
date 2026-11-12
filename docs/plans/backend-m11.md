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

## Left open

- The desktop and phone shells (ADR 0020, still proposed). All three
  references ship a phone app; Vessel's answer until the decision is the
  page on a phone plus push (track E).

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

The decisions that could have gone another way are ADR 0025 (0023 is
track B's; 0024 is left for track A). A reviewer read this contract
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

## Track W: progress

Branch `backend/m11-page`. The contract above (reviewed) holds, with the
deviations listed below. ADR 0025 is taken.

### Built

- **Preferences kept by the bridge** (ADR 0025): `vessel_web::keep`
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
