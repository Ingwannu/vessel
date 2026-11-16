# Temporary explicit Markdown refusal policy

[Decision Log]
- Purpose: untrusted assistant/plan Markdown must not inherit OS URL opening or
  arbitrary URI/data image loading while the scoped host is absent.
- Existing source: B2's public Actions helper installs callbacks only for Some.
  None preserves engine handle_link_click -> open_url and ImageNode::source.
  W6 has no checked assistant acquisition API; sent-attachment identity cannot
  authorize arbitrary Markdown URIs. This is a source finding, not an OS probe.
- Alternatives: route/cwd fallback, a second URL classifier, generic fetch/cache
  or fabricated pixels would invent authority or a second acquisition path.
- Choice: one A7 Actions::new(noop) intercepts ALL links and with_images returns
  the existing C5 refusal-only custom source: immediate localized Asset error.
  Every URI is ignored. No enum, fetch, decoder, pixels or pending state is added.
- Why: the same Markdown state/parser/selection renderer stays in place, and its
  explicit host callbacks prevent its default path. Existing generic output and
  source-copy behavior are unchanged.
- Effects: file, relative and external links are intentionally unavailable, as
  are assistant/plan images, until checked scoped host contracts land. This is
  refusal behavior, not link/image parity, a lightbox, or image snapshot policy.

Assistant and shared Plan card use the same helper and one localized image label
in the existing Plan Markdown domain. W4 owns its independent legacy permission
Markdown call; its matching owner adaptation remains required. The new tests use
GPUI's fake TestPlatform (open_url records only; FakeHttpClient404) and the public
Markdown helper, with a positive click control before refusal, and trace actual
resolver construction. They also invoke the returned Custom load and assert an
immediate localized error. These tests are authored but not executed yet; no
real OS open, network request or positive image acquisition was attempted.

C5 source provenance is the existing document_element expression at
c77069599 (same as compiled3d3f21570), not a whole C5 branch adoption. ADR0100's
accepted mark/revision engine remains separately held, and this policy does not
confer resident input or transport authority.
