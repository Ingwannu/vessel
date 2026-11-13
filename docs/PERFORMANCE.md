# Performance

What is measured, how to run each measurement again, the figures before
and after each change made for speed, and what was tried and did not
help. Milestone 11, track P (`docs/plans/backend-m11.md`) started it: no
optimisation without a number before and a number after, from the same
harness on the same machine. A change that does not move its number is
reverted and listed under "Tried, did not help".

## Where it stands

Milestone 11, track P, before → after, from the sections below; the last
column is the page once track W's work was merged with it ("Tracks P
and W merged", measured back to back with track P's tree alone on
2026-11-12, so its figures sit beside that run's, not beside the
column before it):

| Measured | Before | After (P) | Merged with W | Changes that moved it |
| --- | --- | --- | --- | --- |
| board usable, cold, over a 1.6 Mbit/s link | 17,903 ms | 2,561 ms | 2,664 ms (P that day: 2,573) | compression, Pretendard subsets, the page divided |
| … the same on a phone (CPU 4× slower) | 18,994 ms | 3,028 ms | 3,274 ms (3,238) | the same |
| first contentful paint over that link | 7,028 ms | 1,636 ms | 1,704 ms (1,652) | compression, the page divided |
| bytes for a cold board | 3,371,739 B | 326,422 B | 335,114 B (326,629) | the same three; the index uncompressed and track W's code (+8,485 B) |
| a 20,000-entry timeline: last entry drawn | 37,127 ms | 2,362 ms | 2,845 ms (2,645) | memoised entries, windowing, the reducer's index |
| … a scroll step | 265 ms | 33 ms | 33 ms (33) | windowing |
| … heap; elements | 107.7 MiB; 180,147 | 12.8 MiB; 501 | 13.1 MiB; 507 (12.8; 501) | memo, windowing |
| a board of 500 cards at rest: long tasks per 10 s | 15 (1,314 ms) | 0 | 0 (0) | an unchanged list draws nothing |
| find over 20,000 entries: typed to the count | — | — | 138 ms (100 of them the field's settle) | find yields by a message |
| … a match far away: typed to it shown; how far it moves after | — | — | 154 ms; 0 px | the window goes to a row by key and holds it |
| CPU-bound hot paths (frame encode/decode, bridge, fan-out) | — | 5–12% faster | not changed | fat LTO, one codegen unit |
| `vesseld` binary | 42.6 MB | 28.5 MB | 28.5 MB | the same |
| a clean release build | 193 s | 694 s | not measured again | the same (the cost) |

Track B-page added the code panel's two measurements (a diff of 4,000
hunks in one file beside 1,500 changed files: 764 ms from the tab to its
first line, 33 ms a scroll step; a folder of 5,000: 390 ms, 33 ms) and
moved the board's load and a long session's opening; its section, below,
has both and why.

## The machine

Every figure here comes from one machine: a virtual machine with 16
cores of a 2.6 GHz Xeon (E5-2697A v4, 2016) and 41 GiB of memory, Linux
6.8, Rust 1.93, Node 24, and the Chromium of Playwright 1.63 (headless).
It is shared: other builds ran beside some measurements, so each block
records the one-minute load average when it started. Absolute figures
will differ elsewhere; the before/after pairs were taken under the same
conditions, back to back.

## How to run them

### Benchmarks of the node's hot paths

Criterion (a dev-dependency only), one bench target per crate:

```bash
cargo bench -p vessel-store   --bench log      # append a batch, read a page
cargo bench -p vessel-proto   --bench frames   # a subscription event's text, encoded and decoded
cargo bench -p vessel-rpc     --bench framed   # frames over a byte stream (local socket, SSH stdio)
cargo bench -p vessel-session --bench fanout   # a terminal's output to 1, 4 and 16 followers; the board's reducer
cargo bench -p vessel-web     --bench bridge   # the bridge's work per forwarded record
```

`-- --save-baseline <name>` keeps a run; `-- --baseline <name>` compares
the next one with it. The benchmarks compile in the gate (`cargo clippy
--all-targets`) and run only on demand.

### The load test

The real `vesseld` and `vessel ui`, each its own process, many sessions
on the node, one browser tab (the test itself, speaking the page's
frames):

```bash
cargo test --release -p vessel-cli --test load -- --ignored --nocapture
```

`VESSEL_LOAD_SESSIONS` sets the terminal sessions (default 32, at most
64, one tab's limit). It reports, per workload, records a second, bytes
a second, latency from a record's `at` (when the node numbered it) to
when the tab decoded it at the median, the 99th percentile and the
maximum, and the resident memory of `vesseld` and `vessel ui` (Linux).
Workloads: a burst (every terminal session writes 5,000 lines of 100
bytes at once: the ceiling, where latency is queueing), a paced stream
(every session 20 lines of 100 bytes every 100 ms for 10 s: the latency
a person watching sees), and an agent flood (8 structured sessions
streaming message chunks for 5 s, then cancelled).

### The page

```bash
cd web && npm run build && cd .. \
  && cargo build --release -p vessel-cli -p vessel-node -p vessel-testkit --bins \
  && cd web && VESSEL_CARGO_PROFILE=release npm run perf
```

`e2e/perf.ts`, run on demand only. Results go to `artifacts/perf.json`
(`VESSEL_PERF_OUT`), each figure the median of several runs with its
minimum and maximum (`VESSEL_PERF_RUNS`). Six measurements: the board
loading cold, a timeline of 5,000 and 20,000 entries, find in that
timeline and going to a match far from the view (`e2e/perf/find.ts`,
since tracks P and W were merged), a terminal under heavy output, a
board of 500 cards, a session's code panel over a large change and a
large tree (`e2e/perf/code.ts`, since track B-page). `VESSEL_TARGET_DIR`
points the fleet at another build's binaries (another tree measured
beside this one); `npm run perf -- --grep-invert "code panel"` leaves
out the code panel for a tree that has none.

## Baseline, before any change (2026-11-11)

Commits `7be984f` (the Rust harness) and `d0d417e` (the page's): the code
as milestone 10 left it. Release binaries, cargo's default release
profile.

### The page

Chromium 153 headless; median (minimum–maximum) of 5 runs. Load averages
during the blocks: 14 → 8 (load), 9 → 15 (timeline), 15 → 12
(terminal), 13 → 7 (board).

**Loading the board, cold.** A fresh browser context (empty cache) and a
fresh launch for each run. The fleet has 3 live sessions; one card's
title is Hangul, so the board draws Pretendard. *Board usable*: from the
navigation's start to the first animation frame in which the first lane
and every card are laid out and no font face is still loading. Bytes are
`encodedDataLength` (on the wire, headers included) of every response
finished by then. Profiles: *desktop* 1440×900, no throttling; *slow
link* 1.6 Mbit/s down, 750 kbit/s up, 150 ms per request; *phone* the
slow link, 390×844 at 3×, touch, the CPU slowed 4×.

| | desktop | slow link | phone |
| --- | --- | --- | --- |
| first contentful paint | 308 ms (280–328) | 7,028 ms (6,992–7,052) | 7,552 ms (7,440–7,852) |
| board usable | 509 ms (437–535) | 17,903 ms (17,889–17,949) | 18,994 ms (18,792–19,324) |
| script | 1,174,434 B | same | same |
| stylesheet | 83,858 B | same | same |
| font | 2,112,132 B (Pretendard 2,057,688) | same | same |
| HTTP total | 3,371,739 B | same | same |

**A timeline of 5,000 and 20,000 entries.** `fake-acp`'s `entries <n>`:
a message (a short Markdown paragraph), a thought and a completed tool
call in turn, so the reducer merges none; the turn has ended before the
page opens. Timed from the click on the session's card. Long tasks are
the Long Tasks API's (50 ms or more). Scrolling: from the bottom to the
top in 100 equal steps, each waiting two animation frames.

| | 5,000 | 20,000 |
| --- | --- | --- |
| first entry drawn | 58 ms (44–62) | 60 ms (45–62) |
| last entry drawn | 3,278 ms (3,244–3,675) | 37,127 ms (34,677–38,710) |
| long tasks until then: total | 2,769 ms (2,684–3,190) | 35,150 ms (33,166–36,760) |
| … the longest | 400 ms (374–428) | 1,510 ms (1,445–2,310) |
| scrolling: long tasks, total | 490 ms (64–743) | 26,536 ms (24,177–27,520) |
| scrolling: a step, median | 62 ms (47–65) | 266 ms (237–277) |
| JS heap | 31.5 MiB | 107.7 MiB |
| elements in the document | 45,147 | 180,147 |

**A terminal under heavy output.** `fake-tui`'s `flood <lines> 100`
typed into the terminal. *Rate*: the flood's bytes over the time from the
Enter key to the first frame showing `flooded <lines>`. A frame is late
when its gap exceeds 1.5× the resting interval (16.7 ms).

| | 2.04 MB | 10.2 MB | 51 MB |
| --- | --- | --- | --- |
| Enter to the last line | 824 ms (494–996) | 1,914 ms (1,855–2,023) | 8,513 ms (7,898–9,161) |
| rate | 2.48 MB/s (2.05–4.13) | 5.33 MB/s (5.04–5.50) | 5.99 MB/s (5.57–6.46) |
| frames missed | 28 (7–35) | 11 (8–22) | 43 (20–107) |
| long tasks | 0 | 0 | 0 |

The 2.04 MB figure moves most with the machine's load: an earlier run at
a load of 2 measured 450 ms and 4.54 MB/s.

**A board of 500 cards.** 500 live terminal sessions on one node, the
board for all machines.

| | |
| --- | --- |
| board usable, every card drawn | 1,398 ms (1,323–1,518) |
| long tasks while loading | 7, 1,085 ms in all, the longest 249 ms |
| long tasks in the 10 s after, untouched | 15, 1,274 ms in all, the longest 111 ms |
| scrolling the fullest lane: a step | 33 ms median, 198 ms the longest |
| JS heap; elements | 28.8 MiB; 10,136 |

### The node's hot paths

Criterion, the medians of its samples; cargo's default release profile
(no `[profile.release]`). Load average 9–11. The store's figures are disk
I/O on a shared virtual disk and move by up to a quarter between runs of
the same build; the others are CPU and move by a few percent.

| Benchmark | Median | Rate |
| --- | --- | --- |
| store: append 1 terminal record (4 KiB) | 473 µs | 2,116 records/s |
| store: append 16 terminal records | 1.98 ms | 8,099 records/s |
| store: append 256 terminal records | 46.2 ms | 5,538 records/s (22 MB/s) |
| store: append 1 message chunk | 176 µs | 5,695 records/s |
| store: append 256 message chunks | 2.32 ms | 110,130 records/s |
| store: read a page of 128 terminal records | 1.59 ms | 80,303 records/s |
| store: read a page of 128 message chunks | 248 µs | 515,556 records/s |
| proto: encode an event frame, terminal 4 KiB | 7.0 µs | 805 MB/s |
| proto: decode it | 5.6 µs | 1,001 MB/s |
| proto: encode an event frame, message chunk | 712 ns | 339 MB/s |
| proto: decode it | 1.8 µs | 137 MB/s |
| rpc: 256 terminal frames through `Framed` | 1.10 ms | 1,305 MB/s |
| rpc: 256 message-chunk frames | 86.2 µs | 654 MB/s |
| session: 200 KB of terminal output to 1 follower | 3.31 ms | 60 MB/s |
| … to 4 followers | 3.35 ms | 60 MB/s |
| … to 16 followers | 3.60 ms | 56 MB/s |
| session: the board's reducer | 6.1 ns | |
| web: the bridge forwards a terminal record (decode + encode) | 12.8 µs | 437 MB/s |
| web: … a message chunk | 2.7 µs | 90 MB/s |

What it says: the session's output path costs about the same for 16
followers as for one (a shared `Arc` per record); per record, the bridge's
decode-and-encode (12.8 µs for 4 KiB) is about the node's own encode
(7.0 µs) plus a decode; the store takes terminal output at about 22 MB/s
a writer, so a node's sessions together cannot persist faster than that
(live followers are not held back until a session's durable channel, 256
records, is full).

### The load test

32 terminal sessions and 8 structured ones on one node, one tab, release
binaries, one run (load average 3–9). Latency is from the record's `at`
to the tab.

| Workload | Records | Rate | Latency p50 / p99 / max |
| --- | --- | --- | --- |
| burst: 32 × 5,000 lines of 100 B at once | 5,348 (15.4 MiB) | 28,760 records/s, 82.9 MiB/s | 39.9 / 95.0 / 103.8 ms |
| paced: 32 × 20 lines every 100 ms, 10 s | 9,306 (6.2 MiB) | 935 records/s | 5.7 / 45.1 / 45.6 ms |
| agent flood: 8 sessions, 5 s, then cancelled | 193,115 | 35,817 records/s | 5.5 / 20.4 / 41.9 ms |

| Resident memory | `vesseld` | `vessel ui` |
| --- | --- | --- |
| started, no sessions | 20.4 MiB | 16.8 MiB |
| 32 terminal sessions, idle | 24.9 MiB | 17.7 MiB |
| after the burst | 44.8 MiB | 23.2 MiB |
| after the agent flood | 61.2 MiB | 23.8 MiB |

The burst lasts a fifth of a second, so one run of it moves by tens of
percent (a second run of the same binaries: 0.32 s, 18,218 records/s);
it says where the ceiling is, not small differences.

### The release build

`cargo build --release -p vessel-cli -p vessel-node --bins` from an empty
target directory (load average 6–16): 193 s. `vesseld` 42,606,320 bytes,
`vessel` (with the page embedded) 54,879,064 bytes.

## Changes, each measured

Each row's "before" is the commit before the change, measured with the
same harness right before it; "after" is the change's own commit.
Medians, with the minimum and maximum of the runs.

### Loading the board, cold

Three changes, in this order: compressed responses (`262518b`),
Pretendard in its unicode-range subsets (`2e6597b`), the page divided
by screen with every chunk pinned (`da42f93`). 5 runs per profile;
load average 3–8.

| Profile, figure | Baseline | Compressed | + subsets | + divided |
| --- | --- | --- | --- | --- |
| slow link: first contentful paint | 7,028 ms (6,992–7,052) | 2,212 ms (2,208–2,232) | 2,256 ms (2,252–2,264) | 1,636 ms (1,636–1,652) |
| slow link: board usable | 17,903 ms (17,889–17,949) | 13,113 ms (13,096–13,130) | 3,175 ms (3,145–3,196) | 2,561 ms (2,535–2,568) |
| phone: first contentful paint | 7,552 ms (7,440–7,852) | 2,664 ms (2,624–2,720) | 2,716 ms (2,696–2,736) | 1,948 ms (1,912–1,956) |
| phone: board usable | 18,994 ms (18,792–19,324) | 14,026 ms (14,011–14,085) | 3,851 ms (3,803–3,873) | 3,028 ms (3,007–3,070) |
| desktop: first contentful paint | 308 ms (280–328) | 268 ms (264–312) | 256 ms (256–280) | 188 ms (180–200) |
| desktop: board usable | 509 ms (437–535) | 431 ms (418–484) | 404 ms (386–449) | 330 ms (315–339) |
| script on the wire | 1,174,434 B | 290,080 B | 290,080 B | 177,535 B |
| stylesheet on the wire | 83,858 B | 13,966 B | 22,754 B | 22,873 B |
| font on the wire | 2,112,132 B | 2,112,132 B | 124,074 B | 124,439 B |
| the index | 1,315 B | 961 B | 960 B | 1,575 B |
| HTTP total | 3,371,739 B | 2,417,139 B | 437,868 B | 326,422 B |

- **Compressed responses.** Brotli for every text file, made once by the
  web build (`web/scripts/compress.mjs`; 1,241 KiB of text become
  300 KiB) and chosen per request by `Accept-Encoding`
  (`vessel-web`'s `encoding`). First paint over the slow link fell by
  4.8 s; the board still waited for the 2 MB font.
- **Pretendard subsets.** The package's 92 unicode-range files instead
  of one: the board's Hangul title took 121 KiB of font instead of
  2,057,688 bytes, and the board was usable 9.9 s sooner over the slow
  link. Their 92 `@font-face` rules cost 8.8 KB more stylesheet and
  44 ms of first paint over the slow link (the ranges do not overlap).
- **Divided by screen.** The entry is 339 KB (108 KB gzip) instead of
  one 1,124 KB script; a session's screen (xterm.js, Markdown) is a
  477 KB chunk (127 KB gzip) fetched when a session opens, Settings
  28 KB, a new session 7 KB, the kit's gallery 15 KB; a 304 KB chunk the
  entry and the screens share is preloaded by the index. The index grew
  by its import map (615 B).

### A timeline of 20,000 entries (and 5,000)

In this order: memoised entries (`98a13ed`), the timeline windowed
(`1f70205`), the reducer's index of tool calls (`5b5b991`). "Divided" is
the commit before them (`39d6c77`, the page divided by screen); 3 runs
each, load average 2–10. Figures for 20,000 entries; 5,000 below.

| Figure (20,000 entries) | Baseline | Divided | + memo | + windowed | + index |
| --- | --- | --- | --- | --- | --- |
| first entry drawn | 60 ms (45–62) | 111 ms (95–112) | 95 ms (94–258) | 95 ms (95–95) | 96 ms (95–96) |
| last entry drawn | 37,127 ms (34,677–38,710) | 32,711 ms (31,377–33,376) | 15,426 ms (15,358–17,077) | 4,361 ms (3,895–4,845) | 2,362 ms (2,263–2,429) |
| long tasks until then | 69, 35,150 ms | 68, 31,250 ms | 69, 13,902 ms | 27, 2,134 ms | 0 |
| the longest of them | 1,510 ms | 1,354 ms | 514 ms | 175 ms | none |
| scrolling: long tasks, total | 26,536 ms | 22,892 ms | 25,557 ms | 50 ms (0–179) | 0 ms |
| scrolling: a step, median | 265 ms (237–277) | 230 ms (204–239) | 239 ms (237–251) | 33 ms (33–34) | 33 ms (33–34) |
| scrolling: a step, longest | 470 ms | 440 ms | 469 ms | 68 ms | 50 ms |
| JS heap | 107.7 MiB | 107.4 MiB | 70.1 MiB | 12.6 MiB | 12.8 MiB |
| elements in the document | 180,147 | 180,149 | 180,149 | 501 | 501 |

| Figure (5,000 entries) | Baseline | Divided | + memo | + windowed | + index |
| --- | --- | --- | --- | --- | --- |
| last entry drawn | 3,278 ms | 3,202 ms | 2,093 ms | 794 ms | 712 ms |
| long tasks until then, total | 2,769 ms | 2,670 ms | 1,657 ms | 0 ms (0–50) | 0 ms |
| scrolling: a step, median | 62 ms | 50 ms | 53 ms | 33 ms | 33 ms |
| JS heap | 31.5 MiB | 31.2 MiB | 22.5 MiB | 8.3 MiB | 8.3 MiB |

- **Divided by screen** costs the first entry 36–51 ms on loopback: the
  session screen's chunk is fetched when the first session opens.
- **Memoised entries.** The reducer kept unchanged entries' objects, but
  every batch drew every entry again; now only the changed ones.
- **Windowed.** Only the entries within 1,200 px of the view are in the
  document (37 of 20,000 here), between two spacers sized from the
  heights measured. A scroll step is two animation frames (33 ms), the
  floor of the measurement.
- **The reducer's index.** Measured first on its own, before memo and
  windowing: last entry 32,711 → 32,777 ms, no change (rendering was the
  cost), so it was taken out (see "Tried, did not help"). Measured again
  on the windowed timeline, it removed the last 2.1 s of long tasks.

### A board of 500 cards, at rest

`d5f15e3`: an unchanged session list (read every 2 s) no longer
replaces the page's state, and the cards' "6m ago" labels read a shared
clock ticking every 15 s. "Before" is `5b5b991`; 3 runs each, load
average 3–7.

| Figure | Before | After |
| --- | --- | --- |
| long tasks in the 10 s after loading, untouched | 15, 1,314 ms (1,302–1,316) | 0 (0–3), 0 ms (0–276) |
| … the longest | 139 ms | 0 ms (0–97) |
| scrolling the fullest lane: long tasks | 3, 231 ms | 0 |
| scrolling: the longest step | 183 ms (168–242) | 34 ms (34–34) |
| every card drawn | 1,294 ms (1,252–1,338) | 1,258 ms (1,214–1,260) |

The run with 3 long tasks at rest caught a clock tick: every card's
label is drawn again then, once in 15 s instead of every 2 s.

### The release profile

`[profile.release]`: `lto = "fat"`, `codegen-units = 1`. Each variant
built from an empty target directory one after another, then all
benchmarks one variant after another, then the load test (load average
6–22 during the builds, 9–11 during the benchmarks).

| | cargo's default | thin LTO | fat LTO, 1 codegen unit |
| --- | --- | --- | --- |
| clean release build | 193 s | 226 s | 694 s |
| `vesseld` | 42,606,320 B | 42,766,592 B | 28,466,480 B |
| `vessel` (page embedded) | 54,879,064 B | 55,092,608 B | 37,073,776 B |
| `vesseld`, symbols stripped | 32,135,944 B | 32,718,448 B | 24,587,888 B |

| Benchmark (median) | default | thin | fat, 1 unit |
| --- | --- | --- | --- |
| proto: encode a terminal frame | 7.0 µs | 7.0 µs, +1% | 6.3 µs, −9% |
| proto: decode it | 5.6 µs | 5.5 µs, −2% | 5.2 µs, −7% |
| proto: encode a message chunk | 712 ns | 713 ns, +0% | 650 ns, −9% |
| proto: decode it | 1.8 µs | 1.7 µs, −4% | 1.6 µs, −11% |
| web: the bridge forwards a terminal record | 12.8 µs | 12.7 µs, −1% | 11.9 µs, −7% |
| web: … a message chunk | 2.7 µs | 2.5 µs, −7% | 2.3 µs, −12% |
| session: terminal output to 1 / 4 / 16 followers | 3.31 / 3.35 / 3.60 ms | −1 / −4 / +17% | −11 / −6 / −7% |
| session: the board's reducer | 6.1 ns | −1% | 3.9 ns, −36% |
| rpc: 256 terminal frames / message chunks through `Framed` | 1.10 ms / 86.2 µs | −5 / +8% | +1 / −5% |
| store: append 256 terminal records / message chunks | 46.2 / 2.32 ms | −25 / −23% | −18 / +4% |

The store's rows move by a quarter between runs of one build (disk I/O
on a shared virtual disk), so they show nothing about the profile. Fat
LTO with one codegen unit was kept: the CPU-bound paths are 5 to 12%
faster and the node a third smaller (an SSH install uploads it), for a
release build 3.6 times as long (development builds are unchanged).
Thin LTO moved nothing beyond the noise. Stripping symbols would save
another 14% but a panic's backtrace in a node's log would lose its
function names; not taken. The panic strategy stays `unwind`: one node
process carries every session, and a task that panics must end alone.

The load test with each (one run each; the burst moves by tens of
percent between runs of one build):

| | default | thin | fat, 1 unit |
| --- | --- | --- | --- |
| burst: records/s | 28,760 | 18,218 | 43,152 |
| burst: latency p50 / p99 | 39.9 / 95.0 ms | 50.1 / 118.0 ms | 54.3 / 95.1 ms |
| paced: latency p50 / p99 | 5.7 / 45.1 ms | 2.4 / 41.9 ms | 2.1 / 42.0 ms |
| agent flood: records/s | 35,817 | 37,418 | 37,252 |
| agent flood: latency p50 / p99 | 5.5 / 20.4 ms | 5.9 / 10.5 ms | 5.8 / 10.5 ms |
| `vesseld` started / after the flood | 20.4 / 61.2 MiB | 20.4 / 59.8 MiB | 16.3 / 55.2 MiB |
| `vessel ui` started / after the flood | 16.8 / 23.8 MiB | 17.1 / 27.7 MiB | 14.1 / 23.4 MiB |

## Tracks P and W merged (2026-11-12)

Track W (preferences and drafts kept by the bridge, shortcuts, find and
the outline, notices, the sidebar's display) was built beside track P,
and merged into it on `backend/m11-int-web`. Both trees were built in
release with the fat-LTO profile (track P's branch alone, `961ccb1`, and
the merged tree) and measured with `npm run perf` one after the other,
on the same machine, the same day: the merged tree, then P's (load
averages 3.2–8.2 during the merged run, 4.2–6.0 during P's). Three such
pairs were taken while integrating; the last is below, with the earlier
ones where they say something. Medians (minimum–maximum) of 5 runs for
the load, 3 for the rest.

### Loading the board, cold

| Profile, figure | P alone | Merged |
| --- | --- | --- |
| slow link: first contentful paint | 1,652 ms (1,648–1,668) | 1,704 ms (1,692–1,712) |
| slow link: board usable | 2,573 ms (2,553–2,580) | 2,664 ms (2,631–2,667) |
| phone: first contentful paint | 1,996 ms (1,952–2,148) | 2,044 ms (2,028–2,108) |
| phone: board usable | 3,238 ms (3,084–3,342) | 3,274 ms (3,067–3,323) |
| desktop: board usable | 338 ms (325–378) | 404 ms (342–466) |
| the index | 1,578 B | 2,413 B |
| script | 177,736 B | 184,011 B |
| stylesheet | 22,876 B | 24,251 B |
| font | 124,439 B | 124,439 B |
| HTTP total | 326,629 B | 335,114 B |

Worse by 52 ms of first paint and 91 ms until the board is usable over
the slow link, and 8,485 bytes. Explained, not fixed:

- **The index is sent uncompressed (+835 B).** Track W writes the
  page's appearance into it for each request (ADR 0028); a form
  compressed at build cannot carry that, so the web build leaves the
  index alone and `vessel ui` sends it as built (`files::index`; ADR
  0017's amendment). Compressing it per request would need an encoder in
  `vessel-web`, a dependency, for these bytes; setting the attributes
  from script instead would paint the wrong theme first. An end-to-end
  test over the slow link reads the first frame and finds it dark
  (`page.spec.ts`).
- **Track W's code in the entry (+6,275 B of script, +1,375 B of
  stylesheet).** Preferences (the store, the change rules, the first
  paint), the one shortcut listener and its registry, notices, the
  sidebar's display, the branded handles' guards and the sheet are used
  on every screen, so they belong to the entry, not to a screen's chunk.
  Settings' sections, the find bar and the outline went into the chunks
  of the screens that use them, as track P's path puts them.
- The bytes are about 42 ms at the slow link's 200 KB/s; the rest of the
  91 ms is the larger entry's compile and track W's start (reading the
  painted preferences, building the shortcut registry, the first drafts
  read when the link opens), not measured apart.

### A timeline of 20,000 entries (and 5,000)

| Figure | P alone | Merged, before the review's fixes | Merged |
| --- | --- | --- | --- |
| 20,000: first entry drawn | 96 ms (95–112) | 160 ms (145–162) | 146 ms (145–161) |
| 20,000: last entry drawn | 2,645 ms (2,512–2,646) | 2,995 ms (2,979–3,193) | 2,845 ms (2,545–2,961) |
| … long tasks until then | 0 (0–0) | 2, 124 ms | 1, 50 ms (0–3) |
| … a scroll step | 33 ms | 33 ms | 33 ms |
| … heap; elements | 12.8 MiB; 501 | 13.2 MiB; 507 | 13.1 MiB; 507 |
| 5,000: first entry drawn | 95 ms (95–127) | 129 ms (128–209) | 162 ms (146–201) |
| 5,000: last entry drawn | 793 ms (711–795) | 876 ms (729–878) | 851 ms (812–879) |

"Before the review's fixes" was measured against P's first run (2,545
ms then). What the review found there, and what was done:

- **The outline read every message on every batch** (`outline()` ran
  `readableText` over each message's Markdown per batch of records, so
  replaying a long log was quadratic in its text). Each preview is now
  made once per entry object and only the last message of a turn is
  read; an unchanged outline keeps its array and draws nothing
  (`726b160`). Below two prompts the outline also stops reading the
  layout on every scroll frame (`5efa82c`).
- **An audio context was made on the first click** whatever the
  notification setting (off by default), inside the click that opens
  the session (`b22cef0`).

What remains, from a CPU profile of opening the 20,000-entry session on
an unminified build of each tree (debug binaries, one run each): the
merged page does about 60–90 ms more JavaScript over the replay: the
outline worked out per batch (21 ms, a walk over the entries with every
preview cached), the session screen and the composer drawn again per
batch with track W's additions (the folds' context, find's and the
outline's state, the composer's draft and its recall list: 43 ms), and
layout effects (+34 ms). The rest of the 200 ms is within the runs'
spread (P's runs reached 2,646 ms, the merged tree's lowest was 2,545).
The first entry is drawn about 50 ms later in every pair: the session
screen mounts more (the composer's draft read from the bridge, three
shortcuts, find's and the outline's state, the folds); not separated
further. The next candidates, if it matters: the composer's recall
list worked out on Up rather than per batch, and the outline updated
from the entries a batch changed rather than walked whole.

### Find, and going to a match far away

New with track W. In Chromium, on the release build:

| Figure | 5,000 entries | 20,000 entries |
| --- | --- | --- |
| first search (every entry's text folded): typed to the count | 122 ms (118–123) | 138 ms (138–154) |
| a match about 300 entries from the top, from the log's end: typed to it shown | 122 ms (118–123) | 154 ms (154–155) |
| … how far it moved in the 60 frames after | 0 px | 0 px |
| a search again, every message (text folded already): typed to the count | 122 ms (106–122) | 122 ms (121–122) |
| a match at 90% of the log: typed to it shown | 117 ms (108–123) | 126 ms (120–126) |
| … how far it moved after | 0 px | 0 px |
| long tasks over the three | 0 | 0 |

Each figure includes the field's 100 ms settle (GOAT's) before the
search starts.

- **Find gives the page a turn by a message, not a timer** (`0a52c3d`).
  The search yields every 200 entries; with `setTimeout(0)` a browser
  holds nested timers back to at least 4 ms, so a 20,000-entry search
  waited about 400 ms between its slices: 547 ms typed to the count, and
  506 ms again over text already folded, on the debug fleet (one run
  each; vitest, under Node, never showed it). With a `MessageChannel`
  message: 144 and 123 ms on the same fleet.
- **Going to a far row** (`c4cdd73`). The window scrolls to where the row
  is estimated to be, so it is drawn in the same frame, then holds it
  while the rows around it are measured and the spacers' estimates
  change: 0 px of movement in every run. Without the hold (the
  correction turned off) the end-to-end test caught it moving 5.5 px.

### A board of 500 cards, and the terminal

At rest the board had no long task in either tree (0, the noisiest run
3 in both): track W's preferences, sidebar grouping and notices did not
bring the redraws back. Drawing all 500 cards: 1,461 ms (1,293–1,691)
for P, 1,707 ms (1,369–1,991) merged, with the previous pair the other
way round (1,706 ms P, 1,531 ms merged): within the noise of a shared
machine. The terminal was not touched by track W; its figures moved
both ways between the pairs (the 51 MB flood 8,598 ms P, 9,313 ms
merged in the last; 8,563 and 8,323 in the one before).

## Track C: terminals, scripts and ports (2026-11-12)

Protocol 11 added terminals beside sessions (ADR 0031). What it touched
on the paths measured here: the session ring gained an optional byte
bound (a terminal's; a session's ring has none and skips it), a
session's handle keeps its program's process id in an atomic, the
client's and the bridge's streams became generic over the log they
follow, and the `Reply` and `Call` enums grew thirteen variants. The
port scanner runs only within 60 s of a `port.list`, so nothing here
exercises it.

Measured before (the tree this track started from, `15b79a2`) and after
(`6956b1c`), back to back on the same machine, release builds
throughout.

### The node's hot paths

Criterion medians, before and after alternated twice (A B A B) within
eleven minutes, load average 8–12 throughout (other builds on the
machine):

| Benchmark | Before, 1st / 2nd | After, 1st / 2nd |
| --- | --- | --- |
| proto: encode an event frame, terminal 4 KiB | 6.91 / 6.45 µs | 6.50 / 6.41 µs |
| proto: decode it | 5.56 / 5.39 µs | 5.72 / 5.42 µs |
| proto: encode an event frame, message chunk | 705 / 665 ns | 744 / 672 ns |
| proto: decode it | 1.64 / 1.58 µs | 1.63 / 1.65 µs |
| session: 200 KB of terminal output to 1 follower | 2.93 / 3.19 ms | 3.14 / 2.99 ms |
| … to 4 followers | 3.30 / 2.98 ms | 3.20 / 3.23 ms |
| … to 16 followers | 3.91 / 3.27 ms | 3.56 / 3.63 ms |
| session: the board's reducer | 4.52 / 4.43 ns | 4.47 / 4.62 ns |

Every pair is within the spread of the same tree's two runs: nothing
moved. A comparison against a baseline saved hours earlier at a lower
load had flagged the decode of a terminal frame (+4.6%, then +10.4%)
and the fan-out (+8%, then +50–90% at load 10); the alternated runs
above show those were the machine, not the change. Against that same
earlier baseline `rpc/framed` and the bridge's forward came out 9% and
4% faster, also within what the machine does between runs.

### The load test

Two back-to-back pairs (after, then before), release binaries, one run
each. The first pair ran while the machine's load fell from 12 to 3, the
second at 2.5–3.5:

| Workload | Before (1st pair / 2nd) | After (1st pair / 2nd) |
| --- | --- | --- |
| burst: records/s | 33,873 / 38,718 | 30,928 / 23,952 |
| burst: latency p50 / p99 | 24.5 / 41.3 · 32.9 / 81.0 ms | 53.7 / 99.9 · 36.8 / 75.8 ms |
| paced: latency p50 / p99 | 2.24 / 41.4 · 2.35 / 41.5 ms | 5.25 / 43.3 · 2.29 / 41.3 ms |
| agent flood: records/s | 37,581 / 37,637 | 34,188 / 37,458 |
| agent flood: latency p50 / p99 | 5.92 / 10.8 · 6.07 / 11.6 ms | 9.01 / 23.3 · 5.98 / 11.0 ms |

| `vesseld` resident memory | Before (2nd pair) | After (2nd pair) |
| --- | --- | --- |
| started, no sessions | 18.6 MiB | 19.1 MiB |
| 32 terminal sessions, idle | 23.4 MiB | 24.0 MiB |
| after the burst | 42.9 MiB | 44.3 MiB |
| after the agent flood | 57.9 MiB | 58.9 MiB |

At the same quiet load (the second pair) the paced and the flood
workloads are the same within a few percent; the first pair's after
run had the busier machine. The burst lasts 0.15–0.32 s and moves by
tens of percent between runs of one build (as above). What moved:
`vesseld` holds 0.5–1.4 MiB more, the code the node gained (thirteen
calls, the terminals' registry and actors, the port scanner's tables)
and nothing kept per session.

## Track B-page: the code panel (2026-11-12)

Track B-page (`docs/plans/backend-m11.md`) put a session's code beside
its timeline: the inspector's Changes and Files tabs (a chunk of their
own, `features/workspace/panel.tsx`), where a session runs on the
new-session page, and projects in the sidebar and Settings. Its tree
and the tree it started from (`15b79a2`, wave 2's base) were built in
release with the fat-LTO profile and measured with `npm run perf` one
after the other on the same machine: this tree, then the base (load
averages 2.0–3.6 for this tree, 2.2–5.0 for the base). Medians
(minimum–maximum) of 5 runs for the load, 3 for the rest. A first pair
taken an hour earlier is not used: other builds took the machine to a
load of 49 during the base's half.

### A large diff and a large tree

New. A repository made in the fleet's temporary directory: one file of
40,000 lines with every tenth line changed (4,000 hunks, about 32,000
diff lines over seven of the node's 5,000-line pages), 1,500 small files
changed beside it, and a folder of 5,000 files (the node's bound for one
listing). The session runs in the project's folder, so Changes compares
Uncommitted. From the tab's click to the first row laid out; then the
whole list scrolled in 100 steps, each file's diff and each next page
asked for as its row is reached, and 5 s at rest:

| Figure | Changes: 4,000 hunks and 1,500 files | Files: a folder of 5,000 |
| --- | --- | --- |
| tab clicked to the first row drawn | 764 ms (748–826) | 390 ms (383–391) |
| a scroll step, median; longest | 33 ms; 48 ms (44–129) | 33 ms; 37 ms (36–39) |
| long tasks while scrolling; at rest | 0; 0 | 0; 0 |
| JS heap used | 10.6 MiB | 8.3 MiB |
| elements in the document; rows drawn | 1,498; 135 | 484; 70 |

The first figure includes loading the panel's chunk the first time
(15.4 KB compressed), the node's `git.changes` over 1,501 files and the
first file's diff; the Files figure the chunk, the root's listing and
the folder's. Only the rows near the view exist at any time (135 of
about 34,000 for Changes): the list is the timeline's window
(`useWindow`), not a second one. The same measurement before the
review's fixes (a load of 4.5–12.6): 1,053 ms (768–1,205) to the first
diff row, one long task of 50 ms while scrolling.

### What moved, and why

| Figure | Base | B-page |
| --- | --- | --- |
| slow link: first contentful paint | 1,692 ms (1,688–1,704) | 1,712 ms (1,704–1,728) |
| slow link: board usable | 2,654 ms (2,644–2,672) | 2,694 ms (2,599–2,702) |
| phone: first contentful paint | 2,048 ms (2,012–2,164) | 2,068 ms (2,024–2,192) |
| phone: board usable | 3,199 ms (3,136–3,422) | 3,269 ms (3,221–3,521) |
| desktop: board usable | 400 ms (341–476) | 414 ms (375–455) |
| the index | 2,413 B | 2,954 B |
| script | 183,990 B | 185,923 B |
| stylesheet | 24,255 B | 26,945 B |
| HTTP total | 335,097 B | 340,261 B |
| WebSocket received at load | 13,322 B | 14,262 B |
| 5,000 entries: first entry drawn | 119 ms (111–127) | 146 ms (144–170) |
| 5,000 entries: last entry drawn | 735 ms (727–811) | 794 ms (746–937) |
| 20,000 entries: first entry drawn | 129 ms (111–162) | 145 ms (145–178) |
| 20,000 entries: last entry drawn | 2,546 ms (2,544–2,779) | 2,678 ms (2,545–2,812) |
| … long tasks until then | 0 (0–2) | 1, 53 ms (0–1) |
| … heap; elements | 13.2 MiB; 514 | 13.8 MiB; 547 |

Worse: the board's load by 20 ms of first paint and 40 ms until usable
over the slow link (70 ms on the phone), 5,164 bytes, and opening a long
session by 16–27 ms to its first entry and 59–132 ms to its last, with
one long task of about 50 ms. Explained as far as measured, not fixed:

- **The stylesheet (+2,690 B).** The code panel's styles (the diff, the
  git bar and dialogs, the tree and the file, setup's card, Settings ›
  Projects, the tray) are in the one stylesheet the index names, though
  their code is in chunks: ADR 0017 keeps every stylesheet inside the
  index's integrity graph, and a stylesheet per chunk would first need
  that graph extended to stylesheets loaded later (not done here).
- **The index (+541 B).** The import map pins five more chunks: the
  panel, and four that the new-session page, Settings and the panel
  share (a project's commands compared, a worktree's words, a
  worktree's calls, the drop button). The chunk the new-session page
  and the session's page already shared now holds the terminal's byte
  decoding too (setup's output uses it), and is named after it.
- **The entry's script (+1,933 B).** What every screen draws: the
  sidebar's Projects group and its store, a session's branch chip (the
  sidebar, the board's card), the kit's seventeen new icons and its
  sheet's right side. The panel itself (56 KB, 15 KB compressed) is not
  in it.
- **The WebSocket (+940 B).** The sidebar reads each connected machine's
  projects (`project.list`) when the page starts.
- **A long session's opening.** Not profiled apart. What the session's
  page now does more on each of the replay's batches and at its start:
  the inspector's tab head and the title bar's chip (their project read
  from the machine's projects), the panel's state (a media query), and
  the sidebar's Projects group drawn beside it (33 more elements, all
  from these). The bytes above are about 26 ms at the slow link's
  200 KB/s; the rest of the board's 40 ms is the larger entry's compile
  and the projects' read. If it matters: the projects read after the
  board is usable rather than with it, and the chip's project found once
  per session rather than per drawing.

Within the runs' spread, either way: find (5,000 entries: 118 against
122 ms to the count; 20,000: 150 against 143), the terminal (untouched:
the 2 MB flood 499 against 451 ms, the 10 MB 1,933 against 1,963, the
51 MB 8,550 against 8,481), a board of 500 cards (1,225 against
1,326 ms, at rest 0 long tasks in both).

## Tried, did not help

- **The reducer's index of tool calls, on its own** (before memoised
  entries and windowing): 20,000 entries drawn in 32,711 ms before,
  32,777 ms after (3 runs each). Rendering was the cost, so it was
  taken out, and applied again once windowing had removed that cost,
  where it took the last 2.1 s of long tasks away (above).
- **Thin LTO**: every benchmark within the noise of cargo's default,
  the binaries 0.4% larger.
- **Stripping symbols** (`strip = "symbols"`): `vesseld` 14% smaller,
  not taken: a panic's backtrace in a node's log would lose its names.
- The first measurement of Pretendard's subsets was discarded: the
  subsets' stylesheet was `@import`ed from `fonts.css`, where Vite's
  plugins never saw it, so the faces kept the name "Pretendard
  Variable" and no stack used them (the board drew Hangul in a system
  face and fetched no font). The stylesheet is imported from `main.tsx`
  now, and an end-to-end test checks that Hangul is drawn with
  Pretendard from its subsets.

## Measured, left as it is

- **The terminal under a flood**: xterm.js draws 2.5 to 6 MB/s with no
  long task and a few dozen frames missed in a 51 MB flood. The time is
  xterm's parser and its DOM renderer; the page's work per record
  (decoding base64, one `write`) is small beside it, and xterm already
  queues writes and parses them within a budget per frame. What would
  move it next: xterm's WebGL renderer (a dependency), or the node
  coalescing PTY reads into fewer, larger records.
- **The store's write path**: WAL with `synchronous=NORMAL`, batched
  commits; about 22 MB/s of terminal output a writer on this machine's
  disk, with run-to-run swings of a quarter. Live followers are not held
  back by it until a session's durable channel (256 records) is full;
  the load test's bursts did not fill it. Moving WAL checkpoints off the
  writer thread is the next candidate, to be measured on a quiet disk.
- **Fan-out**: 16 followers of one session cost about what one does.
- **Many tabs following one session**: each tab's follow is its own
  subscription on the node, so the node encodes, and the bridge decodes
  and encodes, each record once per tab (12.8 µs for a 4 KiB record in
  the bridge). Not measured with many tabs; sharing one stream inside
  the bridge would need no wire change.

## A wire change, described and left

The page reads each node's session list every 2 s because the protocol
has no subscription to it. A `session.watch` (or a list subscription
that pushes changed `SessionInfo`s) would replace the poll and its
500-session JSON every 2 s on a large board; it is a protocol change,
left for the coordinator.

## Not measured

- Find and going far in another browser than Chromium, and with
  entries of very different heights (the measured log's are uniform: a
  far row's estimate is off by little before it is measured; the
  end-to-end test checks the hold, not how far off the estimate was).
- WebSocket frames over the throttled profiles: Chromium's network
  emulation does not add the 150 ms to frames (handshake 159 ms
  throttled against 4 ms, frames 7–23 ms against 0.4–0.9 ms), so the
  slow-link and phone figures undercount the bridge's round trips.
- Opening a session over a slow link (its screen's chunk, 127 KB
  compressed): only loopback, where it costs the first entry 36–51 ms.
- A real phone, a headed browser, a GPU: these are headless Chromium
  figures with software rendering.
- Windows and macOS, and any machine but this one.
