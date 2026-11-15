# A5: existing node inbox and push boundary

## Ownership and scope

The integrated node inbox and account-free Web Push are the existing N1 base.
The attention/notifications app package owns its new node gaps, including
recipient claims and terminal attention, as well as its app implementation.
This audit must not start a second implementation of those gaps. The app
owner controls the A5 tracker rows; this node audit does not mark them passed.

References: the attention sections of `parts/A-lifecycle.md`, notification
routing/presence/push/attention sections of `.area6-parts/B.md`, terminal
activity in `area-4-terminals-scripts-browser.md`, and their notification and
workspace connections. The detailed original source comparisons and deliberate
differences remain in ADRs 0040/0041 and `backend-m12-inbox.md`.

## Available integrated client contract

The integrated calls are protocol 22; consume the node line's assigned numbers,
not the original track's provisional number or its whole staged-diagnostics
branch. Existing `NodeClient` methods are:

- `inbox(ListInbox)` returns a bounded descending page and transactional
  whole-node counts, with an exclusive cursor for older pages.
- `mark_inbox(key, read)` changes shared read attention for one exact source;
  it is not an answer or a bulk consume of future entries.
- `inbox_entry(key)` cheaply confirms the exact current source after a lost
  reply or page movement; absence is answered/withdrawn, not duplicate success.
- `inbox_request(key)` recovers one actual live form independently of retained
  history. Over-limit forms stay explicit compact references; choices are not
  silently shortened.
- `push_key`, `subscribe_push`, `unsubscribe_push` manage the caller device's
  browser Web Push capability and lease, never another device supplied by id.
- `report_push_presence` reports a real activity/focus observation for this
  connection; the node stamps its time. It does not return an exclusive OS
  notification recipient claim.

Answers reuse the existing operate-scoped session permission/question calls.
Recheck current entry, connection and authority at submission; keep submitting
until the live projection confirms resolution. Two devices do not both gain
successful answer ownership. Read-only devices may read/mark shared attention
but cannot answer. Agent origins, including delegation, cannot mutate personal
inbox read state or register push capabilities.

## Semantics the app must preserve

Node-local cursors cannot be globally compared. Fold machines with their node
identity and entry key, and use authoritative counts rather than inferring
all waiting sessions from the first page. Stored archive/read marks are
inherited inside delayed append transactions; answered/withdrawn sources and
archived sessions do not recreate stale push attention.

Push eligibility follows target presence, not socket existence: recently
active visible target suppression, otherwise local delivery when a client is
present, and push only when none is present. Error outcomes do not push.
Registered endpoints and lease identity are rechecked at send admission;
late gone responses remove only the attempted lease. Scope loss/revocation,
read/answered state and renewed leases are not inferred from registration time.

## Deliberate differences and remaining package gaps

- Push is node-owned VAPID Web Push with encrypted generic payloads. There is
  no owner-provided Expo/FCM/APNs project, and no phone subscription UI or real
  push-service delivery proof is implied by the API.
- No title, body preview, session id, path, prompt or file content crosses OS
  or push notification text. Clients reconnect to obtain authorized details.
- The subscription lease is renewed by registration. Paseo heartbeat renewal
  differs; the app must not assume reporting presence alone renews this API.
- The existing session presence API is not a terminal attention API. The app
  package owns that extension and the single-recipient claim. New methods or
  request variants use the canonical metadata/version contract after its head
  is announced; no second policy/transport or dummy version is introduced.
- Desktop routing, sound, settings, badges, toasts, navigation, deferred read
  clearing and platform capability states remain the app package's acceptance
  work. Earlier node evidence is not a pass for those surfaces.

## Evidence and workflow

The original inbox/push node checkpoint `dc9c6e8d` received final node review
and was integrated with final protocol/schema assignments. Its evidence and
known limits are preserved in the inbox plans. Later integrated node gates
cover the same base; this audit changes no source and needs no unchanged
whole-gate rerun.

Use one package gate/review when the actual package is complete, with security
changes retaining their per-commit review. Never prompt a real agent, use the
preview node or claim live OS/phone delivery from fake-node tests. The current
node gaps must be coordinated with the app package's owner, not duplicated.
