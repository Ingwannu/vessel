# ADR 0030: git runs apart from what a repository's folder supplies

Status: accepted, 2026-11-12 (milestone 11, wave 2, track B2,
`docs/plans/backend-m11.md`, "Track B2: contract"; the code is
`crates/vessel-workspace/src/git/run/own_config.rs`, `run.rs`, `pool.rs`,
and the node's `workspace/policy.rs`, `projects/adding.rs`,
`worktrees/place.rs`). Amends ADRs 0023 and 0027.

## Context

ADR 0023 runs the `git` program for every workspace call and turns off
what a read does not need (`fsmonitor`, the hooks, an external diff, text
conversion). It trusted the rest of a repository's configuration as the
user's: "add only folders one would run git in". Wave 1's review found
where that does not hold (plan, "Integration (node)"):

1. **Finding 1.** A repository's `.git/config` (and `config.worktree`,
   and any file either includes) is written by whoever can write in the
   project's folder: an agent whose tools may edit files but not run
   commands (a common permission mode), or whoever packed a folder the
   user added. Several keys there name a program git runs: a filter
   (`filter.<driver>.clean`, chosen by `.gitattributes`, run by a plain
   `git status`), a merge driver, `core.sshCommand`, a credential helper,
   a signing program, a remote's `uploadpack`. The node runs git on the
   user's behalf, often without anyone asking (the page's status, the
   forge's watcher, ADR 0029), so such a key ran a program nobody chose.
2. **Finding 17.** A `.git` folder, or a bare repository, could be added
   as a project: a `git: false` project whose `config` and `hooks/` the
   file calls then write.
3. **Finding 7.** Between `place` checking a worktree and the session
   being live, a removal could take the folder.
4. **Finding 11.** Clones (up to an hour each) shared the four network
   slots with every fetch, push and pull.

Constraints: git has no variable that makes it skip a repository's
configuration (`$GIT_COMMON_DIR/config` is always read); the user's own
global and system configuration must keep working (their credential
helper and `ssh`, `git lfs` installed globally); the node supports git
from 2.25.

Measured on git 2.43 before deciding: a `-c <key>=<value>` on the
command line wins over the repository's files and over everything they
include (`include.path`, `includeIf`); an empty filter command turns a
filter off; an empty merge driver makes git's merge of that file fail,
running nothing; an empty `diff.<driver>.command` or `.textconv` is run
as an empty command and fails, so it cannot be emptied, but
`--no-ext-diff --no-textconv` ignore both; `GIT_ATTR_SOURCE` (2.40+)
hides `.gitattributes` but still reads `.git/info/attributes`; an alias
never shadows one of git's own commands; a pager runs only on a
terminal; `GIT_ALLOW_PROTOCOL` stops `ext::`, a remote's `vcs` helper and
a repository's `protocol.allow=always`.

## Options

1. **Refuse every run** in a repository whose configuration names any
   program. Simple, and wrong for the common case: a per-repository
   `core.sshCommand` for a deploy key would refuse even `git status`.
2. **A configuration of the node's** in place of the repository's (a
   shadow `GIT_DIR`): git reads the common directory's `config` whatever
   `GIT_DIR` says, and refs, the index and objects would all have to be
   redirected. Not possible without patching git.
3. **An empty attribute source** (`--attr-source`, git 2.40+), so no
   path selects a driver: misses `.git/info/attributes` and
   `core.attributesFile`, and older gits.
4. **Overrides for keys known in advance**: every driver is named by the
   repository (`filter.<its name>.clean`), so its keys cannot be known
   before reading the configuration.
5. **List the repository's own configuration before every run, then
   turn off what can be turned off and refuse what cannot.**

## Decision

Option 5.

- **Before every run in a repository**, inside the run's slot and time,
  the node lists the repository's own configuration: `git config --local
  --list -z --includes`, and `--worktree` when `extensions.worktreeConfig`
  is on. That is `.git/config` (a linked worktree reads its repository's),
  `config.worktree`, and every file they include. The user's global and
  system configuration are not listed and are left alone. Outside a
  repository (a clone, an init, the version probe) nothing is listed.
- **Turned off on the command line** for that run:
  - every filter the repository defines: `filter.<driver>.clean=`,
    `.smudge=`, `.process=`, `.required=false`;
  - every merge driver it defines: `merge.<driver>.driver=` (the file's
    merge then fails; nothing runs);
  - on every run, whatever the repository sets: `core.fsmonitor=false`,
    `core.hooksPath=<an empty folder>` (but for commit and push),
    `diff.external=`, `submodule.recurse=false`,
    `fetch.recurseSubmodules=false`, `merge.verifySignatures=false`,
    `tag.gpgSign=false`, `push.gpgSign=false`, `commit.gpgSign=false`
    (but for commit, so only a person's commit signs), `--no-pager`,
    `GIT_EDITOR=true`, `GIT_SEQUENCE_EDITOR=true`, `GIT_ASKPASS=`
    (empty), and `GIT_ALLOW_PROTOCOL=file:git:http:https:ssh` (a clone
    narrows it to `https:http:ssh:git`);
  - status, diffs and the removal's inspection never go into a
    submodule's working tree (`--ignore-submodules=dirty`), where git
    would run under the submodule's own configuration, which this
    listing does not cover; a gitlink keeps a worktree from removal
    anyway;
  - diff drivers (`diff.<driver>.command`, `.textconv`): every diff the
    node asks for passes `--no-ext-diff --no-textconv`, which ignore both
    (they cannot be emptied).
- **Refused**, naming the key and never its value, when the repository's
  own configuration sets one and the run would use it, because turning
  it off on the command line would also turn off the user's own global
  setting:
  - a network run (fetch, pull, push, a person's `pr.open` push), for a
    program: `core.sshCommand`, `core.gitProxy`,
    `core.alternateRefsCommand`, `credential.helper` and
    `credential.<url>.helper`, `remote.<name>.uploadpack`,
    `remote.<name>.receivepack`, `remote.<name>.vcs`; and for where the
    credential the user's own helper hands out would go: `http.proxy`,
    `http.sslVerify`, `http.sslCAInfo`, `http.sslCAPath`, `http.sslCert`,
    `http.sslKey`, `http.sslBackend`, `http.curloptResolve`,
    `http.extraHeader`, `http.cookieFile` (each also per URL),
    `remote.<name>.proxy`;
  - a commit or a push: `gpg.program`, `gpg.<format>.program`,
    `gpg.ssh.defaultKeyCommand`;
  - every run: `core.worktree` (it moves the working tree out of the
    project's folder); a driver whose name holds `=` (git's `-c` would
    split it, so it could not be turned off); more than 64 drivers of one
    kind; a configuration over 256 KiB.

  The wire says it as `GitRefusal::repository_config { key?, reason }`
  (protocol 11) on a git write, and as `conflict` with the key on a read.
  A person who wants Vessel to use such a setting moves it to their
  global configuration (under `includeIf "gitdir:…"` to keep it to that
  repository).
- **Kept, and said**: commit and push run the repository's hooks (its
  `.git/hooks`, or the `core.hooksPath` its configuration names), as the
  user's own `git commit` does; those calls are a person's only (agents
  are refused them, ADR 0027). Keys git runs only from commands the node
  never runs (`difftool.*`, `mergetool.*`, `submodule.<name>.update`,
  `trailer.*.cmd`, `sendemail.*`, `web.browser`) are left alone. A
  bare repository's `core.bare = true` is not refused per run: its linked
  worktrees share that configuration and git ignores the key there.
- **Git's own folders are not projects**: `project.add`, `project.init`
  and a clone's target refuse a folder any part of whose path names
  `.git` by git's own rules (any case, trailing dots or spaces, `git~1`,
  ignored zero-width characters),
  a folder holding `HEAD`, `objects` and `refs` (a bare repository or a
  `.git` folder by another name, told apart without git), and a folder
  git says is inside a git directory or bare (`git rev-parse
  --is-inside-git-dir --is-bare-repository`).
- **Between checking a place and starting in it**: `place` takes a
  "starting" count on the worktree in the same lock as its check, and
  `session.resume`'s check takes it the same way; the caller holds it
  until the session is live or failed to start. A removal claims the
  worktree first (so no `place` or start again gets in), then counts the
  starting ones and reads the live sessions: a session is either seen
  (`kept { in_use }`) or refused.
- **Clones** take slots of their own: 2 at once and 16 waiting, apart
  from the 4 network slots of fetch, push and pull.

## Why

- Listing first is the only way to name what the repository defined (a
  driver's name is its own choice), and a command-line setting is the
  one thing git lets win over a repository's files and their includes.
- Turning off rather than refusing keeps every repository usable that
  merely uses git's features (an LFS filter defined locally, a merge
  driver for lock files): Vessel's reads see the bytes git stores, which
  is what an agent's diff needs anyway.
- Refusing the network and signing keys rather than overriding them,
  because the user's own global value is the one that must keep working
  (their credential helper, their `ssh`); an override can only replace
  it, not restore it.
- Refusing the proxy, TLS, resolver, header and cookie keys on a
  network run because the credential is the user's (their helper hands
  it to whatever URL git asks about), and a repository's configuration
  could otherwise route it through someone else.
- Refusing `core.worktree` because a project's reader would otherwise
  read another folder through git (status, diffs) while the file calls
  read the project's.

## Consequences

- One more `git config` per run, inside the run's slot and time (about
  2 ms on the development machine). The forge's watcher reads a branch
  per poll, so it pays it too.
- A repository whose own configuration sets a deploy key's
  `core.sshCommand` cannot be pushed from the page until the setting
  moves to the user's global configuration; the refusal says which key
  and what to do.
- A repository with a locally defined LFS filter shows the stored
  pointers in Vessel's diffs; the user's own git still smudges and
  cleans in their terminal.
- A merge through the node (stash apply or pop) of a file with a
  repository-defined merge driver fails instead of running the driver.
- Not closed: a process of the user's that rewrites the configuration
  between the listing and the run (ADR 0027: checks inside the node, not
  a sandbox); a key git adds later that names a program, until it is
  added to `own_config.rs` with a test that plants it.
- Tests (`own_config/tests.rs`): each vector planted in a real
  repository (filters on status, diff, staging, a commit and a worktree's
  checkout; a merge driver on a stash apply; `fsmonitor`, pagers,
  editors, an alias, a hooks folder; keys in an included file and an
  `includeIf`; `ssh`, a proxy, a credential helper, an `uploadpack`, a
  `vcs` helper and an `ext::` remote on a fetch; a signing program on a
  commit; a moved working tree; a worktree's own configuration) and shown
  not to run, or the run refused naming the key; the credential keys are
  judged on listings. The node's tests (`workspace/tests/git_apart.rs`)
  refuse a `.git` folder, a folder inside one and two bare repositories
  as projects, keep a worktree while sessions are starting in it, and
  refuse a start again once a removal holds the worktree.
- Measured on git 2.43 only; the planted tests run with whatever git CI
  has, and fail if a driver runs.

## Amended when wave 2's first half was joined (2026-11-12)

- What ADR 0031 starts in a repository's folder (a terminal, a script, a
  service) and ADR 0027's setup and teardown are not git runs of the
  node's: git started inside them (a person typing `git`, a shell prompt
  that shows the branch, a script that commits) runs under the
  repository's configuration as it would in the user's own terminal.
  They run only where a person asked (a terminal; an agent's only where a
  person turned `agent_terminals` on) or approved (the digest of ADR
  0027, which covers scripts), so this ADR's rule, which is about git the
  node runs without anyone asking, does not reach them; neither holds a
  credential of the node's or the forge's sign-in (ADR 0031's amendment).
- The "starting" count also covers a terminal or a script's run starting
  in a worktree (`hold_folder`), held from before its reservation until
  it is registered, so a removal sees it or refuses it.
- Not changed by the join's review, and why: commit and push (a person's
  only, `pr.open`'s push included) run the repository's hooks with the
  user's environment, the forge's token variables included, as the user's
  own `git push` does. Removing them would break a push whose credential
  helper (`gh auth git-credential`) reads `GH_TOKEN`, and would not keep
  a hook from the credential: it can ask the user's helper, or `gh`.
