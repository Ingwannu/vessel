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

## Receiving checks and native proof submitted for final review

Exact b69703e8 on announced b54abbdc has locked Kit all-target Clippy0 and
full Kit lib276/0/0 in schedules-20261116T024942Z-b69703e8 and
schedules-20261116T025302Z-b69703e8. V1 independently accepted the bounded
receiving source and requested narrow results (01945c30); the final privacy
handoff remained held for native proof. The five secure cases include observed
same-frame PasswordInput/label/no-value and whole-snapshot canary absence with
an ordinary exposed-value positive reference. No old252 result is relabelled.

The separate control-only audit derivative76d67699 uses exactly this Kit,
GPUI subtree, manifests and locks. Its only source differences are temporary
App main and its audit plan; that main MUST NEVER merge into the product.
Authorized schedules-20261116T030431Z-76d67699 completed fonts/build/export0,
without compiler warnings. The exported App SHA256 is
5c473b8a96f6608b975155ef741ead5007d183b97e5c6b4b4bc6f0e0be833fb0.
It initializes no book, node, updater, credential store or consumer route.

The first native attempt failed before input or PNG at window focus. The next
showed correct masking/actions but its requested1280 width was clipped to1278
by the virtual screen. Both records are retained; neither substitutes for the
final dimensions. The corrected authenticated-display run uses a larger
virtual screen and asserts actual PNG dimensions. The author opened all28
original phase PNGs: light/dark at exactly1280x780 and390x780, each with empty/
Set, masked replacement, copy/cut, read-only, erased and Set-replacement states.
Four additional nonsecret startup-frame witnesses are retained. No PNG is
painted, composited or edited by the audit driver.

All four actual native clipboard sequences report ordinary-copy success as a
positive control, masked Copy and Cut preserving an unrelated sentinel, Paste
and submit of the exact synthetic replacement, zero read-only edit callbacks,
retained replacement after read-only release, redacted Debug and Unchanged on
erase. Only closed observations reach the evidence file. Synthetic data alone
is used; original capture/log files omit the canary. A real stored key is never
loaded. This is Linux/Xvfb native clipboard evidence, not other-OS clipboard or
physical input/IME evidence.

The App and authenticated Xvfb run as the development account; authority files
are0600, evidence parents0700 and files0600, executable0500. A client without
the cookie is refused. PID/executable/start identities are recorded, all owned
App/display processes are verified stopped and both authority files removed.
No service permission or deployment changes occur. A nonsecret custody JSON
created with a wider mode inside the private parent was narrowed to0600 before
packet verification; the helper now uses a private creation mask.

The immutable private review packet pins source, build/export, binary, driver,
all original PNG/log/observation hashes and cleanup metadata. This section
records author inspection and submission, NOT V1 final privacy acceptance.
No Host Voice consumer, secret.set integration, authority, OS accessibility,
physical IME, zeroization, full F1 or receiving-line gate approval is claimed.

## Final bounded review receipt

V1 same-row7d33547c closes the native/privacy handoff hold for selected
b54..b69703e8 plus documentation leaf1fa30e6a9. Its independent review verifies
the requested receiving Kit strict0/full276, exact authorized native build/
export and binary, all32 original image hashes/dimensions (23 distinct images
opened, nine byte-identical duplicates), four real native clipboard sequences,
framework secure-node/no-value proof, private permissions and process/cookie
cleanup. Prior failed setup/clipped-width records remain excluded from final
geometry proof. Coordinator2f17e2a0 separately opened dark1280 replacement and
light390 blank/Set and passed the controlled visual comparison.

The exact selected producer was handed to the lead; audit main is excluded.
W19 received the API and must consume only an announced green receiving line.
The receiving union must preserve newer Plain boxed-submit constructors and
other owners' field refinements. This documentation receipt changes no Rust
and does not extend the reviewed source range or request an unchanged rerun.
Normal receiving gate and actual Host Voice integration remain separate.
No secretstore/persistence/authority/full F1/whole-test/OS AT/physical IME/
hardware-GPU/other-OS/cross-App clipboard/zeroization approval is claimed.
