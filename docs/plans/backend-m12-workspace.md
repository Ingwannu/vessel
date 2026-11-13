# Milestone 12, track F: the workspace in the native app

Projects, worktrees, the commands approval, the code panel (Changes and
Files) and where a new session runs, brought from the page into the app;
then what the node supports since protocol 11 and no client draws (pull
requests and the forge, shells beside a session, scripts and services,
ports and previews, file create, rename and delete). Branch
`backend/m12-app-workspace`, off the desktop lead's `backend/m12-spike`.

The page's inventory names the area: section 9 ("Projects, worktrees,
git, files"), the sidebar's Projects (section 3) and the workspace half of
the new session (section 5).

## Order of work

1. View logic, without GPUI, ported from the page's `web/src/data/workspace`
   with the page's test cases as its tests (ADR 0036, "behaviour").
2. The node calls in `link`.
3. Screens in `app/src/areas/workspace/`, once the lead's layout commit
   (areas, routes, the string table) is on `backend/m12-spike`.
4. Beyond the page.

## 1. View logic (`spikes/native/view/src/workspace/`)

One module per subject, as the page has them:

| Module | The page's | What it decides |
| --- | --- | --- |
| `place` | `place.ts` | the composer's place (typed folder, a project's folder, a new worktree, an existing branch), the spec's part, the worktree's branch, why Send is blocked |
| `branch_name` | `branch-name.ts` | git's branch-name rules, in its order |
| `start` | `features/compose/start-worktree.ts` | the worktree start's steps and answers (the walk is the link's) |
| `projects` | `project-store.ts` | a machine's projects view, a project taken or replaced, the deepest project holding a folder |
| `root` | `root.ts` | which root a session's panel reads |
| `pace` | `pace.ts` | when a long read waits before asking again |
| `setup_log` | `setup-log.ts` | setup's output as lines: escapes dropped, `\r` and erase applied, UTF-8 across pages, the last 2,000 lines |
| `command_changes` | `command-changes.ts` | the set asked for against the set approved last (or the folder's) |
| `worktrees` | `worktree-words.ts` (`snapshotCovers`), `remove-worktree-dialog.tsx` | what a snapshot keeps, the removal's choices |
| `git_quick` | `git-quick.ts` | the git button's quick action and why an action is off |
| `changes` | `changes.ts` | the comparisons, a file's letter and tone, the Changes list's rows |
| `diff_rows` | `diff-rows.ts` | a file's diff as rows with both sides' numbers, pages added with stable keys |
| `file_tree` | `file-tree.ts` | the Files tree's rows from the folders listed |
| `find` | `find.ts` | a field's search key |
| `save` | `save.ts` | the save guard and what a refused write shows |
| `reference` | `reference.ts` | a selection as agent-orchestrator's captured-code reference |
| `syntax` | `syntax.ts`, `syntax-languages.ts` | the lexer and its languages |
| `size` | `size.ts` | a size in words |
| `problem` | `outcome.ts`'s `Problem` | a call that did not answer, typed |
| `words` | every `*-words.ts`, and the words of the modules above | the area's sentences, English and Korean |

Checked: `cargo test -p vessel-app-view`, 76 tests, every case of the
page's `place.test.ts`, `setup-log.test.ts`, `pace.test.ts`,
`command-changes.test.ts`, `words.test.ts`, `diff-rows.test.ts`,
`panel-model.test.ts`, `save.test.ts`, `syntax.test.ts` and `find.test.ts`,
with the page's inputs and the page's expected outputs, and a few cases of
its own where the port had a choice to make (a broken UTF-8 sequence, an
escape's final outside ASCII, a size just under ten units).

### Differences from the page, and why

- **Typed values, not strings, out of the logic.** The page's functions
  return English sentences; here they return a closed set
  (`PlaceProblem`, `Blocked`, `BranchNameProblem`, ...) and the words are
  chosen by language. The page's string assertions run against the
  English table, so the same inputs give the same English.
- **Rows by index.** The page keys a Changes row and a tree row by a string
  holding the path and holds references to its objects; here a row names
  its file and its diff row by index, and a diff row's key is a small
  value (`DiffKey`). Same order, same rows, nothing copied per row.
- **Columns in characters.** The setup log and the reference's cut count
  characters where the page counts UTF-16 units; they differ only past the
  Basic Multilingual Plane, where characters are the right count.
- **Floating point.** The app's lints deny it: `size_word` works in whole
  tenths and rounds a half up, as `toFixed(1)` does (checked at the
  boundary just under ten units, where the page keeps its decimal).
- **The save's two calls are `FnOnce`.** `save_text` takes the write and
  the read as closures that can be called once, so "at most one write" is
  a property of the type, not of a test.
- **Words in a table of the area's own.** `words::WorkspaceWords` is a
  trait with one implementation per language (`En`, `Ko`), in files of
  this track; the lead's `Text` table stays for the shell's words. The
  same guarantee (a sentence in one language and not the other does not
  compile), without a hundred fields in a file every track edits. The
  language picks the table once the lead's `Lang` is on the branch.

### The syntax highlighter (the brief's question)

The pinned `gpui-component` 0.8.1 has a highlighter on tree-sitter (MIT)
and a code editor; tree-sitter grammars are MIT or Apache-2.0. Not taken:
ADR 0036 makes the page's behaviour the specification, and a different
highlighter colours the same file differently on the two surfaces. The
page's lexer is ported rule for rule instead (no new dependency, a token is
a byte range of the line). If the page moves to grammars, both move
together. The decision log is in `syntax/mod.rs`.

## 3. Screens (`spikes/native/app/src/areas/workspace/`)

| File | Screen | Reference |
| --- | --- | --- |
| `projects.rs`, `detail.rs`, `approval.rs`, `remove.rs` | Settings > Projects: a machine's projects (the machine Settings' picker chose), add (folder, clone with progress, new), a project's page (commands and their approval, worktrees, snapshots, removal) | Paseo's `projects-screen.tsx` and `project-settings-screen.tsx` |
| `place.rs`, `place_rows.rs` | the new chat's place: the folder pill's list leads with the machine's projects, then the folders, then "Add project"; a project chosen brings Isolation (Local, New worktree) and Branch ("Start from") pills | Paseo's `screens/new-workspace/form/` |
| `sidebar.rs` | the sidebar's projects section (the lead's slot `sidebar::projects`): Paseo's section header with an add button, a row per project of every online machine (its mark, its name, the machine when more than one has projects); a row starts a new chat in the project, one not ready opens Settings > Projects; past eight rows, "All projects" | Paseo's `sidebar-workspace-list.tsx` project rows, `sidebar-section-header.tsx` |
| `start_card.rs`, `start_header.rs` | a chat started in a new worktree: the session's screen opens at once with the prompt; the card under it shows the worktree being made, the approval when the project's commands need one, setup's log, and after a failed setup Stop, Start anyway, Try again | Paseo's `panels/setup-panel.tsx`; GOAT's thread that prepares its environment |

The seam with the desktop lead: the lead's `draft/pickers.rs` keeps the
pills and the kit's lists; two pickers (`Isolation`, `Base`) and one row
meaning (`Place`) hand the rows and the choice to this track;
`draft/mod.rs`'s Send asks `place_blocked` first and hands a project's
start to `start_in_place` (a start without a project is the lead's
`Command::Start` as before); the pending session screen draws
`start_card` where it said "starting". The link's start walk
(`link/src/workspace/start.rs`) runs a project's folder as it is
(`SessionPlace::Project`) as well as a new worktree.

### Checked against the fake fleet

`scripts/fleet.sh start` builds the workspace's shapes and stops when one
is missing: a monorepo with long paths, over a hundred changes (modified,
staged, untracked, deleted, renamed, a Hangul name) and a diff of 2,500
lines, with a `vessel.toml` naming setup, teardown, scripts and services;
a small project whose commands are approved; a worktree on a long branch
with uncommitted work; a failed clone. Against it, at 1280 (dark and
light, English and Korean) and 390 (Korean): Settings > Projects on the
second machine, the place list (projects, folders, Add project; a sheet
at 390), Isolation, Start from, and a Send in a new worktree of the small
project whose setup fails (`npm` is not on the fleet's path): the
session's screen opened at once, the card showed the failure and its
log, and Start anyway created the session in the worktree (the sidebar
listed it).

### Differences from Paseo, named

- The place list holds projects and folders under two headings; Paseo's
  new workspace has no folder field (a Paseo workspace is always a
  project's). Vessel keeps the folder for machines without projects.
- Isolation is remembered while the app runs, not across launches.
- The pills' words say what they are (the owner's test: a person must
  know what each pill is without being told): Paseo's "Local" and "New
  worktree" read "In the project folder" / "In a new worktree" ("프로젝트
  폴더에서" / "새 워크트리에서"), and the branch pill carries "Start from"
  ("시작 브랜치") before the branch, where Paseo's badge shows the branch
  alone. Each also has a tooltip.
- A sidebar project row starts a new chat in the project; Paseo's folds
  out the project's workspaces (Vessel lists sessions under machines).
- A remote-tracking branch is listed by git's own name (`origin/main`);
  Paseo shows `main` and marks the local row `main (local)`.
- Pull requests are not in Start from yet (they come with the forge).
- Setup is one log in the start card, not one item per command (the
  node streams it as one).
- A project row in Settings carries its folder under the name: two
  Vessel projects can share a name (any folder can be one).

## Problems met

- `cargo clippy -p vessel-app-view --all-targets -- -D warnings` on the
  branch point (`0235f60`) already fails on five lines of the lead's
  `view/src/board.rs` and `view/src/conversation.rs` (`clone_on_copy` on
  `SessionId` and `PermissionId`, two `integer_division`,
  `single_match_else`). They are the lead's files and the lead's
  restructuring rewrites both; this track does not touch them. Every file
  of this track passes.
- `ProjectId` and `WorktreeId` are `Copy` UUIDs (not the opaque string ids
  the page's fixtures suggest): the tests use fixed UUIDs.
