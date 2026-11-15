# Working on Vessel

Read `README.md`, `docs/ARCHITECTURE.md` and the plan you were assigned
(`docs/plans/`) first. `docs/ORIGIN.ko.md` is the owner's original intent; when
a design question is unclear, that file wins.

Code quality is the first priority, ahead of speed and ahead of feature count.
A smaller feature set with a clean structure beats a larger one that has to be
rewritten. Every rule below is enforced in review; "it compiles" is not the bar.
Section 12 lists the strict rules in one line each; `docs/QUALITY.md` gives
each one's reason, its check, and how many places break it today.

## 1. Gate before claiming anything works

Run these, in this order, and paste the result summary in your report:

```bash
cargo fmt --all
node scripts/check-rust-size.mjs     # file budget (docs/QUALITY.md M3)
node scripts/check-boundaries.mjs    # crate map (docs/QUALITY.md M2)
cargo build -p vessel-testkit --bins
cargo build -p vessel-node --bin vesseld
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p vessel-node --features forge-fixture --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
cargo test -p vessel-node --features forge-fixture --test forge_fixture
```

`node scripts/gate.mjs` runs this sequence with a format check. It keeps
prebuilds and tests in one environment; on a shared machine, wrap the whole
runner in the local build scheduler. The explicit feature steps compile the
separate forge fixture binary and run its feature-only tests; normal product
and release builds keep the feature disabled.

The two scripts build nothing and take about a second, so a violation
fails before the compile starts. CI runs exactly these on Linux, Windows
and macOS (the two scripts on Linux; their answer is the same on every OS)
for every push to `main` and `backend/**` and every pull request (the
repository is public, so hosted runners cost nothing).

Changes under `web/` (the web UI, ADR 0017) add its checks, which CI runs
on Linux: in `web/`, `npm run lint`, `npm run typecheck`, `npm test`,
`npm run build`, then `npm run e2e` against the Rust binaries. Changes to
wire types also regenerate the TypeScript declarations: `VESSEL_WRITE_TS=1
cargo test -p vessel-proto -p vessel-web --features ts --lib typescript`,
and commit `web/src/wire/`.
With a Codex of at least the version `crates/vessel-agent/src/codex/contract.rs`
names installed, a change to the Codex driver adds `VESSEL_CODEX_CONTRACT=1
cargo test -p vessel-agent --lib codex::contract`: it regenerates the
app-server schema from that Codex and fails when a definition the driver
follows changed.
CI does not replace the local gate: run it before pushing, and report both.
Code that only builds on Linux must be behind `cfg(unix)` with a working (or
explicitly erroring) Windows path.

## 2. Types: no "any", no strings pretending to be types

The goal is that the compiler, not a reviewer, catches mistakes.

- **No untyped data in domain or wire types.** Forbidden: `serde_json::Value`,
  `Box<dyn Any>`/`downcast`, `HashMap<String, Value>`, `Vec<u8>` carrying
  structured data, and `String` fields that hold one of a known set of values.
- **The one exception is opaque passthrough.** Payloads Vessel forwards but
  never inspects (for example a tool's arbitrary input arguments) use the
  `RawJson` newtype from `vessel-proto` (a `Box<serde_json::value::RawValue>`):
  no parse into a tree, no field access. If code needs to branch on a field,
  that field gets a typed struct.
- **Parse at the edge, once.** External formats (harness JSON streams, CLI
  output, files) are decoded into typed structs inside the crate that owns that
  format (`vessel-harness` for harness output, `vessel-store` for files). The
  rest of the code only sees typed values. Unknown event kinds decode into an
  explicit `Unknown { kind: String }`-style variant that is logged, never into
  a `Value`.
- **Newtypes for every identifier and unit**: `NodeId`, `SessionId`,
  `EventSeq`, `ToolCallId`, byte counts, durations. Never pass a bare `u64` or
  `String` where a newtype exists. Constructors validate ("parse, don't
  validate"); make invalid states unrepresentable instead of checking later.
- **Closed sets are enums, matched exhaustively.** No `_ =>` arm on our own
  enums (the lint is on); adding a variant must break every place that has to
  care.
- **Static dispatch by default.** Generics or an enum for a known, closed set
  (harness kinds, session modes). `dyn Trait` only at a real open extension
  point, with a comment saying why. No trait with a single implementation
  "for testing"; use a fake behind a generic parameter or a test-only module.
- **Typed RPC.** Every request type names its response type through the
  `Request` trait in `vessel-proto`. Dispatch is an exhaustive `match` on the
  generated method enum, so an unhandled method is a compile error, not a
  runtime 404.

## 3. Modules: one job each, one direction of dependency

The crate map is the table in `docs/ARCHITECTURE.md` ("Crates"): each
workspace crate's job and the `vessel-*` crates it may depend on, and the
external crates that have one owner. ADR 0001 records why each edge exists.
Adding an edge is a row change there and a paragraph in ADR 0001, in the
same commit; `scripts/check-boundaries.mjs` fails on any difference between
the table and the manifests (QUALITY M2).

- No cycles, no reaching into another crate's internals, no "utils" or
  "common" grab-bag crate or module. If two crates need the same thing, it
  belongs to the lower crate whose job it is.
- Visibility: private by default, `pub(crate)` next, `pub` only for the
  crate's deliberate API. Every `pub` item is documented.
- Split modules by responsibility, not by size. Size is the signal: a file
  over 500 lines of product code fails `scripts/check-rust-size.mjs` unless
  it is on its shrinking list, and a function over 60 lines needs a reason
  (QUALITY M3, M4). A module's top doc comment says what it owns and what it
  must not do.
- Wire types live only in `vessel-proto`. A client never depends on
  `vessel-node` internals to talk to a node.
- Harness-specific knowledge (argv, env, output format, resume flags) lives only
  in `vessel-harness`, one module per harness. Nothing outside that crate may
  match on a harness to decide behaviour; ask the harness module instead.

## 4. Efficiency

- Do not clone what you can borrow. Data fanned out to many readers is
  `Arc<str>`/`bytes::Bytes`/`Arc<T>`, not cloned `String`s and `Vec`s.
- Every channel is bounded, and the code states what happens when it is full
  (block, drop oldest, or disconnect the slow reader and let it resync by
  sequence number).
- No blocking calls on async worker threads. Blocking file or PTY I/O goes
  through `spawn_blocking` or a dedicated thread.
- No polling loops or `sleep` for coordination; wait on a channel, a
  `watch`, a `Notify` or a cancellation token.
- Hot paths (output pumps, event fan-out, log append) allocate once per event
  at most. Batch disk writes; do not `fsync` per line.
- Measure before optimising anything that is not one of the rules above.

## 5. Concurrency and failure

- A long-lived stateful thing (a session, a log writer) is a task that owns
  its state. Others hold a cheap handle (channel sender plus `watch`/broadcast
  receivers). No `Arc<Mutex<HashMap<..>>>` god objects.
- Never hold a lock across `.await` unless it is a `tokio` lock and a comment
  says why it must be.
- Cancellation flows down through `CancellationToken` children: node shutdown
  cancels sessions; a session cancels its pumps.
- No `unwrap`/`expect`/`panic`/indexing/raw arithmetic outside tests (the lints
  enforce it). Use checked or saturating arithmetic and say which and why.
  A misbehaving session or client must never take the node down.
- Child processes run in their own process group and are killed as a group,
  so a harness's own children (MCP servers, shells) do not outlive it.

## 6. Errors

- Each library crate has its own `thiserror` enum. No `anyhow` in libraries,
  no `Box<dyn Error>` in signatures, no `String` errors.
- Errors that cross the wire map to `vessel-proto`'s `ErrorCode` enum plus a
  human message. Map once, at the boundary.
- `map_err` keeps the source (`#[source]`); never discard it.

## 7. Logging and output

- Logging goes through `tracing` with structured fields
  (`tracing::info!(session = %id, "...")`), not formatted strings.
- The CLI writes to a locked stdout handle, never `println!`.
- Never log secrets (tokens, credentials, environment values).

## 8. Tests

- Decoders are tested against recorded fixtures under `tests/fixtures/`
  (real harness output, trimmed, with secrets removed).
- Process-level tests use a small fake harness binary built from the
  workspace, never the real `claude`/`codex`, and never the network. One
  exception, opt-in (`VESSEL_CODEX_CONTRACT`, section 1): the Codex
  driver's contract test runs the installed `codex` to generate its
  schema, nothing else, in an empty environment with temporary homes.
- A node started in a test gets a temporary home and no harness home
  variables (`HarnessEnv::new(temp)` in `Config::new`); library code never
  reads the user's home or `CLAUDE_CONFIG_DIR`/`CODEX_HOME`/`HOME` from the
  process. Only the binaries do. A test that must change the environment
  runs a child process: the environment is shared by every test in it.
- RPC is tested by round-tripping every method through encode/decode and
  through an in-memory transport.
- Tests are deterministic: no wall-clock sleeps to wait for something; wait on
  the event.
- A script a test writes and then runs (a fake program, a git hook) is
  created by another process (`cp` from a side file:
  `vessel_testkit::write_executable`, the one helper every crate's tests
  call), never with `std::fs::write`: a child forked by a test
  on another thread holds the write descriptor until it execs, and running
  the script then fails with `ETXTBSY`.
- A test's safety-net timeout (which only turns a hang into a failure) is
  `vessel_testkit::within`: it builds the fakes first and starts the clock
  after. The first fake build of a run can take a minute on a cold runner.
- Inside it, a wait that could hang gets a deadline of its own that names it
  (the node e2e tests' `support::step`, and their output readers, which show
  what they read): a hang on one CI runner then says where it was. The node
  e2e tests also keep the node's debug logs, printed only for a failing test.

## 9. Dependencies

- Prefer std and the crates already in the workspace. A new dependency needs a
  line in the plan or ADR saying what it replaces and why it is worth it.
- Versions live in `[workspace.dependencies]`; crates use `.workspace = true`.

## 10. Documentation (part of done)

- Comments explain why, not what: the intent, the constraint, the rejected
  alternative, and what must stay true when someone changes it.
- Non-trivial decisions get an ADR in `docs/adr/NNNN-title.md` with:
  context, options considered, decision, why, consequences.
- Update `docs/ARCHITECTURE.md`, the crate's module docs and `README.md` in the
  same change when behaviour or structure changes. Docs that disagree with the
  code mean the work is not finished.
- Lints are strict and deliberate. Do not add `#[allow]`; if a lint truly does
  not apply, use `#[expect(..., reason = "...")]`.

## 11. Working as one of several agents

- Stay inside the crates your plan assigns you. If you need a change in a crate
  you do not own (most often `vessel-proto`), stop and ask the coordinator
  instead of editing it; the proto contract is shared.
- Work in your own git worktree and branch. Commit in small, reviewable steps
  with messages that say why. Do not push, do not rewrite shared history.
- Authorship: every commit's author and committer is the maintainer,
  Ingwannu, with the identity the repository's git config sets.
  No `Co-Authored-By` trailers and no tool attribution ("Generated with ...")
  in commit messages, pull request text or comments. The history was
  rewritten on 2026-11-07 to remove them; commit hashes quoted in older
  notes were updated to the rewritten ones.
- Branches: tracks push to their own `backend/<milestone>-<track>` branch;
  the coordinator merges them into the milestone's `backend/m<N>` after the
  local gate. A milestone reaches `main` only by a pull request from its
  branch, once its gate is green, CI is green on all three platforms and
  its plan's manual smokes have passed; it is merged with a merge commit,
  never squashed, so each track's commits and review fixes stay in the
  history. Nobody pushes to `main` directly. Tags `v*` start a release
  (`.github/workflows/release.yml`): only for a release.
- Report: what you built, the three-command gate result, what you deferred
  and why, and any contract change you need.
- The repository is public (ADR 0014): every commit, doc, test fixture and
  CI log can be read by anyone. Never put credentials, customer data,
  internal host names or addresses, or private repository paths in any of
  them; describe the operator's setup in general terms instead.

## 12. The strict rules, one line each

Each line is a rule review cites by its identifier. `docs/QUALITY.md` gives
its reason, its check (a lint, a script, or review where no tool can tell)
and the number of places that break it today; "holds" there means a check in
the gate finds nothing. A rule with open violations is still the rule: no
change adds one (P4).

Rust types:

- **T1** Ids, names, kinds, states, paths, durations, sizes, counts and
  versions crossing a module boundary are newtypes or enums, never a bare
  `String`, `u64` or `bool`.
- **T2** Parse external text once, at its crate's edge, with validating
  constructors; carry the parsed type, never the raw form.
- **T3** No `match` on a string to decide behaviour, except to decode
  external text into an enum at its edge.
- **T4** No `_ =>` arm on the workspace's own enums.
- **T5** No `#[non_exhaustive]` on an enum another workspace crate matches
  or on a wire type; the wire's forward compatibility is an `Unknown`
  variant and the protocol version.
- **T6** No `serde_json::Value`/`Map` outside a decoding edge whose header
  says why; opaque payloads are `RawJson`.
- **T7** No numeric `as`: `From`, `TryFrom`, or a clamped helper with an
  `#[expect]` and reason.
- **T8** Checked or saturating arithmetic, and the code says which.
- **T9** No `bool` parameter or field whose meaning the call site does not
  show; a two-variant enum instead.
- **T10** Borrow over clone; fan-out data is `Arc`/`Bytes`.
- **T11** Private, then `pub(crate)`, then `pub`; a type with an invariant
  has private fields and a validating constructor.
- **T12** A result that matters is `#[must_use]` and is used.
- **T13** `Option` and `Result`, never a sentinel value.
- **T14** Enums or generics for closed sets; `dyn` only at an open
  extension point, with a comment.

Errors and failure:

- **E1** One `thiserror` enum per library crate; no `anyhow`, no
  `Box<dyn Error>`, no `String` errors.
- **E2** Errors keep their source; the wire mapping happens once.
- **E3** No `unwrap`, `expect`, `panic!`, `unreachable!`, `todo!` or
  panicking index outside tests, but an `expect` whose reason proves it.
- **E4** No `unsafe`.
- **E5** No `dbg!`, `println!` or `eprintln!` in product code.
- **E6** `#[expect(lint, reason)]` on the smallest item; never `#[allow]`,
  never file-wide.
- **E7** Library code reads no environment, home or current directory and
  never exits; no `sleep` to wait for something.

Modules:

- **M1** One responsibility per crate, module and file, named by its `//!`
  header; no `utils`/`common`/`helpers` module.
- **M2** `docs/ARCHITECTURE.md`'s crate table is the dependency law;
  `scripts/check-boundaries.mjs` holds it.
- **M3** At most 500 lines of product code per file; the listed exceptions
  only shrink.
- **M4** At most 60 lines per function, or an `#[expect]` saying the length
  is the point.
- **M5** One fact in one place: no repeated block of 12+ lines, no method
  list restated per crate.
- **M6** Every `pub` item documented, saying why it exists.
- **M7** Tests beside the rule they test, in the same commit as the rule.
- **M8** (native app) Spacing, colour, type and radii from the kit's
  tokens, never literals.

The page's TypeScript:

- **S1** The strict `tsconfig` flags stay on, for the page and its scripts.
- **S2** No `any`, no `!`, no assertion but `as const`; no lint switched
  off in a comment.
- **S3** Kinds are discriminated unions, switched exhaustively.
- **S4** Ids are branded by `src/bridge/ids.ts` only.
- **S5** `src/wire/` is generated, never edited.
- **S6** Layers one way; 300 lines per file, 400 per stylesheet.

Process:

- **P1** Every commit passes the gate of section 1.
- **P2** A refactor commit changes no behaviour and no test expectation.
- **P3** Reports state measurements, not adjectives.
- **P4** Debt only shrinks: no new violation, and a new check lands with
  zero failures.
- **P5** Every Cargo workspace in the repository is gated in CI.
- **P6** Review refuses what `docs/QUALITY.md` lists under P6.
