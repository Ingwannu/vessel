# Composer catalog values / projection carrier

Standalone on canonical node ede20014; no runtime/probe/native-policy or RPC registration.

[Decision Log]
- Purpose: W23 and W22 need one checked AgentCommand metadata contract without importing the B2 draft-probe/runtime package.
- Existing assets: AgentCommand and protocol projection already own command reports; Native values start from dd06, with the mechanically identical b7b4 lint correction (root_name and explicit Unicode boundary division).
- Alternatives: duplicate metadata/command kinds; merge B2's whole node branch; infer execution from labels.
- Choice: one dormant composer_commands value namespace, canonical kind/native_command fields, the existing direct/event/forward projection paths and constructor None adaptation only.
- Why: every old-peer route masks both optional metadata fields under COMPOSER_COMMANDS=None, including native-only reports; ordinary command words/order and usage projection remain unchanged. No provider advertises anything new.
- Costs and effects: this carrier includes typed draft configuration/catalog DTOs but no Request impl or method row; B2 registers the pending query in its runtime package. Kind/native values are shared, not a second registry.

Native command values in6988 are byte-identical to b7b4e87e; strict Goal decoding retains the dd06 behavior and serialized shapes. Status derives kind; original command text and same-client-id acceptance remain W23's receipt owner. Current-run offers from W22, not persisted Conversation.commands, authorize pre-admission; goals also require proven launchflag and autonomous root lifecycle. No actual provider fills native_command here.

Paseo references: app client-slash-commands/index.ts:49-69 and hooks/use-agent-autocomplete.ts:231-244; server agent/providers/codex-app-server-agent.ts:226-242,4125-4141,5143-5159,5166-5287. ADR0085 owns catalog metadata; ADR0084 owns receipts/actor acceptance.

Scope: values and old-protocol projection only. Primary tests cover metadata omission/unknown kind, assignment masking, direct snapshots, events, records, legacy logs and nested forward envelopes. Native objective/parser/status/strict-decode behavior is retained. Exactce50 command-filtered tests passed10/0/0 in run `wp-composer-catalog-values-20261115T123619Z-ce50e604` (including one incidental workspace URL test). Exact6988 proto/agent/session all-target Clippy with TS and -Dwarnings passed0 in run `wp-composer-catalog-values-clippy-20261115T125259Z-6988f24b`. Metadata source/test blobs are unchanged between these heads; the intervening native-value lint correction is documented separately. These are scoped test/compile results, not activation, source review or a whole-package gate.
