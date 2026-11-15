# Shared window-aware field notifications

This portable source adds FieldEvents/on_edit/on_focus/on_blur and
TextField::with_events to the canonical Plain/Identified callback dispatcher.
The same line factory/listener owns editing, IME, history and registration.
Focus flags change before hooks; blur reads current engine text. Engine
Change may repeat current text on native commit, so consumers validate
current values. Existing callbacks/defaults remain intact.

It contains no Appearance36 profile, font, selection, editing state, numeric
range, or second production subscription. Standard Form32/44 remains the
caller's presentation. This is not an origin/visit guard: request forms use
the existing canonical FieldId/with_source contract.

The with_events method body is byte-identical to source, moved into the
events child so the base9 root stays within budget. Preserve newer W12
SubmitPress RAII around the ONE common submit call and W13 Plain boxed Fn
when merging their independently checked changes. Never restore an old root.

UNVERIFIED portable assembly. Producer current Kit tests are queued; prior
full package strict/build passed but package tests were red. No standalone
gate, final package, UI or platform acceptance is claimed.
