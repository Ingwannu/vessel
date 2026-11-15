# Canonical workspace composer draft key

Status: native attachment adapters staged; owner creation and selection consumers remain coordinated and unverified.

[Decision Log]
- Purpose: two draft tabs must not share New text, context, byte snapshots, creation tickets or persistence records.
- Existing assets: the workspace owner's WorkspaceId, the layout owner's DraftId and authenticated NodeId. No allocator or registry is added here.
- Alternatives: global New, cwd-derived keys, another draft UUID or an alias-only node. Each loses authority or independent resource identity.
- Choice: DraftKey::Workspace { node, workspace, draft } references those canonical values; existing New and Session forms remain unchanged. Serde retains the same tagged representation for old keys.
- Why and effects: equality, persistence and lookup isolate node, workspace and draft independently. Creation Ticket is local to one editor, so preparation/completion must correlate by (DraftKey,Ticket), keeping the same canonical ClientMessageId through retry.

The draft editor owner publishes workspace_draft_field(&LayoutKey,DraftId) and workspace_draft_context_scope(&LayoutKey,DraftId); a missing authoritative root yields None. Attachment code never falls back to global New or a selected session. History can seed an admitted empty editor before a file-root scope is available, while file/plugin intake still requires authoritative scope. This value-only bridge does not establish live editor maps, resident resource access, create transport, attachment rendering or UI admission. Its native compatibility adapters and owner accessors must land together before app activation.


## Native ownership adapter

The attachment area resolves the exact W3 editor and optional authoritative root. Menus use a bounded presentation-only cache per key; eviction can reset a menu, never its payload. The existing kit autocomplete and pill renderer now accept captured callbacks, so clicking a previously drawn row/pill acts on its original draft instead of whichever global field is selected later.

WorkspaceInput carries the source key, local creation ticket, canonical client message identity and expected node before preparation. Workspace holds are indexed by (DraftKey,Ticket), bounded with session sends, and transfer only captured file keys/context revisions to the created session. New remains on its existing API. Rejecting or completing an equal ticket from a sibling draft cannot release that hold. A context-only history seed does not infer a file root.

The active destination awaits the layout owner's canonical effective-target accessor, and W3 owns source propagation through creation replies. No workspace submit fallback is introduced. VM-only strict checks and the equal-ticket GPUI regression remain required; source staging is not runtime or UI approval.

Workspace history seeding does not focus its field: pre-layout and background admission preserve the layout owner's focus decision. Preparation capacity counts both New and workspace holds before reserving a live-session send.
