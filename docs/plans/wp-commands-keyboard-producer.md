# Keyboard mechanics receiving boundary

This cut extends the existing shortcut registry and native interceptor. It is
prepared for feature owners to register typed callbacks without importing the
unfinished command-center sources. It is not the completed F4 package.

[Decision Log]
- Purpose: share the actual keyboard dispatcher with desktop, composer and
  pane owners while preserving one default table and one interceptor.
- Constraint: the old action enum, override parser and two-step matcher cannot
  consume the canonical predicates, indexed keys or longer sequences alone.
  Copying only the native dispatcher would leave incompatible request types.
- Alternatives: a second key listener, a compatibility action/string table,
  or importing all unfinished palette and file/git feature source.
- Choice: move the coherent pure registry/engine/prefs grammar, native
  listener/help/rebinding and actual kit composition/overlay boundaries
  together. Adapt only the existing palette's local-key and close-focus hooks.
- Why: the native callback API is exercised by the existing core and pane
  consumers. Missing feature owners remain unavailable instead of acquiring
  guessed identity, route or network authority.
- Consequences: this cut includes the canonical 76 reference bindings and named
  native additions. It deliberately excludes new palette pages, Messages mode,
  file search/open, git commands, workspace registry, sidebar geometry and
  desktop effects. A later feature union must preserve those owners' source.

## Responsibilities and flow

`view::shortcuts` owns typed keys, canonical definitions, predicates, effective
overrides, sequence resolution and priority ordering. `view::prefs` stores the
same grammar and bounds; no parser or binding table is duplicated in the app.
Labels and whole conflict sentences use the EN/KO `shortcuts.ftl` catalog.

`app::areas::shortcuts` owns the single GPUI interceptor and Settings recorder.
IME/CharacterInput, actual popup context, visibility and compact availability
are checked before matching or recording. Kit line, multiline and code editor
engines register in the same weak composition registry. Terminal focus comes
from the terminal owner's actual focus query; no terminal preedit capability is
inferred. The existing palette retains plain arrows/Enter/Escape before global
rebinding, with its own frame controller and weak close-focus restoration.
It has no nested-page Backspace behavior on this base.

Resolved `ShortcutRequest` values reach feature-owned `KeyboardHandler`
callbacks in priority order. Registration cleanup is generation-specific.
`enabled` is a current admission query, not permission inferred from action
metadata. A callback returning `Handled::Yes` consumes a matched operation;
only an unhandled operation proceeds to lower priority. The pane owner runs at
priority 100, and a refused/guarded close stays handled. A desktop close-window
fallback must run below that owner and must not retry a refused pane close.

`Action::desktop_action()` and `Action::composer_action()` project typed
intentions. Their presence does not supply a feature controller. Desktop zoom
extensions remain withheld pending a checked desktop adapter. Canonical
FocusComposer and NewWorkspace likewise have no global route fallback.

Settings help, key chips, conflict detection and rebind/reset use the same
effective registry. Preference acceptance here is local model acceptance;
durable success, canonical rollback and write-failure notices remain the sole
desktop preferences coordinator's responsibility. No second writer or receipt
implementation is included.

## Verification

Rust compile, strict checks and tests run through the shared remote gate using
the native workspace's pinned toolchain. App is a binary target: selecting
library tests alone does not execute its GPUI regressions. Source-only size and
token checks are separate from compiled evidence.

Receiving checks are pending. No standalone compilation, package review,
physical keyboard/OS, hidden-retained-opener visibility, terminal IME or live
feature authority approval is claimed by this document.


The first isolated receiving checks stopped before tests: help query trimming
uses the existing ECMAScript whitespace predicate, whose crate visibility was
missing from this extraction. Only that visibility is widened to pub(crate);
the predicate body, Query matching, scoring and marks remain unchanged. Both
failed logs are retained as compile failures, not executed test evidence.
The receiving tree now includes announced base13 before its next checks.

The next receiving strict run reached App and found the existing Areas factory
still passed the old listener constructor signature; the real Window is now
forwarded there. The new desktop stand-in fixture also now uses the existing
Ordinary capture role rather than a nonexistent variant. Neither correction
changes the callback contract or feature policy. The failed strict log is not
an App pass and the immutable earlier test job is retained until completion.

The subsequent strict check found only the desktop fixture's ignored registration
result. The fixture now binds its cleanup token as the existing layout fixture
does; product source and all fixture assertions are unchanged. The previous
immutable test run is retained as exact prior-source evidence, with its actual
result recorded separately rather than inferred from strict compilation.
