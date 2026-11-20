// One session (DESIGN.md §3.1): read from the machine's list (fresher,
// polled) or, before it is listed, asked for once; then its timeline or
// its terminal, each under the session's own title bar.

import "./session-page.css";

import { useEffect, useState } from "react";

import { useCrumbs, useFleet, useFleetState } from "../../context";
import { workspaceTabScope } from "../../data/workspace/tab-layout";
import { readSession } from "../../data/calls";
import { shortId } from "../../data/format";
import { labelOf } from "../../data/nodes";
import { Banner, Skeleton } from "../../ui/kit";
import type { SessionId, SessionInfo } from "../../wire/proto";
import { useSessionRoot } from "../workspace/use-session-root";
import { Workspace } from "../workspace-tabs/workspace";
import { StructuredSession } from "./structured-session";
import { TerminalView } from "./terminal-view";
import type { NodeKey } from "../../wire/bridge";

const NO_CRUMBS: readonly never[] = [];

export function SessionView({ node, id }: { node: NodeKey; id: SessionId }) {
  const fleet = useFleet();
  const label = useFleetState((state) => labelOf(state, node));
  const listed = useFleetState((state) =>
    state.sessions[node]?.find((session) => session.id === id),
  );
  const [fetched, setFetched] = useState<{
    id: SessionId;
    node: NodeKey;
    session: SessionInfo | null;
    error: string | null;
  } | null>(null);
  const missing = listed === undefined;
  useEffect(() => {
    if (!missing) {
      return undefined;
    }
    let current = true;
    void readSession(fleet, node, id).then((outcome) => {
      if (current) {
        setFetched(
          outcome.ok
            ? { id, node, session: outcome.value, error: null }
            : { id, node, session: null, error: outcome.problem.message },
        );
      }
    });
    return () => {
      current = false;
    };
  }, [fleet, node, id, missing]);
  const read = fetched?.id === id && fetched.node === node ? fetched : null;
  const session = listed ?? read?.session ?? null;
  const error = read?.error ?? null;
  // The page names itself (session-bar.tsx); a session not found yet is
  // named by its id.
  useCrumbs(
    session === null ? [{ label: shortId(id), mono: true }] : NO_CRUMBS,
  );
  if (session === null) {
    return (
      <div className="page">
        {error === null ? (
          <Skeleton />
        ) : (
          <Banner tone="error">
            Session {shortId(id)} on {label}: {error}
          </Banner>
        )}
      </div>
    );
  }
  return <SessionWorkspace node={node} label={label} session={session} />;
}

function SessionWorkspace({
  node,
  label,
  session,
}: {
  node: NodeKey;
  label: string;
  session: SessionInfo;
}) {
  const { root, projects } = useSessionRoot(node, session);
  const scope = workspaceTabScope(node, root, session.id);
  return (
    <Workspace
      key={scope}
      node={node}
      label={label}
      anchor={session}
      root={root}
      projects={projects}
      scope={scope}
      agent={(shown, actions) =>
        shown.spec.mode === "structured" ? (
          <div className="session">
            <StructuredSession
              key={shown.id}
              node={node}
              label={label}
              session={shown}
              actions={actions}
            />
          </div>
        ) : (
          <div className="session term">
            <TerminalView
              key={shown.id}
              node={node}
              label={label}
              session={shown}
            />
          </div>
        )
      }
    />
  );
}
