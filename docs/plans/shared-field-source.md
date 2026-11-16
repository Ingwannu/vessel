# Source-aware field callbacks

Purpose: retain a real editor's identity when a form callback can outlive the
form visit that created it. Reuse TextField, its existing engine subscription,
Editability, selection/IME behavior and plain callback signatures. No second
counter, event engine or input renderer is added.

FieldId wraps the actual input engine EntityId. TextField::id and the opt-in
TextField::with_source constructor expose the same typed source on change and
submit. A consumer records its current form's FieldIds and rejects old ones;
FieldId alone does not grant session/network authority or identify a form visit
if a consumer deliberately reuses the same editor across visits. Existing new,
with_change, composer and form_multiline callers remain unchanged.

This isolated producer is based on announced dd13ce37, not the A8 branch. It
copies exactly the field source already checked at6e97f1e32 (all-target native
Clippy PASS) and62c17767 (Kit128/0/0 including the real keyboard source test).
Those are prior-source evidence, not an exact isolated-head gate. An exact Kit
narrow is pending; no consumer-ready review is claimed here.

G2's FormMinimum source is deliberately not included while unreviewed. The
multiline source-aware opt-in is reserved to G2 after this minimal producer's
handoff: it must reuse events::Callbacks::Identified/listen and the existing
TextareaState, preserve FormMinimum::Default in every initializer and retain
Enter/Mod+Enter behavior. Do not add another callback or field-identity system.

B2 base7 union: the checked FieldId/with_source source is retained exactly, including the identified callback signatures and engine source-id path. Only the plain submit extension point accepts an immutable captured Fn boxed once; the same existing listener wraps its single Callbacks::submit invocation in the field-local SubmitPress scope. No extra subscription or replacement identity is introduced. Composer height, retained Entity, Editability and IME registry remain shared. The canonical source-identity GPUI test is unchanged byte for byte. This combined source still needs its own VM check; the base gate is not proof of this union.
