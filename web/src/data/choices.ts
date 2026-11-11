// The options the page's selects offer, decided here and tested, so a
// screen only draws them (the plan's "decisions are data"). Each option
// has the shape the kit's Select takes (`ui/kit/choice.ts`): data does
// not import the kit, and TypeScript checks the two shapes agree where a
// screen passes one to the other.

import type { BookNode, NodeStatus } from "../wire/bridge";
import type {
  Harness,
  HarnessKind,
  ModelRoute,
  ProfileInfo,
} from "../wire/proto";
import { harnessName, nodeState } from "./format";
import { byProvider, profileOn } from "./models";
import { nodeLabel } from "./nodes";

/** One option: a value, what a person reads, and why it may be off. */
export interface Option<V extends string> {
  value: V;
  label: string;
  detail?: string;
  disabled?: boolean;
  group?: string;
}

/** The book's nodes; only a connected one can take a session. */
export function nodeOptions(
  book: readonly BookNode[],
  statuses: Readonly<Record<string, NodeStatus>>,
): Option<string>[] {
  return book.map((node) => {
    const status = statuses[node.key];
    return {
      value: node.key,
      label: nodeLabel(node, status),
      detail: nodeState(status).word,
      disabled: status?.state !== "connected",
    };
  });
}

/** The harnesses a node discovered, by their kind. */
export function harnessOptions(
  harnesses: readonly Harness[],
): Option<HarnessKind>[] {
  return harnesses.map((harness) => ({
    value: harness.kind,
    label: harnessName(harness.kind),
  }));
}

/** A profile's label, and the route it runs on as its detail. */
function profileOption(profile: ProfileInfo): Option<string> {
  const route = profile.model ?? null;
  return {
    value: profile.name,
    label: profile.label ?? profile.name,
    ...(route === null ? {} : { detail: route }),
  };
}

/** `""` (the harness's default profile), then `profiles`. */
export function profileOptions(
  profiles: readonly ProfileInfo[],
): Option<string>[] {
  return [{ value: "", label: "default" }, ...profiles.map(profileOption)];
}

/** Where a session may move (ADR 0015): only to a profile on the route it
 * runs on, so the default profile (no route) only when it has none. */
export function moveOptions(
  profiles: readonly ProfileInfo[],
  route: string | null,
): Option<string>[] {
  return [
    { value: "", label: "default", disabled: route !== null },
    ...profiles.map((profile) => ({
      ...profileOption(profile),
      disabled: (profile.model ?? null) !== route,
    })),
  ];
}

/** The model field (DESIGN.md §7.5): `""` for the harness's own provider,
 * then the node's routes by provider. A route no profile of the harness
 * names cannot be picked: a route reaches a session only through one. */
export function routeOptions(
  routes: readonly ModelRoute[],
  profiles: readonly ProfileInfo[],
): Option<string>[] {
  const own: Option<string> = {
    value: "",
    label: "harness default",
    group: "The harness's own provider",
  };
  const routed = byProvider(routes).flatMap(([provider, list]) =>
    list.map((route): Option<string> => {
      const usable = profileOn(profiles, route.name) !== undefined;
      return {
        value: route.name,
        label: route.name,
        detail: usable ? route.model : `${route.model} · no profile`,
        disabled: !usable,
        group: provider,
      };
    }),
  );
  return [own, ...routed];
}
