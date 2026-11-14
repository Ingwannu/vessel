// The panel beside a session's timeline (agent-orchestrator's inspector):
// a 36px head of icon tabs, Summary, Changes and Files, over the chosen
// tab. Summary is drawn here; Changes and Files are the code panel
// (features/workspace/panel.tsx), a chunk of its own loaded the first
// time one of them opens. The panel is 340px for Summary and widens for
// code, as agent-orchestrator's does for its Files tab.
//
// [Decision Log]
// - Purpose: milestone 11's code on the page: a session's changes and
//   files beside its conversation, where agent-orchestrator puts them.
// - Constraint: the session's chunk must not grow by the panel's code
//   (diff rows, the lexer, the editor) for a reader who never opens it;
//   every chunk must be pinned by the index's import map (ADR 0017's
//   amendment), which `scripts/integrity.mjs` does for any `import()`.
// - Alternatives: a route of its own for the panel (a second page, where
//   the references keep it beside the chat); Summary's sections growing
//   a Changes section (no room for a diff in 340px).
// - Choice: tabs in the inspector's head; `lazy()` for the code panel.
// - Cost: the first Changes or Files waits for one request.

import "./inspector.css";

import { lazy, Suspense } from "react";

import type { Timeline } from "../../data/timeline-model";
import { PanelTabs, Sheet, Skeleton } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type { SessionInfo } from "../../wire/proto";
import { InspectorSummary } from "./inspector-summary";
import type { Panel } from "./panel-state";

const WorkspacePanel = lazy(async () => {
  const module = await import("../workspace/panel");
  return { default: module.WorkspacePanel };
});

export function Inspector({
  node,
  label,
  session,
  timeline,
  panel,
}: {
  node: NodeKey;
  /** The machine's name. */
  label: string;
  session: SessionInfo;
  timeline: Timeline;
  panel: Panel;
}) {
  const tab = panel.tab;
  return (
    <aside
      className={tab === "summary" ? "inspector" : "inspector wide"}
      role="complementary"
      aria-label="Session panel"
    >
      <PanelTabs
        label="Session panel"
        value={tab}
        onChange={panel.setTab}
        tabs={[
          { value: "summary", label: "Summary", icon: "list" },
          { value: "changes", label: "Changes", icon: "git-compare" },
          { value: "files", label: "Files", icon: "folder" },
        ]}
      >
        {tab === "summary" ? (
          <InspectorSummary
            node={node}
            label={label}
            session={session}
            timeline={timeline}
          />
        ) : (
          <Suspense
            fallback={
              <div className="inspector-wait">
                <Skeleton />
              </div>
            }
          >
            <WorkspacePanel
              node={node}
              session={session}
              tab={tab}
              opened={panel.opened}
              onOpened={panel.setOpened}
            />
          </Suspense>
        )}
      </PanelTabs>
    </aside>
  );
}

/** The panel in a sheet from the right, for a window too narrow for the
 * inspector (a phone). */
export function InspectorSheet(props: Parameters<typeof Inspector>[0]) {
  return (
    <Sheet side="right" label="Session panel" onClose={props.panel.closeSheet}>
      <Inspector {...props} />
    </Sheet>
  );
}
