# Native command values (B2 / shared with W23 and W22)

Status: proposed values only. No method, event, driver call or feature is activated.

[Decision Log]
- Purpose and intent: native `/compact` and `/goal` acceptance must use the existing client-message receipt pipeline without inventing a running turn.
- Existing assets and constraints: W23 owns canonical receipt/actor acceptance; W22 owns actual provider transport, launch/version capability and autonomous goal turns. B2 owns command catalog/parser/producer.
- Alternatives considered: ordinary model prompts, fake TurnStarted/Steered events, a second command submit API, freeform success strings.
- Choice: one independent `proto::native_commands` value module, usable before the rest of B2's catalog/probe package lands. Closed statuses derive kind and can be localized by clients. Original submitted text remains separate and is fingerprinted with the typed intent by the receipt owner.
- Why: the values have no runtime dependency, do not duplicate provider dispatch, and let the actor use the same parser before reservation. A real transport must still authorize the operation; parsing grants no capability.
- Advantages, costs and effects: GoalObjective has a documented64KiB UTF8 byte bound and rejects unsupported controls, unlike the reference's unbounded argument. It is redacted in Debug and never silently truncated. Status does not repeat objective text or provider diagnostics.

Primary reference: `packages/server/src/server/agent/providers/codex-app-server-agent.ts:226-242` (goal argument case/whole-match), `:4125-4141` (root parsing), `:5143-5159` (catalog) and `:5166-5287` (out-of-band dispatch). Compact ignores arguments. Goal keywords pause/resume/clear are case-insensitive exact whole arguments; every other nonempty argument is an objective. JavaScript trim/whitespace includes BOM and excludes NEL, so the parser uses that exact closed character set.

Joint contract agreed with W23: SubmitPrompt gains optional intent, original content and the same ClientMessageId participate in identity; native deliveryStart is a selector that never cancels, queues or starts the foreground turn. CommandAccepted carries original content and a NativeCommandStatus; the strong command receipt stores status+echo sequence. W23 owns those fields and common input/actor integration. Known unsupported capability is refused before reservation; ambiguous post-commit results remain unknown with no blind retry.

W22 goal availability additionally requires observed version>=0.128, a proven launch with the goal flag, and tested autonomous root-turn lifecycle. No availability is inferred from version alone. The composer will consume explicit typed native catalog metadata, not infer provider execution from a label.

Tests: exact names, Unicode/BOM/NEL boundaries, whole keyword matching, objective byte limits, redacted errors/debug, status-kind derivation and strict serialized shapes. Checks are pending on the build VM; no local Rust compilation, activation or parity claim is made. V3 joint wire review is required before activation.

Exact50c tests compiled with TS but failed3/1: Serde's internally tagged unit
variants ignored payload fields despite deny_unknown_fields. A private decode
adapter uses empty struct variants to validate the identical wire shape, preserving
the public unit variants and serialized contract. All no-payload goal variants
and invalid compact content now have adversarial fixtures. No assertion was
weakened; the corrected immutable head still needs VM test and strict lint proof.

Exact dd06 passed all four native-command value tests with TS enabled. Its strict all-target check found three lint defects: a documentation identifier, implicit integer division in a Unicode boundary fixture, and `name` colliding with the generated TS trait method. The successor uses the explicit `root_name` accessor and Euclidean division; JSON and intent validation remain identical. Exact b7b4e87e then passed TS-enabled proto all-target Clippy in run `wp-composer-native-command-values-20261115T125158Z-b7b4e87e`. Exact b7 runtime tests remain queued; dd06 four-test and ce50 carrier ten-test runs are prior-source execution evidence. No transport or feature activation is implied.
