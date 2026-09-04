// Saves screen: per world (ctx.session.lastWorldId, set by whoever navigated
// here — the Worlds screen's Branch action, or the title menu's Load Save),
// lists saves and offers Save now / Verify / Branch, per the plan's Saves
// screen (planning/console-and-multi-world-server.md, screen 6). Every
// admin action confirms; Save now and Branch also collect a name, which
// ui/dialog's confirm() has no field for, so this screen builds its own
// name-prompt dialog on the same CSS (.dialog-backdrop/.dialog/.dialog-
// actions/.field) rather than touching that shared file.

import type { ApiResult, SaveRecord } from "../api";
import type { AppContext, Screen } from "../screens";
import { button, el, field } from "../ui/dom";
import { confirm } from "../ui/dialog";
import { liveScreen } from "./live";
import { worldsScreen } from "./worlds";
import "./saves.css";

const FOCUSABLE_SELECTOR =
  'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])';

interface PromptOptions {
  title: string;
  body: string;
  label: string;
  confirmLabel: string;
  danger?: boolean;
  placeholder?: string;
}

/// A confirm dialog with one text field, for the two actions here that need
/// a name as well as a yes/no. Mirrors ui/dialog.ts's confirm(): focus trap,
/// Escape cancels, Enter (in the field) confirms, focus returns on close.
function promptForName(options: PromptOptions): Promise<string | null> {
  return new Promise((resolve) => {
    const returnFocusTo = document.activeElement as HTMLElement | null;

    const backdrop = el("div", { class: "dialog-backdrop" });
    const heading = el("h2", { id: "prompt-dialog-title" }, [options.title]);
    const bodyPara = el("p", { id: "prompt-dialog-body" }, [options.body]);
    const input = el("input", {
      type: "text",
      placeholder: options.placeholder ?? "",
    }) as HTMLInputElement;
    const fieldNode = field(options.label, input);
    const errorNode = el("p", { class: "field-error-message", role: "alert" }, []);
    errorNode.hidden = true;

    const cleanup = (result: string | null) => {
      document.removeEventListener("keydown", onKeyDown, true);
      backdrop.remove();
      returnFocusTo?.focus();
      resolve(result);
    };

    const cancelButton = button("Cancel", () => cleanup(null));
    const confirmButton = button(
      options.confirmLabel,
      () => {
        const value = input.value.trim();
        if (value.length === 0) {
          errorNode.textContent = "A name is required.";
          errorNode.hidden = false;
          input.focus();
          return;
        }
        cleanup(value);
      },
      { variant: options.danger ? "danger" : "primary" },
    );

    const dialog = el(
      "div",
      {
        class: "dialog",
        role: "alertdialog",
        "aria-modal": "true",
        "aria-labelledby": "prompt-dialog-title",
        "aria-describedby": "prompt-dialog-body",
      },
      [heading, bodyPara, fieldNode, errorNode, el("div", { class: "dialog-actions" }, [cancelButton, confirmButton])],
    );
    backdrop.append(dialog);

    function onKeyDown(event: KeyboardEvent): void {
      if (event.key === "Escape") {
        event.preventDefault();
        cleanup(null);
        return;
      }
      if (event.key === "Enter" && event.target === input) {
        event.preventDefault();
        confirmButton.click();
        return;
      }
      if (event.key !== "Tab") return;
      const focusable = Array.from(dialog.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR));
      if (focusable.length === 0) return;
      const first = focusable[0]!;
      const last = focusable[focusable.length - 1]!;
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    }

    document.addEventListener("keydown", onKeyDown, true);
    document.body.append(backdrop);
    input.focus();
  });
}

function formatBytes(value: unknown): string {
  const n = typeof value === "number" && Number.isFinite(value) ? value : Number.NaN;
  if (Number.isNaN(n)) return "n/a";
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

function formatCreated(value: unknown): string {
  const n = typeof value === "number" && Number.isFinite(value) ? value : Number.NaN;
  if (Number.isNaN(n)) return "—";
  return new Date(n).toLocaleString();
}

const REPORT_FIELDS: [string, string][] = [
  ["result", "Result"],
  ["tick", "Tick"],
  ["seed", "Seed"],
  ["config_hash", "Config hash"],
  ["state_checksum", "State checksum"],
  ["population", "Population"],
  ["build_version", "Build version"],
  ["verified", "Verified"],
  ["kind", "Kind"],
  ["bytes", "Bytes"],
];

/// The verify route's real response shape (result/tick/seed/population/
/// build_version) doesn't match the SaveRecord type api.ts declares for it
/// (verified/kind/bytes/...) — the two were built concurrently. Render
/// whatever known fields are actually present instead of trusting the type.
function buildReport(raw: unknown): HTMLElement {
  const record = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const rows: HTMLElement[] = [];
  for (const [key, label] of REPORT_FIELDS) {
    if (!(key in record)) continue;
    rows.push(
      el("div", { class: "report-row" }, [
        el("span", { class: "report-label" }, [label]),
        el("span", { class: "report-value" }, [String(record[key])]),
      ]),
    );
  }
  if (rows.length === 0) {
    rows.push(el("p", {}, ["Verification succeeded; the server returned no report fields."]));
  }
  return el("div", { class: "verify-report", role: "status" }, [el("h3", {}, ["Verification report"]), ...rows]);
}

export function savesScreen(ctx: AppContext): Screen {
  let root: HTMLElement | null = null;
  let listContainer: HTMLElement | null = null;
  let bannerNode: HTMLElement | null = null;
  let headingNode: HTMLElement | null = null;

  const worldId = ctx.session.lastWorldId;
  let worldLabel = worldId === undefined ? "" : `World ${worldId}`;
  let saves: SaveRecord[] = [];
  let bannerMessage: string | null = null;
  const rowErrors = new Map<number, string>();
  const busyIds = new Set<number>();
  let savingNow = false;
  let report: { saveId: number; raw: unknown } | null = null;

  async function loadWorldLabel(): Promise<void> {
    if (worldId === undefined) return;
    const result = await ctx.api.getWorld(worldId);
    if (result.ok && typeof result.value.name === "string" && result.value.name.length > 0) {
      worldLabel = result.value.name;
    }
  }

  function paintBanner(): void {
    if (!bannerNode) return;
    if (bannerMessage === null) {
      bannerNode.hidden = true;
      bannerNode.textContent = "";
    } else {
      bannerNode.hidden = false;
      bannerNode.textContent = bannerMessage;
    }
  }

  function updateHeading(): void {
    if (headingNode) headingNode.textContent = worldId === undefined ? "Saves" : `Saves — ${worldLabel}`;
  }

  function buildRow(save: SaveRecord, isAdmin: boolean): HTMLElement[] {
    const busy = busyIds.has(save.save_id);
    const verifiedBadge = el(
      "span",
      { class: `verified-badge ${save.verified ? "verified-badge--yes" : "verified-badge--no"}` },
      [save.verified ? "Verified" : "Unverified"],
    );
    const actionsCell = el("td", { class: "saves-actions" }, []);
    if (isAdmin) {
      actionsCell.append(
        button("Verify", () => void handleVerify(save), { disabled: busy }),
        button("Branch", () => void handleBranch(save), { disabled: busy }),
      );
    }
    const tr = el("tr", {}, [
      el("td", {}, [save.name]),
      el("td", {}, [save.kind]),
      el("td", {}, [String(save.tick)]),
      el("td", {}, [formatBytes(save.bytes)]),
      el("td", {}, [verifiedBadge]),
      el("td", {}, [formatCreated(save.created_unix_ms)]),
      actionsCell,
    ]);
    const rows = [tr];
    const err = rowErrors.get(save.save_id);
    if (err) {
      rows.push(el("tr", {}, [el("td", { colspan: 7, class: "row-error", role: "alert" }, [err])]));
    }
    return rows;
  }

  function render(): void {
    if (!listContainer) return;
    listContainer.innerHTML = "";

    if (worldId === undefined) {
      listContainer.append(
        el("div", { class: "no-world" }, [
          el("p", {}, ["No world is selected. Choose a world first."]),
          button("Worlds", () => void ctx.stack.push(worldsScreen(ctx)), { variant: "primary" }),
        ]),
      );
      return;
    }

    const isAdmin = ctx.session.role === "admin";
    const toolbar = el("div", { class: "saves-toolbar" }, [
      button("Refresh", () => void refresh()),
    ]);
    if (isAdmin) {
      toolbar.append(
        button(
          "Save now",
          () => void handleSaveNow(),
          { variant: "primary", disabled: savingNow },
        ),
      );
    }
    listContainer.append(toolbar);

    if (saves.length === 0) {
      listContainer.append(el("p", { class: "saves-empty" }, ["No saves yet for this world."]));
    } else {
      const thead = el("thead", {}, [
        el("tr", {}, [
          el("th", { scope: "col" }, ["Name"]),
          el("th", { scope: "col" }, ["Kind"]),
          el("th", { scope: "col" }, ["Tick"]),
          el("th", { scope: "col" }, ["Size"]),
          el("th", { scope: "col" }, ["Verified"]),
          el("th", { scope: "col" }, ["Created"]),
          el("th", { scope: "col" }, ["Actions"]),
        ]),
      ]);
      const tbody = el("tbody", {}, []);
      for (const save of saves) {
        tbody.append(...buildRow(save, isAdmin));
      }
      const table = el("table", { class: "saves-table" }, [thead, tbody]);
      listContainer.append(el("div", { class: "saves-table-wrap" }, [table]));
    }

    if (report) {
      listContainer.append(buildReport(report.raw));
    }
  }

  async function refresh(): Promise<void> {
    if (!root || worldId === undefined) {
      render();
      return;
    }
    const result = await ctx.api.listSaves(worldId);
    if (!result.ok) {
      bannerMessage = `Could not load saves: ${result.message}`;
      paintBanner();
      return;
    }
    bannerMessage = null;
    paintBanner();
    saves = result.value;
    render();
  }

  async function handleSaveNow(): Promise<void> {
    if (worldId === undefined || savingNow) return;
    const name = await promptForName({
      title: `Save ${worldLabel} now?`,
      body: `Create a new save for "${worldLabel}" (#${worldId}).`,
      label: "Save name",
      confirmLabel: "Save now",
      placeholder: "e.g. before-migration",
    });
    if (name === null) return;
    savingNow = true;
    render();
    const result = await ctx.api.createSave(worldId, name);
    savingNow = false;
    if (!result.ok) {
      bannerMessage = `Save failed: ${result.message}`;
      ctx.announce(`Save failed: ${result.message}`);
    } else {
      bannerMessage = null;
      ctx.announce(`Save "${name}" created for ${worldLabel}.`);
    }
    paintBanner();
    render();
    await refresh();
  }

  async function handleVerify(save: SaveRecord): Promise<void> {
    if (worldId === undefined) return;
    const proceed = await confirm({
      title: `Verify save "${save.name}"?`,
      body: `Rebuild save "${save.name}" (#${save.save_id}) in isolation and check it against its recorded checksum.`,
      confirmLabel: "Verify",
    });
    if (!proceed) return;
    busyIds.add(save.save_id);
    render();
    const result: ApiResult<unknown> = await ctx.api.verifySave(worldId, save.save_id);
    busyIds.delete(save.save_id);
    if (!result.ok) {
      rowErrors.set(save.save_id, result.message);
      ctx.announce(`Verify failed for save "${save.name}": ${result.message}`);
    } else {
      rowErrors.delete(save.save_id);
      report = { saveId: save.save_id, raw: result.value };
      ctx.announce(`Verification finished for save "${save.name}".`);
    }
    render();
    await refresh();
  }

  async function handleBranch(save: SaveRecord): Promise<void> {
    if (worldId === undefined) return;
    const name = await promptForName({
      title: `Branch from save "${save.name}"?`,
      body: `Create a new world loaded from save "${save.name}" (#${save.save_id}) of ${worldLabel}.`,
      label: "New world name",
      confirmLabel: "Branch",
      placeholder: "e.g. what-if-drought",
    });
    if (name === null) return;
    busyIds.add(save.save_id);
    render();
    const result = await ctx.api.branch(worldId, save.save_id, name);
    busyIds.delete(save.save_id);
    if (!result.ok) {
      rowErrors.set(save.save_id, result.message);
      ctx.announce(`Branch failed: ${result.message}`);
      render();
      return;
    }
    const newName = typeof result.value.name === "string" && result.value.name.length > 0 ? result.value.name : name;
    ctx.announce(`Branched into new world "${newName}".`);
    ctx.session.lastWorldId = result.value.world_id;
    await ctx.stack.push(liveScreen(result.value.world_id, ctx));
  }

  return {
    id: "saves",
    get title(): string {
      return worldId === undefined ? "Saves" : `Saves — ${worldLabel}`;
    },

    async mount(mountRoot: HTMLElement): Promise<void> {
      root = mountRoot;
      headingNode = el("h1", {}, [worldId === undefined ? "Saves" : `Saves — ${worldLabel}`]);
      const header = el("div", { class: "screen-header" }, [
        headingNode,
        button("Back", () => void ctx.stack.pop()),
      ]);
      bannerNode = el("p", { class: "worlds-banner", role: "alert" }, []);
      bannerNode.hidden = true;
      listContainer = el("div", { class: "saves-list" }, []);
      root.append(el("div", { class: "screen" }, [header, bannerNode, listContainer]));

      if (worldId !== undefined) {
        await loadWorldLabel();
        updateHeading();
      }
      render();
      await refresh();
    },

    unmount(): void {
      root = null;
      listContainer = null;
      bannerNode = null;
      headingNode = null;
    },
  };
}
