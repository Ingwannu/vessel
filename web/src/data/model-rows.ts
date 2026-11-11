// What Settings › Models says of a machine's model routes and profiles
// (`model.list`, `profile.list`; ADRs 0011 and 0015): one line of facts
// per row, in the page's words. Read-only: routes and profiles are edited
// in the machine's own `models.toml` and `profiles.toml`.

import type { ModelRoute, ProfileInfo } from "../wire/proto";
import { agentName } from "./agents";
import { apiName } from "./models";

/** A route's line: its provider, the model it calls, the small model it
 * uses for quick calls, and the API it speaks. */
export function routeFacts(route: ModelRoute): string[] {
  const small = route.small_model ?? null;
  return [
    route.provider,
    route.model,
    ...(small === null ? [] : [`small: ${small}`]),
    apiName(route.api),
  ];
}

/** A profile's line: the agent it runs, the route its models come from
 * (else the agent's own), and how many variables it sets. */
export function profileFacts(profile: ProfileInfo): string[] {
  const route = profile.model ?? null;
  const sets = profile.sets.length;
  return [
    agentName(profile.harness),
    route === null ? "the agent's own models" : `models from ${route}`,
    ...(sets === 0
      ? []
      : [`sets ${String(sets)} ${sets === 1 ? "variable" : "variables"}`]),
  ];
}
