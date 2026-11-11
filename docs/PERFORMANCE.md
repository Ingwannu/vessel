# Performance

What is measured, how to run each measurement again, the figures before
and after each change made for speed, and what was tried and did not
help. Milestone 11, track P (`docs/plans/backend-m11.md`) started it: no
optimisation without a number before and a number after, from the same
harness on the same machine. A change that does not move its number is
reverted and listed under "Tried, did not help".

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
