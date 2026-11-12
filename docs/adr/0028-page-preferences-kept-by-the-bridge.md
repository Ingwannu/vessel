# ADR 0028: The page's preferences and drafts, kept by the bridge

Status: accepted, 2026-11-11 (milestone 11, track W,
`docs/plans/backend-m11.md`, "Track W: contract"; the wire is
`crates/vessel-web/src/wire/prefs.rs`, the behaviour
`crates/vessel-web/src/keep.rs` and `paint.rs`).

## Context

The web UI (ADR 0017) is served by `vessel ui` on a new random origin
every launch, so that nothing cached for an earlier origin can be
replayed into it. The cost is that the page has no memory: whatever it
puts in `localStorage` or a cookie belongs to an origin that is gone
next time. A theme, a text size, a key rebound, the agent last used on a
machine and a prompt half written all vanish on every restart. The three
references the milestone follows (GOAT, Paseo, agent-orchestrator) all
keep these.

Several tabs share one `vessel ui`, and two `vessel ui` processes (two
terminals, two versions of `vessel`) may run against the same files. The
content security policy forbids inline script, so nothing can set the
theme before the stylesheet paints.

Questions:

1. Who keeps the page's memory, and where?
2. Who decides the path: the bridge, or the CLI?
3. What does the file look like, and what happens to what a reader does
   not understand?
4. How do changes from several tabs, and several processes, combine?
5. How does the first paint get the right theme?
6. Where do drafts go, and how often are they written?

## Options

**Keeping**

1. The browser (`localStorage`): lost every launch with the origin.
2. A fixed origin for the page: undoes ADR 0017's defence against a
   poisoned cache.
3. The bridge, in files on this machine, behind ops on the page's
   socket.

**The path**

1. `vessel-web` works it out (`dirs::config_dir()`).
2. The `NodeSource`, the CLI's seam, says where.

**The format**

1. Strict: unknown keys refused, the file rejected.
2. Lenient by dropping: unknown keys ignored and lost on the next write.
3. Lenient by keeping: what is not understood is carried as opaque JSON
   and written back.

**Changes**

1. The page sends its whole document.
2. One change names one setting, applied on the file read right before
   it.

## Decision

- **The bridge keeps them (3).** Preferences in `ui-preferences.json`,
  drafts in `ui-drafts.json`, each owned by one task in `vessel ui`
  (`keep`). The page asks with ops (`preferences`, `prefer`, `drafts`,
  `draft`, `save_draft`); every new preferences state is pushed to every
  tab as `ServerFrame::Preferences`. Drafts are never pushed, so a read
  never overwrites what another tab is typing.
- **The source says where (2).** `NodeSource::preferences()` answers
  `PageFiles { preferences, drafts }`, asked once at start. `vessel`
  answers the book's directory for the preferences (so `--config-dir`
  moves them with the book) and the local data directory (`--data-dir`,
  `~/.vessel` by default) for the drafts. The default answer is memory
  only. A `None` is kept in memory for the launch (`no_place`); an error
  is logged once and kept in memory (`no_path`); the page says which.
  The CLI's answer is `crates/vessel-cli/src/ui/source.rs`, the one change
  outside `vessel-web` this decision needs: it returns the error rather
  than folding it into `None`, and its tests check that `--config-dir`
  moves the file with the book and that a place it cannot work out is an
  error. The file is written as the book is: owner-only, a temporary
  file in its directory renamed over the name, so a link at the name is
  replaced, never followed.
- **Lenient by keeping (3).** Each file is `{ "version": 1, ... }`. A key
  this build does not know, or a value it cannot read (one a newer page
  wrote), is kept as opaque `RawJson` and written back; the page is given
  that setting's default, and a change to the setting replaces it. The
  unit kept whole is a setting; in a map, one entry (one action's keys,
  one machine's defaults, one agent's defaults under it, an unknown agent
  or choice kind included); one draft. `version` rises only with a change
  an older reader would misread; a file with a higher one is used for the
  launch and never written (`newer`). Only what was set is written, so a
  default changed later reaches everyone who never chose otherwise.
- **The file rules.** At most 256 KiB for preferences and 1 MiB for
  drafts, read and written. A read needs the path's metadata to say a
  regular file and the open handle to agree, and reads at most the bound
  plus one byte. A write creates a temporary file in the same directory,
  mode `0600` on Unix, flushes it, and renames it over the name, which
  replaces whatever is there (a link included) and never writes through
  it; a directory it creates is `0700` (the last component only). A file
  that cannot be read, or a read or write that fails later, moves storage
  to memory for the rest of the launch (`unreadable`), the file is not
  touched again, and every tab is pushed the new state.
- **One change, one setting, in order (2).** `PreferenceChange` is tagged
  `set` with its value in `value`. A tab's preferences and drafts ops are
  handed, from its read loop and in arrival order, to the file's task
  through a bounded queue of 64 (full: `busy`), outside the tab's permits
  for node calls; their answers are written back in the same order. Each
  change reads the file again, applies itself, checks the bounds (64
  actions, 32 machines, the file's size: only a change that grows past
  one fails, as `failed`), renames, and publishes the whole document with
  a `revision` that rises by one per published state; the page drops a
  reply or a push older than what it shows. The `preferences` op reads
  the file again and publishes a change another process made. No lock
  between processes, as the book has none: two processes changing one
  setting in the same instant can lose one change.
- **The first paint.** The index (`/index.html` and every route that
  falls back to it) carries `data-theme` (absent for `system`),
  `data-ui-size`, `data-code-size` and `data-content-width` on its
  `<html>`, spliced after the built index's one `<html lang="en"` from
  the preferences task's state in memory. Each value comes from a closed
  set or a bounded integer; the stylesheet has a rule for each. The
  placeholder page is served as it is. Since the index is written per
  request, it is the one text file the web build does not compress (ADR
  0017's amendment of milestone 11 compresses the rest once, at build):
  it is sent as built, with no `Vary` and no entity tag, and an
  end-to-end test over a slow link finds the first frame dark.
- **Drafts.** Keyed by session id (unique across machines, so a renamed
  machine keeps its drafts) or the new-session page. A save is answered
  when queued; the first save after a quiet second is written at once,
  later ones within the second together at its end, and what is held is
  written when `vessel ui` stops. A write reads the file, merges this
  process's saves (per key the later save wins; an empty save removes
  unless the file holds a later one), applies the bounds (64 drafts, 128
  KiB each, the file's 1 MiB as written, 30 days unsaved; the oldest go
  first) and renames.

## Why

- The bridge is the one thing that outlives the page's origin and is
  already trusted with the fleet's credentials; a file next to the book
  is where `vessel` keeps the rest of the user's choices.
- The path is the CLI's to know: only it parses `--config-dir`,
  `--data-dir` and finds the book, and a library never reads the user's
  home directory or environment (AGENTS.md section 8). The bridge's tests
  run with no disk at all through the default.
- Drafts are prompt text: the data directory already holds the
  sessions' logs, while a configuration directory is often a dotfiles
  repository, and on Windows it roams to a server.
- Keeping what is not understood is what lets an older and a newer
  `vessel` share a file without either breaking or trimming it.
- Ordering through one queue per file, with a revision, is what makes two
  changes from one tab land in order and keeps a slow reply from
  overwriting a newer push; a change that names one setting is what lets
  two tabs change two settings without losing either.
- Splicing the appearance into the index is the only way to paint right
  first under a policy with no inline script, without a cookie that the
  new origin would not have anyway.

## Consequences

- The index is served before a tab proves its key, to anything on this
  machine that can reach the port (by the launch's address or a literal
  loopback name) and to whatever a port forward exposes it to. The
  painted values say how the page looks and nothing else.
- A temporary file a crash leaves between create and rename stays (another
  `vessel ui` may be writing it at that moment); it is hidden, named by
  its process, and at most the bound.
- A FIFO swapped in between the metadata check and the open would make a
  read wait; doing better needs `O_NONBLOCK`/`O_NOFOLLOW`, a dependency
  for their flags, against someone who can already replace files in the
  person's own directory.
- Two processes changing one setting, or saving one draft, in the same
  instant can lose one; between processes the window is one
  read-change-rename, as for the book.
- The 1 MiB of drafts is measured as written, so seven drafts at the
  128 KiB limit fit, not eight; text the JSON encoding escapes (control
  characters) counts at its escaped length.
- `vessel ui` reads the preferences once before serving its first page,
  and writes the drafts it holds before exiting.
