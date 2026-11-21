import { describe, expect, it } from "vitest";

import { remainingVerdict, type Merge, type ReviewPills } from "./facts";
import {
  checkCountWords,
  mergeWords,
  outcomeWords,
  phaseWords,
  unavailableWords,
  verdictWords,
} from "./facts-words";
import { unavailableOf, type Unavailable } from "./unavailable";
import { pullWords } from "./words";

const en = pullWords("en");
const ko = pullWords("ko");

describe("the header's facts in words (words.rs)", () => {
  it("keeps Paseo's sentence word for word", () => {
    expect(
      unavailableWords({ kind: "notSupported", host: "github.com" }, en),
    ).toBe(
      "Create PR isn't available right now because github.com isn't connected",
    );
  });

  it("gives a host the Korean particle its last sound asks for", () => {
    const signedOut = unavailableWords(
      { kind: "signedOut", host: "github.com" },
      ko,
    );
    expect(signedOut.startsWith("이 머신이 github.com에 로그인")).toBe(true);
    const refused = unavailableWords(
      { kind: "refused", host: "github.com" },
      ko,
    );
    expect(refused.startsWith("github.com이 ")).toBe(true);
  });

  it("words an access the node could not use, its host a value", () => {
    const why = unavailableOf({
      access: "signed_out",
      host: "github.com",
      gh: "signed_out",
    });
    expect(unavailableWords(why, en)).toContain(
      "isn't signed in to github.com",
    );
    expect(unavailableWords({ kind: "unknown" }, ko)).toBe(
      "지금은 PR 생성을 사용할 수 없습니다",
    );
  });

  it("reads the facts as Paseo's pane does", () => {
    expect(phaseWords("draft", ko)).toBe("초안");
    expect(outcomeWords("failure", en)).toBe("Some checks were not successful");
    expect(checkCountWords({ kind: "failing", count: 2 }, en)).toBe(
      "2 failing",
    );
    expect(checkCountWords({ kind: "passed", count: 5 }, ko)).toBe("5개 성공");
  });

  it("says a verdict only when no pill already says it, in both languages", () => {
    const pills: ReviewPills = {
      approved: true,
      changesRequested: false,
      unresolved: 0,
    };
    const cases: [typeof en, string, string, string][] = [
      [en, "Approved", "Review required", "Changes requested"],
      [ko, "승인됨", "리뷰 필요", "변경 요청됨"],
    ];
    for (const [words, approved, required, changes] of cases) {
      expect(words.approved).toBe(approved);
      expect(
        verdictWords(remainingVerdict(pills, "approved"), words),
      ).toBeNull();
      expect(
        verdictWords(remainingVerdict(pills, "review_required"), words),
      ).toBe(required);
      expect(
        verdictWords(remainingVerdict(pills, "changes_requested"), words),
      ).toBe(changes);
    }
  });

  it("has a sentence for every reason and every merge standing", () => {
    const reasons: Unavailable[] = [
      { kind: "noBranch" },
      { kind: "noRemote" },
      { kind: "notSupported", host: "h" },
      { kind: "signedOut", host: "h" },
      { kind: "refused", host: "h" },
      { kind: "rateLimited", host: "h", until: "2026-11-14T10:00:00Z" },
      { kind: "unreachable", host: "h", retryAt: "2026-11-14T10:00:00Z" },
      { kind: "limit" },
      { kind: "ended" },
      { kind: "unknown" },
    ];
    const merges: Merge[] = [
      "ready",
      "conflicts",
      "blocked",
      "unstable",
      "behind",
      "unknown",
    ];
    for (const words of [en, ko]) {
      const said = reasons.map((why) => unavailableWords(why, words));
      expect(new Set(said).size).toBe(reasons.length);
      const merge = merges.map((standing) => mergeWords(standing, words));
      expect(new Set(merge).size).toBe(merges.length);
    }
  });
});
