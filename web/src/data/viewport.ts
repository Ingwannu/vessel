// A zoomable viewport's geometry, Paseo's
// `components/zoomable-viewport/geometry.ts` (pinned 0.15.4): content
// fitted inside the viewport, then a scale and an offset from its centre,
// kept so the content never leaves a gap it could cover. The image panel
// (F-F40) draws with it.

export interface ViewportSize {
  width: number;
  height: number;
}

export interface ViewportPoint {
  x: number;
  y: number;
}

/** A scale and an offset of the content's centre from the viewport's. */
export interface ViewportTransform {
  scale: number;
  x: number;
  y: number;
}

export interface ViewportScaleLimits {
  minScale: number;
  maxScale: number;
}

/** The content fitted, unzoomed and centred. */
export const FIT_TRANSFORM: ViewportTransform = { scale: 1, x: 0, y: 0 };

/** `content` scaled to fit inside `viewport`, keeping its proportions. */
export function fitContentSize(
  content: ViewportSize,
  viewport: ViewportSize,
): ViewportSize {
  const width = Math.max(0, viewport.width);
  const height = Math.max(0, viewport.height);
  const fitScale = Math.min(width / content.width, height / content.height);
  return { width: content.width * fitScale, height: content.height * fitScale };
}

function clampScale(scale: number, limits: ViewportScaleLimits): number {
  return Math.min(limits.maxScale, Math.max(limits.minScale, scale));
}

function clampOffset(offset: number, maximum: number): number {
  if (maximum === 0) return 0;
  return Math.min(maximum, Math.max(-maximum, offset));
}

/** `transform` with its scale in `limits` and its offset no further than
 * the zoomed content overhangs the viewport. */
export function clampTransform(
  transform: ViewportTransform,
  fitted: ViewportSize,
  viewport: ViewportSize,
  limits: ViewportScaleLimits,
): ViewportTransform {
  const scale = clampScale(transform.scale, limits);
  const maxX = Math.max(0, (fitted.width * scale - viewport.width) / 2);
  const maxY = Math.max(0, (fitted.height * scale - viewport.height) / 2);
  return {
    scale,
    x: clampOffset(transform.x, maxX),
    y: clampOffset(transform.y, maxY),
  };
}

/** Zoom to `scale` keeping the content under `focalPoint` where it is. */
export function zoomContentAtPoint(input: {
  transform: ViewportTransform;
  scale: number;
  focalPoint: ViewportPoint;
  fitted: ViewportSize;
  viewport: ViewportSize;
  limits: ViewportScaleLimits;
}): ViewportTransform {
  const scale = clampScale(input.scale, input.limits);
  const centreX = input.viewport.width / 2;
  const centreY = input.viewport.height / 2;
  const contentX =
    (input.focalPoint.x - centreX - input.transform.x) / input.transform.scale;
  const contentY =
    (input.focalPoint.y - centreY - input.transform.y) / input.transform.scale;
  return clampTransform(
    {
      scale,
      x: input.focalPoint.x - centreX - contentX * scale,
      y: input.focalPoint.y - centreY - contentY * scale,
    },
    input.fitted,
    input.viewport,
    input.limits,
  );
}

/** Move the content by `delta`, kept as `clampTransform` keeps it. */
export function panContent(input: {
  transform: ViewportTransform;
  delta: ViewportPoint;
  fitted: ViewportSize;
  viewport: ViewportSize;
  limits: ViewportScaleLimits;
}): ViewportTransform {
  return clampTransform(
    {
      ...input.transform,
      x: input.transform.x + input.delta.x,
      y: input.transform.y + input.delta.y,
    },
    input.fitted,
    input.viewport,
    input.limits,
  );
}

/** Halfway between the first two of `points` (a pinch's fingers). */
export function midpoint(points: readonly ViewportPoint[]): ViewportPoint {
  const [a, b] = points;
  return a === undefined || b === undefined
    ? { x: 0, y: 0 }
    : { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
}

/** How far apart the first two of `points` are. */
export function distance(points: readonly ViewportPoint[]): number {
  const [a, b] = points;
  return a === undefined || b === undefined
    ? 0
    : Math.hypot(a.x - b.x, a.y - b.y);
}
