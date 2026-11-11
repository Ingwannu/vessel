// The web UI's entry: the launch token, the theme before first paint,
// then the app (ADR 0017).
import "./ui/fonts.css";
// Pretendard's unicode-range subsets (fonts.css says why it is here).
import "pretendard/dist/web/variable/pretendardvariable-dynamic-subset.css";
import "./ui/tokens.css";
// The kit's stylesheets (beside its components) before the areas', so an
// area may place a control where it sits without out-ranking its look.
import "./ui/kit";
import "./ui/styles/index.css";

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "./app";
import { takeToken } from "./bridge/token";
import { applyTheme } from "./data/theme";
import { Gallery, isGallery } from "./features/kit-gallery/gallery";

applyTheme();
const root = document.getElementById("root");
if (root !== null) {
  // The kit's gallery holds no data: it opens without a token, and leaves
  // one in the address for the app (features/kit-gallery/gallery.tsx).
  createRoot(root).render(
    <StrictMode>
      {isGallery(window.location.pathname) ? (
        <Gallery />
      ) : (
        <App token={takeToken()} />
      )}
    </StrictMode>,
  );
}
