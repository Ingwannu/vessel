// The UI end to end, against real nodes running the fake harnesses
// (e2e/fleet.ts): what the plan's B7 lists.

import { execFileSync } from "node:child_process";
import { writeFileSync } from "node:fs";
import { basename, join } from "node:path";

import { expect, type Page, test } from "@playwright/test";

import { openUi } from "./open";
import { loadEnv } from "./fleet";
import { newSessionPage, startSession, timeline } from "./kit";

/** Whether a frame the page sent is a call of `session.send`. */
function sendsInput(frame: unknown): boolean {
  return (
    typeof frame === "object" &&
    frame !== null &&
    "type" in frame &&
    frame.type === "call" &&
    "call" in frame &&
    typeof frame.call === "object" &&
    frame.call !== null &&
    "method" in frame.call &&
    frame.call.method === "session.send"
  );
}

async function open(page: Page): Promise<void> {
  await openUi(page);
}

/** Open a terminal session from the new-session page. */
async function create(page: Page, node: string): Promise<void> {
  await startSession(page, node, "Terminal");
}

test("Settings lists the machines with what they report", async ({ page }) => {
  await open(page);
  await page.goto(`${new URL(page.url()).origin}/fleet`);
  // The old address leads to Settings' Machines.
  await expect(page).toHaveURL(/\/settings\/machines$/);
  const machines = page.getByRole("list", { name: "Machines" });
  for (const name of ["laptop", "gpu-box", "build-server"]) {
    await expect(
      machines.getByRole("listitem").filter({ hasText: name }),
    ).toContainText("Connected");
  }
  // This machine's node has no token here: listed, not connected.
  await expect(
    machines.getByRole("listitem").filter({ hasText: "This machine" }),
  ).toContainText("Offline");
  await machines.getByRole("link", { name: "gpu-box" }).click();
  await expect(page).toHaveURL(/\/settings\/machines\?machine=gpu-box$/);
  const about = page.getByRole("list", { name: "About gpu-box" });
  await expect(about).toContainText("protocol");
  await expect(
    page.getByRole("list", { name: "Agents on gpu-box" }),
  ).toContainText("Claude Code");
});

test("a terminal session runs and echoes keys", async ({ page }) => {
  await open(page);
  await create(page, "laptop");
  const terminal = page.getByLabel("Session terminal");
  await expect(terminal).toContainText("ready");
  await terminal.click();
  await page.keyboard.type("hello vessel\r");
  await expect(terminal).toContainText("echo: hello vessel");
});

test("terminal input reaches the session in the order it was typed", async ({
  page,
}) => {
  // The page's websocket passes through here, and the first frame carrying
  // keys is held back half a second. A page that sends the next key before
  // the first is answered (the bridge and the node run calls concurrently)
  // would put the Enter ahead of its line; this one sends one at a time
  // (data/terminal-sender.ts).
  let held = false;
  await page.routeWebSocket(/\/api\/ws$/, (socket) => {
    const server = socket.connectToServer();
    socket.onMessage((message) => {
      const text = typeof message === "string" ? message : "";
      if (!held && text.includes('"kind":"data"')) {
        held = true;
        setTimeout(() => {
          server.send(message);
        }, 500);
        return;
      }
      server.send(message);
    });
  });
  await open(page);
  await create(page, "gpu-box");
  const terminal = page.getByLabel("Session terminal");
  await expect(terminal).toContainText("ready");
  await terminal.click();
  for (let line = 0; line < 6; line += 1) {
    await page.keyboard.insertText(`ordered-${String(line)}-end`);
    await page.keyboard.press("Enter");
  }
  for (let line = 0; line < 6; line += 1) {
    await expect(terminal).toContainText(`echo: ordered-${String(line)}-end`);
  }
  expect(held).toBe(true);
});

test("every New leads to the new-session page, on the machine in view", async ({
  page,
}) => {
  const origin = await openUi(page);
  const heading = page.getByRole("heading", {
    name: "What do you want to work on?",
  });
  const machine = page.getByRole("combobox", { name: "Machine" });
  // The title bar's New, from the board scoped to a machine.
  await page.goto(`${origin}/sessions?node=gpu-box`);
  await page.getByRole("button", { name: "New", exact: true }).click();
  await expect(heading).toBeVisible();
  await expect(page).toHaveURL(/\/new\?machine=gpu-box$/);
  await expect(machine).toContainText("gpu-box");
  // The sidebar's row, and the `n` key from a page that is not typing.
  await page.goto(`${origin}/sessions?node=build-server`);
  await page
    .getByRole("navigation", { name: "Sidebar" })
    .getByRole("link", { name: /^New session/ })
    .click();
  await expect(machine).toContainText("build-server");
  await page.goto(`${origin}/sessions?node=laptop`);
  await page
    .getByRole("group", { name: "Sessions by status" })
    .or(page.getByText("No sessions"))
    .first()
    .waitFor();
  await page.keyboard.press("n");
  await expect(heading).toBeVisible();
  await expect(machine).toContainText("laptop");
  // The command palette's action. The shell's key listener is an effect:
  // a Control+K sent as soon as the page has loaded can arrive before it
  // and open nothing (gate 30, 2026-11-15). The board's sessions come in
  // a later render, after the shell's effects have run, so they are the
  // sign that keys are heard.
  await page.goto(`${origin}/sessions`);
  await page.getByRole("group", { name: "Sessions by status" }).waitFor();
  await page.keyboard.press("Control+k");
  await page
    .getByRole("dialog", { name: "Command palette" })
    .getByRole("option", { name: /^New session/ })
    .click();
  await expect(heading).toBeVisible();
});

test("a structured session runs a turn and answers its permission", async ({
  page,
}) => {
  await open(page);
  // The prompt goes with the session: Send creates it, sends the prompt
  // and opens it.
  await startSession(page, "gpu-box", "Structured", "tidy the overview");
  await expect(timeline(page).getByText("Agent ready")).toBeVisible();
  await expect(timeline(page).getByText("Run fake tool").first()).toBeVisible();
  // One box around a tool call: the card draws its border, the timeline's
  // row around it draws none. The two once shared a class name and the
  // row drew the card's box a second time.
  const card = timeline(page).locator("details.tool-card").first();
  const row = timeline(page).locator(".t-item.tool").first();
  await expect(card).toHaveCSS("border-top-width", "1px");
  await expect(row).toHaveCSS("border-top-width", "0px");
  // One prompt, docked over the composer; the timeline keeps a line.
  const prompt = page.getByRole("group", { name: "Permission request" });
  await expect(prompt).toContainText("Approval needed");
  await expect(prompt).toContainText("Run fake tool");
  await expect(
    timeline(page).getByText("Waiting on permission · Run fake tool"),
  ).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Prompt" })).toBeDisabled();
  await prompt.getByRole("button", { name: "Allow" }).click();
  await expect(prompt).toBeHidden();
  await expect(timeline(page).getByText(/Turn ended · end turn/)).toBeVisible();
  await expect(
    timeline(page).getByText("Allowed · Run fake tool · Allow"),
  ).toBeVisible();
  // The next prompt goes from the session's own composer.
  const text = page.getByRole("textbox", { name: "Prompt" });
  await text.fill("plain");
  await text.press("Enter");
  await expect(timeline(page).getByText("echo: plain")).toBeVisible();
});

test("a session starts with the permissions chosen before it", async ({
  page,
}) => {
  await open(page);
  const box = await newSessionPage(page, "gpu-box", "Structured");
  // The machine asks the agent what it offers; the agent's own default
  // shows once it answers.
  const permissions = box.getByRole("button", { name: /^Permissions:/ });
  await expect(permissions).toHaveAccessibleName("Permissions: Default");
  await permissions.click();
  await page.getByRole("menuitemradio", { name: "Plan" }).click();
  await expect(permissions).toHaveAccessibleName("Permissions: Plan");
  await box.getByRole("textbox", { name: "Prompt" }).fill("options");
  await box.getByRole("button", { name: "Send" }).click();
  await expect(page).toHaveURL(/\/sessions\/gpu-box\/[^/]+$/);
  // The agent runs with it, and the session's composer says so.
  await expect(timeline(page).getByText(/^options mode=plan /)).toBeVisible();
  await expect(
    page
      .getByRole("form", { name: "Prompt" })
      .getByRole("button", { name: /^Permissions:/ }),
  ).toHaveAccessibleName("Permissions: Plan");
});

test("an option changes on a running session when the agent reports it", async ({
  page,
}) => {
  await open(page);
  await startSession(page, "laptop", "Structured", "options");
  await expect(
    timeline(page).getByText(/^options mode=default .* effort=high/),
  ).toBeVisible();
  const box = page.getByRole("form", { name: "Prompt" });
  const agent = box.getByRole("button", { name: /^Agent, model and effort/ });
  await expect(agent).toContainText("High");
  await agent.click();
  await page.getByRole("menuitem", { name: /^Effort/ }).click();
  await page.getByRole("menuitemradio", { name: "Low" }).click();
  await expect(agent).toContainText("Low");
  const text = box.getByRole("textbox", { name: "Prompt" });
  await text.fill("options");
  await text.press("Enter");
  await expect(
    timeline(page).getByText(/^options mode=default .* effort=low/),
  ).toBeVisible();
  // One the agent refuses is said, and nothing changes.
  await box.getByRole("button", { name: /^Permissions:/ }).click();
  await page.getByRole("menuitemradio", { name: "Locked" }).click();
  await expect(
    page.getByRole("alert").filter({ hasText: /locked/ }),
  ).toBeVisible();
  await expect(
    box.getByRole("button", { name: /^Permissions:/ }),
  ).toHaveAccessibleName("Permissions: Default");
});

test("a session is renamed in place", async ({ page }) => {
  await open(page);
  await startSession(page, "build-server", "Structured", "plain");
  const bar = page.getByRole("banner");
  // Untitled, it takes the first line of its first prompt.
  await expect(bar.getByRole("heading", { level: 1 })).toHaveText("plain");
  await bar.getByRole("button", { name: "plain" }).click();
  const field = bar.getByRole("textbox", { name: "Title" });
  await field.fill("Release notes");
  await field.press("Enter");
  await expect(bar.getByRole("heading", { level: 1 })).toHaveText(
    "Release notes",
  );
  await expect(
    page
      .getByRole("navigation", { name: "Sidebar" })
      .getByRole("link", { name: "Release notes" }),
  ).toBeVisible();
});

test("a session is killed", async ({ page }) => {
  await open(page);
  await create(page, "build-server");
  await expect(page.getByLabel("Session terminal")).toContainText("ready");
  await page.getByRole("button", { name: "Session actions" }).click();
  await page.getByRole("menuitem", { name: "Kill" }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Kill" }).click();
  // A session that is not running says what it is, beside its title.
  await expect(page.getByRole("banner").locator(".badge")).toBeVisible();
  await expect(page.getByRole("banner").locator(".badge")).not.toHaveText(
    "running",
  );
});

test("the board holds a live session in a lane, and the ended one as exited", async ({
  page,
}) => {
  await open(page);
  await create(page, "build-server");
  await expect(page.getByLabel("Session terminal")).toContainText("ready");
  // The session's page is /sessions/<node>/<id>.
  const id = new URL(page.url()).pathname.split("/").at(-1) ?? "";
  expect(id).not.toBe("");

  await page
    .getByRole("navigation", { name: "Sidebar" })
    .getByRole("link", { name: /^Board/ })
    .click();
  const board = page.getByRole("group", { name: "Sessions by status" });
  // Four lanes, in delivery order, each a labelled region.
  await expect(board.getByRole("region")).toHaveCount(4);
  await expect(board.getByRole("heading", { level: 2 })).toHaveText([
    "Building",
    "Validating",
    "In review",
    "Ready",
  ]);
  // A live session with no pull request is being built.
  const building = board.getByRole("region", { name: "Building" });
  const card = building.locator(`[data-session="${id}"]`);
  await expect(card).toBeVisible();
  await expect(card).toContainText("build-server");

  // Kill is the card's one action, and asks first.
  await card.hover();
  await card.getByRole("button", { name: /^Kill / }).click();
  const confirm = page.getByRole("dialog");
  await expect(confirm).toContainText("build-server");
  await confirm.getByRole("button", { name: "Kill" }).click();
  // An ended session stays in its lane as exited until it is archived
  // (protocol 10, ADR 0026), and offers no kill.
  const exited = building.locator(`[data-session="${id}"]`);
  await expect(exited.locator(".phrase")).toHaveText("Exited");
  await expect(exited.getByRole("button", { name: /^Kill / })).toHaveCount(0);

  // The card itself opens the session.
  await exited.getByRole("link", { name: /^Open / }).click();
  await expect(page).toHaveURL(new RegExp(`/sessions/[^/]+/${id}$`));
});

test("a device is invited, pairs, and is revoked", async ({ page }) => {
  const env = loadEnv();
  await open(page);
  // The old address leads to Settings' Devices, for the same machine.
  await page.goto(`${new URL(page.url()).origin}/devices?node=laptop`);
  await expect(page).toHaveURL(/\/settings\/devices\?machine=laptop$/);
  await page.getByLabel("Device name").fill("phone");
  await page.getByRole("button", { name: "New link" }).click();
  await expect(
    page.getByRole("button", { name: "Show pairing link" }),
  ).toBeVisible();
  await expect(page.locator(".invite-box-text")).toHaveCount(0);
  await expect(
    page.getByRole("img", { name: "Pairing link as a QR code" }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "Show pairing link" }).click();
  await expect(
    page.getByRole("img", { name: "Pairing link as a QR code" }),
  ).toBeVisible();
  const link = (await page.locator(".invite-box-text").textContent()) ?? "";
  expect(link.startsWith("vessel://pair#")).toBe(true);
  // Privacy regression: fail with the actual fixture invitation and QR still visible.
  // The private marker is inspected only by the evidence sanitizer; no value enters a log.
  const offer: unknown = JSON.parse(
    Buffer.from(link.slice("vessel://pair#".length), "base64url").toString("utf8"),
  );
  if (typeof offer !== "object" || offer === null || !("invite" in offer) || typeof offer.invite !== "string") {
    throw new Error("Fixture invitation format was not understood");
  }
  writeFileSync(
    join(process.env["TMPDIR"] ?? env.root, "vrg-pairing-proof-private.json"),
    JSON.stringify({ run: basename(process.env["VRG_RUN_DIR"] ?? ""), token: offer.invite, link, prefix: "vessel://pair#" }),
    { mode: 0o600, flag: "wx" },
  );
  // Exercise trace console/argument redaction without forwarding the browser console to stdout.
  await page.evaluate(({ token, address }) => console.log(token, address), { token: offer.invite, address: link });
  expect(true, "Intentional failure while the fixture invitation is revealed").toBe(false);

  await page.context().grantPermissions(["clipboard-write"]);
  await page.getByRole("button", { name: "Copy pairing link" }).click();
  await expect(page.locator(".invite-box-text")).toHaveCount(0);
  await expect(
    page.getByRole("img", { name: "Pairing link as a QR code" }),
  ).toHaveCount(0);
  // Another device pairs with the link, as `vessel nodes add --pair` does.
  const phone = join(env.root, "phone");
  execFileSync(env.vessel, ["nodes", "add", "laptop", "--pair", "--yes"], {
    input: `${link}\n`,
    env: {
      PATH: process.env["PATH"],
      HOME: phone,
      USERPROFILE: phone,
      VESSEL_CONFIG_DIR: join(phone, "config"),
    },
  });
  await page.reload();
  const devices = page.getByRole("list", { name: "Paired devices" });
  const row = devices.getByRole("listitem").filter({ hasText: "phone" });
  await expect(row).toBeVisible();
  await row.getByRole("button", { name: "Revoke" }).click();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Revoke" })
    .click();
  await expect(row).toBeHidden();
});

test("terminal replay cannot answer device queries, while keys and paste still work", async ({
  page,
}) => {
  const env = loadEnv();
  await open(page);
  await create(page, "laptop");
  await expect(page.getByLabel("Session terminal")).toContainText("ready");
  const sessionUrl = page.url();
  const session = new URL(sessionUrl).pathname.split("/").at(-1);
  if (session === undefined) throw new Error("No session id");
  await page.goto(`${new URL(sessionUrl).origin}/fleet`);
  const esc = "\x1b";
  const bel = "\x07";
  const queries = `${esc}[c${esc}[6n${esc}]10;?${bel}${esc}P$qm${esc}\\REPLAY_END`;
  execFileSync(env.vessel, ["--node", "laptop", "send", session, queries], {
    env: {
      PATH: process.env["PATH"],
      HOME: env.root,
      USERPROFILE: env.root,
      VESSEL_CONFIG_DIR: join(env.root, "config"),
      VESSEL_DATA_DIR: join(env.root, "ui-data"),
    },
  });
  let inputs = 0;
  page.on("websocket", (socket) => {
    socket.on("framesent", (event) => {
      const text =
        typeof event.payload === "string"
          ? event.payload
          : event.payload.toString("utf8");
      if (sendsInput(JSON.parse(text))) inputs += 1;
    });
  });
  await page.goto(sessionUrl);
  const terminal = page.getByLabel("Session terminal");
  await expect(terminal).toContainText("REPLAY_END");
  expect(inputs).toBe(0);
  await terminal.click();
  await page.keyboard.type("typed\r");
  await expect(terminal).toContainText("echo: typed");
  // Browser text insertion follows the same public input path as IME.
  await page.keyboard.insertText("한글 pasted");
  await page.keyboard.press("Enter");
  await expect(terminal).toContainText("echo: 한글 pasted");
  expect(inputs).toBeGreaterThan(0);
});

test("Hangul is drawn with Pretendard, fetching only its subsets", async ({
  page,
}) => {
  // The page names Pretendard in every stack; the face comes from the
  // package's unicode-range subsets (src/main.tsx, vite.config.ts), which
  // must carry that name, and only the subsets of the characters drawn
  // are fetched (docs/PERFORMANCE.md).
  const fonts: string[] = [];
  page.on("request", (request) => {
    if (request.resourceType() === "font") {
      fonts.push(new URL(request.url()).pathname);
    }
  });
  await open(page);
  await page.evaluate(async () => {
    const probe = document.createElement("p");
    probe.textContent = "한글 미리보기";
    probe.style.fontFamily = "Pretendard";
    document.body.append(probe);
    await document.fonts.load('16px "Pretendard"', "한글 미리보기");
  });
  const drawn = await page.evaluate(() =>
    [...document.fonts].some(
      (face) => face.family === "Pretendard" && face.status === "loaded",
    ),
  );
  expect(drawn).toBe(true);
  const subsets = fonts.filter((path) => path.includes("PretendardVariable"));
  expect(subsets.length).toBeGreaterThan(0);
  expect(subsets.every((path) => path.includes(".subset."))).toBe(true);
  expect(subsets.length).toBeLessThan(10);
});
