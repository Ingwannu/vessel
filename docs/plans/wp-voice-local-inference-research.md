# Local inference candidate audit (ADR0080, 2026-11-15)

Research status only: no selected runtime, dependency manifest, inference/model
execution, default readiness or distribution approval. Existing candidate
sherpa-onnx2.0.1 remains unselected. No asset is copied from Paseo.

## Verified upstream facts

- [Upstream issue3731](https://github.com/k2-fsa/sherpa-onnx/issues/3731)
  and the2.0 changelog record removal of espeak-ng/piper-phonemize. The safe
  Rust wrapper's data_dir is explicitly deprecated/ignored from2.0; callers
  need a model lexicon or external phoneme codepoints. Existing default TTS
  compatibility cannot be assumed.
- [NVIDIA's Parakeet model card](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v2)
  declares CC-BY-4.0. This does not attest the exact converted ONNX release
  archive or replace required attribution/source/change notices.
- [Kokoro0.19 publisher card](https://huggingface.co/hexgrad/Kokoro-82M/blob/6530b5f49e11e904fa65ddcab90e2e95b9a94243/README.md)
  declares Apache2 for that model release. This does not establish the license
  of every bundled phonemizer-data file.

## Archive evidence in isolated scratch

GitHub release API metadata was fetched from the official k2-fsa repository;
full JSON selections and inventories stay in isolated research scratch.
No weights were extracted or loaded; archive member names/bytes and only small
README/LICENSE files were read by a traversal-checking audit.

| Artifact | Asset id | Archive bytes | SHA256 | Evidence |
|---|---:|---:|---|---|
| kokoro-en-v0_19.tar.bz2 |281054203|319625534|912804855a04745fa77a30be545b3f9a5d15c4d66db00b88cbcd4921df605ac7|downloaded, exact size/hash verified;398 members,369315617 unpacked file bytes|
| parakeet v2 int8 tar.bz2 |719763786|482440461|6fc4426b54aa549a0019b8a75b8dea59164e71790a37ac56a045ad57f915f714|API metadata only, not downloaded/hash-attested|

The current Parakeet archive size differs from Paseo's catalog482468385;
release asset metadata says it was updated2026-11-07. No weight-level
identity/quality inference is made from the stable model name.

Kokoro has model.onnx345555491B, voices.bin5755904B, tokens.txt1078B,
README135B, one root Apache2 LICENSE11358B (SHA256
cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30),
and391 espeak-ng-data members totalling17991651B. No lexicon member or
separate espeak notice was found. The root model license is insufficient
proof to distribute the entire bundle or to claim2.0 phonemization works.

Previously audited2.0.1 shared native candidate archives (Linux9710131B,
Windows7899712B) contain no bundled license/third-party notice files. Their
pinned source/build recipe, ONNX Runtime version/third-party inventory and
correct notices remain owed. No sys build-script automatic fetch is enabled.

## Next acceptance requirements

Choose a demonstrably compatible phonemization path and audit its own code/data
licenses; pin source plus native artifacts/notices/digests; verify model file
allowlist/layout/hash and bounded download/extraction; amend ADR0080 for V3
before adding an inference dependency. Linux/Windows linking, model loading,
ASR/TTS output and physical AEC/audio quality require separate evidence.

## Follow-up source inspection (2026-11-15)

Pinned sherpa2.0.1 source commitaf2d8d81dbfa61c4cea4c568da78f3982edca005
selects ONNX Runtime1.28.2 in its Linux/Windows CMake recipes. This source pin
is not yet a binary-level attestation of the bundled shared libraries, and their
full third-party notices are still absent from the inspected native archives.

The official external lexicon asset6305773B (SHA256
8b67ae549a465eee78f92ecb74644ad90c3c58ce65f19c68baf85878a10ef73a)
uses `word(s) || phones`, and its header identifies build_lexicon.py with
CMUDict+en_list+compounds. It must not be attributed to the older Kokoro v1.0
Misaki gold/silver generator merely because that script also produces an English
lexicon. Exact asset-source and notices remain owed. Misaki's current pinned
fba1236595f2d2bf21d414ba6e57d25256afada3 Apache2 LICENSE and JSON data were
read separately; they do not establish this release asset's provenance.

No inference dependency/model was selected, linked or executed. The coordinator
was asked to confirm ownership/slicing of this prerequisite while endpoint and
fake-driven capture/playback/UI work continues. LocalNotBuilt remains explicit.

Coordinator ruling10:3x: local speech runtime/distribution remains W19's later
slice after endpoint B4, with no second owner. Before any further model/native
runtime download, shipment or load, submit an ADR0080 amendment to V3 naming
exact archives, hashes, licenses/third-party notices and a Vessel notice file.
The Kokoro espeak-ng data requires explicit GPL handling or a different model;
it is not approved by the root model's Apache notice. Previously fetched
research candidates remain isolated scratch; none was linked, loaded, shipped
or enabled as a product download. No further candidate/model download is made.

Endpoint STT/TTS still needs a real reference-compatible turn detector: the pure
Silero boundary is not an inference engine. A minimal VAD/runtime amendment must
precede that activation too. Until then endpoint mechanisms can be tested with
typed detector fakes, but neither endpoint success nor those fakes advertise
working conversation voice.

## Minimal VAD metadata comparison (2026-11-15 11:58 UTC)

No new model/native-runtime artifact was downloaded for this comparison. Hashing
Paseo's already-present reference asset (pinned app source1c00b365) yields643854B
and SHA2569e2449e1087496d8d4caba907f23e0bd3f78d91fa552479bb9c23ac09cbb1fd6.
The official k2-fsa asr-models release API advertises exactly the same size/digest
for silero_vad.onnx. Its [VAD documentation](https://k2-fsa.github.io/sherpa/onnx/vad/silero-vad.html)
and [PR2372](https://github.com/k2-fsa/sherpa-onnx/pull/2372) identify this629KiB
file as k2-fsa's16k-only v4 export, not the newer2.22MiB v5/v6 model. Switching
to current upstream Silero bytes would change the reference model. No asset
was copied into Vessel, parsed with a model runtime or loaded for inference.
The exact converted-model attribution still belongs in the reviewed amendment.

A smaller-scope alternative to the full Sherpa runtime is the safe Rust ort
wrapper plus CPU ONNX Runtime, retaining the reference model and implementing
Sherpa's internal100ms speech/200ms silence decisions as a separately tested
adapter before the existing800ms/1000ms outer boundary. Direct probability
thresholding alone does not establish parity. This alternative is unselected.

Official release/crate metadata, not binary-content attestations:

| Candidate | Declared archive bytes | Published SHA256 |
|---|---:|---|
| onnxruntime-linux-x64-1.28.2.tgz |9128991|d7209b8751b27b862b0c76332c2e20e203396edb5dab700ecf4bb485cf147415|
| onnxruntime-win-x64-1.28.2.zip |78620837|c4eedd29489d5feca21866d054638416f3655bf6b18851b3b6b85c8313e95c35|
| ort2.0.0-rc.15 crate |168000|37c87aa7a90b7ff3e0fe36cccd475df8d67f15f07f4d6c370c4b38464232813c|
| ort-sys2.0.0-rc.15 crate |59054|8d22b0417f1838db5a3ac0b3f98dfe39f6fb73624aa0a27c6bbb5040b9c7db2b|

[Microsoft release metadata](https://api.github.com/repos/microsoft/onnxruntime/releases/tags/v1.28.2)
pins source33ca9628233dc8f002435e868d4c2e9f82766ca1. The
[ort](https://crates.io/api/v1/crates/ort/2.0.0-rc.15) and
[ort-sys](https://crates.io/api/v1/crates/ort-sys/2.0.0-rc.15) records declare
MIT OR Apache-2.0 and Rust1.92. Their default feature set is unsuitable: it
includes automatic binary download, TLS, copy-dylibs and a newer API floor.
A proposal would explicitly disable defaults and select only std/load-dynamic
and the pinned runtime API floor, with an exact installer-owned library path,
no environment/system-library fallback, explicit CPU execution and no model
fetch feature. Transitive selection/licence inventory, loader failure behavior,
exact archive file lists/notices and linked/loaded size remain unproved.

The [runtime privacy document](https://github.com/microsoft/onnxruntime/blob/33ca9628233dc8f002435e868d4c2e9f82766ca1/docs/Privacy.md)
says official Windows builds enable platform telemetry by default; the
[wrapper builder](https://docs.rs/ort/2.0.0-rc.15/ort/environment/struct.EnvironmentBuilder.html)
exposes explicit telemetry disabling and a logger hook. Any proposal must turn
telemetry off before session creation and suppress foreign model/runtime log
payloads, then prove absence of unintended egress rather than assume CPU means
offline. The runtime's top-level MIT licence does not replace its full
ThirdPartyNotices.txt. This comparison is not the required V3 amendment or
permission to download, ship, load or add either runtime dependency.

A proposed minimal-VAD acquisition amendment and candidate notices now live in
ADR0080 and docs/notices/voice-vad. They are not approved acquisition/load or
distribution. Only official text/metadata and already-cached Rust dependency
notices were read for this preparation; no new model/native artifact was fetched.
