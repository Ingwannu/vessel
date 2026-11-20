// The image viewer's pointer gestures, Paseo's web viewport
// (`components/zoomable-viewport/index.web.tsx`, pinned 0.15.4): one
// pointer drags the content (it moves only once zoomed in, the geometry
// keeping it from leaving a gap), two pinch, zooming at their midpoint and
// following it; a finger lifted from a pinch leaves a drag. The transform
// lives in `current` so a gesture reads the latest one between renders.

import { type PointerEvent, type RefObject, useRef, useState } from "react";

import {
  distance,
  midpoint,
  panContent,
  type ViewportPoint,
  type ViewportScaleLimits,
  type ViewportSize,
  type ViewportTransform,
  zoomContentAtPoint,
} from "../../data/viewport";

type Gesture =
  | {
      kind: "drag";
      pointer: number;
      start: ViewportPoint;
      from: ViewportTransform;
    }
  | { kind: "pinch"; distance: number; midpoint: ViewportPoint };

type Handler = (event: PointerEvent<HTMLDivElement>) => void;

export function useViewportGestures({
  current,
  commit,
  fitted,
  viewport,
  limits,
}: {
  current: RefObject<ViewportTransform>;
  commit: (next: ViewportTransform) => void;
  fitted: ViewportSize | null;
  viewport: ViewportSize | null;
  limits: ViewportScaleLimits;
}): {
  /** A drag or a pinch is moving the content (the grabbing cursor). */
  dragging: boolean;
  handlers: {
    onPointerDown: Handler;
    onPointerMove: Handler;
    onPointerUp: Handler;
    onPointerCancel: Handler;
  };
} {
  const pointers = useRef(new Map<number, ViewportPoint>());
  const gesture = useRef<Gesture | null>(null);
  const [dragging, setDragging] = useState(false);
  const begin = (event: PointerEvent<HTMLDivElement>) => {
    if (event.pointerType === "mouse" && event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    pointers.current.set(event.pointerId, {
      x: event.clientX,
      y: event.clientY,
    });
    const points = [...pointers.current.values()];
    if (points.length >= 2) {
      gesture.current = {
        kind: "pinch",
        distance: distance(points),
        midpoint: midpoint(points),
      };
      setDragging(true);
      return;
    }
    gesture.current = {
      kind: "drag",
      pointer: event.pointerId,
      start: { x: event.clientX, y: event.clientY },
      from: current.current,
    };
    setDragging(current.current.scale > 1);
  };
  const move = (event: PointerEvent<HTMLDivElement>) => {
    if (!pointers.current.has(event.pointerId)) return;
    if (fitted === null || viewport === null) return;
    pointers.current.set(event.pointerId, {
      x: event.clientX,
      y: event.clientY,
    });
    const now = gesture.current;
    const points = [...pointers.current.values()];
    if (now?.kind === "pinch" && points.length >= 2) {
      const middle = midpoint(points);
      const apart = distance(points);
      const panned = panContent({
        transform: current.current,
        delta: { x: middle.x - now.midpoint.x, y: middle.y - now.midpoint.y },
        fitted,
        viewport,
        limits,
      });
      gesture.current = { kind: "pinch", distance: apart, midpoint: middle };
      commit(
        zoomContentAtPoint({
          transform: panned,
          scale: panned.scale * (now.distance > 0 ? apart / now.distance : 1),
          focalPoint: middle,
          fitted,
          viewport,
          limits,
        }),
      );
      return;
    }
    if (now?.kind !== "drag" || now.pointer !== event.pointerId) return;
    commit(
      panContent({
        transform: now.from,
        delta: {
          x: event.clientX - now.start.x,
          y: event.clientY - now.start.y,
        },
        fitted,
        viewport,
        limits,
      }),
    );
  };
  const end = (event: PointerEvent<HTMLDivElement>) => {
    pointers.current.delete(event.pointerId);
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    const [rest] = [...pointers.current.entries()];
    if (pointers.current.size === 1 && rest !== undefined) {
      gesture.current = {
        kind: "drag",
        pointer: rest[0],
        start: rest[1],
        from: current.current,
      };
      setDragging(current.current.scale > 1);
      return;
    }
    gesture.current = null;
    setDragging(false);
  };
  return {
    dragging,
    handlers: {
      onPointerDown: begin,
      onPointerMove: move,
      onPointerUp: end,
      onPointerCancel: end,
    },
  };
}
