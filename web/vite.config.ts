// The web UI's build (ADR 0017): a static bundle in `dist/`, which
// vessel-web's build script embeds in the `vessel` binary. Nothing here
// reaches the network at run time; fonts are bundled from `pretendard`
// and `@fontsource-variable/geist{,-mono}` (ADR 0021).
import react from "@vitejs/plugin-react";
import type { Plugin } from "vite";
import { defineConfig } from "vitest/config";

/** xterm.js's stylesheet gives its screen-reader tree a bare `monospace`;
 * the operator's font rule wants a real face and Pretendard before it
 * (scripts/check-fonts.mjs checks the built CSS too). */
function xtermFont(): Plugin {
  return {
    name: "vessel-xterm-font",
    enforce: "pre",
    transform(code, id) {
      if (!id.endsWith("@xterm/xterm/css/xterm.css")) {
        return null;
      }
      return code.replaceAll(
        "font-family: monospace;",
        'font-family: "Geist Mono Variable", Consolas, "SF Mono", Menlo, "JetBrains Mono", "Cascadia Mono", "DejaVu Sans Mono", "Liberation Mono", "Noto Sans Mono", Pretendard, monospace;',
      );
    },
  };
}

/** Pretendard's unicode-range subsets name their face "Pretendard
 * Variable"; every stack of ours names it "Pretendard" (ADR 0017's font
 * rule, scripts/check-fonts.mjs), so the face takes our name. The
 * subsets themselves are the package's, unchanged. The stylesheet must be
 * imported from a module (src/main.tsx) for this to see it. */
function pretendardName(): Plugin {
  return {
    name: "vessel-pretendard-name",
    enforce: "pre",
    transform(code, id) {
      if (
        !id.endsWith(
          "pretendard/dist/web/variable/pretendardvariable-dynamic-subset.css",
        )
      ) {
        return null;
      }
      return code.replaceAll(
        "font-family: 'Pretendard Variable';",
        "font-family: Pretendard;",
      );
    },
  };
}

export default defineConfig({
  plugins: [xtermFont(), pretendardName(), react()],
  build: {
    outDir: "dist",
    assetsDir: "assets",
    target: "es2022",
    sourcemap: false,
    // One chunk per screen that waits to be opened (src/app.tsx), not one
    // per library: a library chunk would only add requests.
    chunkSizeWarningLimit: 2048,
    // A chunk imported later is pinned by the index's import map
    // (scripts/integrity.mjs). Preload links Vite would insert at run time
    // for a screen's chunks are off, and so are per-chunk stylesheets:
    // tags made at run time are outside what the index pins. The entry's
    // own static imports keep their `modulepreload` tags in the index
    // (fetched with the entry instead of after it; integrity.mjs pins them
    // like every tag). One stylesheet, named by the index.
    modulePreload: {
      polyfill: false,
      resolveDependencies: (_file, dependencies, context) =>
        context.hostType === "html" ? dependencies : [],
    },
    cssCodeSplit: false,
  },
  test: {
    include: ["src/**/*.test.ts", "src/**/*.test.tsx"],
    environment: "node",
  },
});
