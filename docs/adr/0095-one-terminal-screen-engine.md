# ADR 0095: one maintained terminal screen engine

Status: prerequisite source/partial-gate accepted; full consumer gate open.
D1 follow-up amendments below remain proposed and independently unreviewed.
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
new engine: one server owner prevents duplicate replies from multiple native
views, and the fixed colors reproduce the reference server. No network or
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

## D1 follow-up: decoder metadata and multiplexed presentation

The same transitive VTE 0.15 decoder may be vendored with exact checksum,
licenses and two numbered minimal patches: preserve CSI parameter presence,
and expose read-only completed-token boundaries. Normalized parameters erase
the difference between omitted Kitty flags and explicit zero; guessing is
rejected because it changes keyboard negotiation. A separate scanner is rejected
because it duplicates terminal interpretation. Dependency classification is
re-audited before its use; the frozen prerequisite source is unchanged.

The node's presentation stream omits the non-rendering query requests it
answers. Incomplete tokens are withheld, other complete bytes remain, and
empty output still advances the event sequence. This also protects older raw
CLI clients and replay without suppressing actual input keys or refusing raw
attach. NodeAuthority stays off until the compatibility path lands and passes.
Current authoritative frame consumers use one revision/anchor/mode contract;
they do not restore mutable parsers from visible cells. The CLI ANSI renderer
is a display backend, not ANSI state replay into a parser.

## Presentation overlap exception

A canceled prefix can share its final ESC with the beginning of an answered
query. Removing that ESC would change the preceding non-query OSC or canceled
CSI at an older raw client. In this rare overlap, preserve the entire span
unchanged, do not send the candidate reply from the node, and return a typed
`AmbiguousOverlap` presentation event. Legacy raw clients may answer it as before; multiple raw clients or replay
can duplicate ambiguous responses. A frame-only renderer does not answer it. This is an explicit exception to node query authority, not a second
parser, guessed mode, stdin suppression or CAN normalization. The regression
`OSC title ESC [6n` must retain every byte and have no node reply.

Executed controls inside a pending query are retained using decoded callback
positions, not byte pattern matching. Their publication waits for completion
of the bounded token. A control token exceeding 64 KiB refuses explicitly;
this resource bound is a stated difference from the reference.

## Negotiated input modes

Decoded CSI metadata retains omitted, explicit u32 and overflow scalar states,
while original VTE numeric callbacks keep their unchanged u16 normalization.
This prevents explicit0 or flags above65535 from acquiring guessed defaults.
One bounded keyboard stack and typed independent switches accompany readonly
cell snapshots. Flags beyond u32 or pushes beyond1024 refuse explicitly; these
are stated type/resource floors versus the reference's unconstrained Number/
array tracker. No runtime dependency or second scanner is introduced. The
reference tracker resets only when disposed, so an in-band RIS does not reset
this negotiated state; new authority construction does.

## Stated difference: chunk-invariant input modes

The actual reference TerminalInputModeTracker loses a standalone ESC chunk:
its raw regex cannot retain a prefix before ESC[ exists. Thus ESC / [ / > / u
leaves its flags0, whereas the same original stream in one chunk gives1.
Vessel keeps the sole VTE decoder's chunk-invariant flags1. Reproducing the
reference's chunk-loss defect would require the forbidden second raw boundary
tracker. The raw239-step oracle and its failing log remain unchanged evidence;
a separate named chunk-loss regression records this deliberate difference.
The parity oracle freshly invokes the real reference tracker on the accumulated
original prefix once at every239 checkpoints. Its outputs are not edited, and
no expected value is deleted or flipped. This is reference observation, not
ANSI replay into a Vessel engine.

## Viewport marker ownership

Use the same grid's bounded physical-row markers, following the pinned server
Buffer's trim/insert/delete/reflow adjustment rather than tracking distance
from the moving tail. One engine allocation, buffer and reset epoch identify
a handle; create/resolve/release/replace are explicit. FollowLive and disconnect
release node-owned view slots. Retained/Evicted/Reset/Released and inactive
buffer state are typed; no anchor selects a transport or authorizes a read.
No per-row identity allocation or unbounded remap log is required. These APIs
are a follow-up contract until their exact implementation/verification lands.

## Ambiguous candidates and the explicit guarantee limit

AmbiguousOverlap preserves typed bounded UnansweredReplies candidates apart
from normal node Replies. They are neither responder authority nor a replay
instruction. Normal suppressed queries have the node's single-answer guarantee.
The unchanged ambiguous legacy span is best-effort: two old raw clients or
history replay can cause duplicate answers, and no stdin filtering is allowed.
Frame-only overlap receives no answer in this slice. Its candidates are kept
for diagnostics/future policy but must not be settled or written without a
separate route. Exactly-once is not claimed for that exception. This limitation
is necessary while preserving the approved unchanged legacy span; tickets
alone cannot constrain old clients that never send a settlement acknowledgement.
NodeAuthority remains off until the reviewed responder contract and whole pass.

## Program cursor and physical row endings

Authoritative frames expose independent optional shape/blink overrides and
outgoing hard/soft row endings from the same borrowed engine cut. Unset cursor
presentation belongs to each renderer's configured defaults; metadata is never
inferred from text, a raw scanner, event sequence or a reconstructed decoder.
Recorded pinned-headless controls exposed lost blink-only state, wrong1049
alternate entry coordinates, and restore retaining a pending-wrap column.
Minimal numbered engine patches preserve decoded u32 parameters, reset cursor
presentation/modes on soft reset without erasing/moving cells, inherit physical
alternate coordinates, and clamp an explicit cursor restore. The actual34-case
metadata/cell oracle supplements the unchanged original cell/mode/marker corpus.
This remains a pure producer extension; actor/transport/native/CLI renderer
adoption and enclosing consumer qualification are separate. NodeAuthority stays
off; no universal Unicode or malformed-stream parity is inferred.

### DECSTR mode sources and redundant alternate activation

DECSTR resets VT application-cursor and bracketed-paste switches in the same
decoded mode owner as the borrowed frame. Pinned headless InputHandler2817-2835
calls CoreService.reset68-72, whose defaults disable those switches. Kitty and
Win32 flags and the saved keyboard stack retain the actual server tracker
contract: protocol/terminal-input-mode.ts regex20-25/feed85-124 does not match
CSI ! p or RIS; terminal/terminal.ts1041 feeds that tracker, and1017 resets it
at lifecycle restart. The optional headless keyboard extensions have different
reset state; they are not substituted for this server negotiation contract.
Both original reference traces are retained, with no second scanner or flipped
accepted239-step expectations. This is an explicit split between the two
reference responsibilities, including the already stated RIS behavior.

Repeated1049 still saves the active buffer cursor, as InputHandler2017-2018
does, but BufferSet103-105 leaves an already active alternate grid intact.
The same engine now preserves its cells and marker epoch on that no-op.
Cursor attributes are saved independently per buffer, matching the reference's
buffer-local savedCurAttrData, so an alternate save cannot replace primary ink.
DECSTR resets the active buffer's saved presentation only. No synthesized Home,
replay, new parser or authority change is involved.

The same BufferSet80-112 transition owns47/1047/1049. Physical coordinates
carry on both entry and exit;1049 alone adds save/restore. Actual alternate
exit clears its rows and disposes its markers immediately, matching the
reference, while a readonly earlier snapshot keeps its own cut. This replaces
the previous delayed alternate invalidation at the next entry. Repeated entry
and exit do not create an extra transition or copy coordinates from an inactive
buffer. The recorded oracle supplements rather than replaces earlier fixtures.


## Staged typed input projection

The same pure terminal crate will own key/paste byte encoding from its typed
InputModes, following the actual production terminal-key-input and paste
functions. This is an output encoding rule, not another decoder or negotiation
tracker. The existing node writer and UI focus/IME/opening admission remain
outside. CLI textual send-keys retains its separate literal/token grammar.
The contract and reference paths are in terminal-input-encoding.md. The staged
producer has no actual native/CLI caller, new RPC or authority activation.
