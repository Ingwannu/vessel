# Setup observation app boundary

The canonical values carrier `8272859f` arrived through announced package base
`db42d9c5`. The app now uses those exact invocation, ordinal, duration and
observation/cancel types. This is a view/link foundation, not an active RPC,
workspace bootstrap grant or completed setup panel.

[Decision Log]
- Purpose: show actual command results without parsing combined terminal text,
  and keep cancellation acknowledgement separate from command completion.
- Existing assets: canonical proto values, Reading/RequestKeys, the shared
  ConnectionContext, the legacy setup log and the kit renderer.
- Alternatives: duplicate setup enums, infer commands from printable log markers,
  or copy an unreviewed runtime into the app tree.
- Choice: use one pure model and a typed backend boundary tested in memory.
  The default backend is Unavailable and selects/polls no transport. It echoes
  the original address, request key and typed operation even when unsupported.
- Why: a backing WorktreeId is historical observation identity. It is neither
  canonical WorkspaceId nor permission to start/release/bootstrap anything.
- Effects: a real adapter requires the reviewed runtime in an announced base,
  negotiated capability, physical admission, authorization, deadline and decoded
  snapshot limits. These are not supplied by this values-only foundation.

Paseo setup-panel.tsx 104–118 supplies the final-carriage-return-segment display;
99–103 supplies whole-unit duration formatting, and 164–265 supplies automatic
expansion with manual collapse. The producer here includes planned Waiting rows,
so automatic expansion chooses the running or last attempted row rather than an
unattempted trailing command. A new invocation resets manual expansion because
the same ordinal can name a different command; the reference retains its Sets
for the mounted component. Within an attempt manual collapse remains stable.

Absence and unknown values never mean success. A cancellation button captures
the displayed invocation and requires a fresh running read plus independent
backend support; Requested is not process death. Controller retirement and
original pending-cancellation retention still belong to the later area adapter.
The canonical scalar DTO by itself does not validate a complete observation;
no network renderer or local DTO validation is claimed by these fake tests.


The Setup renderer is now registered through the single panel registry. A
canonical pane requires its exact WorkspaceId in the LayoutKey and Target.
The installed area has no workspace-to-backing resolver and passes an unavailable
model with no callbacks, so opening it creates no request, cancellation or start.
The shared-kit structured projection is painted against isolated typed fixtures
for command results, both languages and both schemes. Those tests supply no
network adapter and cannot establish live setup or workspace authority.

[Decision Log]
- Purpose: build the screen against the existing canonical values and kit while
  the runtime/registry admission contract is pending.
- Constraint: historical WorktreeId observations cannot authorize a canonical
  Workspace pane or cancel its process.
- Alternative: infer a backing from selected session/cwd, or keep a second pane
  manager. Both discard the canonical registry's ownership boundary.
- Choice: one passive registered renderer and a typed unavailable product state.
- Impact: structured rendering can be tested now; live reads, pending cancellation
  retention, retirement and negotiated authorization remain the future controller.


The unavailable notice intentionally makes no claim about the host's capability:
the missing boundary may also be canonical workspace/backing admission. Neither
a cached protocol number nor a successful old worktree is used to choose a more
optimistic message or enable an effect.
