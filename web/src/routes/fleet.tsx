// Fleet (DESIGN.md §3.1): every node the book knows, with what each
// daemon reports; a node's facts in the inspector.

import { type ReactElement, useEffect } from "react";
import { Link } from "@tanstack/react-router";

import { useCrumbs, useFleet, useFleetState } from "../context";
import type { FleetState } from "../data/fleet";
import { harnessName, isLive, nodeState, shortId, when } from "../data/format";
import { apiName, routeLine } from "../data/models";
import { nodeFacts, nodeLabel } from "../data/nodes";
import { useWhere } from "../data/where";
import type { BookNode, NodeStatus } from "../wire/bridge";
import {
  Badge,
  Banner,
  Button,
  EmptyState,
  Row,
  RowList,
  Skeleton,
  StatusDot,
  Tooltip,
} from "../ui/kit";

export function FleetPage(): ReactElement {
  const where = useWhere();
  const chosen = where.segments[1] ?? null;
  const fleet = useFleet();
  const book = useFleetState((state) => state.book);
  const bookError = useFleetState((state) => state.bookError);
  const statuses = useFleetState((state) => state.statuses);
  const sessions = useFleetState((state) => state.sessions);
  const nodes = book ?? [];
  const selected = nodes.find((node) => node.key === chosen) ?? null;
  // The node in view is the selected node (DESIGN.md §2.5), however the
  // page was reached: the topbar's New then opens on it. The page has no
  // New button of its own, which would repeat the topbar's.
  useEffect(() => {
    if (chosen !== null) {
      fleet.select(chosen);
    }
  }, [chosen, fleet]);
  useCrumbs(
    selected === null
      ? [{ label: "Fleet" }]
      : [
          { label: "Fleet", to: "/fleet" },
          { label: nodeLabel(selected, statuses[selected.key]) },
        ],
  );
  const online = nodes.filter(
    (node) => statuses[node.key]?.state === "connected",
  ).length;
  const live = Object.values(sessions).flatMap((list) => list.filter(isLive));
  const byAgents = live.filter(
    (session) => session.created_by.kind === "agent",
  ).length;
  const harnessCount = new Set(
    Object.values(useFleetState((state) => state.harnesses)).flatMap((list) =>
      list.map((harness) => harness.kind),
    ),
  ).size;

  return (
    <div className="page">
      <div className="page-head">
        <div>
          {/* One line of facts, not a row of tiles: the table below is the
              fleet, and a number repeated in a large tile says nothing more. */}
          <p className="strip" aria-label="Fleet summary">
            <span>
              <strong>{nodes.length}</strong>{" "}
              {nodes.length === 1 ? "node" : "nodes"}
              <span className="s">
                {online} online, {nodes.length - online} not connected
              </span>
            </span>
            <span>
              <strong>{live.length}</strong> live{" "}
              {live.length === 1 ? "session" : "sessions"}
              <span className="s">
                {byAgents === 0
                  ? "all started by a person"
                  : `${byAgents} started by agents`}
              </span>
            </span>
            <span>
              <strong>{harnessCount}</strong>{" "}
              {harnessCount === 1 ? "harness" : "harnesses"}
              <span className="s">installed across the fleet</span>
            </span>
          </p>
        </div>
      </div>
      <div className="page-body">
        {bookError === null ? null : (
          <Banner
            tone="error"
            action={
              <Button size="sm" onClick={() => void fleet.loadBook()}>
                Retry
              </Button>
            }
          >
            The node book could not be read: {bookError}
          </Banner>
        )}
        {book === null ? (
          <Skeleton />
        ) : nodes.length === 0 ? (
          <EmptyState title="No nodes in the book">
            Add one with <code>vessel nodes add NAME ADDRESS</code>, or pair one
            in Settings.
          </EmptyState>
        ) : (
          <div className={selected === null ? "" : "two-col"}>
            <RowList label="Nodes">
              {nodes.map((node) => (
                <NodeRow
                  key={node.key}
                  node={node}
                  status={statuses[node.key]}
                  selected={node.key === chosen}
                />
              ))}
            </RowList>
            {selected === null ? null : <NodePanel node={selected} />}
          </div>
        )}
      </div>
    </div>
  );
}

/** A node in the list (the kit's Row): its state, its name opening its
 * panel, its facts on one line, its state's word at the right. */
function NodeRow({
  node,
  status,
  selected,
}: {
  node: BookNode;
  status: NodeStatus | undefined;
  selected: boolean;
}) {
  const sessions = useFleetState((state) => state.sessions[node.key]);
  const harnesses = useFleetState((state) => state.harnesses[node.key]);
  const state = nodeState(status);
  return (
    <Row
      selected={selected}
      mark={{ node: <StatusDot tone={state.tone} label={state.word} /> }}
      title={
        <Link
          to="/fleet/$node"
          params={{ node: node.key }}
          aria-current={selected ? "page" : undefined}
        >
          {nodeLabel(node, status)}
        </Link>
      }
      detail={{ facts: nodeFacts(node, status, harnesses, sessions) }}
      control={<Badge tone={state.tone}>{state.word}</Badge>}
    />
  );
}

function NodePanel({ node }: { node: BookNode }) {
  const fleet = useFleet();
  const status = useFleetState((state: FleetState) => state.statuses[node.key]);
  const harnesses = useFleetState((state) => state.harnesses[node.key]);
  const profiles = useFleetState((state) => state.profiles[node.key]);
  const routes = useFleetState((state) => state.models[node.key]);
  const sessions = useFleetState((state) => state.sessions[node.key]);
  const info = status?.state === "connected" ? status.node : null;
  const live = (sessions ?? []).filter(isLive);
  return (
    <aside
      className="panel"
      role="complementary"
      aria-label={nodeLabel(node, status)}
    >
      <h2>{nodeLabel(node, status)}</h2>
      <div className="pad">
        <dl>
          <dt>id</dt>
          <dd>{info?.id ?? node.id ?? "not seen yet"}</dd>
          <dt>address</dt>
          <dd>{node.address}</dd>
          {info === null ? (
            <>
              <dt>state</dt>
              <dd>{nodeState(status).word}</dd>
            </>
          ) : (
            <>
              <dt>host</dt>
              <dd>{info.hostname}</dd>
              <dt>os</dt>
              <dd>
                {info.os} / {info.arch}
              </dd>
              <dt>version</dt>
              <dd>
                {info.version} · protocol{" "}
                {status?.state === "connected" ? status.protocol : ""}
              </dd>
              <dt>started</dt>
              <dd>{when(info.started_at)}</dd>
              <dt>state</dt>
              <dd>{info.state}</dd>
            </>
          )}
        </dl>
        {status?.state === "failed" || status?.state === "reconnecting" ? (
          <div className="panel-section">
            <p className="hint">{status.message}</p>
            <Button
              size="sm"
              onClick={() => {
                void fleet.bridge.op({ op: "retry", node: node.key });
              }}
            >
              Retry now
            </Button>
          </div>
        ) : null}
        {harnesses === undefined ? null : (
          <section className="panel-section">
            <h3>Harnesses</h3>
            <div className="stack">
              {harnesses.map((harness) => {
                const names = (profiles ?? [])
                  .filter((profile) => profile.harness === harness.kind)
                  .map((profile) =>
                    profile.model === undefined || profile.model === null
                      ? profile.name
                      : `${profile.name} → ${profile.model}`,
                  );
                return (
                  <div className="inline" key={harness.kind}>
                    <Badge
                      tone={
                        harness.structured.state === "available"
                          ? "ok"
                          : "neutral"
                      }
                    >
                      {harnessName(harness.kind)}
                    </Badge>
                    <span className="muted">
                      {harness.version ?? "version unknown"}
                      {names.length === 0 ? "" : ` · ${names.join(", ")}`}
                    </span>
                  </div>
                );
              })}
            </div>
          </section>
        )}
        {routes === undefined ||
        routes === null ||
        routes.length === 0 ? null : (
          <section className="panel-section">
            <h3>Model routes</h3>
            <div className="stack">
              {routes.map((route) => (
                <div className="inline" key={route.name}>
                  <Tooltip label={apiName(route.api)}>
                    <Badge look="accent">{route.name}</Badge>
                  </Tooltip>
                  <span className="muted">
                    {routeLine(route)}
                    {route.small_model === undefined ||
                    route.small_model === null
                      ? ""
                      : ` · small ${route.small_model}`}
                  </span>
                </div>
              ))}
            </div>
          </section>
        )}
        {sessions === undefined ? null : (
          <section className="panel-section">
            <h3>Live</h3>
            <p className="hint">
              {live.length} live ·{" "}
              <Link to="/sessions" search={{ node: node.key }}>
                open sessions
              </Link>
            </p>
            <div className="stack">
              {live.slice(0, 6).map((session) => (
                <Link
                  key={session.id}
                  className="path"
                  to="/sessions/$node/$id"
                  params={{ node: node.key, id: session.id }}
                >
                  {shortId(session.id)} · {harnessName(session.spec.harness)} ·{" "}
                  {session.spec.mode === "terminal"
                    ? "terminal"
                    : (session.spec.cwd ?? "~")}
                </Link>
              ))}
            </div>
          </section>
        )}
      </div>
    </aside>
  );
}
