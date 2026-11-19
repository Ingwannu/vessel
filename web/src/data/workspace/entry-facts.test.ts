import { describe, expect, it, vi } from "vitest";

import { Bridge } from "../../bridge/client";
import { fixtureId } from "../../testing/ids";
import type {
  InspectedDirectory,
  InspectedEntry,
  RelPath,
  WorkspaceRoot,
} from "../../wire/proto";
import { Fleet } from "../fleet";
import { attempt } from "../outcome";
import { inspectListed } from "./entry-facts";
import type { Listing } from "./file-tree";
import { BACKGROUND_SLOTS } from "./file-work";
import { inspectDir } from "./files";

vi.mock("./files", () => ({ inspectDir: vi.fn() }));

describe("the Files tab's facts (Modified and Size orders)", () => {
  it("inspects more than four listed folders a few at a time, and asks a refused one again", async () => {
    const fleet = new Fleet(
      new Bridge(
        { kind: "bootstrap", key: "a".repeat(64) },
        "ws://127.0.0.1:1/api/ws",
      ),
    );
    const node = fixtureId("NodeKey", "facts");
    const root: WorkspaceRoot = {
      kind: "project",
      project: fixtureId("ProjectId", "shop"),
    };
    // The root and five open folders: six listings.
    const listings = new Map<RelPath, Listing>(
      ["", "a", "b", "c", "d", "e"].map((dir) => [
        dir,
        { kind: "listed", listing: { entries: [], more: false } },
      ]),
    );
    let out = 0;
    let most = 0;
    let busy = true;
    const asked: string[] = [];
    vi.mocked(inspectDir).mockImplementation(
      async (_fleet, _node, _root, directory) => {
        asked.push(directory);
        out += 1;
        most = Math.max(most, out);
        await new Promise((settle) => {
          setTimeout(settle, 5);
        });
        out -= 1;
        return attempt<InspectedDirectory>(async () => {
          if (directory === "c" && busy) {
            throw new Error("workspace file operations are busy");
          }
          await Promise.resolve();
          return {
            entries: [
              {
                path: `${directory}/x`,
                kind: "file",
                size: 1,
                modified: 2,
                ignored: "visible",
              },
            ],
            more: false,
          };
        });
      },
    );
    const seen = new Set<string>();
    const found = (entries: readonly InspectedEntry[]): void => {
      for (const entry of entries) seen.add(entry.path);
    };
    const inspected = new Map<RelPath, Listing>();

    await inspectListed(fleet, node, root, false, listings, inspected, found);
    expect(asked).toHaveLength(6);
    expect(most).toBeLessThanOrEqual(BACKGROUND_SLOTS);
    expect([...seen].sort()).toEqual(["/x", "a/x", "b/x", "d/x", "e/x"]);
    // The refused folder is forgotten; the others stay asked.
    expect([...inspected.keys()].sort()).toEqual(["", "a", "b", "d", "e"]);

    // The next pass asks only the refused folder.
    busy = false;
    asked.length = 0;
    await inspectListed(fleet, node, root, false, listings, inspected, found);
    expect(asked).toEqual(["c"]);
    expect(seen.has("c/x")).toBe(true);
  });
});
