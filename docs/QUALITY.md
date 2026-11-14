# Quality rules

The owner asked for Rust types held as strictly and tightly as the compiler
allows, a modular structure, and code quality at its maximum, TypeScript
included; every rule enforced by a tool where one exists, and every claim
about the code answered with a measurement. `AGENTS.md` section 12 states
each rule below in one line; this file gives its reason, how it is checked
and where the code stands.

Each rule has an identifier (`T` Rust types, `E` errors and failure, `M`
modules, `S` the page's TypeScript, `P` process) that review cites. Each
says:

- **Why**: the mistake it prevents.
- **Check**: the lint, the script or the step of the gate that fails, or
  "review" where no tool can tell.
- **Today**: "holds" only where a check in the gate enforces it and finds
  nothing; otherwise the number of places that break it. Nobody should read
  a rule as already true when it is not.

A rule with open violations is still the rule. A change adds no new
violation of any rule here, and a change that edits an item that breaks one
fixes it or says in its report why not (P4). The counts shrink; they never
grow.

## Where the numbers come from

Measured on 2026-11-14. The node line (`crates/`) at `5440033c` for files
and grep counts; its clippy pass at `e432296b` (the commit before, which
differs from it only in `vessel-agent/src/codex/`), one run with every
candidate lint at `warn` and the settings proposed below, each finding
counted once and split into product and test code as `scripts/rust-source.mjs`
splits them. The native app (`spikes/native`) from the read-only audit of
the same day, at its own branch's head; it was not compiled. The page
(`web/`) with ESLint and `tsc` run with the candidate rules and flags. The
lints not yet on, with their counts and an order to turn them on in, are a
separate list kept beside the work, not in the repository: what is here is
what holds or is owed.

"Product code" is everything a release compiles: not `tests/`, `examples/`,
`benches/`, `vessel-testkit`, `tests.rs`, `testing.rs`, `*_tests.rs`, items
under `#[cfg(test)]` or `#[cfg(all(test, ...))]`, or files a
`#[cfg(test)] mod x;` names.

## Rust: types

The compiler, not a reviewer, should catch the mistake. Every rule here
narrows what a value can be, so that a wrong value does not type-check.

### T1. Domain values are types

**Rule.** Identifiers, names, kinds, states, paths, durations, sizes,
counts and versions that cross a module boundary are newtypes or enums,
never a bare `String`, `&str`, `u64`, `usize` or `bool`. Where a newtype
exists, nothing passes the bare value instead.
**Why.** Two `String` parameters swap silently; a `NodeId` and a
`SessionId` do not. A `String` that holds one of a known set of values
accepts every misspelling.
**Check.** Review. `clippy::fn_params_excessive_bools` and
`struct_excessive_bools` (pedantic, three or more) catch the worst of the
`bool` case.
**Today.** Node: 13 fields hold a closed set as `String`
(`vessel-forge/src/github/graphql.rs` `state`, `status` and `kind`,
`github/rest.rs` `state`; `vessel-llm/src/error.rs` `kind`;
`vessel-llm/src/upstream/{anthropic,chat,responses}.rs` and
`vessel-node/src/models/config.rs` `provider`;
`vessel-agent/src/mapper/question.rs` `mode`), and one id travels as
`String` (`vessel-agent/src/claude/session.rs` `refuse(request_id: String,
..)`). Native: the machine is `String`/`&str` in 89 declarations (app 40,
link 41, view 8) although `NodeName` exists, `agent: String` where
`HarnessRef` exists, and the palette's id is a `String`.

### T2. Parse at the edge; carry the parsed type

**Rule.** External text (a harness's stream, a file, a command line, an
HTTP body) is decoded once, in the crate that owns that format, into types
whose constructors validate (`FromStr`, `TryFrom`, `#[serde(try_from)]`).
Past that edge no code validates again and no code holds the raw form.
Unknown kinds decode into an explicit `Unknown { .. }`-style variant that is
logged.
**Why.** A value that was checked once and then passed on as text has to be
trusted, or checked again, everywhere it goes. "Parse, don't validate"
makes an invalid value impossible to hold.
**Check.** Review.
**Today.** Not measurable by a tool. Known cases:
`vessel-store/src/codec.rs` round-trips a string through a
`serde_json::Value` three times to reuse serde's validation where a
`FromStr` would say it directly.

### T3. No stringly-typed dispatch

**Rule.** No `match` on a string, and no `if s == ".."` chain, to decide
behaviour on a value of the project's own. Matching on text is allowed only
where external text is decoded into an enum (T2), and then the enum is what
the rest of the code matches.
**Why.** A string match has no exhaustiveness: a new case compiles and
falls through.
**Check.** Review.
**Today.** 11 `match …as_str()` sites. Nine decode external text at its
edge (URL schemes, MIME types, a CLI argument, a Codex setting). Two match a
stored `String` that T1 says should be an enum:
`vessel-forge/src/github/graphql/read.rs` (`node.state`) and
`github/reads.rs` (`pull.state`).

### T4. Exhaustive matches on our own enums

**Rule.** No `_ =>` arm on an enum the workspace defines. Adding a variant must break every
place that has to decide about it.
**Why.** A wildcard arm turns "a new case nobody handled" into a silent
default.
**Check.** `clippy::wildcard_enum_match_arm` (workspace, warn, failing the
gate under `-D warnings`) and `match_wildcard_for_single_variants`
(pedantic).
**Today.** Holds: no `#[expect]` of either in product code.

### T5. `#[non_exhaustive]`

**Rule.** Not on an enum that another workspace crate matches, and not on
a wire type: forward compatibility on the wire comes from an explicit
`Unknown`/`Other` variant and the protocol version (`Call::since`), not from
forcing every reader to add a wildcard. It is allowed on a struct, where it
means "built only through its constructor" and lets a field be added.
**Why.** The workspace compiles as one; `#[non_exhaustive]` on an enum
another crate matches forces the `_ =>` arm T4 forbids, in exactly the
places that must change when a variant arrives.
**Check.** Review.
**Today.** 14 enums carry it (`vessel-secure` 2, `vessel-relay` 6,
`vessel-llm` 5, `vessel-node` 1); 4 of them (`SecureError`, `Tamper`,
`Reach`, `LinkState`) are named by variant in another crate. 10 structs
carry it, which is allowed.

### T6. No untyped JSON past the decoding edge

**Rule.** `serde_json::Value` and `serde_json::Map` appear only in a module
that decodes or encodes an external format, and that module's header says
why a tree is needed there. Everywhere else: a typed struct, or `RawJson`
(from `vessel-proto`) for a payload Vessel forwards but never inspects. A
function that returns `Value` is a decoding edge or it is wrong.
**Why.** A `Value` is a type that accepts every shape: the compiler checks
nothing about it, and every reader re-decides what it holds.
**Check.** Review today. `clippy::disallowed_types` for both types, with an
`#[expect(clippy::disallowed_types, reason = "...")]` at each documented
edge, is the planned check.
**Today.** 115 uses of `Value` and 11 of `Map` in product code, in 25
files (clippy's count; the audit counted 149 lines mentioning either:
`vessel-agent` 62, `vessel-mcp` 44, `vessel-node` 17, `vessel-llm` 16).
Documented edges: `vessel-agent/src/claude/wire.rs`, `codex/protocol.rs`,
`mapper.rs`, `vessel-mcp/src/tools/conversation.rs`. Not documented:
`vessel-node/src/catalogue/registry.rs`, `vessel-llm/src/client.rs`,
`vessel-agent/src/handshake.rs` (builds `_meta` as a `Value`),
`vessel-store/src/codec.rs`, the MCP tools' `Result<Value, ToolError>`
returns.

### T7. Numeric conversions say what they lose

**Rule.** No `as` between numeric types: `From` where the conversion is
lossless, `TryFrom` with a handled error where it is not, and a named,
clamped helper with an `#[expect(clippy::as_conversions, reason)]` only for
floats and pixels where neither exists.
**Why.** `as` truncates and wraps without a word.
**Check.** `clippy::as_conversions`, `cast_lossless` (deny) and the
pedantic `cast_*` lints.
**Today.** Holds in the node line (0). Native: 6 casts, each clamped first
and each with an `#[expect]` and reason.

### T8. Arithmetic that cannot overflow silently

**Rule.** `checked_*` or `saturating_*` arithmetic, and the code says which
and why. No float arithmetic outside the modules that draw or animate.
**Why.** An overflow in release builds wraps; a count that wraps is a wrong
answer, not a crash.
**Check.** `clippy::arithmetic_side_effects`, `integer_division`,
`float_arithmetic`, `float_cmp`, `lossy_float_literal` (deny).
**Today.** Holds. Native: three files silence `float_arithmetic` and
`arithmetic_side_effects` for the whole file (`view/src/slider/{flair,
motion,state}.rs`); the rule is to scope an `#[expect]` to the item that
does the math (T20).

### T9. No `bool` that needs a comment at the call site

**Rule.** A parameter or field whose meaning is not obvious from its name
at the call site (`open(path, true)`) is a two-variant enum. A `bool` stays
only where the call reads as a sentence (`set_enabled(true)`).
**Why.** `true` at a call site says nothing; `Follow::Links` does, and two
such parameters cannot be swapped.
**Check.** Review; `clippy::fn_params_excessive_bools` (three or more) and
`struct_excessive_bools`.
**Today.** 114 functions in product code take at least one `bool` (clippy
with `max-fn-params-bools = 0`); how many read badly is review. Five
`#[expect(clippy::struct_excessive_bools)]` with reasons.

### T10. Owned values, borrowed where it matters

**Rule.** Borrow instead of cloning; data fanned out to many readers is
`Arc<str>`, `bytes::Bytes` or `Arc<T>` (section 4 of `AGENTS.md`). A
`clone()` on a hot path has a reason.
**Why.** A clone per reader per event is the allocation the hot-path rule
forbids, and it hides who owns the value.
**Check.** `clippy::clone_on_ref_ptr` (warn), `needless_pass_by_value`
(pedantic), `redundant_clone` (nursery, planned).
**Today.** Holds for the lints on; `redundant_clone` finds 1 in product
code.

### T11. Visibility as narrow as it can be

**Rule.** Private by default, `pub(crate)` next, `pub` only for the crate's
deliberate API. A type with an invariant has private fields and a
constructor that upholds it; `pub` fields only on plain data where every
combination of values is valid.
**Why.** A `pub` field on a type with an invariant lets any caller break
it, and every reader then has to check it again (T2).
**Check.** `unreachable_pub` (rustc, warn); review for fields.
`clippy::partial_pub_fields` is planned.
**Today.** `unreachable_pub` holds. `partial_pub_fields` finds 1.

### T12. Results are used

**Rule.** A function whose result matters is `#[must_use]`; a result is
not dropped with `let _ =` unless the type says it is fine.
**Why.** A dropped `Result` is an unhandled error; a dropped builder is a
call that did nothing.
**Check.** `must_use_candidate`, `return_self_not_must_use` (pedantic),
`unused_results` (rustc), `let_underscore_must_use` (warn).
**Today.** Holds.

### T13. `Option` and `Result`, never sentinels

**Rule.** "None" is `None`, failure is `Err`: no `-1`, `""`, `0` or
`u64::MAX` standing for "absent" or "failed".
**Why.** A sentinel is a valid value of its type; nothing stops it from
being used as one.
**Check.** Review.
**Today.** Not measured.

### T14. Static dispatch for closed sets

**Rule.** Generics or an enum for a known set (harness kinds, session
modes); `dyn Trait` only at an open extension point, with a comment that
says why. No trait with a single implementation "for testing".
**Why.** An enum is matched exhaustively (T4); a trait object is not.
**Check.** Review.
**Today.** The audit found no misuse in the node line. Native: `&dyn
WorkspaceWords` over a closed set of two languages.

## Rust: errors and failure

### E1. Typed errors, one enum per library crate

**Rule.** Each library crate has its own `thiserror` enum; no `anyhow` in
libraries, no `Box<dyn Error>` in a signature, no `String` as an error. A
library function's error type says which failures a caller can tell apart.
**Why.** A caller cannot match on a `String`, and an erased error loses the
variants the caller must decide on.
**Check.** Review; `vessel-*` crates' manifests (no `anyhow` dependency).
**Today.** `anyhow`: 0. `Box<dyn Error>`: 2, both trait bounds a
dependency's API requires (`hyper`'s serve functions in
`vessel-relay/src/server/accept.rs` and `vessel-node/src/models/listen.rs`).
`Result<_, String>`: 29 signatures in 20 files (8 are `clap` value parsers
in `vessel-cli/src/args*`, which accept a typed error too; `vessel-web`'s
`keep` 5; `vessel-node`'s models 5; `vessel-proto/src/ts.rs` 2).

### E2. Errors keep their source; map once

**Rule.** `#[source]`/`#[from]` keeps the cause; `map_err` never discards
it. An error crossing the wire maps to `vessel-proto`'s `ErrorCode` and a
message, once, at the boundary.
**Why.** An error without its cause cannot be diagnosed from a log.
**Check.** `clippy::map_err_ignore` (warn), `error_impl_error` (warn);
review for the wire mapping.
**Today.** Holds.

### E3. No panic outside tests

**Rule.** No `unwrap`, `expect`, `panic!`, `unreachable!`, `todo!`,
`unimplemented!`, indexing or slicing that can panic, outside tests. The
one way out is `#[expect(clippy::expect_used, reason = "...")]` whose reason
is a one-line proof (a constant parsed by a test, say).
**Why.** A misbehaving session or client must never take the node down.
**Check.** `unwrap_used`, `expect_used`, `panic`, `unreachable`, `todo`,
`unimplemented`, `indexing_slicing`, `string_slice`, `get_unwrap`,
`unwrap_in_result`, `panic_in_result_fn`, `exit` (deny).
**Today.** Holds: one `expect` with its proof
(`vessel-harness/src/profile.rs`) and one `unreachable` expectation.

### E4. No `unsafe`

**Rule.** `unsafe_code = "forbid"` in every workspace. An exception would be
a named crate whose manifest relaxes it, every block with a `// SAFETY:`
comment that proves it, under `clippy::undocumented_unsafe_blocks` and
`multiple_unsafe_ops_per_block`, and an ADR. None exists.
**Why.** Memory safety is the one guarantee no test can recover.
**Check.** `unsafe_code = "forbid"` (rustc).
**Today.** Holds: 0 in both lines.

### E5. No debug leftovers

**Rule.** No `todo!`, `dbg!`, `println!`, `eprintln!` in product code: logs
go through `tracing` with fields, the CLI writes to a locked stdout handle.
Build scripts print `cargo:` lines, which is cargo's protocol.
**Why.** Stray output corrupts the CLI's machine-readable output and leaks
values a log would redact.
**Check.** `dbg_macro`, `print_stdout`, `print_stderr`, `todo` (deny).
**Today.** Holds.

### E6. Suppressions are narrow and say why

**Rule.** `#[expect(lint, reason = "...")]`, never `#[allow]`; on the
smallest item, never file-wide (`#![expect]`) in product code. The reason
says why the lint does not apply, not what the code does. An `#[expect]`
waiting for another track's code names the track and is deleted with its
first caller.
**Why.** An `allow` outlives the reason it was added for; `expect` fails
when the lint stops firing. A file-wide `expect` hides the next violation
in the same file.
**Check.** `allow_attributes`, `allow_attributes_without_reason` (deny);
review for scope and reason.
**Today.** Holds: 0 `allow`, 46 `#[expect]` in product code, all with
reasons, none file-wide. Weak ones: 15 `unused_async` and 5
`unnecessary_wraps` that hold a shape until it is built (5 in the agent
drivers' stubs), 2 `dead_code`. Native: 49, of which 11 `dead_code`
waiting on other tracks and 3 file-wide.

### E7. Binaries own the process

**Rule.** Library code does not read the process: no environment variables
for configuration, no home directory, no current directory, no
`std::process::exit`. The binaries read them and pass typed values in
(`HarnessEnv`, `Config`). No `sleep` to coordinate: wait on a channel, a
`watch`, a `Notify` or a cancellation token; a `sleep` is a timer
(backoff, a deadline) and says so.
**Why.** A library that reads `HOME` cannot be tested without changing the
environment every test in the process shares, and a sleep that waits for
something is a race.
**Check.** Review today. `clippy::disallowed_methods` for
`std::env::var`, `var_os`, `vars`, `dirs::home_dir` and
`std::process::exit` in library crates is the planned check.
**Today.** Library code: 12 `std::env::var_os` reads (`vessel-agent` 4,
`vessel-harness` 3, `vessel-proc` 2, `vessel-node`'s library 2,
`vessel-mcp` 1) and 2 `dirs::home_dir` (`vessel-harness/src/mcp.rs`, the
Windows profile check; `vessel-store/src/data_dir.rs`,
`DataDir::default_root`); which of them read a home or a harness variable
is review. `tokio::time::sleep`: 29 in product code (`vessel-node` 11,
`vessel-relay` 10), each to be read as timer or wait.

## Rust: modules

### M1. One responsibility per crate, module and file

**Rule.** A crate, a module and a file each own one thing, named by their
top doc comment (`//!`), which also says what they must not do. Split by
responsibility, not by size; size (M3, M4) is the signal that a split is
due. No `utils`, `common`, `helpers` or `misc` crate or module: a shared
thing belongs to the lowest crate whose job it is.
**Why.** A file that owns two things changes for two reasons, and its
readers hold both.
**Check.** Review; the size budgets as the measurable proxy.
**Today.** Every one of the 737 product files starts with `//!`; no
module is named `utils`, `common`, `helpers` or `misc`. Native: all 353
files have the header; one `common` module (`app/src/areas/common.rs`). Files that hold
more than one thing are named in the audit (`vessel-node/src/models/
proxy.rs`, `fleet/links.rs`, `vessel-agent/src/codex/session.rs`).

### M2. The crate map is law

**Rule.** The crate table of `docs/ARCHITECTURE.md` lists, for each
workspace crate, the `vessel-*` crates it may depend on, and nothing else
is depended on. Its companion table names the external crates that have
one owner (ACP only in `vessel-agent`, SQLite only in `vessel-store`, and
so on). Adding an edge means a row change here and a paragraph in ADR
0001 saying why. No cycles, no reaching into another crate's internals,
wire types only in `vessel-proto`, harness knowledge only in
`vessel-harness`.
**Why.** The direction of dependency is the architecture; an edge added in
a manifest without a decision is how layering erodes.
**Check.** `scripts/check-boundaries.mjs` (the edges and the owners, from
`cargo metadata`); review for internals and harness knowledge.
**Today.** Two edges were in the manifests and not in the table:
`vessel-node -> vessel-agent` (since 2026-11-12) and `vessel-testkit ->
vessel-proto` (since 2026-11-14); ADR 0001's map also lacked `vessel-agent
-> vessel-harness`. All three are recorded with the check that now holds
them. Sideways reaching (`super::super::`): 5 sites in the node line.
`HarnessKind::<variant>` outside `vessel-harness`: 5 uses in
`vessel-session/src/runtime/drivers.rs`, not yet read for whether they
decide behaviour. Native: `spikes/native/README.md`'s tables (edges,
owners of GPUI's crates, Fluent and tokio, and names only some files may
use) and `spikes/release/README.md`'s are held by the same script
(`--workspace`); 0 today, after the README named the browser's entry point
(`app/src/web.rs`) beside `main.rs` as a place `gpui_platform` belongs.
`super::super::`: 8 sites.

### M3. File budget: 500 lines of product code

**Rule.** A file holds at most 500 lines of product code (tests not
counted). The files over it on 2026-11-14 are listed in
`scripts/rust-size-allow.txt` with their size; a listed file may shrink,
never grow, and its line goes when it is within budget. Nothing is added
to the list.
**Why.** A long file is a module that does several things; review did not
stop them growing (the target in `AGENTS.md` was "about 400").
**Check.** `scripts/check-rust-size.mjs`.
**Today.** 33 files listed (7 over 800; the largest
`vessel-node/src/models/proxy.rs`, 1,592). Native: 22 listed in
`spikes/native/scripts/rust-size-allow.txt` (6 over 800; the largest
`app/src/areas/draft/pickers.rs`, 1,190); the release tools: none over
500.

### M4. Function budget: 60 lines

**Rule.** A function is at most 60 lines; over that it needs an
`#[expect(clippy::too_many_lines, reason)]` whose reason is that the length
is the point (one arm per RPC method, say), and even then a table it can
ask is preferred (M5).
**Why.** A function that does not fit on a screen is read in pieces, and
its pieces are not named.
**Check.** `clippy::too_many_lines` at clippy's default of 100 lines today
(counting code lines only); lowering `too-many-lines-threshold` is the
planned ratchet.
**Today.** Holds at 100, with 10 `#[expect]`. Over 60: 114 functions in
product code; over 80: 36. Native (audit's upper bounds): 128 over 60, 64
over 80.

### M5. No duplicated knowledge

**Rule.** One fact in one place. No block of 12 or more lines repeated;
no list of the RPC methods restated per crate: a property of a method
(unary, forwardable, refused to agents, its version) is a table in
`vessel-proto` the other crates ask, so a new method still has to decide.
**Why.** Copies drift; the four copies of the method list are four places
a new method can be forgotten in.
**Check.** Review today; a duplicate detector in the gate is planned.
**Today.** 23 repeated blocks of 12+ lines, 669 lines in all (exact
matches, a lower bound). Seven of them are one fact in four places: which
RPC methods are unary, forwardable or refused
(`vessel-rpc/src/client/mod.rs`, `vessel-web/src/calls.rs`,
`vessel-node/src/workspace/policy.rs`, `vessel-proto/src/forward.rs`).
Native: 4 blocks, 61 lines.

### M6. Public items say why they exist

**Rule.** Every `pub` item has a doc comment that says what it is for and
what a caller must know (errors, panics: there are none, E3); a new public
item's comment says why it exists.
**Why.** The public API is the contract other crates build on.
**Check.** `missing_docs` (rustc, warn), `missing_errors_doc` and
`missing_panics_doc` (pedantic); review for the "why".
**Today.** Holds for the lints.

### M7. Tests beside the rule they test

**Rule.** A unit's tests live in its own `tests.rs` (or `#[cfg(test)]`
module) beside it, process-level tests in the crate's `tests/`; a change
that adds or changes a rule adds the test that pins it, in the same
commit. Decoders are tested against recorded fixtures.
**Why.** A rule without a test is a rule the next refactor may drop.
**Check.** Review.
**Today.** By the audit's heuristic (no test in the file, none in a sibling
module; it cannot see `tests/` directories), up to 200 product files of
150+ lines in the node line have no test of their own; native `app`, which
has no `tests/` directory, 51.

### M8. The native app's screens use the kit's tokens

**Rule.** (`spikes/native`) Colours, type, spacing and radii come from the
kit's tokens, never a literal in a screen; no crate of the workspace
writes a hex colour outside the kit's tokens (`kit/src/theme.rs`,
`kit/src/theme/`).
**Why.** A literal spacing is a design decision made once per call site;
the token is the decision made once.
**Check.** `spikes/native/scripts/check-tokens.mjs`: per file and kind,
listed in `spikes/native/scripts/tokens-allow.txt`, only falling; review
for type and radii.
**Today.** In `app/src`, 443 `px(<number>)` equal to a spacing step (0
included) and 90 others; 44 hex colours outside the kit's tokens (40 in
the kit's own files: code highlighting 18, the agents' marks 8; 4 in
`app`).

## The page's TypeScript (`web/`)

### S1. The compiler at its strictest

**Rule.** `strict`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`,
`noImplicitOverride`, `noPropertyAccessFromIndexSignature`,
`useUnknownInCatchVariables`, `noImplicitReturns`,
`noFallthroughCasesInSwitch`, `noUnusedLocals`, `noUnusedParameters`,
`noUncheckedSideEffectImports`, `verbatimModuleSyntax`, unreachable code
and unused labels as errors; the build scripts under the same flags.
**Why.** Each flag removes a class of value the program can hold without
having checked it.
**Check.** `npm run typecheck` (`web/tsconfig.json`,
`web/scripts/tsconfig.json`).
**Today.** Holds: all on.

### S2. No `any`, no `!`, no assertions

**Rule.** No `any`, explicit or implicit; no non-null `!`; no type
assertion but `as const`. A value from the DOM or the wire is narrowed by a
function that checks it. The one file with an assertion
(`src/data/terminal-input.ts`, xterm.js's private core) is exempted in
`eslint.config.js` with its reason.
**Why.** Each is a place where the code tells the compiler to stop
checking.
**Check.** `@typescript-eslint/no-explicit-any`, the `no-unsafe-*` rules of
`strictTypeChecked`, `no-non-null-assertion`,
`consistent-type-assertions` (`never`), `no-unsafe-type-assertion`;
`noInlineConfig`, so no rule is switched off in a comment.
**Today.** Holds.

### S3. Closed sets are discriminated unions, switched exhaustively

**Rule.** A value with kinds is a union discriminated by a literal field; a
`switch` over it covers every member and has no `default` that hides a new
one.
**Why.** As T4: a new kind must break the switch that has to handle it.
**Check.** `@typescript-eslint/switch-exhaustiveness-check`,
`noFallthroughCasesInSwitch`.
**Today.** Holds; with its strictest options (`considerDefaultExhaustiveForUnions:
false`, `requireDefaultForNonUnion`) it also finds 0.

### S4. Ids are branded

**Rule.** A handle (a node, a session, a terminal) is a branded type made
only by `src/bridge/ids.ts`, which checks its shape; tests brand theirs with
`src/testing/ids.ts`, which the page may not import.
**Why.** As T1: a session id passed where a node id goes must not compile.
**Check.** `no-restricted-imports` (the testing helper); review for the
rest.
**Today.** The import rule holds; not measured beyond it.

### S5. Generated wire types are never edited

**Rule.** `src/wire/` is generated from the Rust types; a wire change
regenerates it (`AGENTS.md` section 1).
**Why.** The page and the node must agree on the wire; one source keeps
them agreeing.
**Check.** The `ts` tests fail while `web/src/wire` differs from what the
Rust types declare (CI, Linux).
**Today.** Holds.

### S6. Layers and size

**Rule.** One direction: wire, bridge, data, features, routes; the kit
imports none of them; Base UI only through the kit. A TypeScript file is at
most 300 lines, a stylesheet 400.
**Why.** As M2 and M3.
**Check.** `no-restricted-imports` per layer in `eslint.config.js`;
`web/scripts/check-size.mjs`.
**Today.** Holds; the size list is empty.

## Process

### P1. Every commit passes the gate

**Rule.** The gate of `AGENTS.md` section 1, in order, on every commit
before it is reported; the report pastes the summary.
**Why.** A red commit in the history breaks bisecting and every branch cut
from it.
**Check.** CI on every push; review of the report.

### P2. A refactor changes no behaviour

**Rule.** A commit that moves, splits or renames code changes no behaviour
and no test's expectations; it says what moved where. A behaviour change is
its own commit.
**Why.** A refactor mixed with a change cannot be reviewed as either.
**Check.** Review: the tests of a refactor commit are unchanged or only
moved.

### P3. Reports carry measurements

**Rule.** A report states numbers: lines before and after, violations
before and after, the gate's counts, timings where speed is the point. No
"cleaner", "faster" or "safer" without the figure.
**Why.** The owner asked for answers in measurements.
**Check.** Review.

### P4. Debt only shrinks

**Rule.** No change adds a violation of a rule here, even of a rule with
open violations; a change that edits a violating item fixes it or says why
not. A new lint or check lands with zero failures: the existing ones are
fixed, carry an `#[expect]` with a reason, or are on a list that only
shrinks.
**Why.** A rule that tolerates new violations is not a rule.
**Check.** The ratchets (M3); review for the rest.

### P5. Every workspace is gated

**Rule.** Each Cargo workspace in the repository (`crates/`,
`spikes/native`, `spikes/release`) has its gate in CI, not only on a
developer's machine.
**Why.** A gate nobody else runs is a gate that rots unseen.
**Check.** `.github/workflows/`.
**Today.** Not held: only the root workspace and `web/` run in CI; the
native app's and the release tools' gates are local
(`spikes/release/ci/app-release.yml` is marked "NOT RUN").

### P6. What a reviewer refuses

A change is sent back when it:

- adds a violation of any rule here (P4), or a line to a ratchet list;
- adds `#[allow]`, a file-wide `#![expect]`, an `#[expect]` whose reason
  restates the code, or an ESLint exemption without its reason;
- adds a `vessel-*` or owned external dependency without the table row
  and ADR 0001 paragraph (M2);
- adds a `String`, `bool` or integer where a type exists or should (T1,
  T9), or a `Value` past an edge (T6);
- mixes a refactor with a behaviour change (P2), or reports without the
  gate's numbers (P1, P3);
- leaves documentation that disagrees with the code (`AGENTS.md`
  section 10).
