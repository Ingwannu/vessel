You are one of four surveyors documenting Paseo 0.15.4 (source: ~/refs/paseo-latest, read only) for a Rust port.
FIRST read ~/refs/notes/paseo-features/SURVEY-BRIEF.md and follow its per-feature format exactly (one ## per feature, ### sub-features,
file:line refs, exact names from code, protocol message types and fields, constants, defaults, "not found + where looked" instead of guessing).
The whole area is AGENTS AND SESSIONS (agent session life and conversation). It is split across four surveyors; cover only YOUR slice below,
but note connections to other slices. App folders: packages/app/src/{agent-commands,agent-controls,agent-presentation,agent-profiles,agent-skills,agent-stream,timeline,tool-calls,subagents,provider-selection,create-agent-preferences,stores,screens}; server: packages/server/src/server/{agent,creation,session*,agent-attention-policy.ts,agent-history-search.ts,lifecycle-reasons.ts,message-receipts,migrations}, packages/server/src/services; also packages/protocol/src (messages.ts etc), packages/client/src/daemon-client.ts, packages/cli.
Skip test files except to confirm behavior. Write your output incrementally (save every few features) to the file named in your slice. 
Finish with `## Inventory` (flat list, one line per feature/sub-feature) and `## Cross-area connections` for your slice. Final reply: only the file path and feature count (2 lines).
