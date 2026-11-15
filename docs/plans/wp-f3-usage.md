# F3 native usage presentation

The native area uses view::usage for pure state/formatting and link::usage for
correlated bounded node calls. Settings, the lazy sidebar modal and conversation
quota notice share those values; the node remains the sole source/report engine.
The shared kit owns controls, dialog/popover and meter rendering. Paseo references:
area-6 B.md:321–347; app usage model.ts:107–150,209–236 and card.tsx:37 onward.


### Query state and card ownership correction

Tracker rows 622 (states), 623 (platforms/connections) and 625 (card presentation)
now belong to this F3 package. The same Settings/modal reader uses authenticated
ConnectionContext and host feature support; accounting remains visible when only
external saved-login lookup is Off. Failed inventory reads expose an explicit
cache-only Retry, and known Off exposes the precise existing three-store
lookup disclosure with an Enable action. Enabling only changes policy and reads
cached state; it never grants additional sources or refreshes a vendor.

Recovery callbacks capture both connection and existing parent-modal ticket, so
an old retained action cannot apply to a reconnect or reopened parent. The
mutation also refuses overlap with a source-policy write. Individual typed login
failures now render translated closed reasons with their actual harness/public
account label instead of being discarded in favor of the generic summary.
No raw source diagnostic text is introduced. EN/KO projection checks are authored;
current combined compilation and actual UI checks remain owed. This does not
claim card geometry, freshness tooltip or complete row 625 visual acceptance.

### Report card composition and successful freshness

The generic report card now composes existing settings card/row controls: source
mark and label, optional shared plan badge, current-attempt status dot/text and
Refresh in the header; account label and successful relative freshness in the
footer. Existing approved marks fall back through the shared Generic/Bot path;
no new vendor assets are introduced. Typed per-login problems replace a generic
summary only when actually present. Failed reports keep their status and original
successful time even while values are retained.

Relative ages floor minutes/hours/days, use just-now under one minute, and derive
only from the successful observation. Minute cache ticks redraw them without an
external fetch. The shared Refresh tooltip shows the same freshness; current
reader observation tickets retire tooltip state/places, and policy/connection
retirement clears them. Opaque ids remain typed map keys. All header/refresh and
source-consent callbacks retain connection and parent-modal admission guards.
No new UI generation allocator or tooltip/card renderer is added.

EN/KO whole-message and time-boundary projections plus actual shared card-header
geometry probes are authored. The current delta is uncompiled; no screenshot,
consumer focus, strict generated-catalog or whole-package pass is claimed.

L's bounded wording review accepted the new EN/KO prose but required numeric age
arguments and per-key context. Age messages now use NUMBER($count); the caller
supplies the existing checked Number::count after a checked integer conversion.
Abbreviated units remain whole translated messages, so adding a future plural
selector does not change Rust argument types. This corrects the authored data
path without changing elapsed-time thresholds or accepted earlier Usage words.

### Landed shared header seam

Announced green line a7aab9d6 supplies W19's reviewed header-actions input on the
same dialog renderer and brings the announced root base through the app line.
Usage consumes DialogContent::with_header_actions: host filter, Refresh all and
Options now live before the existing close control, with no duplicate body
Refresh all in the modal. Settings retains its own refresh action. Unavailable
hosts keep only host choice; no-host frames leave the slot absent. Width remains
the existing 640px Wide layout. Callbacks retain their existing modal ticket,
ConnectionContext and selected node, separately from UI leases.

Refresh actions use the already landed RotateCw and typed shared button/loading
control, including disabled/loading admission and compact sizing. The same helper
serves report and whole-host refresh. The actual shared tooltip wraps that control;
it does not introduce another raw button. Header and consumer checks are owed on
this merged source. No first-frame capture or exact HostFilter-control geometry
approval is claimed.

The shared localization owner confirmed that no canonical typed currency/compact
amount formatter exists yet. USD currency, compact token balances and two-fraction
other units remain an explicit formatting gap; raw typed finite amounts and their
units stay visible, including values above Number's u32 decimal bound. The area
will consume the owner's producer rather than fork a formatter or silently drop
large values. Relative-age words at e89334984 are separately accepted as word/key/
context only; that acceptance does not cover number formatting or UI.

The older 3febf32e narrowed run returned RED before Rust tests: the catalog build
required a direct translator-context comment for usage-fresh-days. That exact
failure is retained. L's e893 correction already supplied direct comments for all
ten keys plus numeric arguments; the old run did not execute any card tests and
is not credited as runtime evidence. The merged consumer's actual narrowed check
must cover these fixes, the landed header and the shared typed refresh control.

Pin rows now expose the existing whole-row interaction as a checkbox with its
actual toggled state and source/window identity. The existing whole Pin/Unpin
words describe the action; no prose fragments or new translation API are added.
Both legacy and generic rows retain the current connection/parent-modal receipt
in addition to the already captured observation ticket, so an old modal row cannot
edit pins after a parent replacement with the same account observation. This is
source/typing work only; platform accessibility and keyboard traversal remain
unverified and are not implied by adding the accessibility metadata.
