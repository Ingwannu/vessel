// The owner's mark (docs/design/brand/), in the surrounding text's colour
// (the brand's README: in the interface, the mono mark beside the text;
// below about 24 px its nested lines merge). Its paths come from the
// brand's file through the build (vite-mark.ts), not from a copy here.

import { mark } from "virtual:vessel-mark";

export function BrandMark({
  size,
  className,
}: {
  size: number;
  className?: string;
}) {
  return (
    <svg
      className={className}
      width={size}
      height={size}
      viewBox={mark.viewBox}
      aria-hidden="true"
    >
      {mark.paths.map((path) => (
        <path
          key={path.d.slice(0, 32)}
          fill="currentColor"
          fillRule={path.evenOdd ? "evenodd" : "nonzero"}
          d={path.d}
        />
      ))}
    </svg>
  );
}
