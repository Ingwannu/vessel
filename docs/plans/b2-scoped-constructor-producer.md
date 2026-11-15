# Scoped composer constructor producer

[Decision Log]
- Purpose: a retained session composer must submit through its captured resident and claim after pane moves.
- Existing assets and constraint: the shared `TextField` owns one engine, one event subscription, `FieldId`, `Editability` and multiline factory; the resident registry owns identity and local presentation. A plain function pointer cannot capture that identity.
- Alternatives: another editor/event model or selected-global lookup would duplicate ownership or submit to another session.
- Choice: widen only `Plain.on_submit` to one boxed immutable closure and the four existing plain factories to accept `Fn`. Identified callbacks, change callbacks, IME registration and factory bodies retain their canonical paths.
- Impact: one closure allocation per field; existing function items remain accepted. No extra engine, subscription, protocol, dependency or selected-route fallback. Windowed callbacks from the separate appearance owner must be preserved when combining.

This first leaf contains only the callback control and its captured-Enter regression. It excludes composer height policy, submit-press metadata, send settings, Markdown, attachments and live resident activation. It is prepared on the announced native base `6ebdf8e8`; the source is not an exact standalone compilation or execution pass. The broader composer predecessor contains the same callback design but its evidence does not approve this new combination.


## Optional synchronous press-control leaf

This control-only branch takes the Plain Fn prerequisite and adds the existing SubmitPress value/RAII scope around the sole callbacks.submit invocation. Both real factories initialize one shared Rc slot per engine; identified callbacks and IME/source registration remain unchanged. A caller copies Enter/ModifiedEnter synchronously into its existing typed action before returning. Buttons and later work see None; nested callbacks restore the outer press, and Drop clears the scope on unwinding. No second listener, callback signature or send manager is added.

This branch excludes SessionView, Surface, ButtonOnly, measured layout, preferences, any new delivery policy and App wiring. Existing native runtime source d068 passed212 Kit tests with the same SubmitPress module/test blobs, but standalone strict and this exact root assembly remain unverified. Preserve any owner Windowed callback variant when combining, wrapping the same common submit call rather than broadening edit/focus/blur.


The actual minimal fb83 execution reported a leading newline before retained text on Enter. Unlike the full composer renderer, the bare canonical composer did not stop Enter bubbling after its shared engine callback. The control cut now applies the same existing form/full-composer parent action stop to the bare composer; the engine, field, subscription and submit callback remain singular. The original literal text assertion is retained and must pass on the corrected source. No input trimming, assertion relaxation, layout policy or separate submit listener is introduced.
