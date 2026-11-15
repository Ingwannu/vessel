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
