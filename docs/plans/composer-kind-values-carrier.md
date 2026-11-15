# Composer command-kind values carrier

[Decision Log]
- Purpose: native composer catalogs need the canonical distinction between root commands, inline skills and unknown future kinds without importing a private node implementation into the app line.
- Existing assets: the reviewed ADR0085 enum and masking source, existing AgentCommand/SessionEvent/Conversation/Reply projection paths, Serde and ts-rs. Node93970acb is the required receiving baseline.
- Alternatives: restore private RPC/runtime ancestry, introduce a local parallel kind DTO, or keep a missing symbol behind a fake fallback. Those break one-contract ownership or hide the actual dependency.
- Choice: add only CommandKind, optional AgentCommand.kind and the existing peer projection. COMPOSER_COMMANDS stays None. All current constructor sites supply None; no discovery, classification or transport behavior is enabled.
- Benefits/costs: native source can compile against one reviewed value shape after integrator/base adoption; the wire remains unchanged while unassigned. The required constructor-only field in the ACP mapper changes no driver behavior.

Scope excludes method tables, RPC rows/handlers, node runtime, provider driver behavior, native_command metadata, native intent/status/receipts, quotas and protocol assignment. The unknown enum variant never enables inline completion. Legacy reports decode with no kind. Direct/nested snapshots, events, EventRecords and legacy log projection mask pending metadata while retaining command order/text and existing usage projection.

Reference: Paseo0.15.4 composer autocomplete differentiates command and skill in hooks/use-agent-autocomplete.ts and utils/agent-autocomplete.ts; the native survey area-2-composer-input.md section3. ADR0085's full discovery/producer remains a separate node package.

Required current-parent evidence before adoption: VM root cargo build -p vessel-node --bin vesseld; proto all-target strict with TS; actual metadata projection tests; generated TS bytes and equality; V3 source review. Prior isolated6988/b7 results are not this carrier's proof. Integrator lands the exact carrier in order, and native consumes only an announced App base. No provisional private App root source is restored.
