# Shared secure field producer

ADR0102 is committed before implementation (coordinator ed1d1f71). Parent c81
was the separate Code/MD producer, now V1 source/narrow-passed, not a full F1 branch.
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
At the initial authored checkpoint none had run and TestWindow lacked a11y_init.
The later checked change11 and exact7bfe execution below address framework-tree
observation only. Native pixels/clipboard, OS IME/AT and consumer secret.set/current
form/context/authority remain separate. No Host Voice API/route is enabled.
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

## Executed prior source and mandatory receiving baseline

Exact7bfe6f1a has author-verified locked Kit all-target strict0 and full Kit
lib252/0/0 in schedules-20261116T022428Z-7bfe6f1a and022919Z-7bfe6f1a.
All30 run files, exact specs/checkout and four step digest prefixes match;
actual Kit Checking/Compiling and all five secure cases executed ok. The real
frame had PasswordInput/label/no value, the full snapshot omitted the canary,
and the ordinary reference remained present. Headless clipboard/cut/paste/
read-only, redacted diagnostics and blank/set/erase/source identity cases ran.
No raw snapshot or canary was printed. V1 precheck19703 remains source-only;
final privacy adoption still needs native proof and receiving-source checks.

Lead then announced green package base14.1 b54abbdc (1755/0/2) and explicitly
requires secure actual proof on that baseline. It is merged as063bf5801:
README retains BOTH secure and segmented contracts. Canonical composition::line
now registers the SAME masked engine once instead of the old Known registry;
masking still occurs before installation/listen. All other incoming source is
preserved. No new field/event engine or counter exists. Exact receiving Kit
strict/lib results remain owed;7bfe counts are not relabelled current evidence.

The old capture-only29f5c1a6 build/export succeeded0 after an authorized priority
run and queued-only143 predecessor. Its compiler reported two unused builder
return warnings in audit-only OpenOptions setup. No pixels or native clipboard
were collected. Keep that old source/export evidence; it is NOT the new-base
privacy proof. The revised audit must use this receiving union and consume the
builder results correctly, without allowances or duplicate source painters.
