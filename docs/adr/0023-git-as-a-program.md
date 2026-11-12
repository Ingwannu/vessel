# ADR 0023: git is run as a program, hardened

Status: accepted, 2026-11-11 (milestone 11, track B,
`docs/plans/backend-m11.md`, "Track B: contract"; the code is
`crates/vessel-workspace/src/git/`).

## Context

Protocol 10 gives a node's clients its projects, worktrees, diffs,
commits, pushes and files (ADR 0027). Every one of these needs git. The
node runs as its user, on the user's repositories, often while an agent
works in them; a repository can be someone else's code, and its
configuration (`.git/config`, hooks, attributes) can name programs.

Questions:

1. Link a git library, or run the `git` program?
2. How is a run kept from running what the repository names, asking a
   person for a credential, or taking an option from a caller's string?
3. Which git, and how old may it be?
4. How long may a run take, and how much may it print?

## Options

**Library or program**

1. `git2` (libgit2): a C library built with the crate, with its own TLS
   and SSH stacks (OpenSSL or the platform's, libssh2) on three
   platforms; its own credential callbacks rather than the user's
   credential helpers and `ssh` configuration; GPL-2.0 with a linking
   exception.
2. `gix` (gitoxide): pure Rust, a large dependency tree; reads are
   strong, but what the node must also do (create and remove worktrees
   with git's own safety checks, push through the user's credential
   helpers and `ssh`, honour `includeIf`, filters, `safe.directory`,
   future formats) would have to follow git's behaviour release by
   release.
3. The `git` program the user already runs, with an argument vector.

**Protections**

1. None beyond an argument vector (Paseo and GOAT run git so; GOAT
   turns prompts off only for background fetches).
2. agent-orchestrator's: prompts off on every run, and hooks, external
   diffs and `fsmonitor` off on reads.
3. (2) and more: hooks off on every run but commit and push, text
   conversion and signature checks off, lazy fetches off, literal
   pathspecs, options ended before every caller-supplied value, clone
   URLs and protocols limited, the project's file read as a blob.

## Decision

- **The program** (option 3). All three references run it; none links a
  library. It is what the user's own hooks, credential helpers, `ssh`
  configuration and attributes are written for, and every git feature a
  user's repository uses works by definition. A run costs a process: on
  the development machine (Linux, git 2.43), `git rev-parse HEAD` took
  2.0 ms and `git status --porcelain=v2` 2.5 ms per run, measured over
  200 runs each; a page asks for a handful per action.
- **Never a shell.** `vessel-proc` spawns git with an argument vector, in
  its own process group, standard input closed. Every value a caller
  supplies goes after `--end-of-options` or `--`: a branch named `-f`
  is a branch.
- **The environment of every run:** `GIT_TERMINAL_PROMPT=0`,
  `GCM_INTERACTIVE=never`, `GIT_ASKPASS=` and `SSH_ASKPASS_REQUIRE=never`
  (a credential helper that needs no person still works; one that needs
  a person fails instead of waiting), `GIT_OPTIONAL_LOCKS=0`,
  `GIT_LITERAL_PATHSPECS=1` (no `*` or `:(magic)` in a path),
  `GIT_NO_LAZY_FETCH=1`, `LC_ALL=C` and `LANGUAGE=C` (messages are
  parsed), `GIT_PAGER=cat`, `GIT_EDITOR=true`.
- **The configuration of every run:** `-c core.fsmonitor=false -c
  core.quotePath=false -c diff.external= -c log.showSignature=false -c
  color.ui=false -c core.hooksPath=<an empty folder of the node's>`, and
  `--no-ext-diff --no-textconv` on diffs: no read and no worktree
  creation runs a program the repository's configuration names. Only
  `git.commit` and `git.push` run the repository's hooks, as they would
  for the user. Filter drivers (`git lfs`) still run on a status, as for
  the user's own `git status`: add only folders one would run git in.
- **Clones:** `git clone --no-recurse-submodules --progress --
  <url> <temporary folder>`, `GIT_ALLOW_PROTOCOL=https:http:ssh:git`.
  A `CloneUrl` decodes only for those schemes or `user@host:path`, never
  `ext::` or anything starting with `-`, and refuses user information in
  the URL (a token in a URL would be kept in the project's configuration
  and in the clone's messages).
- **What git prints back** is cut to its last 64 KiB of standard error
  and redacted of anything that looks like user information in a URL
  before it reaches a reply or a log; known refusals (`GitRefusal`:
  `needs_credential`, `non_fast_forward`, `diverged`, `no_identity`,
  `hook_failed`, `conflicts`, …) are typed, the rest is git's message
  (`remote_failed`), and another git holding the index's lock is
  `unavailable`.
- **Limits per run:** 30 s for a local run, 120 s for fetch, push and
  pull, an hour for a clone; 16 MiB of standard output (a call that
  reads less says so, `more` or `too_large`). Two pools: 8 local and 4
  network runs at once, 64 waiting each; past that, `unavailable`. At a
  limit the whole process group is killed.
- **Which git:** the one on the node's search path (the harness lookup),
  found once at start, its version read once with `git version`. Older
  than 2.25 (`--end-of-options` 2.24; `switch`, `restore` and
  `for-each-ref`'s `%(worktreepath)` 2.23; Ubuntu 20.04 ships 2.25), or
  missing: the git calls answer `unsupported`, and `project.list` says
  which (`git: missing | too_old { version } | found { version }`).

## Consequences

- A user's repository behaves under Vessel as under their own git, and
  the node's binary links no git and no TLS stack for it.
- A repository's configuration cannot make a read run a program; what
  still runs is what the user's own commit and push would run (hooks)
  and filter drivers (whose configuration, in a project's own folder, an
  agent can write: see the amendment below).
- A credential that needs a person (a passphrase prompt, a browser
  sign-in) fails with `needs_credential` rather than hanging; the user signs
  in once in a terminal, and the node's runs use the helper's cache.
- A process per call costs a few milliseconds; a page asking for status,
  changes and a diff is well under 50 ms of process start.
- On a node started from a terminal, an `ssh` or `gpg` prompt opened on
  `/dev/tty` stops the run (a background group reading its terminal)
  until the time limit ends it. A new session per run (`setsid`) would
  avoid it but needs `unsafe` `pre_exec` or another crate; a node run as
  a service has no terminal.
- Only CI confirms the Windows side (`cmd.exe`, Git for Windows'
  credential manager honouring `GCM_INTERACTIVE`) and the macOS one.

## Amended when the tracks were merged (2026-11-12)

- What git prints back: every run keeps the last 64 KiB of standard
  error (network and clone runs had kept 256 KiB); what reaches a reply
  or a log is redacted whole first, then cut to its last 2 KiB (cut
  first, a cut through a URL left the end of its user information
  without the `://` that marks it).
- The name index's listing (`ls-files`) may read 64 MiB of standard
  output, past the 16 MiB of other runs: the index's own bound is
  500,000 paths. A longer listing is cut and says so.
- A call's wait for a slot of its pool counts against its time limit
  and ends with its cancellation.
- `--end-of-options` now stands before every value in `log`, `switch`,
  `branch --move`, `branch -D`, `worktree add` and `merge-base` too
  (safe before only because the names refuse a leading `-`).
- Not yet held in a project's own folder (ADR 0027's amendment says
  why): a read runs the filters the repository's configuration names,
  and an agent can write that configuration there. A worktree's `.git`
  is checked against its repository's record of it before every call.
