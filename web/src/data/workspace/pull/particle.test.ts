import { describe, expect, it } from "vitest";

import { particle } from "./particle";

describe("Korean particles (grammar.rs)", () => {
  it("covers Hangul, digits, Latin and empty values", () => {
    const rows: [string, string, string][] = [
      ["서울", "을", "로"],
      ["\u{1109}\u{1165}\u{110b}\u{116e}\u{11af}", "을", "로"],
      ["\u{1107}\u{116e}\u{1109}\u{1161}\u{11ab}", "을", "으로"],
      ["부산", "을", "으로"],
      ["서버", "를", "로"],
      ["0", "을", "으로"],
      ["1", "을", "로"],
      ["2", "를", "로"],
      ["3", "을", "으로"],
      ["4", "를", "로"],
      ["5", "를", "로"],
      ["6", "을", "으로"],
      ["7", "을", "로"],
      ["8", "을", "로"],
      ["9", "를", "로"],
      ["VM", "을", "으로"],
      ["nodeN", "을", "으로"],
      ["SQL", "을", "로"],
      ["R", "을", "로"],
      ["server", "을", "로"],
      ["nodeX", "를", "로"],
      ["nodeF", "를", "로"],
      ["nodeR", "을", "로"],
      ["“서울” ", "을", "로"],
      ["", "를", "로"],
      ["...", "를", "로"],
    ];
    for (const [value, object, direction] of rows) {
      expect(particle(value, "object"), value).toBe(object);
      expect(particle(value, "direction"), value).toBe(direction);
    }
    const roles: [string, string, string, string][] = [
      ["서버", "가", "는", "와"],
      ["부산", "이", "은", "과"],
    ];
    for (const [value, subject, topic, withForm] of roles) {
      expect(particle(value, "subject")).toBe(subject);
      expect(particle(value, "topic")).toBe(topic);
      expect(particle(value, "with")).toBe(withForm);
    }
  });

  it("reads a lone compatibility jamo and a character outside the BMP", () => {
    expect(particle("ㄹ", "direction")).toBe("로");
    expect(particle("ㄱ", "direction")).toBe("으로");
    // An emoji is not alphanumeric: the letter before it decides.
    expect(particle("github.com 🚀", "subject")).toBe("이");
    // A host ends in its last letter's name: m (엠) takes 이.
    expect(particle("github.com", "subject")).toBe("이");
    expect(particle("gitea.example", "subject")).toBe("가");
  });
});
