# Shared bounded raster input and screenshot pixels

[Decision Log]
- Purpose: composer normalization, file previews and browser screenshots reuse one bounded codec path instead of separate decoders or upload stores.
- Existing assets: image0.25.10 is already linked by GPUI and pinned in the lock; the same six total codec features are reused. No root, wire or network dependency is added.
- Alternatives: trust filenames, decode independently in each screen, relax attachment limits globally, or expose unchecked policy numbers. These bypass the shared resource bound or change unrelated acquisition policy.
- Choice: kit::image_input::normalize validates encoded bytes, preserving static PNG/JPEG and normalizing the other four supported codecs or APNG first frame to capped PNG. Attachment defaults remain4096 pixels per side and64 MiB decoder allocation budget. screenshot::decode_png shares that internal path with PNG-only8 MiB encoded,8192-side and32 MiB final RGBA bounds before pixel decode.
- Output: opaque DecodedRaster exposes size in device pixels, a borrowed RGBA slice and a consuming into_render_image that swaps red/blue in place into GPUI's CPU BGRA frame. No extra image library, URL fetch, uploader, context store or frame engine is introduced.
- Tradeoffs: static first frame only; no resize or HEIC/SVG raster support. CPU work belongs on a background executor. Codec allocation budgets have a best-effort component; no peak-process, global cache, GPU upload, visibility or browser runtime claim is made. CSS viewport dimensions are not raster dimensions.

## Exact checked code and portable source

The original helper e1a171623 passed Kit all-target strict in run wp-composer-image-kit-20261115T141015Z-e1a17162. The screenshot extension at exact703268a01 passed Kit all-target strict in run wp-composer-image-kit-20261115T155020Z-703268a0 and actual focused image tests in run wp-composer-image-kit-20261115T161215Z-703268a0:5 passed/0 failed/0 ignored. These exercise encoded PNG preservation/BMP normalization, invalid input/output refusal, exact decoded RGBA/BGRA values, browser8192-side versus attachment4096-side,32 MiB RGBA boundary,8 MiB encoded bound and non-PNG screenshot refusal.

The first extension strict run39ecc0a7 refused its old fixed-chunk API; the checked code uses as_chunks_mut::<4>() without a lint exception. Tests did not run on that first failure.

This portable extension is prepared on announced package base6ebdf8e8 plus the original e1 helper prerequisite. All four owned Rust blobs are byte-identical to checked703268a01, with external parity evidence by blob and SHA256. The new receiving-base assembly is source preparation, not a fresh standalone compile, whole package, UI or GPU approval. A consumer takes this bounded leaf with its exact acquisition/context contract and performs its own union checks; it must not infer authority or fetch a URL from this decoder API.
