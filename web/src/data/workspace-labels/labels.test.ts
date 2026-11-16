import { describe, expect, test } from "vitest";

import { fixtureId } from "../../testing/ids";
import type { CatalogCursor, CatalogList, Label } from "../../wire/proto";
import type { Outcome } from "../outcome";
import { CatalogReplica } from "./catalog";
import { filterByLabels, passesLabels, withoutDeleted } from "./filter";
import { ManagerModel } from "./manager";
import { chipsOf, mergeCatalogs } from "./merge";
import { labelKey, normalizeLabelName, UNLABELLED } from "./names";
import { PickerModel } from "./picker";

const GEN = fixtureId("WorkspaceLabelGeneration", "g1");
const at = (after: number): CatalogCursor => ({ generation: GEN, after });
const label = (name: string, color: Label["color"] = "violet"): Label => ({
  name,
  color,
});
const laptop = fixtureId("NodeKey", "laptop");
const gpu = fixtureId("NodeKey", "gpu-box");
const ok: Outcome<unknown> = { ok: true, value: null };
const refused: Outcome<unknown> = {
  ok: false,
  problem: { from: "op", reason: "failed", message: "label name taken" },
};

describe("label names", () => {
  test("collapse whitespace and fold case as the node does", () => {
    expect(normalizeLabelName("  needs \t review  ")).toBe("needs review");
    expect(labelKey("Needs  Review")).toBe(labelKey("needs review"));
    expect(normalizeLabelName("   ")).toBe(UNLABELLED);
  });
});

describe("a machine's catalog", () => {
  const snapshot = (labels: Label[], after: number): CatalogList => ({
    data: { mode: "snapshot", labels },
    head: at(after),
  });

  test("applies changes that follow its cursor", () => {
    const replica = new CatalogReplica();
    expect(replica.apply(snapshot([label("Blocked", "red")], 1))).toBe(true);
    const renamed: CatalogList = {
      data: {
        mode: "changes",
        from: at(1),
        updates: [
          {
            cursor: at(2),
            delta: {
              kind: "upsert",
              label: label("Waiting", "red"),
              previous: "Blocked",
            },
          },
          {
            cursor: at(3),
            delta: { kind: "upsert", label: label("Docs"), previous: null },
          },
        ],
      },
      head: at(3),
    };
    expect(replica.apply(renamed)).toBe(true);
    expect(replica.labels().map((each) => each.name)).toEqual([
      "Docs",
      "Waiting",
    ]);
    expect(replica.head).toEqual(at(3));
  });

  test("refuses a gap so the caller reads a snapshot", () => {
    const replica = new CatalogReplica();
    replica.apply(snapshot([label("Docs")], 1));
    const skipped: CatalogList = {
      data: {
        mode: "changes",
        from: at(4),
        updates: [{ cursor: at(5), delta: { kind: "remove", name: "Docs" } }],
      },
      head: at(5),
    };
    expect(replica.apply(skipped)).toBe(false);
    expect(replica.labels().map((each) => each.name)).toEqual(["Docs"]);
  });
});

describe("labels across machines", () => {
  test("the target machine's definition wins, then the first by key", () => {
    const merged = mergeCatalogs(
      [
        { node: laptop, labels: [label("Blocked", "red"), label("docs")] },
        { node: gpu, labels: [label("blocked", "amber")] },
      ],
      gpu,
    );
    expect(merged).toEqual([label("blocked", "amber"), label("docs")]);
  });

  test("chips come from the workspace's own catalog, in assignment order", () => {
    const catalog = [label("Blocked", "red"), label("Docs", "sky")];
    expect(chipsOf(catalog, ["docs", "gone", "blocked"])).toEqual([
      label("Docs", "sky"),
      label("Blocked", "red"),
    ]);
  });
});

describe("the label filter", () => {
  const rows = [
    { id: "a", labels: ["Blocked"] },
    { id: "b", labels: [] },
    { id: "c", labels: ["Docs", "blocked"] },
  ];

  test("no pick lists everything", () => {
    expect(filterByLabels(rows, [])).toHaveLength(3);
  });

  test("picks include a workspace carrying any of them", () => {
    expect(filterByLabels(rows, ["BLOCKED"]).map((row) => row.id)).toEqual([
      "a",
      "c",
    ]);
  });

  test("Unlabelled is one more key", () => {
    expect(
      filterByLabels(rows, [UNLABELLED, "docs"]).map((row) => row.id),
    ).toEqual(["b", "c"]);
    expect(passesLabels([], ["docs"])).toBe(false);
  });

  test("a deleted label stops filtering", () => {
    expect(withoutDeleted(["Gone", UNLABELLED, "docs"], ["Docs"])).toEqual([
      UNLABELLED,
      "docs",
    ]);
  });
});

describe("the picker", () => {
  test("a second press on a label in flight joins the first", async () => {
    const sent: boolean[] = [];
    const waiting: ((outcome: Outcome<unknown>) => void)[] = [];
    const model = new PickerModel((_label, assigned) => {
      sent.push(assigned);
      return new Promise((resolve) => {
        waiting.push(resolve);
      });
    });
    model.sync({ labels: [label("Docs")], assigned: [], online: true });
    const first = model.toggle(label("Docs"), true);
    const second = model.toggle(label("docs"), false);
    expect(model.snapshot().pending).toEqual(["docs"]);
    waiting[0]?.(ok);
    expect(await first).toBe(true);
    expect(await second).toBe(true);
    expect(sent).toEqual([true]);
    expect(model.snapshot().pending).toEqual([]);
  });

  test("a failure says why and ticks follow the workspace's names", async () => {
    const model = new PickerModel(() => Promise.resolve(refused));
    model.sync({ labels: [label("Docs")], assigned: ["DOCS"], online: true });
    expect(model.snapshot().rows[0]?.assigned).toBe(true);
    expect(await model.toggle(label("Docs"), false)).toBe(false);
    expect(model.snapshot().error).toBe("label name taken");
  });
});

describe("the manager", () => {
  const machine = {
    node: laptop,
    name: "laptop",
    state: "online" as const,
    labels: [label("Blocked", "red"), label("Docs", "sky")],
  };

  test("Save is offered only for a real change and sends what changed", async () => {
    const updates: unknown[] = [];
    const model = new ManagerModel({
      update: (_node, name, change) => {
        updates.push({ name, ...change });
        return Promise.resolve(ok);
      },
      inspect: () => Promise.resolve({ ok: true, value: 0 }),
      remove: () => Promise.resolve(ok),
    });
    model.syncMachines([machine]);
    model.edit(label("Blocked", "red"));
    model.setName("  Blocked ");
    expect(model.snapshot().dirty).toBe(false);
    model.setName("   ");
    expect(model.snapshot().dirty).toBe(false);
    model.setColor("amber");
    model.setName("Blocked");
    expect(model.snapshot().dirty).toBe(true);
    await model.save();
    expect(updates).toEqual([{ name: "Blocked", color: "amber" }]);
    expect(model.snapshot().draft).toBeNull();
  });

  test("a failed save keeps the draft as typed", async () => {
    const model = new ManagerModel({
      update: () => Promise.resolve(refused),
      inspect: () => Promise.resolve({ ok: true, value: 0 }),
      remove: () => Promise.resolve(ok),
    });
    model.syncMachines([machine]);
    model.edit(label("Docs", "sky"));
    model.setName("Documentation");
    await model.save();
    expect(model.snapshot().draft?.typed).toBe("Documentation");
    expect(model.snapshot().error).toBe("label name taken");
  });

  test("delete asks with the inspected count and stops on no", async () => {
    const removed: string[] = [];
    const asked: number[] = [];
    const model = new ManagerModel({
      update: () => Promise.resolve(ok),
      inspect: () => Promise.resolve({ ok: true, value: 3 }),
      remove: (_node, name) => {
        removed.push(name);
        return Promise.resolve(ok);
      },
    });
    model.syncMachines([machine]);
    model.edit(label("Docs", "sky"));
    await model.delete((count) => {
      asked.push(count);
      return Promise.resolve(false);
    });
    expect(removed).toEqual([]);
    await model.delete(() => Promise.resolve(true));
    expect(asked).toEqual([3]);
    expect(removed).toEqual(["Docs"]);
  });

  test("it opens on the first machine that answers until one is picked", () => {
    const model = new ManagerModel({
      update: () => Promise.resolve(ok),
      inspect: () => Promise.resolve({ ok: true, value: 0 }),
      remove: () => Promise.resolve(ok),
    });
    const off = {
      ...machine,
      node: gpu,
      name: "gpu-box",
      state: "offline" as const,
    };
    model.syncMachines([off, machine]);
    expect(model.snapshot().node).toBe(laptop);
    model.selectMachine(gpu);
    model.syncMachines([off, machine]);
    expect(model.snapshot().node).toBe(gpu);
  });

  test("search narrows the list", () => {
    const model = new ManagerModel({
      update: () => Promise.resolve(ok),
      inspect: () => Promise.resolve({ ok: true, value: 0 }),
      remove: () => Promise.resolve(ok),
    });
    model.syncMachines([machine]);
    model.setQuery(" doc ");
    expect(model.snapshot().labels.map((each) => each.name)).toEqual(["Docs"]);
  });
});
