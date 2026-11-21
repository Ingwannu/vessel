// Opening a plugin surface (ADR 0115, "Surfaces"): a screen is its own
// route; a settings screen is Settings › Plugins' detail, as Paseo routes
// `openSettings(id)` to `/settings/hosts/<host>/plugins/<plugin>/<screen>`
// (packages/app/src/plugins/settings/routes.ts).

import { useNavigate } from "@tanstack/react-router";
import { useCallback } from "react";

import type { SurfaceTarget } from "../../data/plugin-surfaces";

/** A function that opens `target`. */
export function useOpenSurface(): (target: SurfaceTarget) => void {
  const navigate = useNavigate();
  return useCallback(
    (target: SurfaceTarget) => {
      if (target.kind === "screen") {
        void navigate({
          to: "/plugin/$node/$plugin/$screen",
          params: {
            node: target.node,
            plugin: target.plugin,
            screen: target.screen,
          },
        });
        return;
      }
      void navigate({
        to: "/settings/$section",
        params: { section: "plugins" },
        search: {
          machine: target.node,
          plugin: target.plugin,
          screen: target.screen,
        },
      });
    },
    [navigate],
  );
}
