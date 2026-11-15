# File close ownership in the sole layout dispatcher

Status: integration contract preparation. The File owner and opener are staged;
there is no product File close-dispatch or payload-retirement approval here.

## Decision Log

- Purpose and intent: let the File owner inspect its actual editor, pending read
  or write and unknown outcome before removing a File presentation. A cached
  `Modified::Clean` flag alone is not enough to dispose of an owned buffer.
- Existing assets and constraints: immutable ClosePlan captures the original
  target and tab; keyed completion rechecks both. The workspace File owner has
  the retained buffer, original root receipt, write tickets and shared dialog.
  The existing layout dispatcher owns keyboard, menu and sheet placement.
- Alternatives: duplicate File confirmation in layout; infer a buffer from the
  active session; trust cached Clean; or call the concrete owner before removal.
- Choice: the same dispatcher calls `request_layout_file_close(original_key,
  original_plan, window, cx)` before generic effect/modified handling, including
  Clean plans. `FileCloseRequest::NotOwned` permits generic handling only for a
  non-File target; `Handled` includes refusal, cancellation and pending dialog.
- Why: only the buffer owner can mirror accepted engine text and distinguish
  a safe local discard from an original pending or unknown remote effect.
- Benefits, costs and effects: no second dialog, close manager or registry.
  The async owner must retain the original key/plan and use keyed completion
  after a safe disposition. A successor target cannot be closed by an old reply.

## Required write-path inventory

- Keyboard and menu current/before/after/others/pane plans route each File plan
  through the owner before generic Clean/local checks. A handled refusal stays
  handled so the desktop CloseWindow fallback cannot discard the window.
- Both compact sheet close controls and backdrop dismissal use the same owner
  dispatch. They cannot call the pure `dismiss_sheet` directly for a File.
- Revealing an earlier compact sheet can remove later sheets inside the model.
  Before committing an open/reveal, compare the planned metadata with the
  original layout. Any implicit File removal needs the owner protocol, or the
  transition refuses without publishing that metadata. Postcommit notification
  cannot grant discard permission retroactively.
- Generic passive retarget is not an owner-confirmed File close. Until a concrete
  preview replacement protocol exists, replacing/removing an existing File via
  that route must refuse. Opening a distinct file may allocate its actual tab.
- Canonical preference replacement and acknowledged locator loss are external
  retirement/recovery events. They must preserve dirty/unknown owned payloads
  and invalidate UI admission; they never grant discard or remote cancellation.
- Keyed `layout_close_settled` is completion, not a user-facing request. Its
  trusted File caller uses it only after owner approval and releases the tab's
  buffer binding only after actual removal. Other tabs may share the buffer.

An automatic-removal preflight must not create a second book or persistent ticket
issuer. It may compare temporary metadata trees before the existing atomic commit.
An asynchronous dialog cannot be awaited inside the layout borrow. Conservative
refusal keeps the current tree and input while a supported explicit close settles.

## Required consumer evidence

The product consumer removes only the owner entry's required literal test gates
and installs its real caller in the same union. It must prove cached Clean with
new accepted text, dirty Discard/Cancel, pending/unknown reads and writes, stale
target completion, shared-buffer bindings and compact automatic removals. No
original remote attempt may be aborted, replayed or treated as resolved by a new
read, reconnect, discarded local text or a retirement notice. The caller retains
the original connection, root receipt and owner operation correlation.

The raw ClosePlan target getter and current-target query are presentation
snapshots. Neither proves an incarnation, UI visit, transport lease or close
authority. Actual owner tests and the combined source/gate remain required.
