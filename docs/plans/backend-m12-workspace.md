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
| `code/` (`frame.rs`, `changes.rs`, `files.rs`) | the code panel beside a session: Changes (the comparison chosen from Paseo's mode list: Uncommitted, Staged, Unstaged, since the base; every file with its status letter, folder and counts, its diff under it as the kit draws a diff's lines, read when its row first comes into view, folded by a click; "Show the rest" for a diff past the node's page) and Files (a tree read folder by folder, a file shown with numbered lines coloured by the page's lexer, Back) | Paseo's Explorer (`docs/explorer-sidebar.md`, `git/diff-pane.tsx`, `panels/files-panel.tsx`) |
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

The code panel's seams: `code_beside(shell, screen, tokens, window, cx)`
in the lead's `session.rs` (the session screen comes back with the panel
beside its conversation column when the panel is out; the row is this
track's so a drag at the panel's edge measures the row and leaves the
conversation 480, and a narrower window takes the panel's width first,
down to 320), `code_toggle(shell,
tokens, narrow, cx)` in track C's bar; track G1's git actions at the head
of the Changes tab through `code::changes::git_head(shell, machine, root,
tokens, window, cx) -> Option<AnyElement>` (its body is G1's one call).
The panel reads where the session runs with `session.get` (the card has
no root) and says so when the session is in no project. After a git
write of G1's succeeds, G1 calls `Shell::code_git_changed(machine, root,
cx)`: when the panel shows that root, the open tab is read again and
what it shows stays until the answers replace it; the tab not shown,
or the whole panel while it is closed, is marked stale and read again
when it is next shown (G1's finding: a closed panel was being read,
and a hidden tab kept what was true before the write). The Changes
tab's refresh button reads the open tab again.

A file opened at a desktop's width is edited in place (Paseo's Source
panel, `file-panels/source/`): the model is `view::workspace::editor`
(Paseo's `FileEditorModel` as a state machine with its test cases: the
800 ms autosave, a save guarded by the hash the edit started from,
Overwrite, Reload asked first when the buffer has changes, a re-read
after a git write that a clean buffer follows and a modified one meets
as "Changed on disk", a file gone as "File deleted on disk", the write
made when the file is left). The control is the kit's `code_editor`
(the engine's code-editor kind in the kit's colours, the code face with
Pretendard behind it, syntax from the view's lexer through
`LineSyntax`, its kinds coloured by the current theme on every draw;
code keeps each line on one row and scrolls sideways, Markdown wraps,
Paseo's `WRAPPED_EXTENSIONS`, `view::workspace::editor::wraps`).
Paseo's toolbar sits over it (size and lines; the dot, "Saving..." or
"Save failed"; "Ln, Col" and "Add lines … to chat" in a small view of
their own, since the editor redraws on every blink of its caret).
Paseo's rules for what is editable are kept: at most 1 MiB (read whole
in one page, the node's largest), never at a phone's width; anything
else is shown read-only as before. Difference, named: the node has no
watch of a file, so a change on disk shows at the next save or re-read,
not at once.

Motion (the owner's word 39): what the person's own action brings in
comes in once, as a fade on the kit's tokens (`code/appear.rs`): the
dot, "Saving..." and "Save failed" and the selection's "Add lines … to
chat" over `QUICK`, the conflict bar as a surface over `STANDARD`. Each
is keyed by an id naming the mark, not its text, so it does not play
again while it stays (a wider selection, Overwrite coming and going); a
mark that leaves is gone at once. While motion is held the end is drawn
at once (GPUI's one-shot transitions under `reduce_motion`).

Track P's seams (the composer): a selection or a file handed to the
composer is a value, `view::workspace::reference::FileReference` (root,
path, an optional `LineRange`, the text sent), keyed, titled and
subtitled as Paseo's `WorkspaceFileComposerAttachment`
(`attachments/workspace-file.ts`); P's `Shell::composer_add_reference`
shows it as a chip and sends its text. Until that entry exists the panel
puts the text in the draft as before. `@` in the composer finds files
through `Shell::code_find_files(key, query, limit)` (P's own search key;
the open session's root is read once per session and kept, a find asked
before it is known waits for it and says `FindRefused::Reading`); an
answer under a key that is not the panel's goes to
`code::elsewhere::found_elsewhere`, P's slot. The node's find returns
files only (its index is `git ls-files`), so `@` offers no folders.

The Files tab searches: a file by its name as it is typed (`file.find`,
keyed so a newer search stops an older one) or lines inside files on
Enter (`file.grep`, literal, smart case), the two chosen by a segmented
control beside the field; a hit opens the file at its line. An answer
to any query but the one last sent is dropped: the node replaces an
older find only when a newer one is sent, so after the field is
emptied, refused as typed or switched, a late answer would otherwise
show results the field no longer asks for. A line
over the results says how many were found, and when the machine
stopped early (a find over part of a large folder, a search in files
past its 200 lines) it says so and asks for a narrower search. What
made a result match is marked as Paseo's `components/ui/highlighted-text.tsx`
marks it (the surface3 tint, the text in the foreground): a name's
matched characters (the node counts them over the whole path, so the
name and its folder are marked apart) and the words on a line (moved
by the indent the row takes off). Paseo's explorer has no search
inside files; the count line is this panel's own. In a file
shown, a click on a line's number chooses it, Shift-click extends the
choice, and "Add lines 12–20 to chat" puts agent-orchestrator's
reference (the page's `reference.ts`) at the end of the session's
draft. At a phone's width, where the panel covers the conversation, the
panel closes after it (and after the tree's "Add to chat") so the draft
is seen with what was added, as Paseo's compact layout closes its
Explorer sheet. The panel is resized by dragging its left edge with the
kit's resize handle, the sidebar's too (the owner's word 29; 320 at least,
the conversation keeps 480 of the row, whatever the sidebar's width); the
width is kept across restarts and a double-click on the edge returns to
440 (`docs/plans/backend-m12-spike.md`, "Resizable side panels").

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
listed it). The code panel against the monorepo: 110 changed files
(a binary one, a rename, a Hangul path, 2,500 rewritten lines), the tree
with an empty folder, the 3,000-line file in the viewer, a file the
machine may not read; dark and light, English and Korean, 1280 and 390
(the panel takes the whole screen there).

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
- The code panel starts 440 wide and is dragged no narrower than 320
  (Paseo's Explorer: 320 and 240) because a file's diff is drawn in it
  (Paseo opens a diff in the main pane).
- A search inside files runs on Enter (Paseo's runs as typed, after a
  pause): the node's grep has no key to stop an older one. A diff's lines wrap (Paseo scrolls them by
  default, wrapping on request): a row of a long list cannot scroll
  sideways on its own.
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
- The editor's first captures (2026-11-14) showed two rows of the file:
  the engine's editor is as tall as its parent, and the editing body sat
  in the viewer's plain block with `flex_1`, which only a flex parent
  honours, so the body was as tall as its content. The body now takes
  its slot's whole size. The engine also wraps lines unless told not
  to; the kit's editor now keeps code's lines unwrapped.

## Where this track stands (2026-11-14 17:30 UTC, paused for budget)

- Done: the landing 232717df (in 0.0.0.0.13); P's seams 46ed5a7f; the editor's model 91d79ec6; the kit's code editor 97b41744 and d129faf3 (code unwrapped, Paseo's padding, no current-line fill); the Files tab's editor 363802d5 and 6922382b (fills its slot, wraps only Markdown, its marks fade in on the motion tokens); the try-out fleet paired as a person pairs fe2becce; the lead's df74c3e0 merged as 894ea4f9. Whole gate green at 6922382b: fmt, clippy, 727 tests. Words 26 and 27 checked in this panel at 1.25 and 1.5 (menu, delete sheet, match marks, editor selection): fills stay inside their rounded shapes, and the kit's menu and dialog occlude what is under them.
- Measured (release, virtual display, software renderer, the 2,500-line file open and focused): 20.5 % of a core idle with motion held; with motion on, 393 %, which includes the conversation's waiting spinner on the same screen: the editor's own share is not yet separated.
- Next, in order: the Changes list following changes made outside the app (Paseo's behaviour first, then one hook with G1b's refresh, `code_git_changed`); a paragraph here on how the approval card draws a set's terminal commands when the node adds them (every one shown, or a typed refusal; never approved unseen); the audit's rows for this track; open by name from anywhere; reading past a file's first page.
