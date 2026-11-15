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

Context/history intake refuses a new occupied-slot overflow before calling the legacy LRU tray accessor. The seventeenth context-bearing workspace draft receives typed Capacity; existing siblings and their suppressed/pending context remain intact. File intake similarly refuses a new full tray instead of dropping a sibling.

## Reserved destination slots

Creation now reserves an additional byte/context tray slot before any create effect, because newer input may remain in the source while its captured snapshot transfers. New owners count those reservations during intake; empty remnants alone are pruned, while suppressed context and pending revisions remain protected. If every slot contains retained data, preparation refuses Capacity before the create call. A regression fills the remaining slots, adds newer source input, completes in the background and requires every sibling plus both source/destination to survive. Completion also refuses an already-owned session destination instead of replacing its pending submission.

The context-only pump uses a temporary empty byte tray when none exists. Merely submitting context cannot allocate a persistent byte slot or evict an unrelated file owner; its regression fills all byte slots and requires them to survive context-only dispatch.


## Scoped card and effective selection

The scoped card reuses the existing card, attachment menu, captured pills, file/drop handlers and IME-safe field hook. Its creation owner supplies the exact admitted key and editability; Enter remains with the original W3 editor. Attachment menu state now retains DraftKey rather than a New/session boolean, so menu selection cannot redirect its asynchronous file dialog to another composer. The legacy button API remains a wrapper while scoped owners pass an explicit key. B2 owns slash/fullscreen policy and its remaining workspace adapter.

Destination lookup consumes the checked layout effective-target getter: only an explicit selected Draft with a Workspace key or the currently owned Agent may receive input. Files, terminals, launchers, plugin panels and top compact sheets do not select a hidden draft. Root Workspace registry/provisioning and the owner renderer activation remain separate prerequisites; this source does not claim those runtime paths are active.

The bounded workspace menu cache may evict widget state, so its local query counter must not restart at zero. The composer now preserves one existing typed mention-ticket high-water mark across all of its menu observations and seeds recreated models from it. This is a local lookup counter, not a wire or resident identity allocator. An old reply cannot collide with the first lookup after A→B→A presentation eviction. Removed editors are checked before debounce/reply handling, and a click revalidates its exact destination and root before changing text. The pure regression recreates a model with the same token/scope and rejects its previous incarnation's late reply.

The canonical connection owner approved adding Hash only to its existing MachineKey, ConnectionEpoch and ConnectionContext values, so attachment cache keys can retain exact admissions. The non-clone epoch allocator, constructors, fields and serialization are unchanged. A HashSet regression requires clone equivalence and separate same-node reconnect/alternate-alias admissions without assuming different numeric hash values. Source permission does not approve the attachment transport adapter.
