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
//   index.html. It is a chunk of its own, imported when the path is /kit
//   (milestone 11): the operator's every page load no longer carries the
//   kit's specimen. integrity.mjs pins the chunk in the index's import
//   map, like every other one (ADR 0017's amendment), and `isGallery`
//   lives apart (path.ts) so deciding does not load it.
// - Cost: opening /kit waits for one more request.

import "./gallery.css";

import { showTheme } from "../../data/theme";
import { Segmented, TooltipProvider } from "../../ui/kit";
import { THEMES, ThemeMenu, useTheme } from "../theme/theme";
import { Buttons } from "./buttons";
import { Choices } from "./choices";
import { Composer } from "./composer";
import { Conversation } from "./conversation";
import { Display } from "./display";
import { Fields } from "./fields";
import { Popups } from "./popups";
import { QuotaMeters } from "./quota-meters";

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
        <Conversation />
        <Display />
        <QuotaMeters />
      </main>
    </TooltipProvider>
  );
}
