// An image to look at closely, Paseo's `ZoomableImage` over its web
// viewport (`components/zoomable-viewport/index.web.tsx`, pinned 0.15.4):
// fitted and centred on open; the wheel zooms at the cursor (always, as
// the image panel sets it), two fingers pinch, a drag pans once zoomed in,
// a double click fits again; Zoom in, Zoom out and Reset view sit in the
// top right corner while the pointer or the focus is in the viewer. The
// scale stays between 1 (fitted) and 8; the geometry is
// `data/viewport.ts`.

import "./file-viewers.css";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import {
  FIT_TRANSFORM,
  fitContentSize,
  type ViewportPoint,
  type ViewportSize,
  type ViewportTransform,
  zoomContentAtPoint,
} from "../../data/viewport";
import { IconButton } from "../../ui/kit";
import { useViewportGestures } from "./use-viewport-gestures";

const LIMITS = { minScale: 1, maxScale: 8 };

export function ZoomableImage({ src, label }: { src: string; label: string }) {
  const canvas = useRef<HTMLDivElement | null>(null);
  const current = useRef<ViewportTransform>(FIT_TRANSFORM);
  const [natural, setNatural] = useState<ViewportSize | null>(null);
  // A new image (the file changed) is measured and fitted again, as
  // Paseo's is.
  const [measured, setMeasured] = useState(src);
  if (measured !== src) {
    setMeasured(src);
    setNatural(null);
  }
  const [viewport, setViewport] = useState<ViewportSize | null>(null);
  const [transform, setTransform] = useState(FIT_TRANSFORM);
  const fitted = useMemo(
    () =>
      natural === null || viewport === null
        ? null
        : fitContentSize(natural, viewport),
    [natural, viewport],
  );
  const commit = useCallback((next: ViewportTransform) => {
    current.current = next;
    setTransform(next);
  }, []);
  const reset = useCallback(() => {
    commit(FIT_TRANSFORM);
  }, [commit]);

  useEffect(() => {
    const element = canvas.current;
    if (element === null) return;
    const observer = new ResizeObserver(([entry]) => {
      const box = entry?.contentRect;
      if (box === undefined || box.width <= 0 || box.height <= 0) return;
      setViewport({ width: box.width, height: box.height });
      // A new viewport fits again, as Paseo's does.
      commit(FIT_TRANSFORM);
    });
    observer.observe(element);
    return () => {
      observer.disconnect();
    };
  }, [commit]);

  const zoomAt = useCallback(
    (scale: number, focalPoint: ViewportPoint) => {
      if (fitted === null || viewport === null) return;
      commit(
        zoomContentAtPoint({
          transform: current.current,
          scale,
          focalPoint,
          fitted,
          viewport,
          limits: LIMITS,
        }),
      );
    },
    [commit, fitted, viewport],
  );
  const zoomBy = (factor: number) => {
    if (viewport === null) return;
    zoomAt(current.current.scale * factor, {
      x: viewport.width / 2,
      y: viewport.height / 2,
    });
  };

  useEffect(() => {
    const element = canvas.current;
    if (element === null) return;
    const wheel = (event: WheelEvent) => {
      event.preventDefault();
      const bounds = element.getBoundingClientRect();
      const factor = Math.min(
        1.25,
        Math.max(0.8, Math.exp(-event.deltaY * 0.01)),
      );
      zoomAt(current.current.scale * factor, {
        x: event.clientX - bounds.left,
        y: event.clientY - bounds.top,
      });
    };
    // Not passive: the wheel zooms the image instead of scrolling the page.
    element.addEventListener("wheel", wheel, { passive: false });
    return () => {
      element.removeEventListener("wheel", wheel);
    };
  }, [zoomAt]);

  const { dragging, handlers } = useViewportGestures({
    current,
    commit,
    fitted,
    viewport,
    limits: LIMITS,
  });
  const zoomed = transform.scale > 1;
  return (
    <div className="zoomable">
      <div
        ref={canvas}
        className={`zoomable-canvas${zoomed ? (dragging ? " grabbing" : " grab") : ""}`}
        role="img"
        aria-label={label}
        onDoubleClick={reset}
        {...handlers}
      >
        {fitted === null ? null : (
          <div
            className="zoomable-content"
            style={{
              width: fitted.width,
              height: fitted.height,
              transform: `translate3d(${String(transform.x)}px, ${String(transform.y)}px, 0) scale(${String(transform.scale)})`,
            }}
          >
            <img src={src} alt="" draggable={false} />
          </div>
        )}
        {/* Measures the image before it is fitted. */}
        {natural === null ? (
          <img
            className="zoomable-probe"
            src={src}
            alt=""
            onLoad={(event) => {
              const image = event.currentTarget;
              if (image.naturalWidth > 0 && image.naturalHeight > 0) {
                setNatural({
                  width: image.naturalWidth,
                  height: image.naturalHeight,
                });
                commit(FIT_TRANSFORM);
              }
            }}
          />
        ) : null}
      </div>
      <div className="zoomable-tools">
        <IconButton
          label="Zoom in"
          icon="zoom-in"
          size="xs"
          disabled={transform.scale >= LIMITS.maxScale}
          onClick={() => {
            zoomBy(1.25);
          }}
        />
        <IconButton
          label="Zoom out"
          icon="zoom-out"
          size="xs"
          disabled={transform.scale <= LIMITS.minScale}
          onClick={() => {
            zoomBy(0.8);
          }}
        />
        <IconButton
          label="Reset view"
          icon="scan"
          size="xs"
          disabled={transform.scale === 1}
          onClick={reset}
        />
      </div>
    </div>
  );
}
