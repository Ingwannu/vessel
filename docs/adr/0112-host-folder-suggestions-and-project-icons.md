# ADR 0112: Folder suggestions from the node's home in Add project, and project icons, as Paseo's

Status: accepted with conditions by the coordinator on 2026-11-18 (impl F, TRACKER catalogue
F-F17/F-F46; number 0112 assigned). Amends ADR 0083 for `folder.suggest` only: `file.suggest` keeps its
rule that it never searches the home.

## Context

Paseo's Add project flow (`components/add-project-flow.tsx` 402-440) searches folders as the user types:
`directory_suggestions_request` with no `cwd`, so the daemon searches the user's home
(`server/src/utils/directory-suggestions.ts` `searchDirectoryEntries`: `~` alias, absolute paths, limit
30, at most 100, depth 12, 20,000 entries scanned, hidden folders and node_modules, venv, env,
virtualenv, dist, build, target, out, coverage, vendor, __pycache__, .git skipped; on macOS `~/Library`
is skipped because its privacy prompts block the scan). Paseo also draws each project's icon
(`project_icon_request {cwd}`, `utils/project-icon.ts`: favicon, icon or logo as svg or png before ico,
first in public, static, priv/static, assets, images, img; square only, at most 32 KiB) in its sidebar
project rows, the projects screen, project settings, the new workspace screen, the command center and the
folder rows of its project suggestions.

Vessel's node has `file.suggest` (protocol 27, ADR 0083), which searches inside a registered project or
worktree only: ADR 0083 states as a difference that suggestions never expose arbitrary home paths. Add
project starts outside every project, so the native app's Add project has a bare path field. The node has
no icon lookup (`file.inspect` is folder metadata).

## Decision Log

- Purpose and intent: Add project suggests folders as Paseo's does, and projects show their own icon.
- Existing implementation and constraints: `project.add` (Operate) already accepts any absolute folder
  an Operate principal types, so such a principal can already name any folder on the node; nothing lists
  them. Reads go through the bounded blocking pool and the path checks of `vessel-workspace`. Read
  principals (a paired viewer) must not learn the node's folder tree.
- Alternatives considered:
  1. Search on the client's own disk: wrong for every remote node, and the page has no disk.
  2. Suggest only inside folders already registered (their parents and siblings): not Paseo's flow, and
     useless for the first project.
  3. Widen `file.suggest` to take no root: one method with two permission rules and two path forms.
  4. A new method for the home search (Operate), and a new project icon method (Read, inside the
     project's root).
- Choice: 4.
  - `folder.suggest {query, limit?}` (Operate): folders only, rooted at the node user's home, Paseo's
    limits and skip list, `~/Library` skipped on macOS; the walk is bounded in time and in entries
    scanned; a link is never followed out of the home; the answer holds folder paths only, never a file
    name. A query that is an absolute or `~/` path lists that folder's subfolders (Paseo's exact-path
    retrieval) when it lies under the home; paths come back absolute, with the home shown as `~` for
    drawing.
  - `project.icon {project}` (Read): Paseo's patterns and folders inside the registered project's root
    only (the existing root checks; links are not followed out), square only, at most 32 KiB; the type
    is decided by the bytes (PNG, ICO or SVG signatures), never by the file name; the bytes and that
    type, or none. Every client draws an icon only as an image (an `img` from a blob or data address on
    the page, an image element in the native app), never as inline SVG markup, and the page gives it the
    type the node decided. Custom icons (Paseo's `project-custom-icon.ts`) are not in this ADR.
  - One protocol number for both, assigned at the node-line merge; an older node offers neither (the
    clients hide the suggestions and draw the folder mark).
  - Callers: native Add project's path field (debounced, Paseo's rows: the short path, the folder above
    it), and project icons where the native app and the page draw a project (projects list, sidebar
    projects, Add project rows naming a registered project), cached per project.
- Why this over the others: the permission matches what an Operate principal can already do with
  `project.add`; Read principals see nothing new but icons inside projects they can already read; each
  method has one root rule, so ADR 0083's boundary for `file.suggest` is untouched.
- Advantages, costs and effects: Add project works as Paseo's for local and remote nodes; a scan costs
  up to 20,000 directory entries on the blocking pool per query (debounced by the client, results cached
  8 s as Paseo's); an icon read is at most 32 KiB per project, cached by the client. A node's home layout
  becomes visible to Operate principals (they could already add any folder by name). Not covered: custom
  icons, icons for worktrees (they show their project's), Windows drive roots outside the home.

## Stated differences from the reference

The search never leaves the home and never returns files (Paseo's `includeFiles` is not used by its Add
project flow); Read principals get no folder suggestions (Paseo has one owner principal).

## Implementation notes (node, drafted protocol 32)

- `vessel_proto::workspace::host_folders` holds both methods' types and `HOST_FOLDERS_PROTOCOL`; the disk work is
  `vessel_workspace::files::host_folders` and `files::project_icon`, run on the node's file workers (the same
  four-slot pool and 60 s wait as the file panels).
- Folder walk: Paseo's query plan and breadth order, ranked by the scorer `file.suggest` uses (ADR 0083's accepted
  difference from Paseo's tiers). No link is walked, so a folder reachable only through a link inside the home is
  found only when typed. Time bound: 3 s, after which the folders found so far come back with `more`.
- Icon search: entries in name order (Paseo's are the disk's `readdir` order), at most 5,000 entries read per
  search, links neither read nor walked (also a link on the way, as `priv` in `priv/static`). Only PNG, ICO and
  SVG bytes become an icon; Paseo's JPEG, GIF and WebP sniffing is left out, since its names are SVG, PNG and ICO
  only. SVG is recognised when its first element (after a byte order mark, XML declaration, comments and a
  doctype) is `svg`.
