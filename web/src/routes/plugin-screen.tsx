// `/plugin/$node/$plugin/$screen`: a plugin's full-page screen (ADR 0115,
// "Surfaces"), Paseo's plugin screen route
// (packages/app/src/plugins/surface-screen.tsx:140-250): the host owns the
// header (title, machine, close) and the body is the plugin's.
//
// [Decision Log]
// - Purpose: a sidebar row or a Command Center item opens a plugin screen
//   by machine, plugin and screen id; a reload or a shared link must open
//   it again (its first segment is in APP_ROUTES, crates/vessel-web).
// - Paseo's states: a screen whose machine is not connected says the
//   machine is offline, a screen no running plugin declares is
//   unavailable; Close goes back, or to the sessions when there is no page
//   to go back to. The body keeps no params (stated difference, ADR 0115).
// - Every device that reads the catalog draws screens; what needs admin is
//   disabled with one muted line (coordinator ruling on P3b, Q1).

import "../features/plugins/surface/surface.css";

import { useCanGoBack, useNavigate, useRouter } from "@tanstack/react-router";
import { type ReactElement, useEffect, useMemo } from "react";

import { nodeKeyOf } from "../bridge/ids";
import { useCrumbs, useFleetState } from "../context";
import { pluginCatalogs } from "../data/plugin-catalog";
import {
  findSurface,
  rememberScreen,
  SURFACE_WORDS,
  type SurfaceTarget,
} from "../data/plugin-surfaces";
import { useStore } from "../data/store";
import { useWhere } from "../data/where";
import { PluginSurface } from "../features/plugins/surface/plugin-surface";
import { useOpenSurface } from "../features/plugins/use-open-surface";
import { IconButton } from "../ui/kit";

export function PluginScreenPage(): ReactElement {
  const where = useWhere();
  const node = nodeKeyOf(where.segments[1] ?? "");
  const plugin = where.segments[2] ?? "";
  const screen = where.segments[3] ?? "";
  const navigate = useNavigate();
  const open = useOpenSurface();
  const connected = useFleetState(
    (state) => node !== null && state.statuses[node]?.state === "connected",
  );
  const held = useStore(pluginCatalogs, (all) =>
    node === null ? undefined : all.get(node),
  );
  const found = useMemo(
    () =>
      node === null ? null : findSurface(held, node, plugin, "screen", screen),
    [held, node, plugin, screen],
  );
  const title = found?.item.title ?? plugin;
  // The frame's crumbs name the machine and the plugin; the header below
  // carries the screen's title, so the two do not repeat each other.
  const crumbs = useMemo(
    () => [{ label: node ?? "", mono: true }, { label: plugin }],
    [node, plugin],
  );
  useCrumbs(crumbs);
  // The sidebar's merged row opens this machine next (node review R4).
  useEffect(() => {
    if (node !== null && found !== null) rememberScreen(node, plugin, screen);
  }, [node, plugin, screen, found]);
  // Paseo's close: back when there is a page to go back to, else replace
  // (surface-screen.tsx:161-164; node review note 3).
  const canGoBack = useCanGoBack();
  const router = useRouter();
  const close = () => {
    if (canGoBack) router.history.back();
    else void navigate({ to: "/sessions", replace: true });
  };
  let body: ReactElement;
  if (node === null || found === null) {
    body = <p className="plugin-unavailable">{SURFACE_WORDS.unavailable}</p>;
  } else if (!connected) {
    body = <p className="plugin-unavailable">{SURFACE_WORDS.offline}</p>;
  } else {
    const scope = {
      node,
      installation: found.installation,
      mayInvoke: found.mayInvoke,
      title: found.item.title,
      context: { kind: "global" } as const,
      open: (target: SurfaceTarget) => {
        open(target);
      },
    };
    body = (
      <PluginSurface scope={scope} surface={found.surface} machine={node} />
    );
  }
  return (
    <div className="plugin-screen">
      <header className="plugin-screen-head">
        <h1>{title}</h1>
        <IconButton
          label={SURFACE_WORDS.close}
          icon="close"
          variant="ghost"
          size="sm"
          onClick={close}
        />
      </header>
      <div className="plugin-screen-body">{body}</div>
    </div>
  );
}
