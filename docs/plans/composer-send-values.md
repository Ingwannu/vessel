# Composer send action value

Status: source-only portable value leaf on packagebase7 plus the canonical surface prerequisite.

[Decision Log]
- Purpose and intent: the retained field callback captures Default or Alternate before returning, and the shared attachment/receipt ingress carries that choice through one submission.
- Existing assets and constraints: B2 already owns this exact Action enum. Kit SubmitPress exists only during its synchronous callback; W6 owns the one canonical message-id/snapshot/preparation path; W23 owns keyed actor delivery.
- Alternatives: reading window modifiers later, a second send pipeline, or importing the legacy UI capability fallback as the new wire policy.
- Choice: publish only the unchanged Action value at its existing module path. Existing submit_prompt will delegate Default to the owner's future submit_prompt_action entry.
- Why: no modifier/owner state is guessed after a callback or preparation await. This leaf has no legacy Delivery/SendFacts dependency and grants no send authority.
- Advantages, costs and effects: no new identity, queue, listener or dependency. Canonical delivery mapping and activation remain unavailable until the checked W23 SteerOrInterrupt/receipt/echo seam exists. The enum alone does not implement alternate send.

The value declaration is byte-identical to the existing B2 sending module. Consumers must preserve their canonical surface module and the later Surface.overlay getter on union. This leaf does not overwrite a consumer's policy helpers or activate transport. Its fresh VM check is pending; earlier pure model checks are prior-source evidence only.
