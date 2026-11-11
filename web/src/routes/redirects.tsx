// Addresses that moved (track W2): Fleet, Sync and Devices are sections
// of Settings now, and `/settings` alone opens its default section. Each
// keeps the machine its old address named.
//
// Components rather than `beforeLoad` redirects: a redirect's typed
// target would make the route tree's type refer to itself (as `app.tsx`
// found for the index before). A redirect renders once more after the
// address has changed, before it unmounts; reading the machine from the
// new address then would replace the redirect's own target, so each
// reads it only while the address is still the old one.

import { Navigate } from "@tanstack/react-router";
import type { ReactElement } from "react";

import { DEFAULT_SETTINGS_SECTION } from "../data/settings";
import { useWhere } from "../data/where";

/** `/settings` → its default section. */
export function SettingsIndex(): ReactElement {
  return (
    <Navigate
      to="/settings/$section"
      params={{ section: DEFAULT_SETTINGS_SECTION }}
      replace
    />
  );
}

/** `/fleet` and `/fleet/<machine>` → Settings › Machines. */
export function FleetRedirect(): ReactElement | null {
  const where = useWhere();
  if (where.segments[0] !== "fleet") {
    return null;
  }
  const machine = where.segments[1];
  return (
    <Navigate
      to="/settings/$section"
      params={{ section: "machines" }}
      search={machine === undefined ? {} : { machine }}
      replace
    />
  );
}

/** `/sync?node=` → Settings › Sync. */
export function SyncRedirect(): ReactElement | null {
  const where = useWhere();
  if (where.segments[0] !== "sync") {
    return null;
  }
  const machine = where.node;
  return (
    <Navigate
      to="/settings/$section"
      params={{ section: "sync" }}
      search={machine === undefined ? {} : { machine }}
      replace
    />
  );
}

/** `/devices?node=` → Settings › Devices. */
export function DevicesRedirect(): ReactElement | null {
  const where = useWhere();
  if (where.segments[0] !== "devices") {
    return null;
  }
  const machine = where.node;
  return (
    <Navigate
      to="/settings/$section"
      params={{ section: "devices" }}
      search={machine === undefined ? {} : { machine }}
      replace
    />
  );
}
