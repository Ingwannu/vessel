# Staged attachment resident resources

[Decision Log]
- Purpose: protected attachments must move once into W12's sole SessionResources, without a second session map or global clone/swap.
- Existing assets: the current Tray, ContextTray, Waiting/final Arc, Origin leases, capability state, intake and recovery facts already express ownership. The application-wide immutable byte Store/serialized writer and window lightbox remain services.
- Alternatives: copy the global composer into each pane, snapshot only filenames, or refresh connection identity on retirement. These lose source lifetime or can resend stale input.
- Choice: a literal cfg(test) module owns the concrete per-session fields, canonical readiness Holds and a moved snapshot. FollowRetirement matches the exact resident/binding, Attachment namespace and opaque operation ticket before retaining uncertainty; it does not fabricate a transport failure or clear/retry input.
- Tradeoffs: this is an isolated staged owner kernel. It does not move any product field, register another resident manager, add an RPC or change authority. W12 alone adds the bundle field and promotes all owner/global/follow paths together after checks.

The constructor receives an already-admitted SessionContext only for resource identity. Snapshot refusal returns the same protected resources. A clean snapshot restores only to the same registry and authenticated address, with the receiving context's incarnation; it grants no connection or follow permission. Retained context includes suppressed entries. Unknown outcomes and unconfirmed recovery hold before pending/local-loading/overlay/dirty checks. Debug output has counts/readiness only, no paths, bytes or text.

One canonical Attachment operation ticket covers the immutable submission; progress still requires current W12 receive admission plus existing message/item/upload checks. The actual queue-to-write guard remains G4/W12's prerequisite. This adapter cannot determine a reliable wire-dispatch fact from the App phase, and never claims delivery authority from the constructor or snapshot.

The minimal source dependencies are W12's announced c342 foundation and the canonical scoped B1 input owner types, including the W23 ClientMessageId route. This leaf is prepared against the frozen composer source so the concrete types are real; root files are unchanged and canonical batch13 adoption is still required for a clean-line consumer. No fresh source-review, narrow execution, populated-resource UI, hydration or product approval is claimed before recorded checks.

The initial component is session-resident scoped. Workspace draft payloads remain under their existing canonical DraftKey ownership until W3's owner adopts the same moved component; this leaf does not create another draft or session registry. The first tests exercise actual payload Arc identity and byte leases, suppressed context, holds, wrong binding and retirement, plus clean same-registry snapshot transfer. They remain unexecuted until the VM checks complete.

The first exact kernel strict run wp-composer-resident-20261115T161201Z-3f6bb97b reached App tests and refused the intentionally partial Debug formatter. It now ends with finish_non_exhaustive, making omitted fields explicit while keeping diagnostics free of paths, bytes and text. No fields or lint exceptions are added to the log; tests did not execute after that strict failure.
