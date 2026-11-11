// The chrome every page sits in (DESIGN.md §2.2): topbar, sidebar, main.

import { Link, Outlet, useRouterState } from "@tanstack/react-router";
import { Fragment, type ReactElement } from "react";

import { openNewSession, useChrome, useFleet, useFleetState } from "../context";
import { isWorking, needsYou, placement } from "../data/board";
import {
  isLive,
  nodeState,
  sessionTitle,
  shortId,
  type StateTone,
} from "../data/format";
import { nodeLabel, selectedNode } from "../data/nodes";
import { SidebarSearch } from "../features/palette/palette";
import { useGoShortcuts } from "../features/palette/shortcuts";
import { ThemeMenu } from "../features/theme/theme";
import { NewSessionDialog } from "../routes/new-session";
import type { SessionInfo } from "../wire/proto";
import { Banner, Button, Icon, IconButton, StatusDot, Tooltip } from "./kit";
import { Toasts } from "./toasts";

const MARK = (
  <span className="mark" aria-hidden="true">
    <svg
      width="12"
      height="12"
      viewBox="0 0 12 12"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M2 8.5 6 2.5 10 8.5" />
      <path d="M4 8.5h4" />
    </svg>
  </span>
);

export function Shell(): ReactElement {
  const newSession = useChrome((chrome) => chrome.newSession);
  useGoShortcuts();
  return (
    <>
      <a className="skip" href="#main">
        Skip to main
      </a>
      {/* The sidebar and the margin around the page are one shell; the
          page is a framed surface set into it, with its title bar inside
          the frame (DESIGN.md §2.2). */}
      <div className="app">
        <Sidebar />
        <div className="frame-wrap">
          <div className="frame">
            <FrameTop />
            <main className="main" id="main" role="main">
              <Disconnected />
              <Outlet />
            </main>
          </div>
        </div>
      </div>
      {newSession === null ? null : <NewSessionDialog node={newSession.node} />}
      <Toasts />
    </>
  );
}

/** After repeated failures the tab stops redialling (bridge/client.ts):
 * say so, and offer to try again with the same key. */
function Disconnected() {
  const fleet = useFleet();
  const link = useFleetState((state) => state.link);
  if (link.kind !== "disconnected") {
    return null;
  }
  return (
    <Banner
      tone="error"
      action={
        <Button
          size="sm"
          onClick={() => {
            fleet.bridge.retry();
          }}
        >
          Retry
        </Button>
      }
    >
      Disconnected — vessel ui may have restarted. If it did, open the address
      it printed.
    </Banner>
  );
}

/** The title bar inside the frame: where the page is (the last crumb is
 * the page's one heading), the one primary action, and this client's
 * link to `vessel ui`. */
function FrameTop() {
  const crumbs = useChrome((chrome) => chrome.crumbs);
  const link = useFleetState((state) => state.link);
  const selected = useFleetState((state) => selectedNode(state, undefined));
  const chip: { tone: StateTone; text: string } =
    link.kind === "open"
      ? { tone: "ok", text: "vessel ui · connected" }
      : link.kind === "refused"
        ? { tone: "danger", text: "vessel ui · refused" }
        : link.kind === "disconnected"
          ? { tone: "danger", text: "vessel ui · disconnected" }
          : { tone: "warn", text: "vessel ui · reconnecting" };
  return (
    <header className="frame-top" role="banner">
      <nav className="crumb" aria-label="Breadcrumb">
        {crumbs.map((crumb, index) => {
          const last = index === crumbs.length - 1;
          return (
            <Fragment key={`${crumb.label}-${index}`}>
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
      <div className="frame-actions">
        <Link
          className={link.kind === "open" ? "chip" : "chip reconnecting"}
          to="/settings"
        >
          <StatusDot tone={chip.tone} label={chip.text} />
          {chip.text}
        </Link>
        <Button
          variant="primary"
          size="sm"
          icon="plus"
          onClick={() => {
            openNewSession(selected);
          }}
        >
          New
        </Button>
      </div>
    </header>
  );
}

/** How many of a node's live sessions the sidebar lists before it links
 * to the board for the rest: the sidebar is a way to switch, not a second
 * board. */
const SIDEBAR_SESSIONS = 5;

/** The mark a session row leads with: a spinner while its agent works, a
 * filled dot in the needs-you colour while it waits on its owner, else a
 * hollow dot. From the node's own placement (protocol 8); a session of an
 * older node gets the hollow dot, since nothing says what it is doing. */
function SessionMark({ session }: { session: SessionInfo }) {
  const placed = placement(session);
  if (needsYou(placed)) {
    return (
      <span className="sdot needs-you" role="img" aria-label="needs you" />
    );
  }
  if (isWorking(placed)) {
    return <span className="spinner" role="img" aria-label="working" />;
  }
  return <span className="sdot" aria-hidden="true" />;
}

function Sidebar() {
  const book = useFleetState((state) => state.book);
  const statuses = useFleetState((state) => state.statuses);
  const sessions = useFleetState((state) => state.sessions);
  const selected = useFleetState((state) => selectedNode(state, undefined));
  const path = useRouterState({ select: (state) => state.location.pathname });
  const live = Object.values(sessions).reduce(
    (total, list) => total + list.filter(isLive).length,
    0,
  );
  const item = (active: boolean, extra = "") =>
    `nav-item${extra}${active ? " active" : ""}`;
  return (
    <nav className="sidebar" role="navigation" aria-label="Fleet">
      <div className="side-head">
        <Link className="brand" to="/fleet">
          {MARK}
          <span>Vessel</span>
        </Link>
      </div>
      <SidebarSearch />
      <div className="side-list">
        <Link className={item(path === "/fleet")} to="/fleet">
          <Icon name="fleet" size={16} />
          <span className="label">Fleet</span>
        </Link>
        <Link className={item(path === "/sessions")} to="/sessions">
          <Icon name="sessions" size={16} />
          <span className="label">Sessions</span>
          <span className="count">{live}</span>
        </Link>
        <Link className={item(path === "/sync")} to="/sync">
          <Icon name="sync" size={16} />
          <span className="label">Sync</span>
        </Link>
        <Link className={item(path === "/devices")} to="/devices">
          <Icon name="devices" size={16} />
          <span className="label">Devices</span>
        </Link>
      </div>
      <div className="side-label">
        <span>Nodes</span>
        <Tooltip label="New session">
          <IconButton
            label="New session"
            icon="plus"
            size="xs"
            onClick={() => {
              openNewSession(selected);
            }}
          />
        </Tooltip>
      </div>
      <div className="side-scroll">
        {(book ?? []).map((node) => {
          const status = statuses[node.key];
          const state = nodeState(status);
          const running = (sessions[node.key] ?? []).filter(isLive);
          const connected = status?.state === "connected";
          return (
            <Fragment key={node.key}>
              <Link
                className={item(
                  path.startsWith(`/fleet/${node.key}`),
                  connected ? " node" : " node offline",
                )}
                to="/fleet/$node"
                params={{ node: node.key }}
              >
                <StatusDot tone={state.tone} label={state.word} />
                <span className="label">{nodeLabel(node, status)}</span>
                {connected ? (
                  <span className="count">{running.length}</span>
                ) : (
                  <span className="dash">—</span>
                )}
              </Link>
              {/* The node's live sessions, to switch between them from any
                  page (the board's archive has the ones that ended). */}
              {running.slice(0, SIDEBAR_SESSIONS).map((session) => (
                <Link
                  key={session.id}
                  className={item(
                    path === `/sessions/${node.key}/${session.id}`,
                    " sub",
                  )}
                  to="/sessions/$node/$id"
                  params={{ node: node.key, id: session.id }}
                >
                  <span className="slot">
                    <SessionMark session={session} />
                  </span>
                  <span className="label">
                    {sessionTitle(session.spec.cwd)}
                  </span>
                  <span className="id">{shortId(session.id)}</span>
                </Link>
              ))}
              {running.length > SIDEBAR_SESSIONS ? (
                <Link
                  className="nav-item more"
                  to="/sessions"
                  search={{ node: node.key }}
                >
                  {running.length - SIDEBAR_SESSIONS} more
                </Link>
              ) : null}
            </Fragment>
          );
        })}
      </div>
      <div className="side-foot">
        <Link className={item(path === "/settings")} to="/settings">
          <Icon name="settings" size={16} />
          <span className="label">Settings</span>
        </Link>
        <ThemeMenu />
      </div>
    </nav>
  );
}
