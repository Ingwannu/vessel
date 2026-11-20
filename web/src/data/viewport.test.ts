import { describe, expect, it } from "vitest";

import {
  clampTransform,
  distance,
  FIT_TRANSFORM,
  fitContentSize,
  midpoint,
  panContent,
  zoomContentAtPoint,
} from "./viewport";

// Paseo's `zoomable-viewport/geometry.test.ts` cases.
const viewport = { width: 800, height: 600 };
const fitted = { width: 800, height: 400 };
const limits = { minScale: 1, maxScale: 8 };

describe("a zoomable viewport's geometry", () => {
  it("fits content inside the viewport without changing its proportions", () => {
    expect(fitContentSize({ width: 1600, height: 800 }, viewport)).toEqual(
      fitted,
    );
    expect(fitContentSize({ width: 400, height: 800 }, viewport)).toEqual({
      width: 300,
      height: 600,
    });
  });

  it("returns to the centred fit at the minimum scale", () => {
    expect(
      clampTransform({ scale: 0.5, x: 200, y: -200 }, fitted, viewport, limits),
    ).toEqual(FIT_TRANSFORM);
  });

  it("keeps the content under the cursor while zooming", () => {
    expect(
      zoomContentAtPoint({
        transform: FIT_TRANSFORM,
        scale: 2,
        focalPoint: { x: 600, y: 300 },
        fitted,
        viewport,
        limits,
      }),
    ).toEqual({ scale: 2, x: -200, y: 0 });
  });

  it("clamps panning to the scaled content's edges", () => {
    expect(
      panContent({
        transform: { scale: 2, x: 0, y: 0 },
        delta: { x: 900, y: -900 },
        fitted,
        viewport,
        limits,
      }),
    ).toEqual({ scale: 2, x: 400, y: -100 });
  });

  it("measures a pinch between its first two fingers", () => {
    const fingers = [
      { x: 0, y: 0 },
      { x: 30, y: 40 },
    ];
    expect(midpoint(fingers)).toEqual({ x: 15, y: 20 });
    expect(distance(fingers)).toBe(50);
    expect(distance(fingers.slice(0, 1))).toBe(0);
  });
});
