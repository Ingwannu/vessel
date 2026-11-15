// Closed status words are shared by the visible echo and live announcement.
import type { NativeCommandStatus } from "../wire/proto";
const STATUS = {
  compact_started: "Context compaction started",
  goal_set: "Goal set",
  goal_paused: "Goal paused",
  goal_resumed: "Goal resumed",
  goal_cleared: "Goal cleared",
  goal_usage: "Goal command help",
} satisfies Record<NativeCommandStatus, string>;
export function commandStatus(status: NativeCommandStatus): string {
  return STATUS[status];
}
