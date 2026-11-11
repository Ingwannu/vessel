// Settings (track W2): one route, `/settings/<section>`, composing the
// section the address names. An unknown section goes to the default one;
// the sections are `data/settings.ts`'s one list, which the sidebar draws
// too.

import { Navigate } from "@tanstack/react-router";
import type { ReactElement } from "react";

import { useCrumbs } from "../context";
import { settingsSectionLabel } from "../data/settings";
import { useSettingsPlace } from "../data/settings-place";
import { AgentsSection } from "../features/machines/agents-section";
import { MachinesSection } from "../features/machines/machines-section";
import { AboutSection } from "../features/settings/about";
import { AppearanceSection } from "../features/settings/appearance";
import { DevicesSection } from "../features/settings/devices-section";
import { ModelsSection } from "../features/settings/models";
import { SyncSection } from "../features/settings/sync-section";

export function SettingsPage(): ReactElement {
  const place = useSettingsPlace();
  useCrumbs([
    { label: "Settings", to: "/settings" },
    { label: settingsSectionLabel(place.section) },
  ]);
  if (!place.exact) {
    return (
      <Navigate
        to="/settings/$section"
        params={{ section: place.section }}
        replace
      />
    );
  }
  switch (place.section) {
    case "appearance":
      return <AppearanceSection />;
    case "agents":
      return <AgentsSection machine={place.machine} agent={place.agent} />;
    case "models":
      return <ModelsSection machine={place.machine} />;
    case "machines":
      return <MachinesSection machine={place.machine} />;
    case "devices":
      return <DevicesSection machine={place.machine} />;
    case "sync":
      return <SyncSection machine={place.machine} />;
    case "about":
      return <AboutSection />;
  }
}
