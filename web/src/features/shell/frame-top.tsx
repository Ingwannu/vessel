// The title bar inside the frame (DESIGN.md §2.2): where the page is (the
// last crumb is the page's one heading) or what the page draws there
// (title-bar.tsx), then the one primary action. The link to `vessel ui`
// says something only when it is not up.

import "./frame-top.css";

import { Link } from "@tanstack/react-router";
import { Fragment } from "react";

import { useChrome, useFleetState } from "../../context";
import type { StateTone } from "../../data/format";
import { selectedNode } from "../../data/nodes";
import { useWhere } from "../../data/where";
import { Button, StatusDot } from "../../ui/kit";
import { machineInView, useNewSession } from "./use-new-session";

/** The link's state in words, when it is not open. */
function linkChip(
  kind: "connecting" | "open" | "closed" | "disconnected" | "refused",
): { tone: StateTone; text: string } | null {
  switch (kind) {
    case "open":
      return null;
    case "refused":
      return { tone: "danger", text: "vessel ui · refused" };
    case "disconnected":
      return { tone: "danger", text: "vessel ui · disconnected" };
    case "connecting":
    case "closed":
      return { tone: "warn", text: "vessel ui · reconnecting" };
  }
}

export function FrameTop({
  slot,
}: {
  slot: (element: HTMLElement | null) => void;
}) {
  const crumbs = useChrome((chrome) => chrome.crumbs);
  const link = useFleetState((state) => state.link);
  const where = useWhere();
  const selected = useFleetState((state) => selectedNode(state, undefined));
  const open = useNewSession();
  const chip = linkChip(link.kind);
  const onNew = where.segments[0] === "new";
  return (
    // The new-session page is the chat before it has a session: its bar
    // is bare chrome, with no hairline (agent-orchestrator's).
    <header className={onNew ? "frame-top bare" : "frame-top"} role="banner">
      {crumbs.length === 0 ? null : (
        <nav className="crumb" aria-label="Breadcrumb">
          {crumbs.map((crumb, index) => {
            const last = index === crumbs.length - 1;
            return (
              <Fragment key={`${crumb.label}-${String(index)}`}>
                {index > 0 ? <span className="sep">/</span> : null}
                {last ? (
                  <h1 className={crumb.mono === true ? "mono" : undefined}>
                    {crumb.label}
                  </h1>
                ) : crumb.to === undefined ? (
                  <span>{crumb.label}</span>
                ) : (
                  <Link to={crumb.to}>{crumb.label}</Link>
                )}
              </Fragment>
            );
          })}
        </nav>
      )}
      <div className="frame-title" ref={slot} />
      <div className="frame-actions">
        {chip === null ? null : (
          <span className="chip reconnecting">
            <StatusDot tone={chip.tone} label={chip.text} />
            {chip.text}
          </span>
        )}
        {onNew ? null : (
          <Button
            variant="primary"
            size="sm"
            icon="plus"
            onClick={() => {
              open(machineInView(where) ?? selected);
            }}
          >
            New
          </Button>
        )}
      </div>
    </header>
  );
}
