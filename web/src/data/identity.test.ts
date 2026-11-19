import { describe, expect, it } from "vitest";

import { identityFill, projectInitial } from "./identity";

describe("a project's identity square (TRACKER 809, Paseo's)", () => {
  it("colours a key as Paseo's hash and table do", () => {
    // Values from Paseo's own `deriveIdentityColorName` over the same keys.
    expect(identityFill("00000001-0000-4000-8000-000000000000")).toBe(
      "#5179b0",
    );
    expect(identityFill("8b015dc1-ab79-482e-866e-f36ceb7d5dfa")).toBe(
      "#a4673a",
    );
    expect(identityFill("한글")).toBe("#b05c80");
    // An astral character counts its first UTF-16 unit, as Paseo's does.
    expect(identityFill("😀x")).toBe("#b06260");
  });

  it("letters it with the last segment's first character", () => {
    expect(projectInitial("shop")).toBe("S");
    expect(projectInitial(" acme/web-app ")).toBe("W");
    expect(projectInitial("한글-프로젝트")).toBe("한");
    expect(projectInitial("  ")).toBe("");
  });
});
