# Shared secure field producer

ADR0102 is committed before implementation (coordinator ed1d1f71). Parent c81
is the separately reviewed, execution-owed Code/MD producer, not a full F1 branch.
Only the same field line factory and Callbacks dispatch are extended: ordinary
lines choose Ordinary; SecretField chooses always-masked Secret with localized
PasswordInput label and value_exposed(false). InputState/listen/FieldId/Editability
remain the existing engine path. Secure callbacks convert text into a redacted
SecretEdit before it crosses the control boundary; no second subscription exists.

The facade's private TextField has no public raw field/value/reveal setter. Its
new input starts empty; stored presence selects only a localized placeholder.
Blank/erased replacement means Unchanged, never a delete. Typed SecretInput has
no Display/Serialize/Clone; into_value consumes it at an explicitly documented
secret-store boundary. This is a handling policy, not memory zeroization or a
promise against malicious caller misuse. No live key is used in tests/captures.

Authored tests cover redacted diagnostics, secure value-suppressed accessibility
configuration, real rendered-engine Copy/Cut/Paste and read-only behavior, actual
FieldId/clone/submit, and absent/set/edited/erased states in both schemes/layouts.
None has run yet. The test platform's PlatformWindow has no a11y_init override,
so policy/source inspection is NOT an observed accessibility tree or native AT
proof. Actual value-free PasswordInput tree/OS IME and consumer secret.set/current
form/context/authority remain explicitly owed. No Host Voice API/route is enabled.
No extra whole gate or new label/priority is requested. Exact Kit narrow and V1
source review are required before a bounded producer handoff.

## First exact narrow result and complete lint repair

schedules-20261116T010338Z-de055fdd ended fonts0/cargo101/helper1; tests did
not run. The reported batch is docs/backticks, Secure callback semicolons,
explicit cast instead of coercion, and equality assertion style. A test-only
Hidden<T> equality wrapper keeps exact comparisons while its Debug always
redacts; failed comparisons cannot print replacement/clipboard contents. No
assertion or lint is weakened. Product privacy policy remains the same.

Coordinator6bafe9e8 assigns G2 the two GPUI test-support files for change11;
lead6d5654b4 reserved the number. That producer is an isolated base13 leaf:
real adapter activation/defaults unchanged, observed debug tree and nonsecret
ordinary-value control. It must pass V1 and land on the line before this secure
producer counts actual accessibility proof. A control-only native build/export
and authenticated isolated display supply masked Linux pixels/clipboard proof;
no new label, extra whole run, real key or secret-store action is authorized.

## Real adapter/frame proof source

After V1 final033a1ceb and lead-confirmed source adoption60a08b99, this isolated
producer merges ONLY the checked change11 range2eb..31ed..87ad +docs51ef.
This is selected producer consumption, not adoption of an unannounced package
base or a claim that the receiving line gate passed. Field module registrations
unioned automatically; secure factory/events, Code/MD and all other children
remain intact. The main F1 consumer is a different, separately checked tree.

The new actual-frame case renders SecretField beside an ordinary editor with a
nonsecret reference value, activates the REAL callback, types only a synthetic
canary into the masked field and observes a real frame. It requires exactly one
PasswordInput with the localized label and no value; the ENTIRE debug snapshot
must omit the canary. The ordinary TextInput must expose its reference value,
so a disabled tree or decoder that drops all values cannot pass. No snapshot
or candidate is printed. Both themes and existing compact/pointer layouts are
covered by authored assertions; execution is still owed on this changed head.

The old de055 lint-red and d2a complete repair remain separate. Exact corrected
Kit strict/full lib, native masked PNG/clipboard and V1 final proof are still
owed. No OS AT/IME, Host Voice, secret store, authority or F1 approval follows.
