# Shared window-aware field notifications

TextField::with_events(placeholder,on_submit,FieldEvents,window,cx) delegates
to the ONE existing line factory/listener and Plain/Identified/Windowed
dispatcher. FieldEvents has optional on_edit/on_focus/on_blur function hooks.
Text hooks receive current String plus Window/Context; focus receives Window/
Context. Focus flags update first. Blur reads current engine text. on_edit
follows engine Change, including same-value native commit notifications.

There is no extra production subscription/editor/history/selection state,
font, Appearance profile, numeric range or origin counter. Existing signatures
and no-extra-hook defaults remain. Standard Form32/44 remains unchanged.
Request-scoped forms still use canonical FieldId/with_source for stale-visit
guards; these callbacks do not replace that authority boundary.

The constructor now belongs to events before its test module, with exact
signature/body equality to the earlier root method. Preserve W12's checked
SubmitPress RAII around the ONE submit dispatch and W13's Plain boxed Fn on
future union; never restore an older whole root.

Evidence scope: producer e8051e72e exact Kit lib228/0/0, run
themes-narrow-20261115T184906Z-e8051e72. Prior2c170f575 whole strict/build
passed and App99/0/2, but its package tests were red; the corrected full F5
gate and UI review remain pending. This portable assembly has not had an
independent native run. Consumer must check its own union; no standalone
PASS, V1/full F5/UI/OS/hardware acceptance is claimed.
