# Performance

What is measured, how to run each measurement again, the figures before
and after each change made for speed, and what was tried and did not
help. Milestone 11, track P (`docs/plans/backend-m11.md`) started it: no
optimisation without a number before and a number after, from the same
harness on the same machine. A change that does not move its number is
reverted and listed under "Tried, did not help".

## Where it stands

Milestone 11, track P, before → after, from the sections below:

| Measured | Before | After | Changes that moved it |
| --- | --- | --- | --- |
| board usable, cold, over a 1.6 Mbit/s link | 17,903 ms | 2,561 ms | compression, Pretendard subsets, the page divided |
| … the same on a phone (CPU 4× slower) | 18,994 ms | 3,028 ms | the same |
| first contentful paint over that link | 7,028 ms | 1,636 ms | compression, the page divided |
| bytes for a cold board | 3,371,739 B | 326,422 B | the same three |
| a 20,000-entry timeline: last entry drawn | 37,127 ms | 2,362 ms | memoised entries, windowing, the reducer's index |
| … a scroll step | 265 ms | 33 ms | windowing |
| … heap; elements | 107.7 MiB; 180,147 | 12.8 MiB; 501 | memo, windowing |
| a board of 500 cards at rest: long tasks per 10 s | 15 (1,314 ms) | 0 | an unchanged list draws nothing |
| CPU-bound hot paths (frame encode/decode, bridge, fan-out) | — | 5–12% faster | fat LTO, one codegen unit |
| `vesseld` binary | 42.6 MB | 28.5 MB | the same |
| a clean release build | 193 s | 694 s | the same (the cost) |

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
minimum and maximum (`VESSEL_PERF_RUNS`).

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

- WebSocket frames over the throttled profiles: Chromium's network
  emulation does not add the 150 ms to frames (handshake 159 ms
  throttled against 4 ms, frames 7–23 ms against 0.4–0.9 ms), so the
  slow-link and phone figures undercount the bridge's round trips.
- Opening a session over a slow link (its screen's chunk, 127 KB
  compressed): only loopback, where it costs the first entry 36–51 ms.
- A real phone, a headed browser, a GPU: these are headless Chromium
  figures with software rendering.
- Windows and macOS, and any machine but this one.
