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
Reloading that same address can restore the selected passive tab.
Closing the routed agent clears its saved route marker, so the next
navigation to that address can reveal the agent again. Use the
existing session route for agent selection and browser back/forward.

Use one set of tab actions across desktop and compact. Keep the workspace header above the selected pane so the project and
agent actions stay accessible on passive tabs. Move Changes,
Diff and file content into the tab host, retaining Summary as session
chrome. A file's editor and original save version survive switching tabs.
Only the visible editor observes disk changes. Closing a modified editor
or leaving the workspace requires confirmation.

Run the existing archive and terminal close calls one by one after a
single bulk confirmation. Compact confirmations fit the content, so the sheet
leaves the workspace visible above it. Remove a tab after a successful close or
when the node reports its target missing. Keep failures available for retry.
A child agent's close affects layout only. This differs from Paseo's single
`closeItems` call.

Project terminals belong to the project root. The current terminal contract
uses a live, unarchived session to anchor new terminals in a worktree or an
unregistered folder. Use another live session in the same workspace when the
routed session has ended. Disable creation when the workspace has no live
anchor. Before archiving a session, look up its live shells, scripts and
services. Count them alongside any terminals explicitly selected for close,
with one count per terminal. Recheck after confirmation and ask again if the
scope grew. The list and archive are separate calls, so a concurrently created
terminal can still appear between them. ADR 0121 covers moving terminal
ownership to the workspace folder.

A successful list carrying an ended terminal clears its tab. Failed or
incomplete lists leave existing targets alone. The page retains its existing
xterm input policy, which forwards user input and suppresses automatic query
replies.

## Alternatives and consequences

A bridge preference document would require a wire change for layout that
belongs to the browser. Encoding every tab in the route would expose
paths and make passive selection alter existing session links. Reusing the
inspector would still have one file slot. Keeping the layout in the page
needs no new dependencies or node methods. The order is browser-local,
and a partially successful close can leave a shorter tab list.

The launcher uses the page shortcut registry for New agent and New
terminal. Its Alt defaults avoid desktop chords reserved by the browser.

The first version provides one pane. Browser, PR, terminal profile and
plugin entries depend on their respective surfaces. The browser entry
becomes visible when an eligible browser surface is available.
