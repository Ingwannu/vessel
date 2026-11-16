# F3 native usage presentation

This continues [wp-c6.md](wp-c6.md). F3 owns tracker rows 622 (query states),
623 (platforms/connections) and 625 (cards) as well as its other assigned usage
rows. The node report engine remains documented in
[wp-f3-sources.md](wp-f3-sources.md) and ADR 0042.

## Architecture and current behavior

`view::usage` owns pure state/formatting; `link::usage` owns bounded correlated
node calls; `app::areas::usage` draws Settings, the lazy sidebar modal and the
conversation quota notice. The kit owns cards/rows, dialog/popover, meter,
buttons and tooltips. No provider parser or second source registry lives in the
native app. The reported token/cost ledger and context occupancy remain separate
from saved-account quota and remain available when external lookup is Off.

Primary references are pinned Paseo area-6 B.md:321–347; app usage
model.ts:107–181,209–236; card.tsx:37–254; usage-modal.tsx:70–90;
options.tsx:38–90; and preferences/pinned implementations cited in wp-c6.

[Decision Log]
- Purpose: preserve query state, account-attempt state and original successful
  timestamps without making old facts look like a new fetch.
- Constraint: the pinned host and agent projections deliberately differ.
- Choice: host queries use the reader's complete cached observation first;
  agent queries use `agent_observation`, which hides values on a query error.
- Alternative: applying error-first to hosts would contradict model.ts:107–150;
  showing an old agent quota without its query error contradicts :155–181.
- Why: the two pure projections retain the existing reader and correlation
  identities, with no additional state machine or generation allocator.
- Impact: pending same-selection reads may show prior complete data. A host query
  shows loading/error/Retry only without data; a failed agent query shows its safe
  error and a cache-only Retry. An account-level Failed/Unavailable entry inside
  a successful query still displays its explicitly retained success and old time.

The pure reader regression covers Timeout and Forbidden, host data preservation,
agent error precedence, old timestamps during retry and the next actual success.
The earlier intermediate error-first host implementation was corrected after
opening the primary implementation and N2's independent source check.

Known global Off displays the existing precise three-store disclosure and an
Enable action. Enabling changes policy and rereads cache; it never grants the
seven additional sources or initiates a vendor fetch. Unknown policy has no
false Off toggle. Source-specific consent remains bound to its typed descriptor
and exact disclosure version. Unsupported sources have no working-looking grant
or refresh action.

## Cards and action admission

A generic card uses the shared settings card/row: source mark and label, optional
plan badge, current-attempt dot/text and Refresh in its header; account label and
successful relative freshness in its footer. Existing approved marks use the
shared Generic/Bot fallback where required by K. No new vendor bytes are added.
Individual login errors use actual harness/public labels plus translated closed
failure reasons, never arbitrary source stderr or credential-store paths.

Freshness derives only from the successful observation. It floors minutes,
hours and days, uses just-now under a minute, and distinguishes a current success
from a retained one. Age fields use NUMBER($count) with checked Number::count
arguments. Each new EN/KO key has direct translator context; future plural rules
can use the same numeric Rust input. Minute cache ticks redraw labels without
external lookup. Query/attempt completion never substitutes a new success time.

Callbacks retain authenticated ConnectionContext and the existing parent-modal
ticket; pins additionally retain the accepted observation ticket. A reconnect,
parent replacement, changed disclosure or retired observation cannot retarget an
old action. Pin rows expose checkbox/toggled metadata and their source/window
identity. This alone does not establish platform accessibility or keyboard
traversal.

The host picker captures rendered NodeIds, not mutable list indices. Its terminal
choice uses the shared close_and_restore after admission. Parent close validates
its own ticket, closes child options and picker, then acknowledges dialog focus.
Options retain the shared popover's per-open lease separately from domain
admission; obsolete body/intents are withheld before deferred lease retirement.

## Shared controls and explicit remaining gaps

Announced green line a7aab9d6 supplies W19's reviewed header-actions input on the
same dialog renderer. The 640px Wide Usage modal puts host choice, Refresh all
and Options before the existing close control. The modal has no duplicate body
Refresh all. Unavailable hosts keep only host choice; no-host frames omit the
empty actions slot. Settings retains its own refresh action. First-frame modal
captures and exact host-filter geometry remain unverified.

The current regular typed buttons are a working first version, not exact Usage
chrome. Required source profiles remain separate:

| Surface | Pinned source | Owner / state |
| --- | --- | --- |
| Fixed sidebar Usage icon | Footer density 28/44, glyph 16/20, muted-to-foreground hover | S2 sole FooterIconButton owner, awaiting W12 lower chrome; no checked producer or footer edit |
| Options | Toolbar small 20/32, adaptive glyph 14/18, source extra-muted ink | W12 producer in announced base 13; consumer integration requires its own strict check |
| Refresh | Same small toolbar profile plus source loading | W12 loading follow-up remains unadopted |
| Toolbar/card tooltip | Source side/offset and 300ms desktop delay | W12 extends existing Tooltips; source-only follow-up, not adopted |
| Toolbar loading | Existing ActivityIndicator ring/750ms profile | W12 follow-up; no area spinner substitution |
| Large Usage word catalog | Shared bounded cached-message generator | L canonical c829 producer landed through base 13; combined Usage compile still required |
| Balance amounts and exact counts | USD currency, compact tokens and exact unsigned counts | L shared ADR 0101 gap; original amounts/units/counts remain visible |

No area size/ink override, Pill fallback, second tooltip engine or parallel footer
registry is introduced. The optional summary still uses S2's single preference,
hidden by default and rendered only with actual pinned data. The always-present
footer launcher awaits S2's checked equivalent and opens the real shared modal.
Values above the current Number decimal bound are not silently dropped or clamped
to fake currency/compact parity.

## Verification and acceptance boundary

L accepted the exact e89334984 EN/KO word/key/context dataset, including the ten
new compact status/freshness keys and numeric age contract. Earlier accepted
Usage messages remain unchanged. This is not number-format, runtime or UI proof.

| Immutable source / run | Result and scope |
| --- | --- |
| 3febf32e / native context 9 | RED before tests: missing direct context comment for usage-fresh-days; corrected in e893 |
| f80dad10 / native context 10 | RED before tests: agent_name returns String, new fallback selected &str; corrected with owned-label map_or_else |
| c1679cc9 / native context 11 | 62/0/0: app 3, link 6, view 53; actual card-header theme/locale/width probes, numeric age, login errors, query distinction and existing transport/lifetime cases |
| 71522d0e follow-up | Removes the unused card import and narrows two private field visibilities; no behavior/assertion change, no separate execution claim |

Native context 11 emitted those three warnings; it is not a strict Clippy pass.
Static size/tokens/diff checks are separate evidence. The exact toolbar/footer
profiles, large-catalog strict check, canonical assigned node wire, whole native
gate, managed same-source captures and final V4 package review remain owed.
No full F3 acceptance follows from a scoped producer review or these 62 tests.

The moving native branch also inherits earlier pending F3 root ancestry. It must
not be landed as a provisional runtime/values carrier. Final native adoption waits
for the reviewed, whole-gated, assigned canonical F3 node through an announced app
base; unassigned new calls remain unavailable and older peers use real limits23.


## Base 13 consumer continuation

The announced base `2ebeedf9` supplies the canonical L cached-message generator
and the reviewed W12 small ToolbarButton. The options cog now uses that same
renderer with adaptive glyphs, ExtraMuted ink and its admitted disclosure state;
the existing captured connection/modal guard and shared popover lease are intact.
No area geometry or tooltip override is added. Refresh loading, delayed tooltips
and the footer profile still await their own checked shared producers.

L also confirmed there is no exact u64 count API: Fluent's f64 number path would
lose integers above 2^53, while current Number count/decimal inputs are u32-bounded.
Older reset-duration/token count Text paths are provisional, not locale-number
parity. ADR 0101 tracks exact unsigned counts separately from finite amounts.
Keep original values; do not clamp, drop, add a formatter here or make a lossy
cast. The accepted numeric fresh-age messages remain a distinct bounded path.


The first combined base-13 consumer strict run (`c124a1740`, native strict 12)
passed font preparation, then stopped on six test-only default-trait-access
lints in Usage view fixtures. Use the named DTO defaults without altering any
values or assertions; the same pattern in the card fixture is corrected in the
same mechanical change. No tests or full native gate ran in this strict check.


Source preflight also found two Usage-owned render functions just above the
100-line bound. Extract the card's account/freshness footer and the legacy
window's meter body into private pure rendering helpers, preserving values,
styles and admitted callbacks. The queued strict-13 source was superseded only
while still queued, with its cancellation record preserved; one changed-source
replacement checks the complete consumer. No allowance or size record is raised.
The remaining preflight finding is the base's large wasm refusal dispatcher;
F3 adds only its typed Unsupported registration arm there. It is not compiled by
the native strict target, so a wasm check remains a separate obligation.


The pinned menu trigger toggles its open state (menu-root.tsx:87). Usage options
now captures the rendered Opening for a close intent: it validates that exact
lease and domain before removing the owner pair or acknowledging shared focus.
A stale A close cannot close B, a stale open cannot replace an existing B, and
an obsolete domain pair is withheld before deferred shared retirement. No new
generation allocator, focus engine or renderer is introduced. Final consumer
capture must exercise open/close/reopen and the stale-opening case; shared
producer tests alone do not establish this consumer interaction.
