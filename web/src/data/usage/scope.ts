// Historical ledger names, not configured accounts. None selects every
// profile in usage.query; it must never mean the current default login.
import type { NodeKey } from "../../wire/bridge";
import type {
  AgentInfo,
  AgentUsage,
  HarnessRef,
  UsageReport,
  UsageScope,
} from "../../wire/proto";
import { agentKey, agentName } from "../agents";

export type RollingUsageScope = Exclude<UsageScope, { kind: "session" }>;
export const MACHINE_USAGE = { kind: "machine" } as const;
type UsageName = Readonly<Pick<AgentUsage, "harness" | "profile">>;

export interface UsageTarget {
  node: NodeKey;
  scope: RollingUsageScope;
}

/** A machine change forgets the old scope, including on a return trip. */
export function usageTargetFor(
  target: UsageTarget,
  node: NodeKey,
): UsageTarget {
  return target.node === node ? target : { node, scope: MACHINE_USAGE };
}

/** One bounded set of names from this machine's last all-agent report. */
export interface UsageNames {
  node: NodeKey;
  groups: readonly UsageName[];
  incomplete: boolean;
}

export interface UsageChoice {
  value: string;
  label: string;
  scope: RollingUsageScope;
}

function copiedAgent(harness: HarnessRef): HarnessRef {
  return typeof harness === "string" ? harness : { ...harness };
}

/** Queued requests own their scope, including an added agent's object identity. */
export function copiedScope(scope: RollingUsageScope): RollingUsageScope {
  switch (scope.kind) {
    case "machine":
      return MACHINE_USAGE;
    case "agent":
      return { ...scope, harness: copiedAgent(scope.harness) };
  }
}

/** A profile name cannot collide with All profiles or another agent. */
export function usageScopeKey(scope: RollingUsageScope): string {
  switch (scope.kind) {
    case "machine":
      return "machine";
    case "agent":
      return JSON.stringify(["agent", agentKey(scope.harness), scope.profile]);
  }
}

/** Do not cache cost/tokens in the name catalogue or merge lists without a bound. */
export function usageNames(node: NodeKey, report: UsageReport): UsageNames {
  return {
    node,
    incomplete: report.incomplete,
    groups: report.agents.map(({ harness, profile }) => ({
      harness: copiedAgent(harness),
      profile,
    })),
  };
}

/** Only recorded named profiles can be selected individually; unnamed
 * reports remain in the agent's All profiles total. */
export function usageChoices(
  groups: readonly UsageName[],
  selected: RollingUsageScope,
  agents: readonly AgentInfo[] = [],
): readonly UsageChoice[] {
  const scopes = new Map<string, RollingUsageScope>();
  const add = (scope: RollingUsageScope) => {
    scopes.set(usageScopeKey(scope), copiedScope(scope));
  };
  add(MACHINE_USAGE);
  for (const { harness, profile } of groups) {
    add({ kind: "agent", harness, profile: null });
    if (profile !== null) add({ kind: "agent", harness, profile });
  }
  // A selected historical name stays visible if a later report omits it.
  // This does not claim that a name has any reports in the new interval.
  add(selected);
  return [...scopes].map(([value, scope]) => ({
    value,
    scope,
    label:
      scope.kind === "machine"
        ? "All agents"
        : `${agentName(scope.harness, agents)} · ${scope.profile === null ? "All profiles" : `Profile: ${scope.profile}`}`,
  }));
}
