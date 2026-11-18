# ADR 0109: A ranked history listing with prompt previews and search, as Paseo's import sheet

Status: accepted (number assigned by the coordinator on 2026-11-18 for TRACKER 790); impl U. The protocol
number is assigned at merge (drafted as 29, `RANKED_HISTORY_PROTOCOL`). Partly supersedes ADR 0053: its
"no previews", "no search on the agent's list", "nothing is loaded" and "two listings at once, nothing waits"
rulings change here; its cursor listing, its claim on a conversation and its import path stay.

## Context

Paseo's import sheet (`import-session-sheet.tsx`, pinned) and its daemon (`import-sessions.ts`,
`agent-manager.ts` `listImportableSessions`, the ACP provider's `listImportableSessions`) give each row a
prompt preview, search on the daemon, page by asking a larger limit, and say "All recent sessions are already
imported." from a count of what they left out:

- `fetch_recent_provider_sessions { providers, cwd, since, limit (default 20, at most 200), query }` answers
  `{ entries, filteredAlreadyImportedCount, providerErrors }`.
- Without a query the provider lists `limit + imported` conversations; with one it looks through 500. Each is
  loaded (`session/load`) to gather its replayed user messages: the first and the last, blanks folded, cut at
  160. A load waits at most 30 s and all loads stop 60 s after the listing began; past that a row has no
  preview. A conversation whose replay held nothing is dropped. Previews are cached per conversation while
  its `updatedAt` is unchanged. A loaded conversation is closed (`session/close`, 2 s) when the agent offers it.
- The daemon drops the already imported (counting them), matches the query (title, first and last prompt,
  the folder's last name, letter case aside), sorts by last activity and keeps `limit`.
- The sheet asks 15 per provider, then 45, 90, 200 ("Load more"): "Paging grows the per-provider limit
  instead of carrying an offset ... an offset into a previous ranking would drift as sessions move." Search is
  debounced 200 ms and gated on the host's `importSessionSearch` feature.

Vessel's `agent.history` (protocol 17, ADR 0053) gave the agent's own pages behind a cursor with no previews,
and the node listed two at once, answering more `unavailable`. The native sheet (TRACKER 777) therefore read
pages until it had its limit of not-yet-imported rows, filtered on the client, showed no preview, and queued
its reads two at a time.

## Decision

[Decision Log]
- Purpose: the node answers what Paseo's daemon answers, so the page and native sheets drop their stand-ins
  (client filtering, client queueing, no preview line).
- Existing assets: `agent.history` and its listing connection (`vessel_agent::history`), the node's
  `in_vessel` lookup (`service/history/known`), the probe launch.
- Alternatives:
  (a) keep ADR 0053: rows without previews, client search over loaded pages (what 777 does; differs from Paseo);
  (b) read each harness's files for prompts (ADR 0053 refused this: breaks with every format change);
  (c) a new method beside `agent.history`;
  (d) previews from a fresh agent run per conversation.
- Choice: `agent.history` gains `limit` (1 to 200) and `query` (needs `limit`), and its reply
  `already_imported` and per entry `first_prompt` and `last_prompt`, gated together on one protocol number.
  - With `limit` the node gives the listing the conversations already on it in that account. The listing reads
    the agent's pages in order and leaves those out, counting the ones that match the query (from a preview
    read before, else the title and folder). It loads every other one on the same connection with no MCP
    servers (`vessel_agent::history::ranked`), gathering the replayed user messages (Paseo's collector: one
    prompt per message id). It drops a replay with nothing of a conversation in it and keeps the query's
    matches. It stops at `limit` rows or 500 looked through.
  - Loads: at most 30 s each, none after 60 s from the first page, close at most 2 s when the agent offers
    `session/close`; a failed or late load keeps its row without prompts. The whole listing is bounded by 90 s
    (Paseo's per-provider timeout) or the handshake timeout, whichever is longer.
  - Previews are kept by the node per harness and account and conversation while its time is unchanged (at
    most 4,096), so a conversation had since is read again and another account's are its own.
  - The reply is sorted newest first (no time last), carries no `next` and no `in_vessel`.
  - A cursor with `limit` and a `query` without it are `invalid_request`. The cursor listing is unchanged for
    older clients and loads nothing.
  - Places (`service/history/places`): four listings at once and sixteen waiting in order, each at most 90 s,
    `unavailable` past either.
- Why: one call gives a sheet everything Paseo's does, with Paseo's bounds; (c) would duplicate the launch,
  the account lookup and the scope rules for the same question; (d) could act (a run is a prompt), which the
  coordinator ruled out: previews come only from a load on the listing connection, which replays and starts
  no turn, and the listing client answers no tool, file, terminal or permission request of the agent's.
- Costs and effects:
  - A ranked listing costs one agent start and a load per row it answers (seconds each, kept after), up to 90 s.
    Claude's adapter starts a process per loaded conversation; they end with the listing process (its group is
    stopped) when the adapter does not close them.
  - An agent whose load acts would act here. ACP's `session/load` replays a conversation; the two installed
    adapters replay only. An adapter found to do more must be left without previews (its listing kept
    cursor-only), not worked around.
  - A client may wait up to 90 s for a place before its listing starts; its deadline for a ranked read covers
    both (the native sheet's read deadline grows with the client packet).
  - Drivers that list but cannot load (none yet: Codex's and Claude's direct drivers do not list) answer their
    first page ranked the same way, without prompts.
  - Paseo also drops its own metadata-generation conversations by their first prompt; Vessel starts none in the
    agent's history, so there is nothing to drop.

## Consequences

- Clients: when a node's protocol is at least `RANKED_HISTORY_PROTOCOL`, Paseo's sheet asks per provider (and
  per account) `limit` 15, 45, 90, 200 with a 200 ms debounced `query`, shows `last_prompt` else
  `first_prompt` else "No prompt preview" in two lines, and "All recent sessions are already imported." when
  nothing is left and the summed `already_imported` is not zero. Older nodes keep the cursor listing and the
  clients' stand-ins. The page on the node line has no import sheet yet; the native sheet's change is an
  app-line packet after this one.
- Tests: the budgets are checked on a paused clock over a scripted agent (`ranked/tests.rs`: a load given up
  at 30 s, loads stopping at 60 s, a close left at 2 s, the 500 scan, the cache), the places likewise
  (`places.rs`), and the wire end to end with `fake-acp` (`tests/e2e/history.rs`).
