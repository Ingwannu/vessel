# G2: schedules, recurring prompts, saved prompts, webhooks

## Contract and references

The page has no schedules, heartbeat, saved-prompt or webhook screen and
no corresponding behaviour fixtures (`features-vessel-page`, section 11).
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
An ended schedule cannot be updated or run. Read-only connections expose
no mutation actions. Runs retain their detailed typed reason; failed,
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
labels and the typed editor's diagnostic copy.

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
