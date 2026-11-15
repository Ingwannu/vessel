# Existing Markdown renderer: optional host actions

[Decision Log]
- Purpose and intent: timeline and file preview hosts need the same optional file-link, image activation and selection-copy behavior without splitting the Markdown control.
- Existing assets and constraints: the pinned TextView already owns parsing, source ranges, find and window selection. Its component facade does not forward the needed callbacks. The renderer and acquisition owners remain separate.
- Alternatives: another renderer, synthetic inline atoms, rewriting image Markdown as links, or another copy/selection controller.
- Choice: extend the existing Base TextView with optional typed actions and let the existing kit entry call it directly with the installed component theme. The old entry point and existing link callback remain compatible. No dependency is added.
- Why: changing controls or source bytes would break partial selection and find mapping. Authorized resource resolution belongs to the host; a rendered href or URI is untrusted text.
- Advantages, costs and effects: one retained parser/state/selection aggregator remains. The vendored engine patch list records why each additive seam exists; no file, URL, image acquisition or lightbox operation is added here.

Public host API is `kit::chat_markdown::chat_markdown_with(id,text,revision,Options{marks,actions,copy},tokens,window,cx)`. `actions::Actions::new(link_handler)` receives typed Href with Authored/Inferred provenance. `.with_inline_links(fn(&str,InlineLinkKind)->Vec<RecognizedLink>)` opts into validated UTF8-byte marks. `.with_images(resolver)` installs an authoritative resource snapshot; pending/refused images must not fall back to untrusted local reads. `.on_image_click(handler)` receives original ImageHref for the owner's shared lightbox. A handler consumes linked-image activation; without one the previous enclosing-link behavior remains.

`CopyStyle::MarkdownCodeAware` retains Markdown across regions and raw code only inside one parsed code region. Whole-view Select All keeps message boundaries; the existing window aggregator owns multi-message selection. Final LF/CRLF and following ASCII indentation share the same cleanup with the existing code-copy button. HTML clipboard flavor remains unavailable.

Reference: Paseo0.15.4 packages/app/src/components/markdown/source.ts:12-55, assistant-file-links/parse.ts:598-646, assistant-selection-copy/{content.web,markup}.ts and components/highlighted-code-block.tsx:95. The patch README lists changed engine paths and invariants.

Initial leaf1b236618 carried17 Rust blobs identical to exact40aaab5d source in actual Kit run `wp-composer-copy-diagnostic-20261115T141044Z-40aaab5d`,195/0/0. This includes plain/inline/linked image activation, backward compatibility, partial inferred-link selection/find offsets, authored-link/fence priority, raw-code/Markdown copy and theme preservation.

The successor is a single owned leaf on announced c3429952. It additionally takes the strict lint corrections from the same source: the private code-copy corner borrows its text, and tests use explicit semicolons/method references, checked UTF8 slices and the same integral pixel endpoints through mul_add. No assertion, public callback, parser or engine behavior is weakened. The earlier all-target native check stopped at those lints; the queued initial standalone leaf was cancelled before execution. Current17 Rust blobs exactly match the corrected main source44b7b55f; independent successor strict Clippy is pending.

This owned cut contains no composer field/submit/layout change, App/View/link/node/Cargo delta, owner navigation/acquisition or product timeline hook. Canonical c342 dependencies are not part of the owned source. No whole-package, UI, OS or authority approval is claimed.
