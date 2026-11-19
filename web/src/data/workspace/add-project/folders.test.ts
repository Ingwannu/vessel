import { describe, expect, it } from "vitest";

import { fixtureId } from "../../../testing/ids";
import type { FolderSuggestions } from "../../../wire/proto";
import type { Outcome } from "../../outcome";
import { folderAnswer, machineFolders } from "./folders";

const LAPTOP = fixtureId("NodeKey", "laptop");
const DESKTOP = fixtureId("NodeKey", "desktop");

const FOUND: Outcome<FolderSuggestions> = {
  ok: true,
  value: {
    folders: [{ path: "/home/dev/code/vessel", shown: "~/code/vessel" }],
    more: false,
  },
};

describe("the Add project field's folders", () => {
  it("shows an answer for the machine and text it was asked for", () => {
    const answer = folderAnswer(LAPTOP, "code", FOUND);
    expect(machineFolders(answer, LAPTOP, "code", true)).toEqual({
      paths: ["/home/dev/code/vessel"],
      shown: new Map([["/home/dev/code/vessel", "~/code/vessel"]]),
      loading: false,
      error: null,
    });
  });

  it("does not show one machine's folders on another, same text", () => {
    const answer = folderAnswer(LAPTOP, "code", FOUND);
    const other = machineFolders(answer, DESKTOP, "code", true);
    expect(other.paths).toEqual([]);
    expect(other.shown.size).toBe(0);
    expect(other.loading).toBe(true);
    expect(other.error).toBeNull();
  });

  it("waits for the text as it is now", () => {
    const answer = folderAnswer(LAPTOP, "cod", FOUND);
    expect(machineFolders(answer, LAPTOP, "code", true)).toMatchObject({
      paths: [],
      loading: true,
    });
  });

  it("says a failed search as Paseo does, for its machine only", () => {
    const failed = folderAnswer(LAPTOP, "code", {
      ok: false,
      problem: { from: "op", reason: "busy", message: "busy" },
    });
    expect(machineFolders(failed, LAPTOP, "code", true).error).toBe(
      "Unable to search directories",
    );
    expect(machineFolders(failed, DESKTOP, "code", true).error).toBeNull();
  });

  it("neither loads nor shows anything on a page that does not search", () => {
    const answer = folderAnswer(LAPTOP, "code", FOUND);
    expect(machineFolders(answer, LAPTOP, "code", false)).toMatchObject({
      paths: [],
      loading: false,
      error: null,
    });
    expect(machineFolders(null, LAPTOP, "code", false).loading).toBe(false);
  });
});
