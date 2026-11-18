import { describe, expect, test } from "vitest";

import { dragged, settle } from "./sheet-drag";

const WINDOW = 1000;
const LOW = 650;
const HIGH = 900;

describe("where a released sheet lands (Paseo's snap points 65% and 90%)", () => {
  const snapping = (start: number, shown: number, velocity = 0) =>
    settle({ fit: "snap", start, shown, window: WINDOW, velocity });

  test("by where it is: past halfway up is 90%, a little down stays", () => {
    expect(snapping(LOW, 800)).toBe("high");
    expect(snapping(LOW, 760)).toBe("low");
    expect(snapping(HIGH, 700)).toBe("low");
    expect(snapping(LOW, 500)).toBe("low");
  });

  test("dragged well below the lower point, it closes", () => {
    expect(snapping(LOW, 400)).toBe("close");
  });

  test("a fling down lowers it a step; a fling up raises it", () => {
    expect(snapping(HIGH, 880, 1.2)).toBe("low");
    expect(snapping(LOW, 640, 1.2)).toBe("close");
    expect(snapping(LOW, 660, -1.2)).toBe("high");
  });

  test("a sheet as tall as its content closes past a quarter, or flung", () => {
    const fit = (shown: number, velocity = 0) =>
      settle({ fit: "content", start: 400, shown, window: WINDOW, velocity });
    expect(fit(350)).toBe("low");
    expect(fit(280)).toBe("close");
    expect(fit(390, 1)).toBe("close");
  });
});

describe("a snapping sheet under the finger", () => {
  test("grows up to 90%, then stops", () => {
    expect(dragged("snap", LOW, 800, WINDOW)).toEqual({
      height: 800,
      offset: 0,
    });
    expect(dragged("snap", LOW, 990, WINDOW)).toEqual({
      height: HIGH,
      offset: 0,
    });
  });

  test("below 65% it keeps that height and slides down", () => {
    expect(dragged("snap", LOW, 500, WINDOW)).toEqual({
      height: LOW,
      offset: 150,
    });
    expect(dragged("snap", HIGH, 500, WINDOW)).toEqual({
      height: LOW,
      offset: 150,
    });
  });

  test("one as tall as its content only slides", () => {
    expect(dragged("content", 400, 300, WINDOW)).toEqual({
      height: 400,
      offset: 100,
    });
    expect(dragged("content", 400, 450, WINDOW)).toEqual({
      height: 400,
      offset: 0,
    });
  });
});
