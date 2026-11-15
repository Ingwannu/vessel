# Scoped composer constructor producer

[Decision Log]
- Purpose: a retained session composer must submit through its captured resident and claim after pane moves.
- Existing assets and constraint: the shared `TextField` owns one engine, one event subscription, `FieldId`, `Editability` and multiline factory; the resident registry owns identity and local presentation. A plain function pointer cannot capture that identity.
- Alternatives: another editor/event model or selected-global lookup would duplicate ownership or submit to another session.
- Choice: widen only `Plain.on_submit` to one boxed immutable closure and the four existing plain factories to accept `Fn`. Identified callbacks, change callbacks, IME registration and factory bodies retain their canonical paths.
- Impact: one closure allocation per field; existing function items remain accepted. No extra engine, subscription, protocol, dependency or selected-route fallback. Windowed callbacks from the separate appearance owner must be preserved when combining.

This first leaf contains only the callback control and its captured-Enter regression. It excludes composer height policy, submit-press metadata, send settings, Markdown, attachments and live resident activation. It is prepared on the announced native base `6ebdf8e8`; the source is not an exact standalone compilation or execution pass. The broader composer predecessor contains the same callback design but its evidence does not approve this new combination.


## Resource construction leaf

Only `SessionView` and its constructor move to `session/resource.rs`; the parent reexports the same type. `Open`, `Pending`, lifecycle/follow and submit implementations remain byte-for-byte on their existing paths. The selected-session product constructor allocates its one existing callback field and delegates to `with_composer`; the latter never allocates another field. The restore slot is `pub(super)` so the parent retains its existing access.

The opt-in `new_scoped` remains literal `cfg(test)`. It captures the original resident token and claim, refreshes only that same local identity/claim through the sole registry at submit, and requires the current Visible and Focused state. It never substitutes a selected route, re-admits identity or refreshes network settlement. The real keyboard regression retains one field across moves and refuses hidden/released claims, while preserving the sibling draft. This is bounded staged construction, not product layout synchronization, real backing admission, stream hydration or authority. The canonical registry/minter remains staged; W12 owns their coordinated cutover.

The retention/snapshot adapter and submit-press metadata remain separate follow-ups. This minimal constructor has no dependency on the B2 surface or send modules. No recorded size ceiling is raised. Exact combined VM verification is outstanding.

The existing ListState initial `800` height moves with its constructor; its one literal exception transfers from the parent (3 to 2) to the resource child (1), preserving the total. The parent size record falls from the announced base's 720 to 686. Consumers with an already smaller parent retain that lower record and resolve their other owner extractions; this leaf never raises an existing record.
