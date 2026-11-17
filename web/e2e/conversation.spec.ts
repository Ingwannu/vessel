// The conversation on the page (milestone 11, Track A-page), against a
// real node and the fake agent: the node's queue over the composer, the
// send button's ways while a turn runs, attachments, the `/` and `@`
// list, a question the agent asks, the context meter and a turn's
// tokens.

import { expect, type Locator, type Page, test } from "@playwright/test";

import { choose, openNew, startSession, timeline } from "./kit";
import { openUi } from "./open";

function composer(page: Page): Locator {
  return page.getByRole("form", { name: "Prompt" });
}

function text(page: Page): Locator {
  return composer(page).getByRole("textbox", { name: "Prompt" });
}

function queue(page: Page): Locator {
  return page.getByRole("region", { name: /queued messages$/ });
}

/** A one-pixel PNG, as a browser hands a picked or pasted image. */
const PNG = Buffer.from(
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg==",
  "base64",
);

/** Settings › General's Default send. The bridge keeps it for every later
 * test of the fleet, so a test that chooses Queue puts the default (Steer,
 * ADR 0108) back. */
async function defaultSend(
  page: Page,
  origin: string,
  choice: "Queue" | "Steer",
): Promise<void> {
  await page.goto(`${origin}/settings/general`);
  const send = page.getByRole("combobox", { name: "Default send" });
  await choose(page, send, new RegExp(`^${choice}`));
  await expect(send).toContainText(choice);
}

test("a message sent during a turn waits in the node's queue, and is edited, moved, held and sent", async ({
  page,
}) => {
  test.setTimeout(120_000);
  const origin = await openUi(page);
  await defaultSend(page, origin, "Queue");
  try {
    await queueFlow(page);
  } finally {
    await defaultSend(page, origin, "Steer");
  }
});

/** The queue's whole round, with Default send on Queue. */
async function queueFlow(page: Page): Promise<void> {
  await startSession(page, "laptop", "Structured", "wait");
  await expect(timeline(page).getByText("turn 1")).toBeVisible();
  await expect(text(page)).toHaveAttribute(
    "placeholder",
    "Next message you send will be queued",
  );
  // The button says what Enter will do.
  await text(page).fill("plain");
  await expect(
    composer(page).getByRole("button", { name: "Queue message" }),
  ).toBeVisible();
  await text(page).press("Enter");
  await expect(queue(page)).toContainText("1 Queued Message");
  await text(page).fill("cwd");
  await text(page).press("Enter");
  await expect(queue(page)).toContainText("2 Queued Messages");
  const rows = queue(page).getByRole("listitem");
  await expect(rows).toHaveText([/plain/, /cwd/]);

  // Edited in the composer, saved with Update.
  await rows
    .nth(1)
    .getByRole("button", { name: "Edit queued message" })
    .click();
  await expect(text(page)).toHaveValue("cwd");
  await expect(queue(page)).toContainText("· editing");
  await text(page).fill("options");
  await composer(page)
    .getByRole("button", { name: "Update queued message" })
    .click();
  await expect(rows).toHaveText([/plain/, /options/]);
  await expect(text(page)).toHaveValue("");

  // Moved up from its grip, by key.
  await rows
    .nth(1)
    .getByRole("button", { name: "Drag to reorder queued message" })
    .press("ArrowUp");
  await expect(rows).toHaveText([/options/, /plain/]);

  // Deleted.
  await rows
    .nth(1)
    .getByRole("button", { name: "Delete queued message" })
    .click();
  await expect(rows).toHaveText([/options/]);

  // Stop holds what waits; Resume lets it go.
  await composer(page).getByRole("button", { name: "Stop turn" }).click();
  await expect(queue(page)).toContainText("Paused after you stopped the agent");
  await queue(page).getByRole("button", { name: "Resume" }).click();
  await expect(timeline(page).getByText(/^options mode=/)).toBeVisible();
  await expect(queue(page)).toBeHidden();
}

test("by default a message sent during a turn steers it, and Mod+Enter queues (ADR 0108)", async ({
  page,
}) => {
  await openUi(page);
  await startSession(page, "laptop", "Structured", "wait");
  // The other way exists only once the page knows a turn runs.
  await expect(timeline(page).getByText("turn 1")).toBeVisible();
  await expect(text(page)).toHaveAttribute(
    "placeholder",
    "Next message you send will steer the agent",
  );
  await text(page).fill("finish");
  await expect(
    composer(page).getByRole("button", { name: "Steer message" }),
  ).toBeVisible();
  await page.keyboard.down("Control");
  await expect(
    composer(page).getByRole("button", { name: "Queue message" }),
  ).toBeVisible();
  await page.keyboard.up("Control");
  await text(page).press("Enter");
  await expect(
    timeline(page).getByText("steered", { exact: true }),
  ).toBeVisible();
  await expect(timeline(page).getByText("steered: finish")).toBeVisible();
  await expect(timeline(page).getByText(/Turn ended · end turn/)).toBeVisible();
});

test("a Queue while a permission waits stops the turn instead, and says so (TRACKER 794)", async ({
  page,
}) => {
  const origin = await openUi(page);
  await defaultSend(page, origin, "Queue");
  try {
    await startSession(page, "gpu-box", "Structured", "tidy the overview");
    const prompt = page.getByRole("group", { name: "Permission request" });
    await expect(prompt).toContainText("Run fake tool");
    // Paseo's: a queued message would be stranded behind the request.
    await expect(text(page)).toBeEnabled();
    await expect(text(page)).toHaveAttribute(
      "placeholder",
      "Next message you send will stop the agent and go next",
    );
    await text(page).fill("later");
    await expect(
      composer(page).getByRole("button", { name: "Interrupt and send" }),
    ).toBeVisible();
    await text(page).fill("");
    await prompt.getByRole("button", { name: "Allow" }).click();
    await expect(prompt).toBeHidden();
  } finally {
    await defaultSend(page, origin, "Steer");
  }
});

test("Default send Queue, kept by the bridge, makes Send queue and Mod+Enter interrupt and send", async ({
  page,
}) => {
  const origin = await openUi(page);
  await defaultSend(page, origin, "Queue");
  // Paseo's hint for the choice, by the send key.
  await expect(
    page.getByText(
      "When the agent is running, Enter queues. Command/Ctrl+Enter submits.",
    ),
  ).toBeVisible();
  try {
    await startSession(page, "laptop", "Structured", "wait");
    await expect(timeline(page).getByText("turn 1")).toBeVisible();
    await text(page).fill("finish");
    await expect(
      composer(page).getByRole("button", { name: "Queue message" }),
    ).toBeVisible();
    // The other way submits: the turn stops and this one starts (Paseo's
    // "Command/Ctrl+Enter submits"), even though this agent could steer.
    await page.keyboard.down("Control");
    await expect(
      composer(page).getByRole("button", { name: "Interrupt and send" }),
    ).toBeVisible();
    await page.keyboard.up("Control");
    await text(page).press("ControlOrMeta+Enter");
    await expect(timeline(page).getByText(/^echo: finish/)).toBeVisible();
    // The fake agent's next turn asks a permission: answer it.
    const prompt = page.getByRole("group", { name: "Permission request" });
    await prompt.getByRole("button", { name: "Allow" }).click();
    await expect(prompt).toBeHidden();
  } finally {
    await defaultSend(page, origin, "Steer");
  }
});

test("files go with a prompt, and the prompt shows them", async ({ page }) => {
  await openUi(page);
  await startSession(page, "laptop", "Structured", "plain");
  await expect(timeline(page).getByText("echo: plain")).toBeVisible();
  const add = composer(page).getByRole("button", { name: "Attach files" });
  await expect(add).toBeEnabled();
  const chooser = page.waitForEvent("filechooser");
  await add.click();
  await (
    await chooser
  ).setFiles([
    { name: "notes.txt", mimeType: "text/plain", buffer: Buffer.from("hello") },
    { name: "dot.png", mimeType: "image/png", buffer: PNG },
  ]);
  const strip = composer(page).getByRole("list", { name: "Attached files" });
  await expect(strip.getByRole("listitem")).toHaveCount(2);
  await expect(strip).toContainText("notes.txt");
  // The image's chip shows it (a `data:` thumbnail: the page's content
  // policy takes no `blob:` address).
  await expect(strip.locator("img.attachment-thumb")).toHaveJSProperty(
    "complete",
    true,
  );
  expect(
    await strip
      .locator("img.attachment-thumb")
      .evaluate((image: HTMLImageElement) => image.naturalWidth),
  ).toBeGreaterThan(0);
  await text(page).fill("describe");
  await text(page).press("Enter");
  // The agent got both: the file as a link it read, the image inline.
  await expect(
    timeline(page).getByText(/link notes\.txt text\/plain 5 read 5/),
  ).toBeVisible();
  await expect(
    timeline(page).getByText(/image image\/png \d+ bytes/),
  ).toBeVisible();
  await expect(strip).toBeHidden();
  // The prompt shows its files: the image from this tab's own copy.
  const files = timeline(page).getByRole("list", { name: "Attached files" });
  const tile = files.getByRole("button", { name: "Open dot.png" });
  await expect(tile).toBeEnabled();
  expect(
    await tile
      .locator("img")
      .evaluate((image: HTMLImageElement) => image.naturalWidth),
  ).toBeGreaterThan(0);
  await expect(files).toContainText("notes.txt");
  // Opened, it is read at full size.
  await tile.click();
  const full = page.getByRole("dialog", { name: "dot.png" });
  await expect(full.getByRole("img", { name: "dot.png" })).toBeVisible();
  await page.keyboard.press("Escape");

  // A removed file is not sent.
  const again = page.waitForEvent("filechooser");
  await add.click();
  await (
    await again
  ).setFiles({
    name: "gone.txt",
    mimeType: "text/plain",
    buffer: Buffer.from("x"),
  });
  await strip.getByRole("button", { name: "Remove gone.txt" }).click();
  await expect(strip).toBeHidden();
});

test("a new session's first message takes its files with it", async ({
  page,
}) => {
  await openUi(page);
  await openNew(page);
  const box = composer(page);
  await box.getByRole("textbox", { name: "Prompt" }).fill("describe");
  await box.getByRole("button", { name: "Add", exact: true }).click();
  const chooser = page.waitForEvent("filechooser");
  await page
    .getByRole("button", { name: "Attach files to this message" })
    .click();
  await (
    await chooser
  ).setFiles({
    name: "brief.md",
    mimeType: "text/markdown",
    buffer: Buffer.from("# brief"),
  });
  await page.keyboard.press("Escape");
  await expect(box.getByRole("list", { name: "Attached files" })).toContainText(
    "brief.md",
  );
  await box.getByRole("button", { name: "Send" }).click();
  await expect(page).toHaveURL(/\/sessions\/[^/]+\/[^/]+$/);
  await expect(
    timeline(page).getByText(/link brief\.md text\/markdown 7 read 7/),
  ).toBeVisible();
});

test("/ lists the agent's commands and @ says when there is nothing to search", async ({
  page,
}) => {
  await openUi(page);
  await startSession(page, "laptop", "Structured", "plain");
  await expect(timeline(page).getByText("echo: plain")).toBeVisible();
  await text(page).fill("/");
  const list = page.getByRole("listbox", { name: "Commands" });
  await expect(list.getByRole("option")).toHaveText([/\/compact/, /\/review/]);
  await text(page).press("ArrowDown");
  await expect(list.getByRole("option", { name: /\/review/ })).toHaveAttribute(
    "aria-selected",
    "true",
  );
  await text(page).press("Enter");
  await expect(text(page)).toHaveValue("/review ");
  await expect(list).toBeHidden();
  // Escape hides the list until the word changes.
  await text(page).fill("/co");
  await expect(list.getByRole("option")).toHaveText([/\/compact/]);
  await text(page).press("Escape");
  await expect(list).toBeHidden();
  // A session started in a bare folder has no project for `@`.
  await text(page).fill("look at @ma");
  await expect(
    page.getByRole("listbox", { name: "Files" }).locator(".."),
  ).toContainText("This session has no project to search.");
});

// The caret a chosen suggestion puts after itself is placed with the text,
// not a frame later: a later frame undid a selection made meanwhile, so
// typing over a selected box appended instead (W11's run, 2026-11-15:
// "/review /co" listed no command). The page's animation frames are held
// while /review is chosen and the box is selected, then let go, and only
// then is the box typed over.
test("a box selected just after a command is chosen is typed over, not added to", async ({
  page,
}) => {
  await page.addInitScript(() => {
    const frame = window.requestAnimationFrame.bind(window);
    const held: FrameRequestCallback[] = [];
    window.requestAnimationFrame = (callback) =>
      frame((time) => {
        if (document.documentElement.dataset["holdFrames"] === "on") {
          held.push(callback);
        } else {
          callback(time);
        }
      });
    document.addEventListener("release-frames", () => {
      document.documentElement.dataset["holdFrames"] = "off";
      const now = performance.now();
      for (const callback of held.splice(0)) {
        callback(now);
      }
    });
  });
  await openUi(page);
  await startSession(page, "laptop", "Structured", "plain");
  await expect(timeline(page).getByText("echo: plain")).toBeVisible();
  await text(page).fill("/");
  const list = page.getByRole("listbox", { name: "Commands" });
  await expect(list.getByRole("option")).toHaveText([/\/compact/, /\/review/]);
  await text(page).press("ArrowDown");
  await page.evaluate(() => {
    document.documentElement.dataset["holdFrames"] = "on";
  });
  await text(page).press("Enter");
  await expect(text(page)).toHaveValue("/review ");
  await text(page).evaluate((area: HTMLTextAreaElement) => {
    area.select();
  });
  await page.evaluate(() => {
    document.dispatchEvent(new Event("release-frames"));
  });
  await page.keyboard.insertText("/co");
  await expect(text(page)).toHaveValue("/co");
  await expect(list.getByRole("option")).toHaveText([/\/compact/]);
});

test("a question the agent asks is answered in the dock over the composer", async ({
  page,
}) => {
  await openUi(page);
  await startSession(page, "laptop", "Structured", "ask claude");
  const dock = page.getByRole("region", { name: "Question from the agent" });
  await expect(dock).toContainText("Question · 1 of 2");
  await expect(dock).toContainText("Which database should the cache use?");
  // Next without an answer: it is not required, so it goes on.
  await dock.getByRole("radio", { name: /SQLite/ }).click();
  await dock.getByRole("button", { name: "Next" }).click();
  await expect(dock).toContainText("Question · 2 of 2");
  await dock.getByRole("checkbox", { name: /Metrics/ }).click();
  await dock.getByRole("checkbox", { name: /Backups/ }).click();
  await dock.getByRole("textbox", { name: "Other" }).fill("and alerts");
  await dock.getByRole("button", { name: "Submit answer" }).click();
  await expect(dock).toBeHidden();
  await expect(
    timeline(page).getByText(
      'answered: accept {"question_0":"SQLite","question_1":["metrics","backups"],"question_1_custom":"and alerts"}',
    ),
  ).toBeVisible();
  // The timeline keeps the answers, by their titles.
  await expect(timeline(page).locator(".question-card")).toContainText(
    "Extras",
  );
  await expect(timeline(page).locator(".question-card")).toContainText(
    "Metrics, Backups",
  );

  // Skipped.
  await text(page).fill("ask");
  await text(page).press("Enter");
  await dock.getByRole("button", { name: "Skip" }).click();
  await expect(timeline(page).getByText("answered: decline")).toBeVisible();
});

test("a question's dock on a phone leaves the timeline's end in view", async ({
  page,
}) => {
  await openUi(page);
  await startSession(page, "laptop", "Structured", "ask claude");
  const dock = page.getByRole("region", { name: "Question from the agent" });
  await expect(dock).toBeVisible();
  // The window narrows to a phone's under the open dock.
  await page.setViewportSize({ width: 390, height: 844 });
  const waiting = timeline(page).getByText(/^Waiting on an answer/);
  await expect(async () => {
    const line = await waiting.boundingBox();
    const over = await dock.boundingBox();
    expect(line).not.toBeNull();
    expect(over).not.toBeNull();
    if (line !== null && over !== null) {
      expect(line.y + line.height).toBeLessThanOrEqual(over.y);
    }
  }).toPass();
});

test("the context meter and a turn's tokens are the agent's own figures", async ({
  page,
}) => {
  await openUi(page);
  await startSession(page, "laptop", "Structured", "usage");
  await expect(
    timeline(page).getByText(
      /Turn ended · end turn · 30 tokens · 20 in · 10 out/,
    ),
  ).toBeVisible();
  const ring = composer(page).getByRole("img", { name: "Context 1% used" });
  await expect(ring).toBeVisible();
  await ring.hover();
  await expect(page.getByText("1% used", { exact: true })).toBeVisible();
  await expect(
    page.getByText("1k / 200k tokens", { exact: true }),
  ).toBeVisible();
  await expect(page.getByText("Session cost $0.50")).toBeVisible();
});
