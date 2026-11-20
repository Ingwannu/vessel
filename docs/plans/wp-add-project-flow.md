# Add project as Paseo's sheet, page and native (TRACKER 234-238, 242-243)

Impl F, 2026-11-19. Node line head `b274faf8a`, package base 14.65
(`761fc0391`). Reference: pinned Paseo 0.15.4 (`1c00b365c`):
`components/add-project-flow.tsx`, `add-project-flow/model.ts`,
`add-project-flow/options.ts`, `components/project-picker-options.ts`,
`utils/working-directory-suggestions.ts`, `stores/add-project-flow-store.ts`,
`hooks/use-open-add-project.ts`; the survey's section 01-04
(`docs/parity/survey/parts/p1-projects-workspaces.md`).

Status: plan approved by the coordinator (2026-11-19) with the rulings at
the end. The GitHub search rows came with TRACKER 440 (`forge.repos`,
ADR 0088): on the page with its part 1, natively after the sheet's
adoption, on a machine at its protocol.

## Paseo

One modal sheet, opened by a request (a monotonic id and an optional
preferred host) from the sidebar's Projects "+", the `agent.new` shortcut,
the new-workspace screen and the open-project screen. Closing drops its
state; a new request starts it fresh.

The sheet is a page stack. Back (the arrow, or Esc) pops a page and closes
on the first. Pages, their titles and their field's placeholder:

| Page | Title | Field |
|---|---|---|
| host | Choose host | Search hosts... |
| method | Add project | (none) |
| directory-search | Search for directory | Search directories or enter a path... |
| github-search | Clone from GitHub | Search or enter a GitHub repository... |
| github-location | Choose destination | Search parent directories or enter a path... |
| new-directory-parent | Choose parent directory | Search parent directories or enter a path... |
| new-directory-name | Name directory | Directory name |

- Hosts: the online ones. A preferred host, or the only one, skips the
  host page (also when hosts arrive late, while the host page is the only
  page). Host rows: the label, its id under it, a server mark; filtered by
  the field; "Add host" / "No connected hosts" when there are none (it
  closes the sheet and goes to adding a host).
- Methods: Search for directory ("Find a directory on {host}"); Browse
  ("Choose or create a directory in Finder", only in the desktop app on its
  own local daemon); Clone from GitHub ("Search projects available to your
  GitHub account", or "Enter a GitHub URL or owner/repo" without search, or
  disabled "Update this host to clone GitHub repositories"); New directory
  ("Create an empty directory on {host}", or disabled "Update this host to
  create directories"). A host that cannot add projects shows no methods
  and "Update the host to use Add Project.".
- Directory search: the field is asked 250 ms after the last key
  (`limit 30`, directories only). Rows: the host's workspaces' project
  roots that match the field (all of them for an empty field or `~`), then
  the host's answer, without repeats; a typed absolute or `~` path not
  among them comes first as "Open this path". A row shows the path with
  the home as `~`, and the whole path under it when that differs. Choosing
  one adds the project (`project.add`): on success the sheet closes and
  the new-workspace screen opens for the project; else "Directory not
  found" or "Unable to add project" on the page.
- New directory: a parent page (rows as directory search, a typed path
  says "Use this parent"), then a name page: its field keeps a draft per
  host and parent across Back; the joined path shows under the field;
  Enter creates it (`project.create_directory {parentPath, name}`: one
  `mkdir`, then registration, removed again when registration fails); an
  empty name, `.`, `..` or a separator says "Enter a directory name";
  failure "Unable to create directory" or the host's error.
- GitHub (TRACKER 440): repository search (30), manual choices for a full
  remote ("Clone this repository URL") or `owner/repo` (two rows, "via
  HTTPS" / "via SSH"), then a destination page: the parents of the host's
  projects, then `~/dev`, `~/Developer`, `~/src`, `~/projects`,
  `~/workspace`, `~`, the last parent used first; a destination that
  exists is disabled "Already exists", else "Parent directory: {parent}".
- States, in the results area: the joined-path preview; progress
  ("Adding project...", "Cloning project...", "Creating directory..."),
  replacing everything else while it runs (the field is not editable); the
  page's error (destructive, sm); a query error ("Unable to search
  directories"); "Loading..." while the field waits for its answer; the
  rows; an empty text ("No matching options" and the page-specific ones).
- Keyboard: Up/Down move the active row, wrapping and skipping disabled
  rows; Enter chooses it (on the name page, creates); Esc goes back or
  closes. The footer shows "Up Down Navigate", "Enter Select", "Esc Back"
  (or "Close" on the first page), only with a hardware keyboard. The
  active row scrolls into view.
- Look: a backdrop (black 50%) that closes on a press; the panel 640 wide
  (at most 92%), at most 80% high, 48 from the top, centred, 1 border,
  radius lg, surface0, shadow lg. Header: 16 in, 8 top, 12 bottom, a
  rule under it; title row (the 18 back arrow, muted, the foreground on
  hover; the title in sm normal weight, the host label after it, muted);
  the field in base. Rows: 16 in, 8 high, 12 apart, an 18 slot with a 16
  muted mark, title base, subtitle sm muted 2 down; the active or hovered
  row surface1; disabled 50%. Footer: 16 in, 8 high, 16 apart, a rule
  above, keys sm foreground, words sm muted.

## Vessel before

- Page: Settings > Projects has an "Add project" menu: Add a folder...
  (a path field, `project.add`), Clone a repository... (URL and parent,
  `project.clone`, which answers at once with the project `cloning`),
  Start a new project... (`project.init`: makes the folder with its
  parents, `git init`, registers). The sidebar's Projects "+" opens
  Settings > Projects.
- Native: the same three as inline forms in Settings > Projects; Add a
  folder has the F17 suggestions (`folder.suggest`, protocol 32). The
  sidebar "+" opens Settings > Projects.
- Node: `project.add`, `project.clone` (any URL), `project.init`,
  `folder.suggest` (folders under the user's home, `{path, shown}`, at most
  100). No `project.create_directory`, no GitHub search.

## Plan

[Decision Log]
- Purpose and intent: the whole Add project flow as Paseo's sheet, the
  same on the page and in the native app, replacing the three inline
  forms and dialogs.
- Existing implementation and constraints: the page's dialogs and the
  native inline forms are Vessel's own; the node has no create-directory
  call; GitHub search and its clone wait on TRACKER 440; a browser page
  has no folder dialog; a machine below protocol 32 has no
  `folder.suggest`.
- Alternatives considered: keep the forms and restyle them (not Paseo's
  flow: no host or method pages, no keyboard model); New directory over
  `project.init` (it makes a Git repository and the parents, which
  Paseo's does not; row 244 says Paseo has no Git init here).
- Chosen approach: Paseo's page-stack model ported as pure code (page:
  `data/workspace/add-project-flow.ts` with `model.ts`'s and `options.ts`'s
  tests; native: `view::workspace::add_flow`, tested in the view crate);
  one sheet per side drawing it (page
  `features/projects/add-project-flow.tsx` on the kit dialog layer;
  native `app/src/areas/workspace/add_flow/` on the kit dialog/sheet
  layer), opened from every place that adds a project; a new node call
  `project.create_directory` for New directory (ADR, its own protocol
  number), staged after the sheet.
- Why this approach: the model is where Paseo's behaviour lives (back
  stack, drafts, wrap and skip); porting it with its tests keeps page and
  native the same and testable without drawing.
- Pros, cons and effects: the inline forms go (Settings > Projects keeps
  its list and an "Add project" button that opens the sheet); one more
  node call and protocol number; "Start a new project" leaves the add
  flow (ruling 3).

Steps (each its own packet and gate):

1. Model, page: the state machine and options with Paseo's tests, plus
   Vessel's row building (`folder.suggest` answers, recent roots).
2. Sheet, page: host, method, directory search (with "Open this path"),
   states, keyboard, footer; opened by the sidebar "+", Settings >
   Projects' "Add project" and the shortcut row's `agent.new` where it
   exists; on success "New session here" for the project. Frames: 1280
   and 390, dark and light, each page and state.
3. Model and sheet, native: the same (`view::workspace::add_flow`,
   `areas/workspace/add_flow`, the kit's `flow_sheet`); Browse waits on
   TRACKER 816 (ruling 5). Frames as above.
4. Node: `project.create_directory {parent, name}` as Paseo's (one
   `mkdir`, name rules, registration, `rmdir` on failure, its errors),
   ADR, the next protocol number; then the New directory pages on both
   sides.
5. Clone from GitHub in Paseo's no-search state on both sides: the manual
   rows (a full remote; `owner/repo` via HTTPS and via SSH) and the
   destination page, over `project.clone`.
6. After TRACKER 440: the search rows (done: the account's repositories
   for an empty field, a search for text, 30 rows; the typed rows stay
   while it searches and under its error, Paseo's rule).

## Differences (each deliberate)

1. Words: Paseo's "host" is Vessel's "machine": "Choose machine", "Search
   machines...", "No connected machines", "Add a machine" (Vessel's own
   add-machine flow), "Find a directory on {machine}", "Update this
   machine to ...". Native also in Korean, as all its screens; the page
   in English, as the rest of the page.
2. Host gate: Paseo needs `projectAdd` and `stableProjectIdentity`. Every
   Vessel machine has `project.add` and stable ids, so every online
   machine can add; the gates are per method (below).
3. Directory search on a machine below protocol 32: no `folder.suggest`,
   so its rows are the recent roots and a typed path only; no query error
   is shown for it (it is not a failure).
4. Recent roots: Paseo uses the host's workspaces' project roots; Vessel
   uses the machine's projects' roots from `project.list` (the
   sidebar's), as its workspaces belong to projects.
5. Browse: native only, on this computer's own machine (Paseo: Electron
   on its local daemon); the page shows no Browse, as Paseo's browser web.
   Its description says the platform's file manager instead of Finder off
   macOS (question 4). Vessel's app runs no node of its own, so "this
   computer's machine" is a node that reports this computer's hostname
   and OS (ruling 5). Another machine with the same hostname and OS is
   taken for this one: a folder picked here that it lacks gets its
   `project.add`'s "Directory not found".
6. Completion: Paseo opens its new-workspace screen; Vessel opens a new
   session in the project (the page's "New session here", the native
   `chat_in_project`), the closest screen it has.
7. A clone's progress: Vessel's `project.clone` answers at once with the
   project `cloning` (the row then shows it); the sheet closes on that
   answer rather than waiting for the clone to end as Paseo's does.
8. New directory waits for step 4; until then, and on a machine below its
   protocol, the method shows disabled "Update this machine to create
   directories" (Paseo's own words for a host without it).
9. Methods are Paseo's four in its order (Search for directory, Browse,
   Clone from GitHub, New directory). Vessel's "Start a new project"
   (`project.init`, a new Git repository) is dropped from the add flow;
   a Git init action is a possible extra after parity (Paseo has none
   here, row 244).
10. Browse's description names the platform's own file manager: "Choose
   or create a directory in Finder" on macOS, "in File Explorer" on
   Windows, "in the file manager" on Linux (Paseo says Finder on every
   platform).
11. Clone from GitHub before 440 is Paseo's no-search state ("Enter a
   GitHub URL or owner/repo"); a machine that cannot clone (no Git found
   by `project.list`) shows it disabled with Paseo's "Update this machine
   to clone GitHub repositories". For `owner/repo` the client builds the
   remote (`https://github.com/owner/repo.git` or
   `git@github.com:owner/repo.git`), as `project.clone` takes a URL
   (Paseo's server builds it from the protocol choice).
12. Marks: GitHub's rows draw its brand mark (the kit's provider mark),
   and a clone destination draws the folder mark, as the page's icon set
   has no hard drive (Paseo's lucide `HardDrive`). The page uses its own
   colours (as every ported page surface); Paseo's sizes and spacing.

## Rulings (coordinator, 2026-11-19)

1. New directory: a new node call like Paseo's (an empty directory, no Git
   init), with an ADR and the next protocol number; on an older node the
   row is disabled with Paseo's words, host as machine.
2. GitHub before 440: Paseo's no-search state now over `project.clone`
   (manual URL and `owner/repo` rows, then the destination page); the
   search rows come with 440; a machine that cannot clone shows the row
   disabled with Paseo's `githubMethodDescription` words.
3. "Start a new project" is dropped; the methods are Paseo's four in its
   order; Git init is noted as a possible extra after parity.
4. Browse names the platform's own file manager (Finder on macOS, File
   Explorer on Windows, the file manager on Linux), a stated difference.

5. Native Browse (2026-11-19, rule settled 2026-11-20): the first native
   packet ships the other three methods with Browse hidden, as on the
   page. TRACKER 816, after the file viewers, marks the app's own machine
   and turns Browse on with the system folder chooser (`prompt_for_paths`).
   The rule is the same OS and the same hostname (ASCII case aside); a
   loopback route is neither needed nor enough, since a loopback port can
   be an SSH tunnel to another machine or a container's published port,
   whose nodes report other hostnames. The link decides it per fleet read
   (`view::this_computer`) and sends `Event::ThisComputer` before
   `Event::Fleet`, so `board::Machine` and `Fleet` are unchanged.
6. Page frames (2026-11-19): the structure passed; the field draws no
   focus ring and the footer's keys are Paseo's Shortcut badges (`↑↓` on
   macOS, `↑+↓` elsewhere, `⏎`, `Esc`), both sides.

The other differences were accepted.
