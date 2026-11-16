# ADR 0102: one secure single-line field

Status: reserved design, implementation and V1 adoption owed.
Owner: G2; coordinator ed1d1f71. First consumer: Host Voice API-key entry (W19).

[Decision Log]
- Purpose: enter a replacement credential without displaying a stored credential
  or exposing new input through diagnostic, clipboard or accessibility surfaces.
- Existing assets: the canonical TextField line factory, FieldId, Editability,
  InputState/listen and gpui-base's masking and clipboard capability checks.
  InputAccessibility supports PasswordInput and explicit value suppression.
- Alternatives: a plain text field, an area-painted mask, a second text engine,
  or loading the existing secret into a masked editor. These create disclosure
  or divergent identity/input behavior and are rejected.
- Choice: a typed secure wrapper around the same one-line engine, always masked,
  with no reveal operation or raw TextField/value getter. Secret event payloads
  use a private-value carrier with redacted Debug and no Display. Conversion to
  the existing secret.set value occurs only at the caller's explicit boundary;
  the kit does not persist, serialize, log or infer a secret name/authority.
- Why: rendering bullets alone is not a privacy boundary. Copy and Cut stay
  disabled in engine capabilities and actual action dispatch; Paste is allowed
  under the same editability/focus/paste-target checks as ordinary fields.
  Accessibility uses PasswordInput, a localized label, no value, and no value
  exposure even if a future presentation changes its mask.
- States: Absent is an empty replacement input. Set renders an already-localized
  set-status without loading the stored value. A person's edit creates only a
  replacement candidate; erasing it returns to no replacement, not a deletion.
  Explicit secret deletion remains a separate owner action. Existing credentials
  are never passed to this control. Placeholder/status are never saved as input.
- Impact: all ordinary field defaults, source identity and input-method path stay
  unchanged. The wrapper retains existing Editability; current form/context and
  authority remain the consumer's responsibility. Existing-value status and real
  secret input are never conflated. No zeroization or protection against a
  malicious process, screenshot length inference or caller misuse is promised.

## Required proof before handoff

Narrow exact Kit strict/tests and V1 source review. Real keyboard actions must
prove Copy/Cut leave both input and sentinel clipboard unchanged, Paste enters
masked input, disabled/read-only refuses edits, and source FieldId remains the
actual engine identity. Inspect the actual accessibility node for secure role
and absence of value. Synthetic canaries must be absent from Debug/Display,
rendered text/accessibility and any capture/log packet; never use live keys.
Show blank/set/replace/erased states in both themes. Mark OS IME/AT coverage and
consumer secret.set/persistence/authority separately; headless tests are not an
OS or Host Voice acceptance claim. No capture may contain an exposed value.
