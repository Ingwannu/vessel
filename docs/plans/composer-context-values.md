# Shared composer context values

This isolated source exports composer::context for history, files and other producer intake without depending on native byte storage or uploads. The files are identical to the attachment area's context values at the source recorded in the local handoff metadata. Existing Name, WorkspaceRoot, Drafts bounds and thiserror/serde dependencies are reused; no wire registration or dependency is added.

[Decision Log]
- Purpose: producers can validate immutable ChatHistory/Payload snapshots without merging the entire composer feature stack.
- Alternatives: duplicate a history store or fabricate another context DTO; both break the single intake contract.
- Choice: keep the canonical module paths and validators in this value-only bridge; screens and transport remain with their owners.
- Effects: ContextText permits up to the existing1MiB draft budget, and ContextTray enforces aggregate bounds, exact revision settlement, suppression and immutable snapshots. Values alone do not establish UI/OS input, durable storage or delivery behavior. Narrow checks and the final feature review remain separate evidence.
