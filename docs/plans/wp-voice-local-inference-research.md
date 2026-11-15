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
full JSON selections and inventories stay in /tmp/work/wp-voice/dependencies.
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
