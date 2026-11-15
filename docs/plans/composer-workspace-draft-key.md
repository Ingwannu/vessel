# Canonical workspace composer draft key

Status: value contract staged; native editor/attachment adapters are a coordinated cutover.

[Decision Log]
- Purpose: two draft tabs must not share New text, context, byte snapshots, creation tickets or persistence records.
- Existing assets: the workspace owner's WorkspaceId, the layout owner's DraftId and authenticated NodeId. No allocator or registry is added here.
- Alternatives: global New, cwd-derived keys, another draft UUID or an alias-only node. Each loses authority or independent resource identity.
- Choice: DraftKey::Workspace { node, workspace, draft } references those canonical values; existing New and Session forms remain unchanged. Serde retains the same tagged representation for old keys.
- Why and effects: equality, persistence and lookup isolate node, workspace and draft independently. Creation Ticket is local to one editor, so preparation/completion must correlate by (DraftKey,Ticket), keeping the same canonical ClientMessageId through retry.

The draft editor owner publishes workspace_draft_field(&LayoutKey,DraftId) and workspace_draft_context_scope(&LayoutKey,DraftId); a missing authoritative root yields None. Attachment code never falls back to global New or a selected session. History can seed an admitted empty editor before a file-root scope is available, while file/plugin intake still requires authoritative scope. This value-only bridge does not establish live editor maps, resident resource access, create transport, attachment rendering or UI admission. Its native compatibility adapters and owner accessors must land together before app activation.
