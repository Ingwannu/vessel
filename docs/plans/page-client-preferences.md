# Page client preferences

M12 page settings packet for F-L7 Terminal, F-L9 Files and F-L10 Open location.

## Decision log

The browser needs client preferences that survive a fresh `vessel ui` origin
and stay in agreement across tabs. The UI bridge already keeps revisioned,
field-level preferences in its local file (ADR 0028). Extend that owner with
the terminal's bounded history and per-action destinations. Browser storage
would lose the preferences on relaunch, and node settings would tie a
client's layout to the machine it happens to be using. This packet adds no
node method, migration or dependency.

The page already owns each workspace target in its tab registry. The secondary
pane and explorer reuse that registry and the existing file, diff and PR
components. A file has one editor and dirty guard even when it moves. A later
secondary view returns the previous target to the main strip with its state.
Pane placement lasts for the open workspace, while each saved destination
applies across workspaces and UI launches. Narrow workspaces use the main
strip to keep both controls and content readable.

## Paseo 0.15.4 parity

| Section | Reference | Page behavior |
| --- | --- | --- |
| Terminal | `screens/settings/terminal/terminal-section.tsx:10–56`, `hooks/use-settings/storage.ts:54–56,480–493` | Whole-line input with a 0–1,000,000 range and a default of 10,000. Enter or blur commits. The xterm buffer updates for open and future terminals. The node owns its independent replay and retention limits. |
| Files | `screens/settings/files-section.tsx:28–53,64–109`, `file-panels/settings/rows.ts:23–39`, `file-panels/resolve.ts:103–112,123–142` | The Files card shows the reference empty state. Markdown, HTML and Image claim disjoint types, with Source as the fallback. Plugin FilePanel consumption is P3b-2. Association selection and Ask Again apply once a real competing panel registers. |
| Open location | `screens/settings/open-location/open-location-section.tsx:11–24,41–59,87–120`, `workspace-tabs/open-supporting-view.ts:42–88` | The Files tree and search, workspace and chat Changes entry, and source files clicked in Changes or Diff use Main panel or On the side. The PR launcher also offers the Explorer sidebar, its default. On compact screens the existing tab opens the view. |

The workspace's explorer PR position is exposed through the current PR
launcher, with a Files control in that explorer pane. The page's chat
Markdown has no workspace file-link action, and chat has no subagent
opener. Those two settings wait for their entry points. Service URL
placement waits on TRACKER 507/513. The file editor and Vim toggle belong
to the separate F-L8 packet.

## Verification

The bridge tests check scrollback bounds and preservation of unread settings.
The browser cases exercise a kept Terminal edit, the Files empty state,
side placement from the real explorer, and retaining one dirty buffer when
that pane's next file opens.
