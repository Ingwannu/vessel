# How to document a Paseo area (survey brief, 2026-11-15)

Paseo 0.15.4 source: ~/refs/paseo-latest (read only). Packages: app (React Native / web UI,
packages/app/src), server (the daemon, packages/server/src), client (packages/client, the typed client
the app and CLI use), protocol (packages/protocol, message schemas), cli (packages/cli), desktop
(packages/desktop, Electron shell), relay, plugin.

Goal: a complete catalogue of every user-facing feature in your area, how it works end to end, and
how it connects to other features. It will be the basis for porting Paseo to Rust feature by feature,
so exactness matters more than prose. Use exact names from the code. Never guess: if you did not find
something, say "not found" and where you looked.

For each feature (one `##` heading per feature; sub-features as `###`):
1. **What the user sees and does**: entry points (screen, menu item, button, shortcut, CLI command),
   with file:line in packages/app/src (or cli/desktop).
2. **How it works end to end**: app store / state (file) → client call (packages/client, method name)
   → protocol message (packages/protocol, message type and its fields) → server handler
   (packages/server/src, file:line) → storage (files, sqlite tables, config keys, paths) → events or
   streams pushed back and how the app applies them.
3. **States, errors, limits, defaults**: every state the UI shows, error cases, numeric limits and
   timeouts (with the constant name), settings keys and defaults.
4. **Platforms**: differences between desktop, web and mobile, if any.
5. **Connections**: which other features this one reads from, writes to, triggers or is triggered by,
   and which shared entities it touches (agent/session, workspace/project, host/daemon, timeline,
   permission/question, attachment, notification/attention, settings, provider/model).

Keep each feature compact (10-40 lines). Cover sub-features (menus, sheets, keyboard shortcuts, empty
and loading states). At the end of the file add:
- `## Cross-area connections`: every link from your area to features outside it.
- `## Inventory`: a flat list of every feature and sub-feature you documented, one line each.

Write the file incrementally (save every few features). Return only: the file path, the number of
features, and the most important connections (at most 20 lines).
