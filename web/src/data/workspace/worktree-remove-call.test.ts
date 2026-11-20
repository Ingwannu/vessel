import { describe, expect, it, vi } from "vitest";

import { Bridge } from "../../bridge/client";
import { fixtureId } from "../../testing/ids";
import { Fleet } from "../fleet";
import { removeWorktree } from "./worktrees";

describe("worktree.remove asks to end terminals only when told (ADR 0121)", () => {
  it("leaves end_terminals out unless the dialog said the count", async () => {
    const fleet = new Fleet(
      new Bridge(
        { kind: "bootstrap", key: "a".repeat(64) },
        "ws://127.0.0.1:1/api/ws",
      ),
    );
    const call = vi
      .spyOn(fleet.bridge, "call")
      .mockResolvedValue({ outcome: "removed" });
    const node = fixtureId("NodeKey", "laptop");
    const worktree = fixtureId("WorktreeId", "w-fix");
    await removeWorktree(fleet, node, worktree, "keep");
    await removeWorktree(fleet, node, worktree, "discard", true);
    expect(call.mock.calls).toEqual([
      [node, "worktree.remove", { worktree, if_lost: "keep" }],
      [
        node,
        "worktree.remove",
        { worktree, if_lost: "discard", end_terminals: true },
      ],
    ]);
  });
});
