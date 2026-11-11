// The fleet controller, shared through React context, and the page-level
// UI state the chrome needs (the breadcrumb, a request for the
// new-session page).

import { createContext, useContext, useEffect } from "react";

import type { Fleet, FleetState } from "./data/fleet";
import { Store, useStore } from "./data/store";

export const FleetContext = createContext<Fleet | null>(null);

/** The fleet controller. */
export function useFleet(): Fleet {
  const fleet = useContext(FleetContext);
  if (fleet === null) {
    throw new Error("useFleet outside the app");
  }
  return fleet;
}

/** A slice of the fleet's state (see `useStore` for what `select` may
 * return). */
export function useFleetState<S>(select: (state: FleetState) => S): S {
  return useStore(useFleet().store, select);
}

/** One step of the topbar's breadcrumb. */
export interface Crumb {
  label: string;
  to?: string;
  mono?: boolean;
}

export interface Chrome {
  crumbs: readonly Crumb[];
  /** A request for the new-session page on this node (`null`: the page
   * picks), from code outside the router's tree, such as the palette's
   * commands. The shell turns it into the page and clears it
   * (features/shell/use-new-session.ts). */
  newSession: { node: string | null } | null;
}

export const chrome = new Store<Chrome>({ crumbs: [], newSession: null });

/** Whether two breadcrumbs say the same. */
function sameCrumbs(a: readonly Crumb[], b: readonly Crumb[]): boolean {
  return (
    a.length === b.length &&
    a.every(
      (crumb, index) =>
        crumb.label === b[index]?.label &&
        crumb.to === b[index].to &&
        crumb.mono === b[index].mono,
    )
  );
}

/** Set the breadcrumb while a page is shown. A page rebuilds its crumbs
 * on every render; they are stored only when they say something else, so
 * the chrome does not redraw for each of the page's renders. */
export function useCrumbs(crumbs: readonly Crumb[]): void {
  useEffect(() => {
    chrome.update((state) =>
      sameCrumbs(state.crumbs, crumbs) ? state : { ...state, crumbs },
    );
  }, [crumbs]);
}

export function useChrome<S>(select: (state: Chrome) => S): S {
  return useStore(chrome, select);
}

export function openNewSession(node: string | null): void {
  chrome.update((state) => ({ ...state, newSession: { node } }));
}

export function closeNewSession(): void {
  chrome.update((state) => ({ ...state, newSession: null }));
}
