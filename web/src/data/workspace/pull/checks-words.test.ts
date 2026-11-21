import { describe, expect, it } from "vitest";

import type { CheckRun, CheckState } from "../../../wire/proto";
import { checksSummary } from "./checks";
import { detailWords, groupWords, timingWords } from "./checks-words";
import { pullWords } from "./words";

const en = pullWords("en");
const ko = pullWords("ko");

function run(name: string, state: CheckState): CheckRun {
  return { name, state };
}

describe("the checks section's words (checks_words.rs)", () => {
  it("reads the detail line and the groups as Paseo's", () => {
    const runs = [
      run("a", "failed"),
      run("b", "failed"),
      run("c", "pending"),
      run("d", "passed"),
    ];
    const read = checksSummary(runs);
    expect(detailWords(read, en)).toBe(
      "2 failing, 1 in progress, 1 successful checks",
    );
    expect(detailWords(checksSummary(runs.slice(3)), en)).toBe(
      "1 successful check",
    );
    expect(detailWords(checksSummary([]), en), "no checks, no line").toBe("");
    expect(groupWords("failure", 2, en)).toBe("2 failing checks");
    expect(groupWords("ignored", 1, en)).toBe("1 skipped check");
    expect(groupWords("pending", 1, en)).toBe("1 in progress check");
    expect(groupWords("success", 3, ko)).toBe("성공한 검사 3개");
    expect(detailWords(read, ko)).toBe(
      "검사 4개: 2개 실패, 1개 진행 중, 1개 성공",
    );
  });

  it("says when a check still runs", () => {
    expect(timingWords({ running: false, seconds: 90 }, en)).toBe("1m 30s");
    const running = { running: true, seconds: 120 };
    expect(timingWords(running, en)).toBe("running 2m");
    expect(timingWords(running, ko)).toBe("2m 실행 중");
  });

  it("counts skipped and cancelled checks together in the detail line", () => {
    const read = checksSummary([
      run("a", "skipped"),
      run("b", "cancelled"),
      run("c", "passed"),
    ]);
    expect(detailWords(read, en)).toBe("1 successful, 2 skipped checks");
    expect(detailWords(read, ko)).toBe("검사 3개: 1개 성공, 2개 건너뜀");
  });
});
