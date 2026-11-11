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

- `ProjectName`: trimmed, 1 to 80 characters, no control characters.
- `BranchName`: git's `check-ref-format --branch` rules, checked here:
  not empty, at most 255 bytes, no ASCII control or space, none of
  `~ ^ : ? * [ \`, no `..`, no `@{`, not `@`, no component starting with
  `.` or ending with `.lock`, no leading, trailing or doubled `/`, not
  ending with `.`, and **no leading `-`** (it would read as an option).
- `RefName`: the same rules; names a branch, a remote-tracking branch
  (`origin/main`) or a tag, as a base or a starting point.
- `CommitId`: 7 to 64 lowercase hexadecimal digits (SHA-1 or SHA-256).
- `RemoteName`: `check-ref-format` rules for one component, no leading
  `-`.
- `RelPath`: a path inside a root, `/`-separated whatever the node's
  platform: not absolute, no drive or UNC prefix, no backslash, no NUL,
  no empty, `.` or `..` component, no component equal to `.git` (ASCII
  case ignored), at most 4,096 bytes and 255 per component. The empty
  path is the root itself (accepted only where a call says so).
- `CloneUrl`: `https://`, `http://`, `ssh://`, `git://`, or the scp form
  `user@host:path`; no whitespace or control character, no leading `-`,
  no `<transport>::` form (`ext::`, `fd::`), no `file://` and no local
  path (a folder on the node is `project.add`), no password in an
  `http(s)` URL (it would be written into `.git/config` and echoed in
  errors).
- `CommitMessage`: not empty once trimmed, at most 64 KiB, no NUL.
- `ProjectCommand`: one shell command from the project's file, not empty,
  at most 4 KiB, no NUL.
- `CommandsDigest`: 64 lowercase hexadecimal digits.
- `FindQuery` (1 to 256 characters), `GrepQuery` (1 to 1,000).

Projects:

- `ProjectInfo { id, name, root, git, state, created_at, revision }`.
  `root` is the absolute folder on the node (the repository's top level
  when the folder added is inside one); `git` says whether it is a git
  repository (a folder that is not has files but no git and no
  worktrees). `revision` grows by one at every change, for waiting.
- `ProjectState`: `ready`; `cloning { stage, percent? }` with `stage`
  one of `connecting`, `counting`, `compressing`, `receiving`,
  `resolving`, `checking_out`; `clone_failed { reason, message }` with
  `reason` one of `needs_credential`, `not_found`, `network`,
  `timed_out`, `other` (`message` has credentials removed).
- `ProjectList { git, projects }`; `git` is `GitSupport`: `found {
  version }`, `too_old { version, minimum }` or `missing`. This is where
  the node reports the git it found, checked once at start.
- `ProjectCommands`: `none` (no file, or a file naming no command),
  `listed { setup, teardown, digest, approved }`, or `invalid { message
  }` (the file does not parse; nothing in it runs).

Worktrees:

- `WorktreeBranch`: `new { name?, base? }` (a new branch, named
  `vessel/<8 hex>` when no name is given, from `base`: a `WorktreeBase`,
  `ref { name: RefName }` or `commit { id }`, the project's default branch
  when absent) or `existing { name }`.
- `SetupChoice`: `run` (the default) or `skip`.
- `WorktreeInfo { id, project, path, branch?, created_branch, base?,
  created_at, created_for?, sessions, setup, kept? }`: `branch` absent
  when detached; `created_branch` says Vessel made it (only such a
  branch is ever deleted); `created_for` the session it was created for;
  `sessions` the live sessions in it; `kept` why tidying left it (below).
- `SetupState`: `none` (nothing to run), `skipped`, `running {
  started_at }`, `succeeded { finished_at }`, `failed { failure,
  finished_at }` with `failure` one of `exit { command, code? }` (the
  index of the command that failed), `timed_out`, `spawn`.
- `KeptReason`: `uncommitted { files }`, `unique_commits { commits }`,
  `operation { operation }` (`merge`, `rebase`, `cherry_pick`,
  `revert`, `bisect`), `inspection_failed`.
- `SetupOutput { from, bytes, next, skipped, truncated, setup }`: a page
  of the setup and teardown output (stdout and stderr together, as
  `TerminalBytes`), `skipped` the bytes before `from` no longer kept,
  `truncated` when the log reached its limit and stopped.
- `Snapshot { worktree, branch?, commit, head, created_at }`: kept at
  `refs/vessel/snapshots/<worktree id>` in the project's repository.

Sessions (the only shared types touched):

- `SessionSpec.place?: SessionPlace`: `project { project, dir?: RelPath
  }` (the folder as it is), `worktree { worktree }` (an existing one),
  `new_worktree { project, branch, setup }`. With a place, `cwd` must be
  absent; the node fills `spec.cwd` with the folder the place resolves
  to, so a client that predates places still sees where the session
  runs.
- `SessionInfo.workspace?: SessionWorkspace { project, worktree?,
  branch? }`.

Git (`WorkspaceRoot`: `project { project }` or `worktree { worktree }`;
every git and file call names one):

- `GitStatus { branch?, head?, upstream?, base?, operation?, files,
  more }`: `upstream { name, ahead, behind }`; `base { name, ahead, behind
  }` for a worktree, against the base it was made from; `files` each
  `{ path, old_path?, index, worktree }` with `FileState` one of
  `unmodified`, `modified`, `added`, `deleted`, `renamed`, `copied`,
  `type_changed`, `untracked`, `ignored`, `conflicted`.
- `DiffScope`: `unstaged` (working tree against the index, untracked
  files as added), `staged` (index against `HEAD`), `uncommitted`
  (working tree against `HEAD`, untracked as added), `base { base?:
  WorktreeBase }` (working tree against the merge base of `HEAD` and the
  base: committed and uncommitted together), `commit { id }` (against
  its first parent).
- `DiffSummary { from?, files, more }`: `from` the commit compared
  against; each `FileChange { path, old_path?, status, additions?,
  deletions?, binary }` (counts absent for binary files).
- `FileDiff { path, old_path?, status, content, more_from? }`, `content`
  one of `hunks { hunks }`, `binary`, `too_large { bytes }`. A `Hunk {
  old_start, old_lines, new_start, new_lines, section?, lines }`; a
  `DiffLine { kind, text, cut }`, `kind` one of `context`, `added`,
  `removed`, `no_newline`. `more_from` is the index of the first hunk not
  sent.
- `BranchList { branches, more }`, `Branch { name, remote?, current,
  default, head, upstream?, worktree? }` (`worktree` the folder that has
  it checked out). `CommitPage { commits, more }`, `CommitInfo { id,
  parents, author, time, subject }`. `StashList { entries, more }`,
  `StashEntry { index, message, branch?, commit }`.
- `GitRefusal` (what a write expectedly refuses with, typed, so no
  client reads git's text): `needs_credential`, `timed_out`,
  `non_fast_forward`, `diverged`, `would_overwrite { paths }`,
  `conflicts { paths }`, `nothing_to_commit`, `no_identity`,
  `hook_failed { output }`, `no_upstream`, `detached`,
  `checked_out_elsewhere { path }`, `nothing_to_stash`, `operation {
  operation }`, `remote_failed { message }` (credentials removed), and
  `unknown` for a kind a newer node sends.
- Outcomes: `CommitOutcome` (`committed { commit }`, `refused { refusal
  }`), `PushOutcome` (`pushed { remote, branch, upstream_set }`,
  `refused`), `PullOutcome` (`pulled { from, to }`, `up_to_date`,
  `refused`), `SwitchOutcome` (`switched { branch }`, `refused`),
  `StashOutcome` (`done { entries }`, `refused`).
- `PathSet`: `all` or `paths { paths }` (at most 1,000).
- `CommitStage`: `staged` (what is staged), `tracked` (`add -u` first),
  `everything` (`add -A` first).
- `StashAction`: `save { message?, untracked }`, `apply { index }`,
  `pop { index }`, `drop { index }`.

Files:

- `DirListing { entries, more }`, `DirEntry { name, kind, size?,
  ignored }`, `kind` one of `file`, `dir`, `symlink`, `other`.
- `FileVersion { size, modified }`, `modified` in nanoseconds since the
  epoch: Paseo's revision. `FileRead { version, content }` with
  `content` one of `text { text, next? }` (`next` the offset where the
  rest starts) or `binary`.
- `WriteExpect`: `absent` (create only), `version { version }`
  (replace only that version), `any`.
- `FoundFiles { matches, indexed, complete, superseded }`, each match `{
  path, positions }` (the characters that matched, for highlighting).
- `GrepMode`: `literal`, `regex` (extended); `GrepCase`: `sensitive`,
  `insensitive`, `smart` (sensitive when the query has a capital).
  `GrepResult { matches, more }`, each `{ path, line, column, text, cut
  }`.

### Calls

Every method is protocol 10, forwardable through `fleet.forward`, and in
`vessel-web`'s list. "Agents" says whether an agent principal (ADR 0013,
a local one or one whose call was delivered) may call it; the guard
refuses the others with `forbidden` before the call runs, since an agent
already has its own file and git tools and only needs what orchestration
needs.

| Method | Scope | Agents | Request | Reply |
|---|---|---|---|---|
| `project.list` | read | yes | `ListProjects {}` | `ProjectList` |
| `project.get` | read | yes | `GetProject { project, after? }` | `ProjectInfo` |
| `project.add` | operate | no | `AddProject { path, name? }` | `ProjectInfo` |
| `project.clone` | operate | no | `CloneProject { url, parent, name? }` | `ProjectInfo` |
| `project.init` | operate | no | `InitProject { path, name? }` | `ProjectInfo` |
| `project.remove` | operate | no | `RemoveProject { project }` | `Ack` |
| `project.commands` | read | no | `GetProjectCommands { project, base? }` | `ProjectCommands` |
| `project.approve` | operate | no | `ApproveCommands { project, digest, approved }` | `ProjectCommands` |
| `worktree.list` | read | yes | `ListWorktrees { project? }` | `Vec<WorktreeInfo>` |
| `worktree.create` | operate | no | `CreateWorktree { project, branch, setup }` | `WorktreeInfo` |
| `worktree.output` | read | no | `ReadSetupOutput { worktree, from, wait? }` | `SetupOutput` |
| `worktree.setup` | operate | no | `RunSetup { worktree }` | `WorktreeInfo` |
| `worktree.remove` | operate | no | `RemoveWorktree { worktree, snapshot }` | `WorktreeRemoved { snapshot? }` |
| `worktree.snapshots` | read | no | `ListSnapshots { project }` | `Vec<Snapshot>` |
| `worktree.drop_snapshot` | operate | no | `DropSnapshot { project, worktree }` | `Ack` |
| `git.status` | read | no | `GetStatus { root }` | `GitStatus` |
| `git.changes` | read | no | `GetChanges { root, scope }` | `DiffSummary` |
| `git.diff` | read | no | `GetFileDiff { root, scope, path, from_hunk? }` | `FileDiff` |
| `git.branches` | read | no | `ListBranches { root }` | `BranchList` |
| `git.log` | read | no | `ReadLog { root, from?: RefName, skip, limit }` | `CommitPage` |
| `git.stashes` | read | no | `ListStashes { root }` | `StashList` |
| `git.stage` | operate | no | `Stage { root, paths }` | `GitStatus` |
| `git.unstage` | operate | no | `Unstage { root, paths }` | `GitStatus` |
| `git.discard` | operate | no | `Discard { root, paths, untracked }` | `GitStatus` |
| `git.commit` | operate | no | `Commit { root, message, stage }` | `CommitOutcome` |
| `git.push` | operate | no | `Push { root, remote?, set_upstream }` | `PushOutcome` |
| `git.pull` | operate | no | `Pull { root }` | `PullOutcome` |
| `git.switch` | operate | no | `Switch { root, branch, create? }` | `SwitchOutcome` |
| `git.stash` | operate | no | `StashChanges { root, action }` | `StashOutcome` |
| `file.list` | read | no | `ListDir { root, dir, ignored }` | `DirListing` |
| `file.read` | read | no | `ReadFile { root, path, offset, limit? }` | `FileRead` |
| `file.write` | operate | no | `WriteFile { root, path, text, expect }` | `FileVersion` |
| `file.create_dir` | operate | no | `CreateDir { root, path }` | `Ack` |
| `file.rename` | operate | no | `RenamePath { root, from, to }` | `Ack` |
| `file.delete` | operate | no | `DeletePath { root, path, recursive }` | `Ack` |
| `file.find` | read | no | `FindFiles { root, query, limit? }` | `FoundFiles` |
| `file.grep` | read | no | `SearchFiles { root, query, mode, case, dir?, limit? }` | `GrepResult` |

`session.create` takes a `place` (above) from agents too: `project` and
`new_worktree` on any project, `worktree` only on a worktree created for
the agent's own session or a session it may operate (ADR 0013's
ownership). A session started by an agent in a new worktree runs setup
only if a person approved the commands; an agent cannot approve.

What each call does, and its failures. Every call can also answer
`forbidden` (scope or agent), `not_found` for a project, worktree or
snapshot the node does not have, `unsupported` when the node has no
usable git and the call needs it (`missing` or `too_old`), and
`internal`.

- `project.list`: every project, oldest first.
- `project.get`: one; with `after`, waits (at most 30 s) until its
  `revision` is past `after`, then answers it as it is (as `session.read`
  waits).
- `project.add`: `path` absolute and an existing folder. Inside a git
  repository the project's root is the repository's top level. A root
  already known answers that project (adding twice is opening).
  `invalid_request` for a relative path or one that is not a folder.
- `project.clone`: answers at once with the project `cloning`; the clone
  runs in the node (it continues when the caller goes away), into
  `<parent>/.<name>.vessel-clone-<8 hex>`, renamed to `<parent>/<name>`
  when it succeeds (`name` defaults to the URL's last component without
  `.git`). `invalid_request` for a `parent` that is not an existing
  absolute folder; `conflict` when `<parent>/<name>` exists. A failure
  leaves the project `clone_failed` and removes the temporary folder.
- `project.init`: creates the folder if needed (it must be absent or
  empty: `conflict` otherwise) and runs `git init`.
- `project.remove`: forgets the project; never deletes its files.
  `conflict` while it has worktrees or live sessions. A project still
  cloning is cancelled: git is stopped and its temporary folder removed.
- `project.commands`: the commands the project's file names, at `base`
  (a `WorktreeBase`, read with `git show`) or in the project's folder as
  it is, with whether that set is approved. Read by a client to show a
  person what would run before asking to approve it.
- `project.approve`: records (or, with `approved: false`, withdraws) the
  approval of the set whose digest is `digest`, for this project, and
  answers the commands of the project's folder with their approval. A
  digest is approved as given, not looked up: the set it names may be on
  a branch the folder does not have checked out. Approving a digest that
  no file has runs nothing.
- `worktree.list`: every worktree, or a project's.
- `worktree.create`: `git worktree add` into
  `<data dir>/worktrees/<project folder name>/<8 hex>`, then setup in the
  background (it continues when the caller goes away). `unsupported` for
  a project without git; `conflict` when the project is not `ready`, has
  no commit yet, the new branch exists, the existing branch is checked
  out in another worktree, or `setup` is `run` and the base's commands
  are listed but not approved (nothing is created then: the client asks
  `project.commands` with that base, a person approves, the client asks
  again); `invalid_request` for a base that does not resolve to a
  commit or an existing branch that does not exist.
- `worktree.output`: a page of at most 64 KiB from `from`; with `wait`
  (seconds, at most 30), waits for more output or the end of setup.
- `worktree.setup`: runs setup again (after an approval, a failure, or
  `skip`). `conflict` while setup or teardown runs, or when the commands
  are not approved.
- `worktree.remove`: removes the worktree, its setup log, and its branch
  when Vessel created it, if nothing would be lost (the rule below);
  otherwise, with `snapshot: true`, keeps a snapshot first and removes it
  anyway; without, `conflict` with what would be lost. Teardown runs
  first when its commands are approved. `conflict` while a live session
  runs in it or setup runs.
- `worktree.snapshots`, `worktree.drop_snapshot`: a project's snapshots;
  deleting one deletes its ref. A snapshot is restored by
  `worktree.create` with `base: commit { id: snapshot.commit }`.
- `git.status`, `git.changes`, `git.diff`, `git.branches`, `git.log`,
  `git.stashes`: `unsupported` for a project without git;
  `invalid_request` for a path the rules refuse; `git.diff` answers
  `not_found` for a path with no change in that scope.
- `git.stage`, `git.unstage`, `git.discard`: answer the status after.
  `discard` restores tracked paths from `HEAD` (both index and working
  tree) and, only with `untracked: true`, deletes the untracked ones it
  names (never ignored files). `unavailable` when another git process
  holds the index lock.
- `git.commit`: in the root's repository with its configured identity
  (`no_identity` when it has none); hooks run as they would for the
  user. `git.push`: the current branch to its upstream, or to `remote`
  (default `origin`) with `set_upstream`; never forced. `git.pull`:
  fast-forward only (`diverged` otherwise). `git.switch`: an existing
  branch, or a new one from `create`; a worktree's `branch` follows.
  `git.stash`: as named. Every refusal in `GitRefusal` is an outcome,
  not an error.
- `file.list`: one folder's entries, sorted folders first, then by name;
  `.git` left out; `ignored` marks entries git ignores (asked with `git
  check-ignore`), listed only when `ignored` is asked. `invalid_request`
  for a path that is not a folder.
- `file.read`: a page of at most 1 MiB (256 KiB by default) from
  `offset`, cut back to a character boundary; `binary` when the first
  8 KiB hold a NUL byte or the page is not UTF-8. `invalid_request` for a
  folder.
- `file.write`: at most 1 MiB of text, written to a temporary file
  beside the target and renamed over it (keeping the old file's
  permissions); never through a symbolic link. `conflict` when `expect`
  does not hold; `not_found` when the parent folder is missing.
- `file.create_dir`, `file.rename` (never over an existing path:
  `conflict`), `file.delete` (a folder only when empty, unless
  `recursive`; never the root; a symbolic link is removed, not what it
  points to).
- `file.find`: by name over an index of the root's files (below);
  `superseded: true` with no matches when a newer `file.find` from the
  same connection replaced it before it finished.
- `file.grep`: `git grep` over tracked and untracked files that are not
  ignored (`--no-index` for a folder without git), binary files left out.

**Every path is checked.** A `RelPath` that decodes cannot be absolute
or climb; the node then joins it to the root, resolves the result with
symbolic links followed (for a target that does not exist yet, its
parent), and refuses (`invalid_request`) a result outside the root's
resolved folder, or inside its `.git`. A write, rename or delete never
follows a symbolic link in the last component. These are checks inside
the node, not a sandbox: a process of the same user can still race them
(as ADR 0016 says for the language servers).

### Where a session runs, and tidying

- `session.create` with a `place`: `project` runs in the project's folder
  (or `dir` inside it); `worktree` in that worktree; `new_worktree`
  creates one as `worktree.create` does. When the worktree's setup is
  running, the call waits for it (at most the setup limit); when setup
  failed, or the commands need approval, it fails with `conflict` (the
  worktree stays, for `worktree.setup`). The session's log starts with
  `vessel:` notices: the folder, the branch and base, and setup's
  result.
- **Tidying.** When the last live session in a worktree ends (exited,
  failed or killed; not interrupted by the node stopping, which a
  restart may resume), teardown runs if its commands are approved, and
  then the worktree is removed only when nothing would be lost:
  1. no merge, rebase, cherry-pick, revert or bisect in progress;
  2. no uncommitted change: `git status --porcelain --untracked-files=all`
     prints nothing (ignored files, such as build output, are not
     counted: they are what the ignore files say can be made again);
  3. no commit reachable from `HEAD` that is on no other branch, no
     remote-tracking branch and no tag: when Vessel created the branch it
     is deleted with the worktree, so its commits must be elsewhere.
     A branch Vessel did not create is never deleted, so this step only
     applies to Vessel's own branches.
  Otherwise the worktree stays, with `kept` saying why, until a person
  removes it (`worktree.remove`, with a snapshot or after committing and
  pushing).
- A **snapshot** is one commit whose parent is `HEAD` and whose tree is
  the working tree as `git add -A` would stage it (untracked files
  included, ignored ones not), made through a temporary index so the
  worktree's own index is untouched, and kept at
  `refs/vessel/snapshots/<worktree id>`. The ref also keeps every commit
  the branch had. Refused (`conflict`) above 100 MiB for one changed file
  or 256 MiB in all, and while an operation is in progress. The index's
  staged-but-different state is not kept apart.

### The project's file and its trust

- `vessel.toml` at the repository's root (the folder's, for a project
  without git). TOML, as the node's other files are (`profiles.toml`,
  the node book), and comments are allowed. Only `[worktree]` is read
  now:

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
  environment plus `VESSEL_PROJECT_ROOT`, `VESSEL_WORKTREE` and
  `VESSEL_BRANCH`, standard input closed, as a process group stopped
  together. The first failing setup command stops setup. Limits: 30
  minutes for a setup, 5 for a teardown.
- **Trust.** A repository's commands are someone else's code. They run
  only when a person has approved that exact set for that project: the
  digest is SHA-256 over the setup and teardown lists, so any change to
  either (a new branch, a pull, an agent editing the file in its
  worktree) is a new set, asked again. Teardown reads the worktree's
  file as it is at teardown and runs only an approved set; otherwise it
  is skipped with a notice. `project.approve` needs `operate` and is
  refused to agents; a read-only device cannot approve. Nothing runs
  before an approval.

### What an older client and an older node see

- A client asks no protocol 10 method of a node that speaks 9 or less:
  it fails with `NotInProtocol` before sending, as every newer method
  does. A `session.create` with a `place` needs 10 (`Call::since`): an
  older node would start the session in its home folder instead.
- A protocol 9 client reading a protocol 10 node sees sessions with a
  `cwd` (the node filled it) and ignores `place` and `workspace`.
- No event is added, so no stand-in is needed.

### Bounded, and by how much

| What | Bound | Beyond it |
|---|---|---|
| Every git run | 30 s (local), 120 s (push, pull), 1 h (clone); 16 MiB of stdout, 64 KiB of stderr | killed as a group; `timed_out`, or `internal` naming the limit |
| git processes at once | 8 per node | the next waits for a slot |
| Status entries, files in a summary | 5,000 | `more: true` |
| A file's diff | 5,000 lines per call; 4 KiB per line; 4 MiB of git output per file | `more_from`; `cut`; `too_large` |
| Branches; a log page; stashes | 1,000; 200 commits; 100 | `more: true` |
| Folder listing | 5,000 entries | `more: true` |
| File read; write | 1 MiB a page; 1 MiB | `next`; `invalid_request` |
| Find by name | 200 results; an index of 500,000 paths per root, 8 roots, idle ones dropped after 5 minutes | `complete: false` |
| Search in files | 500 matches; 512 characters a line; 4 MiB of git output | `more: true`; `cut` |
| Setup and teardown output | 1 MiB per worktree on disk; 64 KiB a page | `truncated` |
| A long poll (`project.get`, `worktree.output`) | 30 s | answers as it is |

Store: schema 7 adds `projects`, `worktrees` and `approvals` (the
next free number; the coordinator renumbers if track A takes it too).

### Not in this track (wave 2), and the doors left open

- A forge (pull requests): `WorktreeInfo` and `GitStatus` gain fields;
  nothing here assumes there is none.
- Checkpoints per turn: snapshots are built the way a checkpoint would
  be (a commit through a temporary index under a Vessel ref).
- Carrying a worktree when a session moves to another machine: the
  session's `workspace` names what to carry.
- Scripts and services: `vessel.toml` keeps every key it does not read,
  and the approval digest grows with what runs.

## Left open

- The desktop and phone shells (ADR 0020, still proposed). All three
  references ship a phone app; Vessel's answer until the decision is the
  page on a phone plus push (track E).
