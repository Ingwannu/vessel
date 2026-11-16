# Exact timeline source membership prerequisite

This isolated package is based on the announced package base13. It contains only
View model changes and this decision record, without App rendering, rich protocol
variants, node calls or resident/resource changes. Validation is pending.

[Decision Log]
- Purpose: a global search result at a later user chunk must reveal the canonical
  prompt, even when a diagnostic Log entry starts between its chunks. A late tool
  update may overlap that span without owning the user's source record.
- Existing assets and constraints: Timeline applies records in order and preserves
  stable entry indices. Entry revisions are cache invalidations, not source cursors.
  The existing bounded finder and W1's cache/counters remain unchanged.
- Alternatives: choosing the latest earlier start or treating a first/through span
  as exact membership misattributes sparse and interleaved records. Refolding or a
  second transcript manager would duplicate the canonical projection. No external
  parser or dependency is needed; the standard ordered vector suffices.
- Choice: the one append/change path records Entry.through as an exclusive upper
  bound and ExtraSources as exact additional contributing EventSeq values.
  entry_at_source prefers exact creating records and then recorded membership.
  Human chunks retain their Prompt/Steer identity across HarnessLog and metadata;
  real agent content, lifecycle, terminal and rewind events end that stream.
- Why: one explicit classifier aligns with the prompt-index source contract while
  preserving conservative fork bounds and precise navigation in the same model.
- Effects: conservative bounds still contain gaps and grant no runtime authority.
  Callers must keep session/resident/source identity and rewind visibility checks.
  The existing find start_at signature prefers exact membership before its old
  approximate fallback when there is no contributing entry.

The public borrowed presented_prompt helper is included without changing the
canonical text, metadata previews or raw outline. The baseline App still renders
raw user bodies, so searchable remains raw in this packet too. A7's later body
renderer/local-find/copy/rewind union must switch those together; this helper by
itself is not a rendered-range or selection-engine adoption. Assistant examples
remain literal. No reveal_turn shim or fake noop is introduced: the real App
collapse/reveal implementation remains a separate A7 renderer integration.

Tests cover chunk/log/metadata interleaving, sparse source membership despite
late tool updates, stable Steer identity, actual content/terminal boundaries,
exclusive bounds, and the borrowed envelope rules with source text retained.
Existing W1 cache-work assertions and page tests remain intact. Strict and View
all-target runtime on this receiving source remain required before handoff.

The first receiving strict run on2d008494 stopped at a missing ToolView import
in fold.rs after moving append methods: the public tool_call query still names
that type. Its existing import is restored; no query signature, dependency or
behavior changes. The source classifier and membership tests have not executed
on the correction yet. The original run remains compiler-failure evidence.
