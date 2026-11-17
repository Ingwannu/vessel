// The label hub against a node that never waits: every machine reached
// through fleet forwarding answers its long reads at once (a Delegated
// caller), so the loops must keep their own pace (pace.ts).

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

import type { Method, ParamsOf, ResultOf } from "../../bridge/frames";
import { fixtureId } from "../../testing/ids";
import type { NodeKey } from "../../wire/bridge";
import { connectedAs } from "../fixtures/machines";
import type { FleetState } from "../fleet";
import { Store } from "../store";
import { QUIET_MS } from "../workspace/pace";
import { WorkspaceLabels } from "./hub";

const laptop = fixtureId("NodeKey", "laptop");
const generation = fixtureId("WorkspaceLabelGeneration", "g1");

type Answers = { [M in Method]?: (params: ParamsOf<M>) => ResultOf<M> };

/** A bridge that answers at once with nothing new, and stops answering
 * after `limit` calls so an unpaced loop fails the test instead of
 * spinning it. */
function eagerBridge(answers: Answers, limit: number) {
  const asked: Method[] = [];
  return {
    asked,
    call<M extends Method>(
      _node: NodeKey,
      method: M,
      params: ParamsOf<M>,
    ): Promise<ResultOf<M>> {
      asked.push(method);
      const answer = answers[method];
      if (asked.length > limit || answer === undefined) {
        return new Promise(() => undefined);
      }
      return Promise.resolve(answer(params));
    },
  };
}

describe("the label hub", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  test("asks a machine that answers at once about once per quiet time", async () => {
    const bridge = eagerBridge(
      {
        "workspace.registry": () => ({
          node: fixtureId("NodeId", "n-1"),
          revision: 1,
          workspaces: [],
          memberships: [],
        }),
        "workspace.label.list": () => ({
          data: { mode: "snapshot", labels: [] },
          head: { generation, after: 0 },
        }),
      },
      200,
    );
    const store = new Store<Pick<FleetState, "statuses">>({
      statuses: { [laptop]: connectedAs("laptop") },
    });
    const hub = new WorkspaceLabels({ store, bridge });
    const release = hub.hold();
    const windows = 5;
    await vi.advanceTimersByTimeAsync(QUIET_MS * windows);
    release();
    const count = (method: Method) =>
      bridge.asked.filter((each) => each === method).length;
    // One read at the start, then one per quiet time that has passed.
    expect(count("workspace.registry")).toBeLessThanOrEqual(windows + 1);
    expect(count("workspace.label.list")).toBeLessThanOrEqual(windows + 1);
    expect(count("workspace.registry")).toBeGreaterThanOrEqual(windows);
    expect(hub.store.get().catalogs[laptop]?.state).toBe("online");
  });
});
