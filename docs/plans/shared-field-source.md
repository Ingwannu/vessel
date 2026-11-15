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
Those remain prior-source evidence, not an exact isolated-head runtime run.
Exact dcbdbb018 now has V1 bounded SOURCE ADOPTION and requested Kit Clippy PASS:
wp-side-narrow-20261115T120940Z-dcbdbb01 ran clippy --locked -p vessel-app-kit
--all-targets -- -D warnings, fonts/cargo0 with actual changed Kit compilation
and no skip. Audit: /tmp/work/m12-review/w4-field-source-dcb/audit.json.
Packet: /tmp/work/wp-side/field-source-handoff.json. This is the approved minimal
consumer source; it is not a whole A8, stale consumer, multiline or IME pass.

G2's FormMinimum source is deliberately not included while unreviewed. The
multiline source-aware opt-in is reserved to G2 after this minimal producer's
handoff: it must reuse events::Callbacks::Identified/listen and the existing
TextareaState, preserve FormMinimum::Default in every initializer and retain
Enter/Mod+Enter behavior. Do not add another callback or field-identity system.
