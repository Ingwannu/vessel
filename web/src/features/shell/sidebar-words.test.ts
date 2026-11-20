import { describe, expect, test } from "vitest";

import { sidebarWords } from "./sidebar-words";

describe("sidebar pill words", () => {
  test("English and Korean name the changes pill", () => {
    expect(sidebarWords("en-US").openChanges).toBe("Open Changes tab");
    expect(sidebarWords("ko-KR").openChanges).toBe("변경사항 탭 열기");
    expect(sidebarWords("ko").openChanges).toBe("변경사항 탭 열기");
    expect(sidebarWords("en").openChanges).toBe("Open Changes tab");
  });
});
