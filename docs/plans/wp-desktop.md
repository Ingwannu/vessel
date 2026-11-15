# Desktop lifecycle and first-run setup

## Reference and scope

Paseo 0.15.4 is the behavioural reference, read before implementation:

- `packages/desktop/src/features/menu.ts:80-231`: File/New Window,
  Edit, View zoom/reload/fullscreen, Window; zoom withheld during shortcut capture.
- `packages/desktop/src/window/window-manager.ts` and
  `settings/window-state.ts`: first window restores geometry, later windows do not;
  default 1200×800, minimum 400×300, visible-work-area clamping, 400 ms save debounce.
- `packages/protocol/src/agent-deep-link.ts:1-80`: reject credentials, ports,
  queries and fragments; navigate by host identity, queue until the renderer is ready.
  `desktop/src/agent-navigation.ts` and `window/desktop-window-owner.ts` own delivery.
- `packages/app/src/onboarding/internal/model.ts:1-187`: decide once after connected
  setup host and hydrated directory; existing projects suppress automatic setup;
  appearance/providers/projects/orchestration/phone; monotonic visits reject stale advance.
- `onboarding/internal/orchestration-setup.ts:16-144`: all choices initially on,
  only supported effects offered, apply on Continue and advance only after every success.
- `changelog/internal/parse-changelog.ts:1-180`: release and section headings outside
  fences; retain Markdown; `use-revealed-releases.ts`: reveal 1, then 5, then +10.
- `desktop/updates/desktop-app-updater.ts` and
  `desktop/updates/resolve-update-callout.ts`: manual and focus checks,
  download/install/failure status, restart confirmation, What's new.
- The desktop survey found no system tray. None is added.

## Existing asset analysis and decision

**Modify existing assets.** Reuse About, updater and its signed channel feed,
Prefs' atomic owner-only persistence, settings pages, workspace project state,
provider discovery, device pairing and kit dialogs. GPUI already supplies windows,
menus, URL callbacks and window bounds; std and the existing serde/chrono crates
suffice for new pure models. Electron/Tauri would duplicate the established GPUI
host, so no new toolkit or dependency is introduced.

The pure rules belong to `view::desktop` and `view::onboarding`; desktop-specific
I/O belongs to `link::desktop` and the GPUI entry point; screen state and drawing
belong to `app::areas::desktop` and `app::areas::onboarding`. Shared shell/link files
receive registration only. Node service execution remains owned by the node
lifecycle implementation; the app must not invent commands or signal arbitrary PIDs.

## Decision log

- Purpose: desktop windows and startup should preserve identity and make setup
  repeatable without letting old asynchronous work advance a new visit.
- Constraints: a native host replaces Electron; an updater and private prefs store
  already exist; the node service control contract is separate.
- Alternatives: another desktop runtime; local shell commands for lifecycle; new
  settings storage; one wizard implementation per screen.
- Choice: extend the existing four layers and reuse operational screens and typed
  calls. Keep navigation targets parsed, bounded and credential-free.
- Why: this preserves one owner per state and the existing update trust boundary.
- Consequences: GPUI OS dispatch is verified separately from pure model tests;
  unsupported node operations stay hidden and are reported as blocked, never faked.

## Deliberate differences and dependencies

Vessel uses `vessel://h/<node>/session/<session>` rather than Paseo's scheme and
agent terminology; both identifiers are validated protocol IDs. Workspace links
require the workspace contract before registration. Update artifacts remain
Vessel's signed manifests, never Paseo's GitHub installer or unsigned Markdown.
The built-in feed channel remains usable until stable/beta artifacts are published.
An offline desktop can read bundled release notes without a client-side request.

ADR 0071's earlier proposal is superseded where it inserts a Machine step or
opens without a setup host: pairing remains the no-host route; this wizard follows
the reference's five steps and connected/hydrated host gate. Phone is omitted at
compact width as Paseo's current model requests. Phone native builds do not open
setup automatically. Completion remains client-local; unreadable preferences
suppress automatic setup and are never overwritten.

## Validation and handover

Per-commit checks: format, size/boundary/token checks, changed-crate tests and
clippy through the build slot. Package end: one native remote gate, inspected
captures in both languages/themes at desktop/compact/minimum sizes, then review.
Results and exact unsupported operations are recorded here before handover.
