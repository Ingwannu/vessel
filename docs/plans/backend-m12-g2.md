# G2: schedules, recurring prompts, saved prompts, webhooks

## Contract and references

The page has no schedules, heartbeat, saved-prompt or webhook screen and
no corresponding page fixtures (`features-vessel-page`, section 11).
Paseo's `utils/schedule-format.test.ts` is the formatting, interval,
product-name, title and validation case list being cloned here.
New portable JSON cases live in
`spikes/native/view/tests/fixtures/schedules/`; a future page should run
the same inputs and expected decisions rather than copying the cases.

Reference: Paseo's `screens/schedules-screen.tsx`,
`components/schedules/schedule-row.tsx`, `schedules/schedule-derivation.ts`,
`screens/schedules-screen-state.ts`, and `utils/schedule-format.ts`.
The entry is the sidebar's Schedules item. The list has Active (including
paused) and Ended filters, rows with identity, target, cadence, history,
next run and state, and a menu for edit, pause/resume, run now and delete.
Paseo's title precedence is name, new session title, first nonblank prompt
line. It is retained intact, without truncating in the view model.

## Slice 1: pure schedule decisions

`view/src/schedules/` owns list partition/order, title, available actions,
next-due visibility, typed cadence parsing, safe replacement, and history
classification/session navigation. It depends only on the existing
`vessel-proto` contract (ADR 0033); it performs no I/O and uses no UI type.
No dependency was added. Existing cadence parsing is reused instead of
introducing a second cron grammar or converting a rule back into strings.

The opened revision and unchanged interval anchor survive replacement.
Unknown definition variants cannot be edited. Node-side validation still
owns zone existence, possible due times, target access and existence.
An ended schedule cannot be updated or run. Known read-only authority
exposes no mutation actions; unknown authority offers actions and lets the
node decide, as the rest of the app does. Runs retain their detailed typed reason; failed,
refused, cancelled, limited and interrupted turns are never labelled
normal completion. Session links exist only when a run names a session,
including a partially failed creation.

Deliberate differences from Paseo:

- Node `ScheduleState` is authoritative. Client clocks and incomplete
  session caches never decide expiry or target deletion. Vessel already
  sends these states; guessing them would contradict a node's decision.
- Unknown node states have their own inspection bucket, rather than
  being silently called ended or active.
- Existing-session schedules support pause/resume/run-now for an operating
  device, as Vessel's node allows. Paseo hides those menu items for its
  heartbeat targets. The node remains the final authority for permission.
- Elapsed intervals keep exact seconds (including 90 seconds); no rounding
  to a whole minute, which would change the rule.
- Ended definitions are read-only: Vessel refuses `schedule.update` for
  them, whereas the reference still offers an editor.

Tests cover all state variants and ended reasons, every run outcome and
turn/session end shape, read-only actions, title precedence, Korean and
long text, deterministic ordering, interval boundaries, cron/zone shape,
unknown variants, terminal/resume refusal and revision/anchor retention.
Fixtures have fixed public example IDs and timestamps, no private data.
The fixtures cover eight schedule states/reasons and 29 run shapes,
including their English and Korean labels. `text/schedules.rs` owns those
labels and localised UI error copy. Diagnostic Display is separate, static
log copy and never reads the language table.

No drawing is delivered in this slice, so there are no captures yet.
Screen, link calls, fake-fleet shapes, forms and runtime checks follow.

## Contract gaps found before drawing

- **Webhooks are not callable.** Despite the plan's intended method table,
  ADR 0034's final “Where it stands” is accurate: hook shapes/store exist,
  but `crates/vessel-proto/src/rpc.rs` registers no `hook.*` method; the
  node has no listener or CLI command. No working webhook list/create/
  delete screen can be built against this checkout. This item is stopped
  and reported to the coordinator; no wire/node change or inert UI added.
  The node track needs `hook.list`, `hook.create`, `hook.update`,
  `hook.rotate`, `hook.delete`, and `hook.deliveries`, plus the listener
  and ADR 0034's permission-mode guarantee. No tab, disabled row or
  placeholder will be drawn for hooks.
- **Machine time zone is not sent.** Only a cron cadence's explicit IANA
  zone exists in the contract. It is a schedule's chosen zone, not proof
  of the machine's system zone. An elapsed cadence has no zone at all.
  Showing machine-zone times needs node metadata. Do not substitute the
  client's zone or call the cron zone the machine's. Reported upstream.
  Coordinator decision: show the rule's own zone and the viewer's zone,
  explicitly named. Show next-run times in the viewer's zone, with the
  rule-zone equivalent beside them when different. Machine-zone metadata
  remains a node gap; it does not block these supported displays.

## Verification

Slice 1, pinned native workspace, whole workspace (no package selection):
`cargo fmt --check` passed; `cargo clippy --all-targets -- -D warnings`
passed; `cargo test` passed: 4 app, 8 kit, 10 link, 216 view unit and 73
page-parity tests (311 total, zero failures), plus zero-failure doc tests.
The seven new schedule tests run eight state/reason and 29 outcome cases.
The final repeat was delayed by shared build-disk reads; it exited zero.

Runtime and visual claims are deferred until a screen runs against the fake fleet. No live
node, preview unit, real harness or credential was accessed.

Problems met while running the gate:

- The first `cargo clippy --all-targets -- -D warnings` found three imports
  through private protocol modules. Changed them to the crate's public
  reexports. Its next run found a missing `EverySeconds` import in the
  tests; added it. The following whole-workspace Clippy run passed.
- During fixture review, replaced the CLI alias `claude` with the wire's
  `claude-code`, and corrected nested `SessionStatus`'s tag to `state`.
  The fixture decoder is exercised by the tests, not bypassed.
- Self-review found a replacement could be handed a known new definition
  after opening an unknown old one. Replacement now checks both, with a
  regression assertion: hiding the Edit action alone is insufficient.
- The first gate builds GPUI twice: Clippy's metadata then the test
  binaries. Both use the checkout's own configured build directory with
  four jobs; no shared distribution directory or release was used.


## Slice 1 review correction

The first slice was not accepted: its labels exposed raw seconds/cron,
its interval form did not model units, its UI errors could only speak
English, and its portable decisions depended on Rust Debug spellings.
The correction is one follow-up commit; no screens were started.

- `description.rs` recognises fixed-time, weekday/weekend, single-day,
  hourly and every-minute rules through `CronExpr::fields()`. It never
  reparses the expression. Every month and all 31 date values must agree
  with the described weekday group, so a restricted date and Vixie's OR
  semantics cannot silently become a daily rule. Unrecognised shapes keep
  the original expression as Paseo does.
- Intervals have a number and `IntervalUnit`. Opening a stored value uses
  the largest whole unit that loses nothing: 90 seconds remains 90
  seconds; 7,200 seconds becomes 2 hours. Labels decompose mixed units:
  “Every 1 minute 30 seconds” / “1분 30초마다”. Singular whole units read
  naturally (“Every hour”, “매일”). Range errors name the chosen unit.
  Negative, fractional, oversized and extreme integer inputs are tested.
- UI errors are `Lang::cadence_error` and `Lang::edit_error`, with Korean
  and English cases per variant/field/notation. The original grammar,
  number, bound and conversion errors remain available as sources.
  `InvalidCron` exposes a field and diagnostic prose, not a typed
  value/range/step subcode: at the input edge, UI notation comes from
  that field's slash/range markers, never by parsing error prose. This
  classifies an error only; the protocol remains the only grammar parser.
- The Korean feature is “예약”. Existing-chat targets are “Recurring
  prompt” / “반복 프롬프트”, in titles and action labels. This deliberately
  replaces Paseo's “Heartbeat”: it says what the prompt does without a
  server-health metaphor. Unknown filter, badge and explanatory sentence
  have three separate words. Terminal/resume errors describe the person's
  available operation rather than internal session-mode vocabulary.
- JSON decisions now use stable kebab-case ids decoded into small
  test-side enums, each mapped exhaustively to a real decision. No Debug
  spelling is compared. `format.json` carries the reference's inputs and
  expected English labels, Korean and contract-specific extensions.
- Impossible live statuses in a finished-session outcome are Unknown.
  The node's `automation/turns.rs::signal` only constructs `RunEnd::Session`
  under `!status.is_live()`; Starting/Running cannot prove a session ended.
- `Access` was deleted. `Authority` holds actual `vessel_proto::Scope`
  reports, unknown evidence, or an actual Forbidden response for one row
  and connection. Unknown offers actions; known insufficient and Forbidden
  show the same tested bilingual explanation and hide actions. A new
  connection starts fresh; a row's refusal must not suppress another row.
  The node's `principal.rs::permit` checks the exact requested scope;
  Admin alone does not imply Operate.
- Badge meaning is tested with the eight state cases: active Success;
  paused, expired, finished and unknown Muted; target/creator gone Error.
  No kit control is added here. The accepted shared badge commit was
  cherry-picked separately; schedules now reexport `view::badge::Variant`.
  The temporary local type was deleted in the same correction as the swap.
  The kit control and app mapping remain the shared badge track's code.

Reference differences that remain intentional: the node's interval is
exact, with no rounding/clamping to minutes; a zero/out-of-range interval
is an error rather than the reference helper's default; the protocol
requires an explicit zone (UTC is explicit in the corresponding cases);
its grammar accepts named weekdays case-insensitively, whereas Paseo's
validation test rejects lowercase `mon`. Parsed equivalent names and
shorthands can receive the same human label. Relative next-run distance
copy is not implemented in this slice; times/drawing remain later work.

Authority gap: Welcome and DeviceCredential do not report the current
device's scopes; `device.list` requires Admin. The link may know owner
bearer authority (all scopes), or use scopes actually read for the paired
credential's device id. Otherwise authority remains Unknown, never an
inferred read-only or operating grant. The coordinator has assigned the
missing self-scope metadata to a node track. No wire change is made here.

The accepted badge commit `840a941` was brought in as `c328d14`, separately
from this correction; its view/kit type and app adapter are unchanged.
For later fake-fleet checks, `cargo build -p vessel-cli -p vessel-node
-p vessel-testkit --bins` passed in the main workspace. No main-workspace
source was changed and its tests were not run. No fleet or display was
started; visual and runtime schedule checks remain unverified.

Gate problems during correction: Clippy caught an unseparated large test
literal and a wildcard over known IntErrorKind variants; both were fixed
without suppression. One intermediate retry queued behind an earlier
Clippy invocation's build-directory lock; there were no simultaneous
compilers, and later gate commands are sequential. A local edit command
used the wrong working-directory-relative path, failed before writing,
and was rerun with the correct checkout-relative path.

Correction verification, after importing the shared badge and freezing code:
`cargo fmt --check` passed; `cargo clippy --all-targets -- -D warnings`
passed; `cargo test` passed (4 app, 10 kit, 10 link, 226 view unit and 73
page-parity: 323 tests, zero failures, and zero-failure doc tests). The view
includes 17 schedule tests; portable cases cover 8 states, 29 outcomes and
37 cadence descriptions. Every source field/error notation and each selected
interval unit has English and Korean assertions. No captures exist because
this remains view logic; screens await the coordinator's acceptance.

The last reference comparison added 25-hour and 61-minute cases: retain
the reference's exact whole-unit labels and only decompose a seconds
remainder that the reference rounds. That last change was included in
the final sequential three-command gate. One formatting patch did not
match rustfmt's line wrapping, wrote nothing, and was reapplied after
reading the source.
