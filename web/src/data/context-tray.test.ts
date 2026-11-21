import { describe, expect, it } from "vitest";

import { fixtureId } from "../testing/ids";
import type { AttachmentRef } from "../wire/proto";
import {
  appendPills,
  type ContextPill,
  contextOf,
  ContextTray,
  filePill,
  pillKey,
  promptContent,
  textPrompt,
} from "./context-tray";
import { contextWords, pillLine, removeLabel } from "./context-words";

function pill(kind: ContextPill["kind"], id: string, text = id): ContextPill {
  return { kind, id, title: id, detail: null, text };
}

describe("a session's context pills (TRACKER 833)", () => {
  it("replaces a pill added again and moves it to the end, as Paseo's", () => {
    const first = appendPills([], [pill("comment", "a"), pill("review", "b")]);
    const again = appendPills(first, [pill("comment", "a", "newer")]);
    expect(again.map(pillKey)).toEqual(["review:b", "comment:a"]);
    expect(again[1]?.text).toBe("newer");
    // The same id under another kind is another pill.
    expect(appendPills(again, [pill("check", "a")])).toHaveLength(3);
  });

  it("takes out only the pills a prompt carried", () => {
    const tray = new ContextTray();
    tray.add([pill("comment", "a"), pill("check", "b")]);
    const carried = tray.pills.get();
    tray.add([pill("review", "c")]);
    tray.sent(carried);
    expect(tray.pills.get().map(pillKey)).toEqual(["review:c"]);
    tray.remove("review:c");
    expect(tray.pills.get()).toEqual([]);
  });

  it("keeps a pill added again while the prompt was on its way", () => {
    const tray = new ContextTray();
    tray.add([pill("comment", "a", "older")]);
    const carried = tray.pills.get();
    tray.add([pill("comment", "a", "newer")]);
    tray.sent(carried);
    expect(tray.pills.get().map((kept) => kept.text)).toEqual(["newer"]);
  });

  it("keeps one tray per session", () => {
    expect(contextOf("s1")).toBe(contextOf("s1"));
    expect(contextOf("s1")).not.toBe(contextOf("s2"));
  });

  it("names a workspace file as Paseo's pill and text do", () => {
    const file = filePill("./src/upload/retry.rs");
    expect(file).toEqual({
      kind: "file",
      id: "src/upload/retry.rs:whole_file",
      title: "retry.rs",
      detail: "src/upload/retry.rs",
      text: "Workspace file: src/upload/retry.rs",
    });
  });

  it("sends the words, the files, then one text block per pill", () => {
    const ref: AttachmentRef = {
      id: fixtureId("AttachmentId", "att_1"),
      name: "dot.png",
      mime: "image/png",
      kind: "image",
      size: 68,
    };
    const pills = [pill("comment", "a", "First"), pill("check", "b", "Logs")];
    expect(promptContent("  look  ", [ref], pills)).toEqual([
      { type: "text", text: "look" },
      { type: "attachment", attachment: ref },
      { type: "text", text: "First" },
      { type: "text", text: "Logs" },
    ]);
    expect(promptContent("", [], pills)).toEqual([
      { type: "text", text: "First" },
      { type: "text", text: "Logs" },
    ]);
    expect(textPrompt("look", pills)).toBe("look\n\nFirst\n\nLogs");
    expect(textPrompt("", pills)).toBe("First\n\nLogs");
  });

  it("says each kind as Paseo's pills do, in English and Korean", () => {
    const en = contextWords("en-US");
    const ko = contextWords("ko-KR");
    expect(pillLine(pill("comment", "a"), en)).toBe("Comment");
    expect(pillLine(pill("review", "a"), en)).toBe("Review");
    expect(pillLine(pill("check", "a"), en)).toBe("Check logs");
    expect(pillLine(filePill("docs/a.md"), en)).toBe("docs/a.md");
    expect(removeLabel(pill("check", "a"), en)).toBe(
      "Remove context attachment",
    );
    expect(removeLabel(filePill("docs/a.md"), en)).toBe(
      "Remove file attachment",
    );
    expect(pillLine(pill("check", "a"), ko)).toBe("체크 로그");
    expect(removeLabel(filePill("docs/a.md"), ko)).toBe("파일 첨부 제거");
  });
});
