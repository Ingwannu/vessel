import { describe, expect, it } from "vitest";

import { forgeOpenWords } from "./forge-open";

describe("the forge button's name (Paseo's openInEditor words)", () => {
  it("reads in English and Korean", () => {
    const en = forgeOpenWords("en-US");
    const ko = forgeOpenWords("ko-KR");
    expect(en.openIn("GitHub")).toBe("Open workspace in GitHub");
    expect(en.openFileIn("main.rs", "GitLab")).toBe("Open main.rs in GitLab");
    expect(ko.openIn("GitHub")).toBe("GitHub에서 워크스페이스 열기");
    expect(ko.openFileIn("main.rs", "GitLab")).toBe("GitLab에서 main.rs 열기");
  });
});
