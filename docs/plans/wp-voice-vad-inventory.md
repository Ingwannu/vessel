# Minimal VAD Phase A inventory (ADR0080)

V3 row221 approved quarantined acquisition/inventory only on docs commit
0a6b0e641. Six anonymous downloads, each with a parent-enforced60s whole
operation and at most one official HTTPS redirect, matched exact published
size/SHA256. No implicit retry or fallback artifact was used.

The quarantine is private: owner UID/GID1000, directory0700, files0600.
No system/service permission or account was changed. No model/native library
was loaded, no archived code/build script/installer was executed, no archive was
extracted, and no manifest/lock/distribution/readiness was changed.

## Results and remaining blockers

| Artifact | Result | Members | Expanded member bytes |
| --- | --- | ---: | ---: |
| Silero v4 | opaque size/hash only |0|643854|
| Linux ORT CPU1.28.2 | inventory complete;2links rejected for layout |41|25678250|
| Windows ORT CPU1.28.2 | stopped before member26 payload: expanded-size cap | incomplete |16958054 prior to refusal|
| ort2.0.0-rc.15 | inventory complete |89|720995|
| ort-sys2.0.0-rc.15 | inventory complete |25|325653|
| libloading0.9.0 | inventory complete |27|143648|

Windows member26 is `onnxruntime-win-x64-1.28.2/lib/onnxruntime.pdb`,
declared408244224 bytes. That exceeds the approved268435456-byte expanded
archive cap before any of that member is inflated. The cap was NOT raised.
Its payload was not inspected or hashed; remaining Windows member/notice
correspondence is incomplete. The first private progress log's generic expanded
summary fallback used the compressed archive size on this refused row; the
structured refusal details above and committed inventory are authoritative.

Linux aliases `lib/libonnxruntime.so` and `lib/libonnxruntime.so.1` are
symlinks and remain rejected, never extracted. The actual versioned regular
library24277040 bytes has SHA256
088f24b1fc56714d3efaaeb3ac2ee486a5d7b50ccfbb3dd26fd1a612534a05fb.
Its regular providers-shared library14632 bytes has SHA256
086ec1d5388f64153d9c63470d126693db9a182c8ce236d3a1119068471b8a0d.
These are inventory digests, not load/ABI/operator tests.

Linux LICENSE/full ThirdPartyNotices and both ort/ort-sys licence files match
the previously pinned Vessel notice bytes exactly. libloading's ISC notice
also matches. libloading's source archive includes two prebuilt test DLLs;
they were only streamed for a digest, never loaded or executed, and are
explicitly excluded from any product layout. Root3rd-party source notices are
not a substitute for unfinished Windows notice correspondence.

Exact public [acquisition](../evidence/voice-vad-phase-a/acquisition.json)
and [member inventories](../evidence/voice-vad-phase-a/inventory.json) contain
only official artifact/member names, digests, sizes and notice comparisons.
There are no signed redirect queries, credentials, private paths or binary
bytes. Quarantine evidence additionally pins the downloader/parser/test sources.

The parser checks ZIP EOCD member count before allocating directory records,
then streams central/local records and bounded decompression. Tar headers and
PAX/GNU metadata are parsed with byte/member limits before body allocation.
Six synthetic guard cases passed: excess ZIP count, traversal, PAX/notice/body
caps, rejected links, bounded deflate digest and UTF8 path length. A refusal
continuation preserves the Windows cap finding and inventories the independent
small crates under the same bounds. This is parser evidence, not Rust gates
or real detector/device quality.

Next: return the rejected-link and Windows-PDB findings to V3. A proposed
metadata-only Windows continuation must be reviewed before bypassing any
member's expanded-size admission; no automatic cap increase or partial
product adoption follows. Full runtime layout, Cargo feature proof, loader
privacy/failure isolation, inference compatibility, resource/platform tests
and final voice activation remain unapproved.

## Approved continuation result

V3 accepted only the exact PDB metadata skip and new bounded metadata totals
on docs commitda77d7d18. The [completed continuation](../evidence/voice-vad-phase-a/windows-continuation.json)
has34entries,4646central bytes,425969843declared bytes; other payloads total
17725619actual inflated bytes. The408244224B PDB remains uninflated, unhashed,
payload-CRC-unverified and excluded from every product/load plan. Its local
header/path/type/method/encryption/bounds were checked. The405504B separate
providers-shared PDB received normal CRC/SHA checks, with no extra exception.

Windows licences differ from the source raw bytes. Bounded full-payload
[comparison](../evidence/voice-vad-phase-a/windows-notice-reconciliation.json)
found exactly21/6121 CRLF pairs in LICENSE/ThirdPartyNotices respectively,
zero bare CR and byte-exact source contents after only CRLF-to-LF conversion.
No prefix/first-line match is substituted for full correspondence. Five new
synthetic guard cases passed; source/output hashes are retained privately.

The initial refusal stays authoritative for the first policy/run. This
continuation does not approve extraction into product, dependency manifests,
native/model loading, inference, final file layout/distribution or activation.

## Next proposed gate: positive private layout

ADR0080's Phase B proposal and `voice-vad-phase-b-layout.json` select only
regular runtime candidates, the common opaque model and complete notices/
metadata with exact member hashes. It requests private extraction/static import
inspection plus a fixed node dev-dependency build proof on the VM, **no native
or model loading**. No part has run. The fixture handoff into build labels
also needs an agreed owner path because the gate has no general external
binary fixture import API. No runtime/distribution permission is inferred
from completed Phase A evidence or this proposed file list.
