# ADR 0037: Searching every session's messages

Status: accepted, 2026-11-13 (milestone 11, track A2,
`docs/plans/backend-m11.md`, "Track A2: contract", "1. Search every
session's messages", and its review). The wire is protocol 12's
`session.search` (`crates/vessel-proto/src/search.rs`); the index is
schema 13 (`crates/vessel-store/src/search/`).

## Context

All three references let a person find something said in a session that
is not open: agent-orchestrator and GOAT from their command palettes over
every chat's text, Paseo only over titles, branches and project names
(its own words: "never an incomplete subset of transcripts"). A node keeps
up to 500 ended sessions of up to 64 MiB of log each, as records, not
messages: an agent's reply is a run of small `message_chunk` records
spread over many appends.

The questions:

1. What is a message, out of records?
2. How is it indexed, so a query does not read the logs, and a Korean
   word with its particle (`세션을`) or an identifier inside a longer one
   is found?
3. What does indexing cost the appends every session makes, and what does
   removing a session cost, on the one writer thread?
4. How is a query bounded, and paged?

## Options considered

- **Read the logs at query time.** Nothing to keep in step; every query
  decodes every record of every session. Refused: the cost grows with
  the log, per keystroke.
- **FTS5 with `unicode61`** (agent-orchestrator's). Words as tokens,
  ranked by bm25. A Hangul word with its particle is one token, so `세션`
  is found only as a prefix, never inside a compound; `frame` is not
  found in `parse_frame`. Refused for the owner's language.
- **FTS5 with `trigram`** (GOAT's). Substrings of three characters or
  more; shorter words cannot use it. Taken, with GOAT's answer for short
  words (read the newest messages, bounded).
- **Index inside each append.** Measured by the contract's review: a
  commit carrying a message cost 14 ms on average with the index against
  under 1 ms without, and up to a second when the index merged its
  segments; deleting a session of 2,000 messages through a trigger took
  6 s. On the writer, every session's appends wait for that. Refused.
- **Index on a thread of its own.** A second writer to one SQLite file:
  every write would contend for the lock, and the cursor rules below
  would need locks across threads. Refused.

## Decision

- **A message** is a prompt (`turn_started`, `steered`, a run of
  `user_message_chunk`) or a run of `message_chunk` text, ended by the
  next timeline record that is not one of its chunks; records outside the
  timeline (`harness_log`, usage, commands, queue, renames, model calls)
  do not end it. Thoughts, tool calls and their output, plans,
  permissions, questions and terminal output are not indexed. A message
  is cut into pieces of at most 8 KiB (after white space where one is
  near) and a run into pieces of at most 512 records; each piece points at
  the record where its run starts. Matching is per piece.
- **On an append**, inside its savepoint, the writer reads the records it
  already holds for a session whose index has read everything before
  them; a run still open stays in the writer's memory, mirrored by the
  session row's cursor (`search_seq`, `search_open`), and is read again
  from the log when the memory and the row disagree. An ended message is
  written as a plain row of `search_messages`. Indexing runs in a nested
  savepoint: a failure is logged and left to the idle steps; it never
  fails the append.
- **When the writer's queue is empty**, it runs idle steps, each one
  transaction doing one bounded piece of one kind of work: take waiting
  rows into the FTS5 table (32 KiB of text), delete what a removed
  session or pruned records left (`search_gone`, filled by triggers on
  `sessions`; 32 KiB, 128 rows), or read a session whose index is behind
  its log (256 records or 256 KiB of them; sessions stored before schema
  13). A write waits for at most one step. A step that finds nothing, or
  fails, stops the steps until the next commit.
- **Removed is hidden at once**: a query joins `sessions` and its
  `first_seq`, so a deleted session's rows and pruned records' rows are
  never shown, while their deletion waits for idle steps.
- **The index's order is the page's**: a piece's id is its time in
  nanoseconds (moved forward past one taken), so the FTS5 walk by rowid,
  descending, is newest first and a cursor is an id. Rows not yet in the
  FTS5 table are walked beside it (a partial index), so a message is found
  the moment it is written.
- **Every candidate is checked in Rust** against every word, with one
  per-character fold (lower case when that is one character), which is
  also how a snippet marks its matches. The index only narrows. The
  tokenizer folds case and not diacritics, so both agree.
- **Bounded per page**: 10,000 candidates through the index, 20,000
  messages when every word is shorter than three characters or the search
  is kept to one session (its own rows); a page that runs out returns
  what it found with `next` at the last piece read and `reached` its
  time. At most two searches run at once on a node (`unavailable`).

## Why

The trigram tokenizer is the only built-in one that finds Hangul inside
words; FTS5 ships in the bundled SQLite, so no dependency is added.
Moving every FTS5 write (insert, delete, merge) out of the append path
keeps the appends as they were: measured on a store of 220,000 appends
(20,000 messages), the median and tails of an append are the same with
the index as with records it passes over. Hiding removed rows by a join
instead of deleting them in the transaction makes `session.delete` and
retention cost what they did. Checking every candidate in Rust makes the
semantics one rule (every word, as a substring, without case) whichever
walk found it, and is what lets a snippet always mark its words.

## Consequences

- A store of 220,000 records and 20,000 messages grew from 49 to 76 MiB
  (the text a second time, and the trigram index); none of it counts
  toward retention's per-session cap.
- Queries measured at about 1 ms for words of three characters or more
  and for short words that match often; a short word that matches
  nothing reads its whole bound (about 130 ms, under a second on a
  loaded machine).
- After an upgrade, a node indexes its old sessions while idle; until it
  has, a search misses them (the store says when it has, `Store::indexed`).
- Diacritics are not folded (`café` does not find `cafe`); text is not
  normalised (NFD Hangul does not match NFC). GOAT folds both; Vessel
  would need a Unicode table of its own to do it in Rust as well.
- A word split across two pieces of one long message is not found.
  Messages a session's runs replay or import carry the time they were
  written to the log, not the time they were first said.
- Session titles are not searched: a client matches them in the list it
  has.
