# ADR 0113: A new empty project directory, and `~` in project paths

Status: proposed (impl F, 2026-11-19; TRACKER 242-243 for the Add project
sheet, TRACKER 234; the coordinator's ruling 1 of
`docs/plans/wp-add-project-flow.md`). The coordinator recorded 0113 as this
ADR's number. The protocol is drafted as 34 (`PROJECT_DIRECTORY_PROTOCOL`),
as 33 is drafted by ADR 0111; the integrator assigns it at the node-line
merge.

## Context

Paseo's Add project sheet has a "New directory" method: choose a parent,
type a name, and the host makes that one empty folder and adds it as a
project (`project.create_directory.request {parentPath, name}`;
`server/src/server/project-directory-service.ts`, pinned 0.15.4):

- the name must be one name: not empty, no leading or trailing white
  space, not `.` or `..`, no `/`, `\` or NUL, not absolute, no drive
  prefix; the joined path must lie inside the parent;
- the parent must exist and be a directory (a leading `~` is the user's
  home);
- one non-recursive `mkdir`, then registration; when registration fails
  the folder is removed again (`rmdir`), and a failed removal is said
  separately;
- its errors: invalid name, parent not found, permission denied, exists,
  create failure, registration failure; no Git repository is started
  (survey row 244: Paseo has no Git init in this flow).

Vessel's node has `project.init`, which makes the folder with its parents,
runs `git init` and registers it. That is not Paseo's New directory (the
coordinator ruled for a call like Paseo's). Paseo's server also expands a
leading `~` in every project path it is given (`project.add`, a clone's
parent, a new directory's parent); Vessel's node takes absolute paths
only, while the sheet's rows (the suggested parents `~/dev`, `~`, a typed
`~/x`) are written with `~`.

## Decision

- New unary method `project.create_directory { parent, name }` (`operate`,
  forwardable; refused to an agent principal, as the other project
  writes) answering the new project (`ProjectInfo`). Its values are the
  registry's staged `CreateProjectDirectory` and `DirectoryName`
  (2026-11-15, ADR 0082's values without a row), which already hold
  Paseo's name rules, checked when the request is decoded. `parent` is a
  path, absolute or starting with `~` (the node user's home).
- The node resolves the parent (`~` expanded), requires an existing
  directory, joins the name, checks the result against the refused roots
  (`policy::check_root`, as every project root), makes it with one
  non-recursive `create_dir`, and registers it as a project without Git
  (`git: false`, its state `Ready`). When registration fails it removes
  the folder (`remove_dir`, which only removes it empty) and answers the
  registration failure; a failed removal is named in the message.
- Errors (`WireError`, the node's sentence as the message, which the
  sheet shows as Paseo's shows `payload.error`), the node's usual kinds
  as `project.add` answers them: an invalid name, a relative parent, a
  parent that is missing or not a folder, or a refused root
  `invalid_request`; a target that exists, or the projects' bound,
  `conflict`; permission denied or any other disk failure `internal`
  with the disk's words.
- From the same protocol, `project.add`'s `path` and `project.clone`'s
  `parent` may also start with `~`, expanded the same way: only `~` alone
  or before `/`, to the node user's home that `folder.suggest` walks;
  `~user` stays relative and refused (the coordinator's confirmation,
  2026-11-19). An older node
  refuses such a path as relative (its message says so), and the sheet
  sends `~` paths only to a node of this protocol; for older nodes it
  sends what was typed, as today.
- The Add project sheet (page and native) offers New directory on a
  machine of this protocol, else disabled "Update this machine to create
  directories" (Paseo's words, host as machine).

## Alternatives

- `project.init` for New directory: it also runs `git init` and makes
  missing parents; ruled out by the coordinator.
- Expanding `~` in the clients: neither the page nor the app knows the
  node user's home (`node.info` does not say it); deriving it from
  `folder.suggest`'s `{path, shown}` pairs works only after a search and
  only on protocol 32.
- One Paseo error code per kind (`ErrorDetail` reasons): no client acts on
  the kind; the message is what is shown.

## Consequences

- One more method and protocol number; the guard, policy, forward policy,
  version tables, samples, TypeScript bindings and the client's unserved
  list carry it as every method.
- `~` is accepted by three project paths from protocol 34; everything
  else (a worktree path, a file path) stays absolute.
- A new directory is a project without Git until the person runs
  `git init` (a possible Git init action is listed after parity in the
  plan).
