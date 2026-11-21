// The shown tab's store: listeners hear a change only, so presence sends a
// heartbeat only when what the window shows changes.

import { expect, test } from "vitest";

import { fixtureId } from "../testing/ids";
import { setShownTab, shownTab, subscribeShownTab } from "./shown-terminal";

const LAB = fixtureId("NodeKey", "lab");

test("the shown tab tells its listeners of a change only", () => {
  let heard = 0;
  const stop = subscribeShownTab(() => {
    heard += 1;
  });
  const terminal = fixtureId("TerminalId", "t1");
  setShownTab({ node: LAB, kind: "terminal", terminal });
  setShownTab({ node: LAB, kind: "terminal", terminal });
  expect(heard).toBe(1);
  setShownTab({ node: LAB, kind: "aside" });
  setShownTab({ node: LAB, kind: "aside" });
  expect(heard).toBe(2);
  expect(shownTab()).toEqual({ node: LAB, kind: "aside" });
  setShownTab(null);
  expect(heard).toBe(3);
  stop();
  setShownTab({ node: LAB, kind: "agent" });
  expect(heard).toBe(3);
  setShownTab(null);
});
