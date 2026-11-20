# 0118. Workspace tabs in the page

## Context

The page's session address is a durable deep link. Its inspector used one
slot for Changes and Files, so opening a second file replaced the first
file and its editor state. Standalone terminals already had a node
lifecycle and a bridge follower but lacked a workspace surface.

## Decision

Keep an ordered tab layout in the browser, scoped by machine and resolved
worktree or project. Unregistered folders use the session as their scope.
Persist target identities and selection in local storage for the served
origin. A different launch origin starts with a fresh layout. A session
address selects its agent when entered directly or through navigation.
The last visited session address anchors passive tabs and the launcher.
Reloading that same address can restore the selected passive tab. Use the
existing session route for agent selection and browser back/forward.

Use one set of tab actions across desktop and compact. Move Changes,
Diff and file content into the tab host, retaining Summary as session
chrome. A file's editor and original save version survive switching tabs.
Only the visible editor observes disk changes. Closing a modified editor
or leaving the workspace requires confirmation.

Run the existing archive and terminal close calls one by one after a
single bulk confirmation. Compact confirmations fit the content, so
the sheet leaves the workspace visible above it. Remove each tab when its action succeeds and
keep failures available for retry. A child agent's close affects layout
only. This differs from Paseo's single closeItems call. Project terminals
belong to the project root. For a worktree or unregistered folder, the
existing terminal contract uses a session in that location. A successful
list carrying an ended terminal clears its tab. Failed or incomplete
lists leave existing targets alone. The page retains its existing xterm
input policy, which forwards user input and suppresses automatic query
replies.

## Alternatives and consequences

A bridge preference document would require a wire change for layout that
belongs to the browser. Encoding every tab in the route would expose
paths and make passive selection alter existing session links. Reusing the
inspector would still have one file slot. Keeping the layout in the page
needs no new dependencies or node methods. The order is browser-local,
and a partially successful close can leave a shorter tab list.

The first version provides one pane. Browser, PR, terminal profile and
plugin entries depend on their respective surfaces. The browser entry
becomes visible when an eligible browser surface is available.
