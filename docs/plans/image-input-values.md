# Bounded raster input values

[Decision Log]
- Purpose: composer acquisition and file previews share one bounded raster policy rather than separate decoders/upload stores.
- Existing assets: the kit and GPUI already link image0.25.10; the six codec features are already in the lock. No root/wire dependency or external fetch is added.
- Alternatives: decode arbitrary formats directly in the renderer, trust filenames, or add another image library. These lose shared resource policy and signature validation.
- Choice: kit::image_input::normalize accepts encoded bytes and explicit existing source/result byte bounds. Strict4096 side bounds and a64MiB decoder allocation budget precede renderer exposure. Static PNG/JPEG stay encoded; other accepted raster/APNG first frames become PNG through a capped writer.
- Tradeoffs: animation becomes a static frame; excessive dimensions/bytes refuse instead of resizing. Codec allocation bounds include a best-effort component; this is not a global GPU/cache bound. HEIC/SVG and arbitrary URL acquisition are outside this helper. Callers own the one admitted acquisition loop and must run CPU conversion away from the UI.

This leaf is exactly the kit helper/registration/feature policy from the composer candidate, isolated on announced package base8. No App, plugin, uploader, node or runtime attachment path is included. Existing source6987 codec tests passed2/0 on the build VM; its strict wildcard policy warning was corrected before this leaf. Exact leaf e1a171623 subsequently passed Kit all-target strict in run wp-composer-image-kit-20261115T141015Z-e1a17162. Encoded normalization is not RGBA or GPU proof.


[Decision Log: screenshot pixels]
- Purpose: file previews and browser frames must not grow independent codec or render-image conversions.
- Existing constraint: normalize returns encoded bytes; GPUI expects BGRA frames while the codec returns RGBA. CSS viewport dimensions cannot bound raster bytes.
- Alternatives: decode again in each screen, relax attachment limits globally, or expose arbitrary unvalidated policy numbers. These bypass shared limits or make browser admission alter file intake.
- Choice: normalize and screenshot::decode_png share one internal bounded decode path. The screenshot API is PNG-only with explicit 8 MiB encoded, 8192-side and 32 MiB final RGBA bounds, checked before pixel decoding. It returns opaque DecodedRaster with a borrowed RGBA view and a consuming CPU RenderImage conversion that swaps red/blue in place. Attachment limits remain 4096-side/64 MiB.
- Tradeoffs: static first frame only; CPU conversion belongs on a background executor. Codec allocation limits are best-effort and each decoded/native buffer is bounded; no peak-process, GPU/cache, frame acquisition, browser engine or actual UI proof is claimed. No URL fetch, uploader, context store, or new dependency is introduced.

The pixel extension is isolated source preparation until its changed-source strict and actual pixel/policy tests execute. It is not part of the frozen composer candidate.
