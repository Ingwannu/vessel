# Explorer host integration

Status: the metadata projection is literal test-only preparation. The existing
Explorer dock still renders through its legacy workspace owner. This is not a
typed host mount, root admission, renderer or visibility approval.

## Decision Log

- Purpose and intent: give Files and Changes the actual selected Explorer tab
  from the same layout book used by ordinary panes, without fabricating TabId.
- Existing assets and constraints: `Layout::explorer` already retains its pane
  and focused tab; `PanelContext` is the sole renderer context. The workspace
  owner's existing dock retains width, scrim, edge and content controls. Its
  legacy tab selection must cease being authoritative in the typed host.
- Alternatives: copy the dock, infer a tab from the active session, or expose the
  private book. These duplicate presentation ownership or allow guessed identity.
- Choice: `Shell::layout_with_explorer(focus, render)` copies only the selected
  tab metadata, releases the book borrow, then lends the existing PanelContext
  to one synchronous callback. The callback cannot retain a borrowed book guard.
- Why: an ephemeral projection supplies the real LayoutKey, Explorer PaneId,
  TabId and Target without another stored map, controller or generation issuer.
- Benefits, costs and effects: owners can consult the book during rendering.
  The small metadata copy is per visible projection, never a resource clone or
  transcript copy. The context establishes no authenticated node/root scope.

The current helper uses the explicit legacy Session layout key only. An absent
entry produces no body and allocates no layout; initializing remembered entries
is not creating a workspace. Hidden Explorer and a covering compact sheet also
invoke no body. Hiding preserves the original tab and resources. The host must
provide the actual focus policy rather than treating visibility as input focus.

The workspace owner will extend its existing dock with one content callback.
For the actual mount, header selection and tab clicks must use the same current
layout target, and hide/show must use canonical ExplorerVisibility. Files and
Changes then dispatch through the sole panel registry with this actual context.
Until that union is installed, `layout_panel_visible` continues to exclude the
unmounted Explorer. A helper test cannot establish painted bounds or polling.

File content requires its original root receipt, connection and scoped buffer
binding; Changes requires the git owner's corresponding admission. Neither may
derive a node/root from an alias, cwd or this presentation snapshot. Deferred
callbacks keep their own original context/tickets and validate again at dispatch.

The requested checks cover the real book's selected tab, released borrow, hidden
retention, sheet suppression and absent-key behavior. Actual dock rendering,
header selection, callback/retirement ABA, focus and owner effects require the
later consumer union.
