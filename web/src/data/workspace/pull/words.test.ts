import { describe, expect, it } from "vitest";

import { pullWords } from "./words";

describe("the pane's words (pulls.ftl)", () => {
  it("picks Korean for ko and its regions only", () => {
    expect(pullWords("ko").activity).toBe("활동");
    expect(pullWords("ko-KR").activity).toBe("활동");
    expect(pullWords("KO").activity).toBe("활동");
    expect(pullWords("kok").activity).toBe("Activity");
    expect(pullWords("en-US").activity).toBe("Activity");
    expect(pullWords("").activity).toBe("Activity");
  });

  it("groups a count's digits as Fluent's NUMBER does", () => {
    expect(pullWords("en").countFailing(1234)).toBe("1,234 failing");
    expect(pullWords("ko").groupPassed(1234)).toBe("성공한 검사 1,234개");
  });

  it("chooses English's plural by the count and Korean's by none", () => {
    const en = pullWords("en");
    expect(en.unresolved(1)).toBe("1 unresolved thread");
    expect(en.unresolved(0)).toBe("0 unresolved threads");
    expect(en.checksDetail("1 failing", 1)).toBe("1 failing check");
    expect(pullWords("ko").unresolved(1)).toBe("해결되지 않은 스레드 1개");
  });

  it("puts a forge's name in the pane's frame", () => {
    expect(pullWords("en").openOn("GitHub")).toBe("Open on GitHub");
    expect(pullWords("en").refreshState("GitHub")).toBe(
      "Refresh git and GitHub state",
    );
    expect(pullWords("ko").openOn("GitHub")).toBe("GitHub에서 열기");
  });
});
