// The kit's gallery at /kit (DESIGN.md §6.13): every control in every
// state and size, with long and Korean text, for review beside the
// references and for the screens in both themes. Nothing links here.
//
// [Decision Log]
// - Purpose: a control is judged by its look, and the screens show each
//   only in the states a fixture happens to reach. One page that holds all
//   of them lets a reviewer, and `npm run screens`, see every state in
//   light and dark at once.
// - Constraint: the page holds no data, so it must open without a launch
//   token or a node; the content security policy allows no inline script,
//   and `integrity.mjs` pins one closed graph of scripts.
// - Options: a route inside the app (needs the token and the fleet to
//   render the shell); a second Vite entry (a second HTML page and bundle
//   for integrity.mjs to pin); a branch in main.tsx on the path.
// - Choice: main.tsx renders this page instead of the app when the path is
//   /kit, before the token is read; `vessel ui` already answers /kit with
//   index.html. A static import keeps the one bundle, so integrity.mjs and
//   the CSP need nothing new.
// - Cost: the gallery's few kilobytes ship in the bundle every page loads;
//   it is the kit's specimen, not a feature an operator reaches.

import "./gallery.css";

import { showTheme } from "../../data/theme";
import { Segmented, TooltipProvider } from "../../ui/kit";
import { THEMES, ThemeMenu, useTheme } from "../theme/theme";
import { Buttons } from "./buttons";
import { Choices } from "./choices";
import { Composer } from "./composer";
import { Display } from "./display";
import { Fields } from "./fields";
import { Popups } from "./popups";

/** Whether the page asked for is the gallery. */
export function isGallery(path: string): boolean {
  return path === "/kit" || path === "/kit/";
}

export function Gallery() {
  const preference = useTheme();
  return (
    <TooltipProvider>
      <main className="kit-page">
        <header className="kit-head">
          <div>
            <h1>Kit</h1>
            <p className="kit-note">
              Every control the screens use, in each state and size. 모든
              컨트롤을 상태와 크기별로 보여 줍니다.
            </p>
          </div>
          <div className="kit-theme">
            <Segmented
              label="Theme"
              size="sm"
              iconsOnly
              value={preference}
              onChange={showTheme}
              options={THEMES}
            />
            <ThemeMenu choose={showTheme} />
          </div>
        </header>
        <Buttons />
        <Fields />
        <Choices />
        <Popups />
        <Composer />
        <Display />
      </main>
    </TooltipProvider>
  );
}
