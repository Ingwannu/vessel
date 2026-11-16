# Secure-field native audit surface (not a product route)

This branch is a control-only audit derivative of secure producer7bfe6f1a.
Never merge its replacement App main into the product. No node, book, updater,
persistence, real credential or Host Voice controller is initialized.

[Decision Log]
- Purpose: ADR0102 requires observed Linux mask/copy/cut/paste/read-only evidence
  in addition to real framework accessibility nodes and headless actions.
- Existing assets: public SecretField, ordinary TextField, the same Kit init/
  font/assets, GPUI native clipboard and the approved native-build/export path.
- Alternatives: a plain input, copied field painter, external clipboard helper,
  or main product startup. These either bypass the control or add unrelated
  state/authority to the probe.
- Choice: one explicit temporary App main renders two SecretFields (Absent/Set)
  and an ordinary nonsecret copy reference. The real native key path performs
  paste, copy/cut and read-only attempts. Only closed observations/counters
  reach the status file; neither candidate nor clipboard content is formatted.
- Why/cost: an independently pinned source/binary can be inspected without real
  services or secrets. Its layout and words are audit scaffolding, never F1/
  Host Voice UI or source-parity evidence. Existing field chrome is unchanged.

The synthetic canary and clipboard sentinel are public test data compiled into
the probe, never real keys or command-line inputs. Blank/set states load no
stored value. An ordinary-copy reference must succeed before a preserved
sentinel can qualify the masked copy/cut observations. Read-only changes are
counted, then a real submit after release verifies the original candidate is
retained. Erasing produces Unchanged, not deletion. Debug checks expose flags
only. The status file is mode0600 on a fresh private evidence path.

Use only the approved remote native-build with export-app on the regular track
label after the secure Kit suite; no local Cargo, new label or extra whole.
The external audit driver checks the exported binary SHA, starts its own Xvfb
with an authority cookie (never -ac), fresh home/runtime directories and the
development account, and records process identity before PID-owned cleanup.
Both schemes at pointer/compact widths show blank/set, masked replacement,
copy/cut, read-only and erased states. Captures/logs must contain no canary;
inspect original PNGs, retain failed evidence and distinguish inspection from
V1 acceptance. The native clipboard witness is not OS AT/IME or secret-store
proof. Real PasswordInput/no-value is tested separately through change11.

Current status: prepared source, no compile/export/run/pixel/clipboard PASS.
No actual key, production service, deployment, consumer acceptance or receiving
line gate is claimed.

## Required receiving baseline

The first29f5c1a6 native build/export succeeded0 in the one authorized priority
run after its queued-only143 predecessor. Its two unused OpenOptions builder
return warnings are corrected by using the same mutable builder fluently;
permissions, paths and observations do not change. No old binary was launched.

Lead then announced b54abbdc base14.1 and required actual secure proof there.
This audit merges the receiving secure producer b69703e8 (including b54),
preserving its exact Kit/GPUI/manifest/lock source. The App main remains solely
the explicit audit scaffolding. Prior7bfe Kit252 and old29f5 export are retained
as PRIOR-source evidence; current receiving Kit checks and a NEW same-source
export/capture are owed. No allowance, new dependency, painter or whole gate.
