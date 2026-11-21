# Page diff and plugin follow-ups

## Decision log

- Purpose and intent — align visible line counts, explain the changes
  threatened by a worktree removal, and make plugin actions identify the
  correct machine without overwriting a prompt.
- Existing implementation and constraints — the node already inspects what
  removal would lose and has a shared diff counter (ADR 0120). The page
  builds plugin commands from per-machine catalogs. The queue is stored
  on the node and its existing removal operation has no revision guard.
- Alternatives — recount on the client, read the whole fleet in the plugin
  memo, or take a prompt out of the server queue to edit it. These would
  duplicate Git work, rebuild plugin actions for unrelated fleet updates,
  or allow a concurrent prompt edit to disappear.
- Choice — attach optional diff counts to the existing kept reply, retain
  the existing palette sections and command ids, and keep the in-place
  queue edit. A local slash command can execute from that edit while its
  contents and the saved draft are retained.
- Impact — no new method or dependency. Older replies work without count
  details. Focused plugin entries depend on their own machine and session.

## Reference behavior

- TRACKER 835. Paseo uses ASCII signs in `components/diff-stat.tsx:11–16`.
  Apply the signs to the page's file rows, comparison menu, composer,
  sidebar and workspace hover card.
- TRACKER 823. Paseo's `git/worktree-archive-warning.ts:52–98` inserts counts
  only in the uncommitted-changes line, skipping zero sides. The English
  singular and plural forms are in `i18n/resources/en.ts:930–935`. Vessel
  uses the same copy in its removal warning and keeps existing safeguards
  and the separate unique-commits reason.
- ADR 0115. Paseo's `plugins/command-center/registration.tsx:19–45,51–79`
  scopes the palette to the focused server, and `contributions.ts:75–88`
  gives each item an identity and one section per plugin. On a Vessel
  fleet route, each machine's contribution includes the machine label.
- ADR 0115. In Paseo `composer/index.tsx:1457–1474,1935–1945,1978–1984`,
  editing moves the queued prompt into the composer, and a local slash
  command runs before queueing. Vessel's queue is server-owned. Its
  in-place editor keeps both the queued entry and the draft when a local
  slash command runs. A future edit-as-take operation needs a revision
  guard before it can safely remove the server entry.

## Verification

Run the Rust and web gates. The removal dialog and palette should be checked
in both themes, including a compact frame when machine labels are shown.
