# ADR 0095: one maintained terminal screen engine

Status: source reviewed; gate accepted with the named base defect below.
Separate prerequisite package before the terminal UI.

## Decision log

- Purpose: node capture and native restore/query must use one screen
  interpretation; actor/channel/PTY ownership stays with vessel-session.
- Existing assets: native ADR 0073 model and W17's staged vt100 0.18.1
  adapter. Both were measured over the same actual terminal output against
  Paseo's server factory. Neither unchanged candidate matched every field.
- Alternatives: repair the native parser, use maintained vt100 with narrow
  projection/grid patches, or keep divergent engines. The coordinator chose
  vt100 from the measured core semantics, rather than effort already spent.
- Choice: pure root crate `vessel-terminal-screen`, no Vessel dependency,
  runtime, PTY, harness, filesystem, environment or UI toolkit. It alone owns
  vt100 and exposes typed cells, geometry, retention and capture. Session
  consumes it through its existing ordered actor; D1 later retires native parsing.
- Why: vt100 already matches the corpus's raw text, cell widths and cursor.
  Native loses original combining text and emoji presentation; normalization
  cannot make those raw data differences disappear.
- Costs: exact MIT vt100 0.18.1 is vendored with its original license,
  archive checksum, per-member baseline hashes and numbered minimal patches.
  Missing public grid APIs require capacity/reflow/access additions. Synthetic
  ANSI replay is forbidden because it destroys saved cursor, regions and modes.

## Measured baseline and required corrections

The frozen corpus contains actual isolated coreutils ls, Git diff and Vim PTY
output (colors, CJK/combining text, alternate screen, scroll regions, cursor,
resize), eight checkpoints. Oracle: exact Paseo server `headless-terminal.ts`
and `terminal-capture.ts`, headless 6.1.0-beta.334 plus UnicodeGraphemes
0.5.0-beta.330. Browser xterm is not substituted. This runs the factory and
capture function, not the whole daemon or PTY transport.

Raw baseline differences, preserved in the engine comparison evidence packet:

| Candidate | Text cells | Width cells | Attributes | Capture arrays |
| --- | ---: | ---: | --- | ---: |
| native c66357e8 | 155 (65 NFC-equivalent, still different raw text) | 7 | 0 | 5 checkpoints |
| staged vt100 c0b9d71c | 0 | 0 | 130 foreground + 2 flag fields, all wide continuations | 1 checkpoint after resize |

Required fixes: continuation attributes come from their lead cell; the grid
retains hidden right columns so capture immediately after alternate resize
returns the full row as xterm does. Primary width resize reflows soft wraps.
A capacity setter trims oldest history while preserving recent rows. Query
and retention contracts remain owned by this one engine, not another parser.
The same corpus must reach zero raw differences; add primary resize cases.

31 feed/resize trials and process high-water measurements are preserved, but
are not a hardware-GPU, whole-app, or maximal-memory claim. Native was built
without vte/std, staged vt100 with its original default features; builds were
separate so feature unification did not change the native decoder. The core
uses vt100 without default std to retain vte's bounded OSC storage. Its corpus
rerun must validate that production configuration too.

## Bounds and consumer boundary

Geometry reserves both screens before admitting history under one cell budget.
The minimum width follows xterm's complete wide-cell pair. The reference
silently clamps a narrower request; this validated boundary refuses it before
the engine allocates or mutates anything, so PTY and screen geometry cannot
silently diverge. Native fitting must apply the same floor before requesting
a resize. This is a stated validation difference, not support for narrower rows.
The requested history setting retains Paseo's 0..1,000,000/default10,000 range;
cell admission can lower the effective count for unusually large geometry.
One capture has a named byte limit, includes empty rows and separators and
refuses a too-large response rather than truncating it. Signed inclusive row
ranges clamp and count negatives from the end. Text stays unnormalized; JS
trimEnd semantics include BOM and exclude NEL. No ANSI is invented by capture.
Paseo's `stripAnsi=false` returns that same translated row; it does not
regenerate SGR styling from cells. CLI Plain/Preserve stays a wire/glue choice.
Native rendering reads typed retained cells (including continuation ink),
without another row parser or another capture renderer. Immutable snapshots
retain those cells, cursor, active buffer and typed history count after owner
exit or later live eviction. They do not claim mutable decoder restoration.

Query replies follow the explicit server handlers at
`packages/server/src/terminal/terminal.ts:884–925` and its fixed OSC color
map at 45–49. Construction defaults to a silent observer; only an explicit
`ReplyDuty::NodeAuthority` can return ordered reply bytes. This supersedes
ADR 0073's provisional viewer-owned, theme-colored response contract for the
new engine: the intended normal-query owner prevents duplicate replies from
native observers, subject to the overlap limitation below, and the fixed colors
reproduce the reference server. No network or
PTY write occurs inside the crate. Unsupported queries and clipboard requests
produce no replies. The reply allocation bound derives from the input budget
and longest allowed reply, including a query begun in the previous feed; an
admission failure is returned explicitly with the consumed prefix.

Seven query oracle cases execute the reference server's actual registered
handlers on its own headless factory. Golden inputs cover pending wrap,
origin mode/scroll regions, alternate screen, OSC whitespace and unsupported
or clipboard queries. Rust tests repeat whole and one-byte feeds, silent
observers and a carried-query burst. These are source/runtime oracle tests,
not a real PTY transport claim.

Two small upstream corrections also preserve this boundary: unknown CSI
callbacks keep their space intermediate so a non-query cannot alias an
admitted query, and bold/dim remain independent until normal-intensity reset.
The latter matches the exact server InputHandler's separate flags and the
existing native cell contract. The numbered patch list records both; golden
cases use actual server handlers and raw xterm cell flags in both orders.

Wire DTOs remain in vessel-proto. W17 owns mapping, bounded actor inbox,
output/resize/capture ordering and immutable final-state lifetime. No screen
is reconstructed from a pruned event ring. The native parser stays only until
D1 switches to the passed engine; this prerequisite does not claim retirement,
full D1 UI, physical input, platform runtime or renderer parity.

## Node presentation and ambiguous-overlap limitation

The coordinator's compatibility ruling preserves the rare AmbiguousOverlap raw
span unchanged. Two old raw clients, or live output plus history replay, can each
automatically answer an embedded query (for example OSC title with ESC[6n).
Those clients do not understand candidate settlement, and their CPR bytes cannot
be distinguished from real keys. Filtering stdin is forbidden. Therefore no
universal exactly-once claim is possible under this compatibility contract.

The required node exactly-once guarantee covers ONLY normal queries whose
tokens are suppressed from published presentation/history. An ambiguous legacy
reply is best-effort, with possible duplicates just as with multiple raw attaches.
A frame-only client receives no answer for an overlap span. The pure follow-up
retains its bounded typed UnansweredReplies candidate separately from normal
replies; that candidate grants no response authority and is not a replay command.
It must not be answered/settled by a frame reader until a separate route is
designed. No exclusive routing or overlap rewriting is authorized now.

W17 owns one node screen/frame/query adapter for typed Session(SessionId) and
Standalone(TerminalId) targets, with no ID conversion or parallel parser. CLI
capture remains TerminalId-only. Current adoption uses Screen::new Silent and
NodeAuthority is OFF. Normal-query activation requires the reviewed node responder
contract and C2's full whole PASS, with tests for suppression, split boundaries,
replay and absence of duplicate normal responses. The unreviewed engine extension,
frame/mode/marker API and typed candidate are not adopted by this frozen prerequisite.

## Verification record and limits

Exact pre-format e792 passes strict changed-package checking and11 tests,
including31 raw-cell checkpoints and7 server-handler query cases. Independent
review audited the expected artifacts and the formatting-only0d7 extension.
The first whole run failed formatting; the pinned official configuration fixed
that without behavioral or expected-value changes.

The corrected whole0d7 run is RED only at the named LimitPercent base alias
classification defect, fixed separately by9dfdf0a1. Seven actual whole steps
passed, including all three Clippy variants; protocol TS tests3/0 and web TS
1/1 stopped execution. Ten remaining planned whole steps did not run. The
explicit gate exception accepts this named base defect; it does not turn
unexecuted tests into evidence or justify another unchanged whole run.

No native gate is owed before a native dependency exists. Node actor ordering,
transport, native parser retirement/rendering, actual input, UI captures and OS
runtime remain outside this prerequisite. Upstream Cell has finite22-byte
content; the bounded corpus is not arbitrary-grapheme losslessness proof.

## Full consumer qualification remains open

Bounded source and partial-gate adoption is accepted with the named base
defect; this is not consumer-ready. Before ANY consumer-ready claim or
activation, this engine must pass one FULL whole on a base carrying the
gate49 alias fix. The ten unexecuted steps remain unverified until then.
No third unchanged worker gate is requested; consumer integration must
satisfy this enclosing qualification before shipping or activation.
