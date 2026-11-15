# Scoped composer constructor producer

[Decision Log]
- Purpose: a retained session composer must submit through its captured resident and claim after pane moves.
- Existing assets and constraint: the shared `TextField` owns one engine, one event subscription, `FieldId`, `Editability` and multiline factory; the resident registry owns identity and local presentation. A plain function pointer cannot capture that identity.
- Alternatives: another editor/event model or selected-global lookup would duplicate ownership or submit to another session.
- Choice: widen only `Plain.on_submit` to one boxed immutable closure and the four existing plain factories to accept `Fn`. Identified callbacks, change callbacks, IME registration and factory bodies retain their canonical paths.
- Impact: one closure allocation per field; existing function items remain accepted. No extra engine, subscription, protocol, dependency or selected-route fallback. Windowed callbacks from the separate appearance owner must be preserved when combining.

This first leaf contains only the callback control and its captured-Enter regression. It excludes composer height policy, submit-press metadata, send settings, Markdown, attachments and live resident activation. It is prepared on the announced native base `6ebdf8e8`; the source is not an exact standalone compilation or execution pass. The broader composer predecessor contains the same callback design but its evidence does not approve this new combination.
