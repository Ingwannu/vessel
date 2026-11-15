# Panel owner retirement notifications

Status: contract proposal; no implementation or product admission is claimed.

## Decision Log

- Purpose and intent: notify a panel owner when its actual tab is closed or
  retargeted, before another local callback can execute. Polling current target
  equality cannot observe Scripts to another target and back between polls.
- Existing assets and constraints: the sole layout book and panel registry own
  placement. Features already own their UI instance, request tickets and retained
  payloads. They must not add another tab map, guessed epoch or layout listener.
- Alternatives: poll faster; invent feature generations; change stable TabId on
  every retarget; or notify owners synchronously from committed layout changes.
  Polling misses transitions, independent generations duplicate ownership, and
  changing TabId breaks the established draft/preview promotion contract.
- Choice: extend the same `areas::layout::registry` with typed owner hooks and
  borrowed retirement notices. There is no event channel, persistent ledger,
  new connection identity or per-feature allocator.
- Why: a committed transaction is the exact boundary at which the old panel
  owner's UI admission ends. Its existing instance/tickets fence later work.
- Benefits, costs and effects: immediate invalidation without polling. Every
  committed book-write path must use the observer, including canonical preference
  replacement. A feature still supplies its actual retirement transition.

## Proposed boundary

A closed `PanelOwner` set identifies the existing feature owners. Each owner
registers one noncapturing typed retirement function with the same panel registry;
duplicate registration refuses instead of silently replacing the first owner.
Registration carries no resource identity or mutable feature state.

`PanelRetirement` borrows the original LayoutKey, TabId and Target from the
pre-transaction snapshot. Its reason distinguishes retarget, local close and
external/unavailable replacement. The notice cannot authorize a backend stop,
archive, save, discard, snapshot release or new request.

After a successful transaction, the layout owner compares the previous and
committed targets. A surviving identical target keeps its resources; moving a
pane or retaining hidden content is not retirement. A changed/removed target
notifies its original owner after the book borrow ends and before the public
mutation returns or schedules publication/render. Failed transactions emit
nothing. Two committed retargets notify independently even without a render or
poll between them. An atomic transaction whose final target is unchanged never
published an intermediate resource and does not create a new visit.

Handlers run synchronously without awaiting, issuing RPC, reentering the layout
mutation or constructing admission. They invalidate only their own UI instance
and read/action tickets. Already-sent effects retain their original context,
payload and correlation and become uncertain when appropriate. A retirement
notice alone never deletes unsent text or an unknown write. Owner-confirmed close
remains a separate typed decision followed by the keyed ClosePlan completion.

Current target equality, visibility, original connection lease and operation
tickets remain independent required checks. This notification is neither a
replacement for DeliveryGuard nor an authenticated resource lifetime.

## Required checks

- Scripts to another target and back in separate transactions retires the first
  owner before any poll; an old instance/action cannot revive on return.
- Failed/dirty retarget emits nothing and preserves the existing instance.
- Pane moves, hidden retention and unrelated metadata changes preserve resources.
- Close and canonical preference replacement notify the original target once;
  a reentrant book borrow is not held during the callback.
- Unknown mutation outcomes retain actual owner payload and do not retry or clear
  a successor. A registered real owner, not a marker fixture, proves this state.

The current `layout_tab_matches` proposal remains a current-value predicate only.
It does not establish any of these notification or ABA guarantees.
