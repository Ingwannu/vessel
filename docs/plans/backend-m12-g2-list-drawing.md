# F1 list drawing checkpoint

[Decision Log]
- Purpose: expose real schedule rows and their already wired mutations through
  the shared controls, without embedding fixture data or formatting in paint.
- Constraints: View owns body/filter/permission/action decisions; Link owns
  bounded requests; controls need actual initialized state, never an Option
  unwrapped by a menu callback.
- Alternatives: area-local controls, lazy UI state with a panic in a callback,
  raw transport messages, or a fixture switch in the installed application.
- Choice: always initialized UI state beside a separate testable answer/cache
  boundary. Prepared comparable rows own shared text. Real node replies settle
  the original ticket; shared renderer menu/selector/Alert/badge are passed the
  View's meanings. Cache preparation runs at data/language/timer updates.
- Why and cost: no fake data path or paint-time RPC/system lookup/formatting.
  More small area files keep each responsibility and function readable.

References read whole: Paseo schedules-screen.tsx1–455 (header/body/filter/
empty/loading/load-error/HostErrors), schedules-table.tsx (one card/delete),
schedule-row.tsx1–end (leading16x20, title14, target/meta12, mt4, gap12, trailing
badge/menu gap8); hosts/host-filter.tsx (nonsearchable, bottom-start),
utils/confirm-dialog.ts (whole warning/title/destructive meaning), and
hooks/use-time-ago.ts plus utils/relative-time-ticker.ts (metadata updates,
not periodic schedule/list calls). Reference source is packages/app/src.

Shared data/render split: data.rs retains the existing correlated answer path;
state.rs prepares text/cache and stable control identities; controller.rs owns
arrival, requests and explicit activations; draw/row/filters/overlays draw only.
The legacy sidebar slot delegates one line to the area using the lead's nav_row
seam. S2's canonical sidebar inventory may replace that adapter; placement and
full sidebar parity remain S2/lead owned. The desktop toggle stays in titlebar.

Words: new list/feedback/relative-time copy uses generated schedules.ftl; accepted
cadence/product/action/status/editor errors still call text/schedules.rs, never
copied into the new catalog. Scope refusals use their one accepted complete
sentence. Targets/paths are separate elements with appropriate faces, and long
row text wraps rather than introducing a new ellipsis.

## Current limits, not excused differences

This is an immutable source checkpoint for narrow compilation, not a complete
F1 screen/package or a capture/UI PASS. Creation/edit/history and their entry
buttons are not yet wired and are not exposed. The requested documentation link
and command-center registration still belong to F1 completion. The current
minute metadata timer was an initial implementation; a following source change
now implements the reference tiers/static-idle cancellation, whose tests and
actual frame behaviour are still unverified. Viewer-zone discovery is
not wired yet, so the honest explicit UTC fallback is shown; machine/rule zone
identities are never guessed. The node's newly available self-scope report is
also not consumed yet; unknown authority still offers actions and remembers an
actual Forbidden per connection. These are remaining work, not parity claims.

HostErrors currently consumes the historical producer returned for radius12;
G4's corrected radius8 handoff is required before final gate/captures. No area
radius override exists. Menu focus correction has pending narrow/actual-input
verification; selector/dialog close focus and removed-row overlay cleanup also
need actual checks. Existing shared official mark adapter is reused, no vendor
artwork drawn here.

Registration delta: Areas constructor; Route variant/label/render/sidebar/arrive
and own-area leave callback; two no-composer/no-session shortcut arms; sidebar
slot delegate (non-const signature/used arguments required); Shell Schedules
arm and Fleet hook; language cache rebuild hook. No generic sidebar styling,
titlebar, drag, composer or other screen body is changed. Final F1 package owns
one whole native gate and inspected node-backed captures, not another whole
gate for this checkpoint.


## Relative-time task follow-up

The reference ticker uses minute/half-hour/hour tiers for minute/hour/day ages,
wall-clock aligned; a week-old absolute date is static. View now returns those
typed tiers and the fastest visible tier. Active next-run countdowns opt into
minute updates even if both past ages are static, to keep the requested future
label current. One cancelable weak area task replaces the initial detached loop;
leaving, empty/static-only rows, or a tier change cancels the old task. Identical
prepared content does not notify a frame. New tests cover thresholds, before-
epoch alignment, static paused rows, active countdowns and empty filters.

The Fleet hook now dispatches initial reads only when book keys change; unrelated
session refreshes may refresh target display but do not poll schedule/list when
its cache happens to pass five seconds. Arrival, real reconnect, mutations and
explicit retry retain their existing read paths. There is no second node timer.
These changed-source tests and task/frame behaviour are not yet a PASS.
