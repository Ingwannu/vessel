import { describe, expect, it } from "vitest";

import { bookNode, connectedAs, liveSession } from "./fixtures/machines";
import {
  HERE,
  machineActions,
  machineFor,
  machineLine,
  machineState,
  nameProblem,
} from "./machines";

describe("a machine's row", () => {
  it("says what it runs on, how it is reached, its state and its sessions", () => {
    expect(
      machineLine(bookNode("box"), connectedAs("box"), [liveSession]),
    ).toEqual(["Linux x86_64", "Direct", "Connected", "1 live session"]);
    expect(
      machineLine(
        bookNode("mac", { transport: "paired" }),
        connectedAs("mac", "macos"),
        [],
      ),
    ).toEqual(["macOS x86_64", "Paired", "Connected", "0 live sessions"]);
  });

  it("says only how it is reached and its state while not connected", () => {
    expect(
      machineLine(
        bookNode("box", { transport: "ssh" }),
        { state: "failed", message: "refused" },
        undefined,
      ),
    ).toEqual(["Over SSH", "Offline"]);
    expect(machineState(undefined).word).toBe("Connecting");
  });

  it("offers forget and rename on book entries only, default unless it is", () => {
    expect(machineActions(bookNode("box"))).toEqual({
      rename: true,
      makeDefault: true,
      forget: true,
    });
    expect(machineActions(bookNode(HERE, { default: true }))).toEqual({
      rename: false,
      makeDefault: false,
      forget: false,
    });
  });
});

describe("a machine's name", () => {
  const book = [bookNode("laptop"), bookNode("gpu-box"), bookNode(HERE)];

  it("takes the book's names and no other", () => {
    expect(nameProblem("studio", book)).toBeNull();
    expect(nameProblem("build.server_2-a", book)).toBeNull();
    expect(nameProblem("", book)).toBe("Give it a name.");
    expect(nameProblem("-box", book)).toMatch(/^Start with/);
    expect(nameProblem("_here", book)).toMatch(/^Start with/);
    expect(nameProblem("two words", book)).toMatch(/no spaces/);
    expect(nameProblem("상자", book)).toMatch(/^Start with/);
    expect(nameProblem("a".repeat(65), book)).toMatch(/^At most 64/);
    expect(nameProblem("a".repeat(64), book)).toBeNull();
  });

  it("refuses a taken name, except the machine's own when renaming", () => {
    expect(nameProblem("gpu-box", book)).toBe(
      "There is already a machine called gpu-box.",
    );
    expect(nameProblem("gpu-box", book, "gpu-box")).toBeNull();
  });
});

describe("the machine a section is about", () => {
  const book = [
    bookNode("laptop"),
    bookNode("gpu-box", { default: true }),
    bookNode("build-server"),
  ];

  it("is the address's, else the default, else a connected one", () => {
    expect(machineFor(book, {}, "laptop")?.key).toBe("laptop");
    expect(machineFor(book, {}, "missing")?.key).toBe("gpu-box");
    const noDefault = book.map((node) => ({ ...node, default: false }));
    expect(
      machineFor(
        noDefault,
        { "build-server": connectedAs("build-server") },
        undefined,
      )?.key,
    ).toBe("build-server");
    expect(machineFor([], {}, undefined)).toBeNull();
  });
});
