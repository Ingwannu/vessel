# Bounded raster input values

[Decision Log]
- Purpose: composer acquisition and file previews share one bounded raster policy rather than separate decoders/upload stores.
- Existing assets: the kit and GPUI already link image0.25.10; the six codec features are already in the lock. No root/wire dependency or external fetch is added.
- Alternatives: decode arbitrary formats directly in the renderer, trust filenames, or add another image library. These lose shared resource policy and signature validation.
- Choice: kit::image_input::normalize accepts encoded bytes and explicit existing source/result byte bounds. Strict4096 side bounds and a64MiB decoder allocation budget precede renderer exposure. Static PNG/JPEG stay encoded; other accepted raster/APNG first frames become PNG through a capped writer.
- Tradeoffs: animation becomes a static frame; excessive dimensions/bytes refuse instead of resizing. Codec allocation bounds include a best-effort component; this is not a global GPU/cache bound. HEIC/SVG and arbitrary URL acquisition are outside this helper. Callers own the one admitted acquisition loop and must run CPU conversion away from the UI.

This leaf is exactly the kit helper/registration/feature policy from the composer candidate, isolated on announced package base8. No App, plugin, uploader, node or runtime attachment path is included. Existing source6987 codec tests passed2/0 on the build VM; its strict wildcard policy warning was corrected before this leaf. Fresh exact-leaf strict evidence is still owed.
