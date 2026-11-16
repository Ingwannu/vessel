# Shared field notifications

TextField::with_events(placeholder,on_submit,FieldEvents,window,cx) uses
the ONE line factory/listener and Plain/Identified/Windowed dispatcher.
Optional on_edit/on_blur receive current String, Window and Context;
on_focus receives Window and Context. Focus flags update first; blur reads
current editor text. on_edit follows engine Change, including same-value
native commit notifications. Old signatures/no-extra-hook defaults remain.

No Appearance36/Number64/Width80, font, numeric range, origin counter,
second production subscription or editing state is added. StandardForm32/44
is preserved. Existing FieldId/with_source remains the stale-visit guard.
The constructor body is unchanged in the events child before its tests.
Preserve W12 common SubmitPress RAII and W13 Plain boxed Fn on later union;
never restore an older whole field root.

Scoped evidence: source c30229ff1 Kit/View/App all-target strict0,
themes-narrow-20261116T003505Z-c30229ff. Product code previously passed
Kit228/0/0 at e805; the test helper split retains all assertion expressions
and input/park calls (only the wildcard is made exhaustive). Full F5 gate,
UI and V1 approval are pending. This portable assembly has no independent
run: receiving consumers must check their union and preserve newer seams.
The earlier base9 39b47/a821 exports are superseded, not silently certified.
