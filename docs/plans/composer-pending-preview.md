# Pending attachment preview ownership

[Decision Log]
- Purpose: show the attachment-only or image-bearing prompt immediately without pretending that a local byte source is a node AttachmentRef.
- Existing assets and constraints: Tray already owns validated names/media types and managed Origin Arc leases; ContextTray owns immutable source snapshots. The timeline ledger owns pending messages and exact canonical reconciliation.
- Alternatives: fabricate temporary remote ids, keep a separate preview catalog, or draw the mutable composer tray. These lose identity, duplicate lifecycle state or display a later draft's content.
- Choice: capture one immutable view::composer::preview::Snapshot before preparation and retain it in the timeline PendingMessage. It has only local file metadata/sources and visible context; no upload state or remote reference.
- Why and effects: capture performs no I/O. Drawing borrows the retained snapshot. Source leases survive composer clearing, accepted-queue delay and unknown outcomes. The canonical message/queue echo releases the ledger row; retry retains its original snapshot and canonical client id. Debug output reports counts and numeric sizes, never names, paths, text or bytes.

The source type and lifetime regression are staged until exact VM checks. The app renderer and keyed ingress must be adopted with the timeline owner's ledger consumer and checked protocol activation. This value does not authorize a second send or transport retry manager.
