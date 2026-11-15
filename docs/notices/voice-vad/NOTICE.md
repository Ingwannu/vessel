# VAD candidate notices — not a shipped component inventory

These notices accompany the ADR0080 minimal-VAD acquisition proposal. No
model, native runtime or inference dependency is installed, linked, loaded or
distributed by this directory. Archive-content correspondence is still owed.

- Silero VAD v4, Silero Team: MIT, `Silero-v4-MIT.txt`. Upstream v4 source
  commit915dd3d639b8333a52e001af095f87c5b7f1e0ac. The proposed model is
  k2-fsa's published16k-only export, unmodified by Vessel. Conversion provenance:
  [k2-fsa PR2372](https://github.com/k2-fsa/sherpa-onnx/pull/2372), merged
  bebd996f934525926ea20370bcac7f17dd62d312. Preserve the upstream MIT notice
  and k2-fsa's Apache2 source/conversion attribution (`Apache-2.0.txt`); a
  repository licence is not proof of every release member's provenance.
- ONNX Runtime1.28.2, Microsoft Corporation: MIT, `ONNXRuntime-MIT.txt`,
  plus the complete pinned source `ONNXRuntime-ThirdPartyNotices.txt`. The
  latter is an upstream source-wide set, including optional vendor components.
  It is not evidence that all those components are present in either CPU
  archive. Inspection must reconcile the actual archive notices/file list;
  unmatched or additional terms block adoption.
- ort and ort-sys2.0.0-rc.15, pyke.io/Nicolas Bigaouette: MIT OR Apache2.
  Their pinned source MIT files are byte-identical (`ort-MIT.txt`); Apache2
  is also retained in `Apache-2.0.txt`.
- libloading0.9.0, Simonas Kazlauskas: ISC, `libloading-ISC.txt`.
- Existing locked smallvec1.16.4, cfg-if1.0.5 and windows-link0.2.1:
  MIT OR Apache2. Their MIT notices are retained separately. They were read
  from already-cached crate archives only after matching their full published
  archive SHA256 and size; no new archive was downloaded.

`source-manifest.json` records exact notice bytes/digests and source URLs.
Third-party notice files remain byte-exact, including upstream whitespace.
This collection changes no licences and grants no permission beyond upstream
terms. A final distribution must include the reviewed notices next to the
actual runtime/model, independently of an About-screen link or online URL.

This VAD-only proposal contains no Kokoro/espeak-ng data, phonemizer, local
ASR/TTS model, Sherpa shared library or provider credential. Those remain
excluded from this acquisition and require their own reviewed amendment.
