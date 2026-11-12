# ADR 0027: Projects, worktrees, the project's file, and its trust

Status: accepted, 2026-11-11 (milestone 11, track B,
`docs/plans/backend-m11.md`, "Track B: contract"; the wire is protocol
10, `crates/vessel-proto/src/workspace/`; the work is
`crates/vessel-workspace`, the node's part `crates/vessel-node/src/workspace/`).
Numbered 0024 on track B's branch; renumbered when the tracks were
merged, track A's 0024 (the queue) having come first. Its store schema,
7 there, is 9.

## Context

Until protocol 9 a session ran in a folder its client named (`cwd`), and
the node knew nothing of repositories. The references (GOAT, Paseo,
agent-orchestrator) all keep projects, give each agent a worktree of its
own, run a project's setup commands in it, and show its diff. The owner
asked for the same.

Questions:

1. Where does the logic live, and what does the node add?
2. What is a project, and which folders may be one?
3. What is the project's file called, what does it hold, and how does a
   later track add to it?
4. Whose commands run, and when?
5. When does a worktree go, and how is nothing lost?
6. How is a path checked?
7. What may an agent do through the node?
8. How does a session say where it runs?

## Options

**The project's file**

1. Paseo's `paseo.json` (`worktree.setup`, `worktree.teardown`,
   `scripts`, `servicePorts`; JSON, unknown keys kept).
2. GOAT's `goat.json` (`scripts[]` flagged `runOnWorktreeCreate`,
   `async`).
3. agent-orchestrator: nothing in the repository; commands typed into
   its page, kept in its store.
4. `vessel.toml`, TOML as the node's other files (`profiles.toml`,
   `fleet.toml`), with comments, `[worktree] setup/teardown` now and
   room for `[scripts.*]`, `[services.*]`, `[env]`.

**Trust**

1. Run the file's commands always (GOAT when the project has no scripts
   of its own; Paseo except for a fork's pull request).
2. Run only commands typed into the product (agent-orchestrator).
3. Run the file's commands once a person approved that exact set for
   that project, asked again whenever it changes.

**Tidying**

1. Remove on the session's end (its process).
2. Remove when the last workspace in it is archived (Paseo).
3. Remove by rules, keeping work (GOAT: uncommitted changes, unshipped
   commits, an operation in progress; a snapshot first).

## Decision

**1. Where.** `vessel-workspace` (new crate, ADR 0001's terms: a
mechanism crate; depends on `vessel-proto` and `vessel-proc` only) knows
git (ADR 0023), paths, files, the name index and the project's file, and
nothing of principals, the store or sessions; its tests run against
temporary repositories with the real git. The node's `workspace` module
decides who may call, keeps the registry (store schema 9: `projects`,
`worktrees`, `approvals`), runs clones and setups as its tasks, and ties
worktrees to sessions. The service only routes.

**2. Projects.** A project is a folder the node was told about
(`project.add`), cloned (`project.clone`) or started (`project.init`);
inside a repository, its top level. Refused as a root, for every caller:
a file system's root, the user's home folder, and any folder holding the
node's data folder or inside it (a read-only device reading a project
could otherwise read the node's token and become its owner). At most
1,000. `project.remove` forgets it and never deletes a file. A folder
that went away is told `missing`.

**3. The file** (option 4). `vessel.toml` at the repository's root:

```toml
[worktree]
setup = ["npm ci", "cp \"$VESSEL_PROJECT_ROOT/.env\" ."]
teardown = "docker compose down"
```

A string or a list of strings each; at most 64 KiB, 32 commands per
list. Every other table and key is kept for later tracks and ignored
now, so a file written for them parses here. Paseo's shape for setup and
teardown, the node's own format.

**4. Trust** (option 3). A repository's commands are someone else's
code. They run only once a person approved that exact set for that
project: the digest is SHA-256 over `vessel.toml worktree commands 1\n`,
then for each of `setup` and `teardown` its name, its count and each
command, every count and length as 8 bytes little-endian; any change to a
command, its order or its list is another set, asked again (a new
branch, a pull, an agent editing the file in its worktree).
`project.approve` needs `operate` and is refused to agents, so a
read-only device and an agent can never approve. `worktree.create` reads
the commands from the blob at the commit the worktree starts from (never
the checkout), and the commands that run are the ones parsed from the
file whose digest matched, never a second read. `worktree.setup` runs
the worktree's file as it is now, under the same rule. Commands run
through the platform's shell (`/bin/sh -c`; on Windows `cmd.exe /d /c`
on a temporary `.cmd` file holding the command, so no quoting is
involved), in the worktree, with the node's environment plus
`VESSEL_PROJECT_ROOT`, `VESSEL_WORKTREE` and `VESSEL_BRANCH`, standard
input closed, as one process group; the first failing setup command
stops setup. 30 minutes for a setup, 5 for a teardown; 1 MiB of output
kept per worktree, in the node's data folder. What an approval cannot
cover: a command that runs a script of the repository (`./setup.sh`,
`npm ci` running package scripts) runs whatever that script says on
that branch.

**5. Worktrees and tidying.** A worktree is a folder of the node's
(`<data>/workspace/worktrees/<project id>/<8 hex>`) made with `git
worktree add` (hooks off), on a new branch (`vessel/<8 hex>` unless
named), an existing one, or a snapshot. `worktree.create` answers at
once (`created`, `needs_approval` with nothing made, or `refused`); setup
runs afterwards and `worktree.output` follows it. A session never waits
for setup: `session.create` in a worktree whose setup runs fails with
`conflict`. Removal follows one rule for a person's `worktree.remove`
and for tidying (`Workspaces::retire`, for a worktree created for a
session, when that session is deleted or retention drops it; amended at
the merge, below): inspect (no operation in progress; no change `git status` shows,
submodules included; no submodule; no assume-unchanged or
skip-worktree entry; no `refs/worktree/*` or `refs/bisect/*`; no commit
reachable from `HEAD` or the created branch that no other branch,
remote-tracking branch, tag or Vessel snapshot has), teardown when
approved, inspect again, then git's own non-forced `git worktree
remove` and `git branch -D` of the created branch. Ignored files are not
counted (they are what setup makes again). Anything found keeps the
worktree, `kept` saying why. A live session inside it, or a setup or
teardown running, keeps it (`in_use`). `if_lost: snapshot` keeps a
snapshot first (one commit of the working tree as `git add -A` stages
it, through a temporary index, by a fixed identity, at
`refs/vessel/snapshots/<worktree id>`; refused above 100 MiB for a file
or 256 MiB in all, with an operation in progress, or over a nested
repository); `discard` removes it anyway. A snapshot comes back with
`worktree.create { branch: snapshot }`, as uncommitted changes.

**6. Paths.** A `RelPath` that decodes cannot be absolute, climb, name a
drive or any `.git` by git's own `verify_path` rules. The node joins it
to the root resolved for the call (a project's folder can be moved or
replaced by a link between calls), resolves the result with links
followed (for a target that does not exist yet, its parent), and
refuses a result outside the root or under a `.git`. Leading out and not
there get one answer, `not_found`, so a caller cannot learn what exists
outside. These are checks inside the node, not a sandbox (as ADR 0016
says for language servers): a process of the same user can race them.

**7. Agents** (ADR 0013). An agent already reads, writes and runs git in
its folder with its own tools. Through the node it gets what
orchestration needs: `project.list`, `project.get`, `project.commands`,
`worktree.list`, `worktree.create` (at most 8 it made and has not
removed), `worktree.output`, and `session.create` in any project and in
a worktree it created or one created for a session it may operate. Every
other workspace call is refused right after the scope check, whatever
its scopes (`guard`).

**8. Where a session runs.** `SessionSpec.place` names a project (and a
folder in it) or a worktree; the node fills `cwd` with the folder it
resolves to, so a protocol 9 client still sees where the session runs,
and starts the session's log with `vessel:` notices (the folder, the
branch and its base, setup's result and, after a failure, its last
lines). `SessionInfo.workspace` (project, worktree, branch) is filled
from the place on every answer. A `session.create` with a place needs
protocol 10.

## Consequences

- The node gains the folders, branches and diffs the references show,
  through one crate whose tests need only git.
- A repository's commands never run unseen; the price is one approval
  per set per project, asked again on any change.
- A worktree with work in it stays until a person decides; work that
  must survive must not be ignored by git.
- Not done here (wave 2): forge and pull requests, checkpoints per turn
  (snapshots are built as a checkpoint would be), carrying a worktree
  when a session moves to another machine, scripts and services (the
  file keeps their keys; the digest's version line changes when more of
  the file runs).

## Amended when the tracks were merged (2026-11-12)

- Tidying runs when the session a worktree was made for is deleted
  (`session.delete`, which answers what became of the worktree:
  `SessionDeleted`) or dropped by retention (a sweep after each prune,
  and at the node's start for any worktree whose session is gone); not
  when the session is archived, as this ADR first said: ADR 0026's
  `session.resume` starts an archived session again in the same folder.
  `crates/vessel-node/src/tidying.rs` says when and why; the rule itself
  is unchanged.
- `session.resume` of a session whose folder is gone anyway (a person
  removed its worktree) is `conflict`, naming the folder; so is one into
  a worktree a setup, teardown or removal holds (the removal could take
  the folder from under the agent).
- The agents' list of workspace calls is an exhaustive `match` over
  every method, so a new one is decided where it is added. An agent's
  `session.create` with a `cwd` of its own inside the node's data folder
  is `forbidden`: the worktree ownership checks are the `place`'s, and a
  raw folder would go around them.
- A worktree kept because git could not inspect it is not tried again
  by tidying (each try would run its teardown again); a person decides.
  One kept because a session was in it is tried again once that session
  has gone.
- What the trust in this ADR rests on, made to hold under a swapped
  folder: every call resolves its root again and refuses one replaced
  since it was recorded, and a worktree whose `.git` no longer names its
  repository's record of it (`git::gitfile`), so git never reads a
  configuration someone put in its place; the node's folders, the empty
  hooks folder among them, are absolute; `vessel.toml` is read only as a
  bounded regular file inside its folder (one leading out is as if there
  were none); a root that holds the home folder is refused, and
  `project.init` checks its folder before making it.
- Still open: in a project (not a worktree) whoever can write its
  `.git/config` chooses the filters and the like that a read runs; this
  ADR's trust says the user wrote that configuration, and an agent can
  write in the project's folder. Closing it needs git's own help (an
  empty attribute source, git 2.40's `--attr-source`, or reads against
  a configuration of the node's) and is left to a decision of its own.
