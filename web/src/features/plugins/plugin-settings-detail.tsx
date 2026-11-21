// Settings › Plugins › a plugin's settings screen (ADR 0115, "Surfaces"):
// Paseo's `PluginSettingsContent` (packages/app/src/plugins/settings/
// index.tsx:68-153) and its detail title `<plugin id> · <title>`
// (packages/app/src/screens/settings-screen.tsx:1314-1321).
//
// [Decision Log]
// - Purpose: a plugin's declared settings screen, opened from its row's
//   menu or a Command Center item, edits its documents in place.
// - Paseo's states, in its order: the machine is not connected (offline),
//   too old to serve settings controls (update), or the plugin no longer
//   declares the screen (unavailable); Back to plugins always works.
// - Only a device holding admin reaches it (settings reads need admin, so
//   the screens are not listed elsewhere; an address typed by hand reads as
//   unavailable).

import { useNavigate } from "@tanstack/react-router";
import { type ReactElement, useMemo } from "react";

import { useFleetState } from "../../context";
import { pluginCatalogs } from "../../data/plugin-catalog";
import { findSurface, SURFACE_WORDS } from "../../data/plugin-surfaces";
import { useStore } from "../../data/store";
import type { NodeKey } from "../../wire/bridge";
import { PLUGIN_SURFACES_PROTOCOL } from "../../wire/proto";
import { Button } from "../../ui/kit";
import { SettingsPage, SettingsSection } from "../settings/settings-page";
import { PluginSurface } from "./surface/plugin-surface";
import { useOpenSurface } from "./use-open-surface";

export function PluginSettingsDetail({
  machine,
  plugin,
  screen,
}: {
  machine: NodeKey | undefined;
  plugin: string;
  screen: string;
}) {
  const navigate = useNavigate();
  const open = useOpenSurface();
  const status = useFleetState((state) =>
    machine === undefined ? undefined : state.statuses[machine],
  );
  const held = useStore(pluginCatalogs, (all) =>
    machine === undefined ? undefined : all.get(machine),
  );
  const found = useMemo(
    () =>
      machine === undefined
        ? null
        : findSurface(held, machine, plugin, "settings_screen", screen),
    [held, machine, plugin, screen],
  );
  const back = () => {
    void navigate({
      to: "/settings/$section",
      params: { section: "plugins" },
      search: machine === undefined ? {} : { machine },
    });
  };
  let body: ReactElement;
  if (machine === undefined || status?.state !== "connected") {
    body = <p className="plugin-doc-note">{SURFACE_WORDS.settingsOffline}</p>;
  } else if (status.protocol < PLUGIN_SURFACES_PROTOCOL) {
    body = <p className="plugin-doc-note">{SURFACE_WORDS.settingsUpdate}</p>;
  } else if (found === null) {
    body = (
      <p className="plugin-doc-note">{SURFACE_WORDS.settingsUnavailable}</p>
    );
  } else {
    const scope = {
      node: machine,
      installation: found.installation,
      mayInvoke: found.mayInvoke,
      context: { kind: "global" } as const,
      open,
    };
    body = (
      <PluginSurface scope={scope} surface={found.surface} machine={machine} />
    );
  }
  return (
    <SettingsPage>
      <div>
        <Button variant="ghost" size="sm" icon="back" onClick={back}>
          {SURFACE_WORDS.backToPlugins}
        </Button>
      </div>
      <SettingsSection title={`${plugin} · ${found?.item.title ?? screen}`}>
        {body}
      </SettingsSection>
    </SettingsPage>
  );
}
