# Native General browser pane

TRACKER 509. The General browser uses the component qualified on
`569f5e219`. ADR 0078 owns browser peer and launch policy. ADR 0119 allows
the Linux General pane to progress independently of service Preview.

## Design

Reuse the workspace pane registry, launcher catalog, shortcut dispatch and
typed BrowserControl values. The pure view model reads a node-owned tab
descriptor, then prepares toolbar commands for that same workspace and
resource. Reuse the staged preset table and bounded frame geometry. This
keeps controls and actual visible state consistent when a device resize or
navigation changes the current page and an earlier response is still queued.

The app must hold resident state by layout and tab identity so hiding a pane
does not destroy its browser target. The toolbar and shortcut eligibility use
the same history and load facts. The shared opener accepts HTTP(S).
The settings action clears General data through direct Admin authority and
requires confirmation. The authenticated node entry and the local resolver
policy check are deferred to the end of this track.

## Verification

The first packet stages the toolbar projection, preset dimensions, and
localized device labels. It proves that a foreign workspace, Preview
descriptor or earlier revision cannot update the General pane. A separate
native packet wires the shell, launcher and controls. The coordinator sees
1280 and 390 pixel frames in dark and light with English and Korean words
before native UI review.
