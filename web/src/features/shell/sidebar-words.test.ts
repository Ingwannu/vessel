import { describe, expect, test } from "vitest";

import { sidebarWords } from "./sidebar-words";

describe("sidebar show words", () => {
  test("English and Korean name the choices and the changes pill", () => {
    expect(sidebarWords("en-US")).toEqual({
      show: "Show",
      diffStats: "Diff stats",
      activity: "Last activity",
      openChanges: "Open Changes tab",
    });
    expect(sidebarWords("ko-KR")).toEqual({
      show: "표시 항목",
      diffStats: "Diff 통계",
      activity: "마지막 활동",
      openChanges: "변경사항 탭 열기",
    });
    expect(sidebarWords("ko").diffStats).toBe("Diff 통계");
    expect(sidebarWords("en").show).toBe("Show");
  });
});
