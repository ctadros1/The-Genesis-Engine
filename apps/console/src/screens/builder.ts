// The New World builder: schema-driven settings editing at creation, per
// planning/console-and-multi-world-server.md's screen 4. The server's
// /api/schema is the single source of fields; this screen never hardcodes
// a field name or a validation rule — it only knows how to render each
// FieldType and how to diff a value against the active preset's default.
//
// Admin-only: an observer profile (no admin token, per main.ts's role
// derivation) gets every control disabled and an explanation instead of a
// Create button, but can still browse the schema and the live preview,
// since neither is a mutation.

import type { CreateWorldRequest, FieldType, Schema, SchemaField } from "../api";
import { confirm } from "../ui/dialog";
import { button, el, field } from "../ui/dom";
import type { AppContext, Screen } from "../screens";
import { liveScreen } from "./live";
import { RECIPES, type Recipe } from "../recipes";
import "./builder.css";

const PREVIEW_DEBOUNCE_MS = 300;

interface TypeRange {
  min: number;
  max: number;
  step: number;
}

/// The server's field types are fixed-width integers or bool/choice; JS
/// numbers only represent integers exactly up to 2^53-1, so u64/i64 ranges
/// are clamped there rather than to the true 64-bit bound. Every field this
/// console ships against today (byte counts, tick counts, Q16 fractions)
/// sits far inside that clamp — see ISSUES in the task report for the
/// residual gap on a hypothetical near-u64::MAX field.
function typeRange(type: FieldType): TypeRange | null {
  switch (type) {
    case "u32":
      return { min: 0, max: 4294967295, step: 1 };
    case "i32":
      return { min: -2147483648, max: 2147483647, step: 1 };
    case "u64":
      return { min: 0, max: Number.MAX_SAFE_INTEGER, step: 1 };
    case "i64":
      return { min: -Number.MAX_SAFE_INTEGER, max: Number.MAX_SAFE_INTEGER, step: 1 };
    case "bool":
    case "choice":
      return null;
  }
}

/// Fields group by everything before their last dot ("chemistry.enabled"
/// -> "chemistry", "genome2.mutation.duplication_q16" -> "genome2.mutation");
/// a field with no dot ("cells_x") lands in "general".
function groupOf(name: string): string {
  const lastDot = name.lastIndexOf(".");
  return lastDot === -1 ? "general" : name.slice(0, lastDot);
}

function computeGroups(fields: SchemaField[]): Map<string, SchemaField[]> {
  const byGroup = new Map<string, SchemaField[]>();
  for (const f of fields) {
    const g = groupOf(f.name);
    const list = byGroup.get(g);
    if (list) list.push(f);
    else byGroup.set(g, [f]);
  }
  for (const list of byGroup.values()) list.sort((a, b) => a.name.localeCompare(b.name));
  const orderedKeys = Array.from(byGroup.keys()).sort((a, b) => {
    if (a === b) return 0;
    if (a === "general") return -1;
    if (b === "general") return 1;
    return a.localeCompare(b);
  });
  const ordered = new Map<string, SchemaField[]>();
  for (const key of orderedKeys) ordered.set(key, byGroup.get(key)!);
  return ordered;
}

/// The server takes seeds as "0x" plus 16 hex digits (a u64); see
/// crates/sim-server's seed parsing. Randomise always produces one in that
/// shape so it round-trips through /api/schema/preview and /api/worlds
/// without a format error.
function randomSeedHex(): string {
  const bytes = new Uint8Array(8);
  crypto.getRandomValues(bytes);
  return "0x" + Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}

interface FieldRowRefs {
  row: HTMLElement;
  control: HTMLInputElement | HTMLSelectElement;
  kind: FieldType;
  errorEl: HTMLElement;
  badge: HTMLElement;
  revertBtn: HTMLButtonElement;
}

interface GroupSection {
  section: HTMLElement;
  body: HTMLElement;
  toggle: HTMLButtonElement;
  countBadge: HTMLElement;
  rendered: boolean;
}

export function builderScreen(ctx: AppContext): Screen {
  let root: HTMLElement | null = null;
  let alive = false;

  // -- schema-derived state, set once /api/schema resolves ---------------
  let schema: Schema | null = null;
  let fieldByName = new Map<string, SchemaField>();
  let groups = new Map<string, SchemaField[]>();
  const readOnly = ctx.session.role !== "admin";

  // -- editable state ------------------------------------------------------
  let worldName = "";
  let preset = "";
  let appliedRecipeId: string | null = null;
  let seed = "";
  const edits = new Map<string, string>();
  const fieldErrors = new Map<string, string>();
  let generalErrors: string[] = [];
  let preview: { config_hash: string; valid: boolean } | null = null;
  let previewPending = false;
  let previewSeq = 0;
  let debounceTimer: number | undefined;

  // -- DOM refs built once at mount ----------------------------------------
  const groupSections = new Map<string, GroupSection>();
  const fieldRows = new Map<string, FieldRowRefs>();
  const presetRadios = new Map<string, HTMLInputElement>();
  const expandedGroups = new Set<string>();
  let searchPriorExpanded: Set<string> | null = null;

  let recipeSelect: HTMLSelectElement | null = null;
  let createButton: HTMLButtonElement | null = null;
  let statusPanel: HTMLElement | null = null;
  let summaryList: HTMLElement | null = null;
  let summaryCount: HTMLElement | null = null;
  let seedErrorEl: HTMLElement | null = null;

  /// `seed` is a request-level field, not a schema field, so it can never
  /// be matched by applyPreviewErrors' name search — the server reports a
  /// bad seed as a 400 with a message naming "seed" instead of a settings
  /// validation entry. Route that case beside the Seed input by hand.
  function applySeedError(message: string | null): void {
    if (!seedErrorEl) return;
    if (message) {
      seedErrorEl.textContent = message;
      seedErrorEl.hidden = false;
    } else {
      seedErrorEl.hidden = true;
    }
  }

  function currentDefault(name: string): string {
    return fieldByName.get(name)?.defaults[preset] ?? "";
  }

  function currentValue(name: string): string {
    return edits.has(name) ? edits.get(name)! : currentDefault(name);
  }

  function pruneEditsAgainstDefaults(): void {
    for (const [name, value] of Array.from(edits.entries())) {
      if (value === currentDefault(name)) edits.delete(name);
    }
  }

  function setControlValue(refs: FieldRowRefs, value: string): void {
    if (refs.kind === "bool") {
      (refs.control as HTMLInputElement).checked = value === "true";
    } else {
      refs.control.value = value;
    }
  }

  function updateGroupBadge(group: string): void {
    const section = groupSections.get(group);
    const fields = groups.get(group);
    if (!section || !fields) return;
    const changed = fields.filter((f) => edits.has(f.name)).length;
    section.countBadge.textContent =
      changed > 0
        ? `${fields.length} field${fields.length === 1 ? "" : "s"}, ${changed} changed`
        : `${fields.length} field${fields.length === 1 ? "" : "s"}`;
  }

  function refreshFieldDisplay(name: string): void {
    const refs = fieldRows.get(name);
    if (!refs) return; // group not yet expanded; picks up current state on ensureGroupRendered
    setControlValue(refs, currentValue(name));
    const isEdited = edits.has(name);
    refs.badge.hidden = !isEdited;
    refs.revertBtn.hidden = !isEdited || readOnly;
    const message = fieldErrors.get(name);
    if (message) {
      refs.errorEl.textContent = message;
      refs.errorEl.hidden = false;
      refs.row.classList.add("field-error");
    } else {
      refs.errorEl.hidden = true;
      refs.row.classList.remove("field-error");
    }
  }

  function refreshAllRenderedFields(): void {
    for (const name of fieldRows.keys()) refreshFieldDisplay(name);
    for (const group of groups.keys()) updateGroupBadge(group);
  }

  function renderSummary(): void {
    if (!summaryList || !summaryCount) return;
    summaryList.innerHTML = "";
    summaryCount.textContent = String(edits.size);
    if (edits.size === 0) {
      summaryList.append(el("p", { class: "summary-empty" }, ["No changes from the preset defaults yet."]));
      return;
    }
    for (const name of Array.from(edits.keys()).sort()) {
      const from = currentDefault(name);
      const to = edits.get(name)!;
      const revert = button("Revert", () => revertField(name), { disabled: readOnly });
      summaryList.append(
        el("div", { class: "summary-row" }, [
          el("code", { class: "summary-name" }, [name]),
          el("span", { class: "summary-arrow" }, [`${from} → ${to}`]),
          revert,
        ]),
      );
    }
  }

  function renderStatus(): void {
    if (!statusPanel) return;
    statusPanel.innerHTML = "";
    if (previewPending) {
      statusPanel.append(el("p", { class: "status-line" }, [el("span", { class: "status-badge status-pending" }, ["Checking…"])]));
      return;
    }
    if (preview) {
      const badgeClass = preview.valid ? "status-ok" : "status-bad";
      const badgeText = preview.valid ? "Valid" : "Invalid";
      statusPanel.append(
        el("p", { class: "status-line" }, [
          el("span", { class: `status-badge ${badgeClass}` }, [badgeText]),
          " Config hash: ",
          el("code", { class: "config-hash" }, [preview.config_hash]),
        ]),
      );
      if (generalErrors.length > 0) {
        const list = el("ul", { class: "general-errors" });
        for (const message of generalErrors) list.append(el("li", {}, [message]));
        statusPanel.append(list);
      }
      return;
    }
    if (generalErrors.length > 0) {
      statusPanel.append(
        el("p", { class: "status-line" }, [
          el("span", { class: "status-badge status-bad" }, ["Error"]),
          ` ${generalErrors[0]}`,
        ]),
      );
      return;
    }
    statusPanel.append(el("p", { class: "status-line" }, [el("span", { class: "status-badge status-pending" }, ["No preview yet"])]));
  }

  function updateCreateEnablement(): void {
    if (!createButton) return;
    const canCreate =
      !readOnly && schema !== null && worldName.trim().length > 0 && preview !== null && preview.valid && !previewPending;
    createButton.disabled = !canCreate;
  }

  function applyPreviewErrors(errors: string[]): void {
    const touched = new Set(fieldErrors.keys());
    fieldErrors.clear();
    const nextGeneral: string[] = [];
    for (const message of errors) {
      let bestMatch: string | null = null;
      for (const name of fieldByName.keys()) {
        if (message.includes(name) && (bestMatch === null || name.length > bestMatch.length)) {
          bestMatch = name;
        }
      }
      if (bestMatch) fieldErrors.set(bestMatch, message);
      else nextGeneral.push(message);
    }
    generalErrors = nextGeneral;
    for (const name of fieldErrors.keys()) touched.add(name);
    for (const name of touched) refreshFieldDisplay(name);
  }

  function schedulePreview(): void {
    if (debounceTimer !== undefined) window.clearTimeout(debounceTimer);
    debounceTimer = window.setTimeout(() => void runPreview(), PREVIEW_DEBOUNCE_MS);
  }

  async function runPreview(): Promise<void> {
    if (!schema) return;
    const seq = ++previewSeq;
    previewPending = true;
    renderStatus();
    const settings: Record<string, string> = Object.fromEntries(edits);
    const body: { preset: string; seed?: string; settings: Record<string, string> } = { preset, settings };
    if (seed.trim().length > 0) body.seed = seed.trim();
    const result = await ctx.api.schemaPreview(body);
    if (!alive || seq !== previewSeq) return;
    previewPending = false;
    if (result.ok) {
      preview = { config_hash: result.value.config_hash, valid: result.value.valid };
      applyPreviewErrors(result.value.errors);
      applySeedError(null);
      ctx.announce(
        result.value.valid
          ? `Preview valid. Config hash ${result.value.config_hash}.`
          : `Preview invalid: ${result.value.errors.length} error${result.value.errors.length === 1 ? "" : "s"}.`,
      );
    } else {
      preview = null;
      fieldErrors.clear();
      applySeedError(result.message.toLowerCase().includes("seed") ? result.message : null);
      generalErrors = [result.message];
      refreshAllRenderedFields();
      ctx.announce(`Preview failed: ${result.message}`);
    }
    renderStatus();
    updateCreateEnablement();
  }

  function onFieldInput(name: string, raw: string): void {
    if (raw === currentDefault(name)) edits.delete(name);
    else edits.set(name, raw);
    updateGroupBadge(groupOf(name));
    refreshFieldDisplay(name);
    renderSummary();
    updateCreateEnablement();
    schedulePreview();
  }

  function revertField(name: string): void {
    if (!edits.has(name)) return;
    edits.delete(name);
    updateGroupBadge(groupOf(name));
    refreshFieldDisplay(name);
    renderSummary();
    updateCreateEnablement();
    schedulePreview();
    ctx.announce(`Reverted ${name} to its preset default.`);
  }

  function buildControl(f: SchemaField): HTMLInputElement | HTMLSelectElement {
    if (f.type === "bool") {
      const input = el("input", { type: "checkbox", class: "toggle-checkbox", disabled: readOnly }) as HTMLInputElement;
      input.addEventListener("change", () => onFieldInput(f.name, input.checked ? "true" : "false"));
      return input;
    }
    if (f.type === "choice") {
      const select = el("select", { disabled: readOnly }) as HTMLSelectElement;
      for (const choice of f.choices ?? []) select.append(el("option", { value: choice }, [choice]));
      select.addEventListener("change", () => onFieldInput(f.name, select.value));
      return select;
    }
    const range = typeRange(f.type)!;
    const input = el("input", {
      type: "number",
      min: range.min,
      max: range.max,
      step: range.step,
      disabled: readOnly,
    }) as HTMLInputElement;
    input.addEventListener("change", () => onFieldInput(f.name, input.value));
    return input;
  }

  function buildFieldRow(f: SchemaField): FieldRowRefs {
    const control = buildControl(f);
    control.id = `builder-field-${f.name.replace(/[^a-zA-Z0-9_-]/g, "_")}`;
    const label = el("label", { for: control.id, class: "field-label" }, [f.name]);
    const badge = el("span", { class: "edited-badge" }, ["edited"]);
    badge.hidden = true;
    const revertBtn = button("Revert", () => revertField(f.name), { title: `Revert ${f.name} to its preset default` });
    revertBtn.classList.add("btn-revert");
    revertBtn.hidden = true;
    const controlRow = el("div", { class: "field-control-row" }, [control, badge, revertBtn]);
    const errorEl = el("p", { class: "field-error-message", role: "alert" });
    errorEl.hidden = true;
    const row = el("div", { class: "field field-row", "data-field-name": f.name }, [label, controlRow, errorEl]);
    return { row, control, kind: f.type, errorEl, badge, revertBtn };
  }

  function ensureGroupRendered(group: string): void {
    const section = groupSections.get(group);
    const fields = groups.get(group);
    if (!section || !fields || section.rendered) return;
    for (const f of fields) {
      const refs = buildFieldRow(f);
      fieldRows.set(f.name, refs);
      section.body.append(refs.row);
      refreshFieldDisplay(f.name);
    }
    section.rendered = true;
  }

  function toggleGroup(group: string): void {
    const section = groupSections.get(group);
    if (!section) return;
    if (expandedGroups.has(group)) {
      expandedGroups.delete(group);
      section.body.hidden = true;
      section.toggle.setAttribute("aria-expanded", "false");
    } else {
      expandedGroups.add(group);
      ensureGroupRendered(group);
      section.body.hidden = false;
      section.toggle.setAttribute("aria-expanded", "true");
    }
  }

  function buildGroupSection(group: string, fields: SchemaField[]): HTMLElement {
    const body = el("div", { class: "group-body" });
    body.hidden = true;
    const countBadge = el("span", { class: "group-count" }, [`${fields.length} field${fields.length === 1 ? "" : "s"}`]);
    const toggle = button(group, () => toggleGroup(group));
    toggle.classList.add("group-toggle");
    toggle.setAttribute("aria-expanded", "false");
    toggle.append(" ", countBadge);
    const header = el("h3", { class: "group-heading" }, [toggle]);
    const section = el("section", { class: "builder-group" }, [header, body]);
    groupSections.set(group, { section, body, toggle, countBadge, rendered: false });
    return section;
  }

  function applySearch(query: string): void {
    const q = query.trim().toLowerCase();
    if (q === "") {
      if (searchPriorExpanded) {
        for (const [group, section] of groupSections) {
          section.section.hidden = false;
          if (section.rendered) {
            for (const f of groups.get(group) ?? []) {
              const refs = fieldRows.get(f.name);
              if (refs) refs.row.hidden = false;
            }
          }
          const shouldBeOpen = searchPriorExpanded.has(group);
          if (shouldBeOpen) ensureGroupRendered(group);
          section.body.hidden = !shouldBeOpen;
          section.toggle.setAttribute("aria-expanded", String(shouldBeOpen));
        }
        expandedGroups.clear();
        for (const group of searchPriorExpanded) expandedGroups.add(group);
        searchPriorExpanded = null;
      }
      return;
    }
    if (searchPriorExpanded === null) searchPriorExpanded = new Set(expandedGroups);
    for (const [group, section] of groupSections) {
      const fields = groups.get(group) ?? [];
      const matches = fields.filter((f) => f.name.toLowerCase().includes(q));
      if (matches.length === 0) {
        section.section.hidden = true;
        continue;
      }
      section.section.hidden = false;
      ensureGroupRendered(group);
      section.body.hidden = false;
      section.toggle.setAttribute("aria-expanded", "true");
      const matchNames = new Set(matches.map((f) => f.name));
      for (const f of fields) {
        const refs = fieldRows.get(f.name);
        if (refs) refs.row.hidden = !matchNames.has(f.name);
      }
    }
  }

  function syncPresetRadios(): void {
    for (const [name, radio] of presetRadios) radio.checked = name === preset;
  }

  function onPresetChange(newPreset: string): void {
    preset = newPreset;
    appliedRecipeId = null;
    if (recipeSelect) recipeSelect.value = "";
    pruneEditsAgainstDefaults();
    refreshAllRenderedFields();
    renderSummary();
    updateCreateEnablement();
    schedulePreview();
  }

  function applyRecipe(recipe: Recipe): void {
    preset = recipe.preset;
    appliedRecipeId = recipe.id;
    edits.clear();
    for (const [name, value] of Object.entries(recipe.settings)) {
      if (value !== currentDefault(name)) edits.set(name, value);
    }
    syncPresetRadios();
    refreshAllRenderedFields();
    renderSummary();
    updateCreateEnablement();
    schedulePreview();
    ctx.announce(`Applied recipe: ${recipe.name}.`);
  }

  async function onRecipeChange(select: HTMLSelectElement): Promise<void> {
    const id = select.value;
    if (id === "") {
      appliedRecipeId = null;
      return;
    }
    const recipe = RECIPES.find((r) => r.id === id);
    if (!recipe) return;
    const body =
      edits.size > 0
        ? `Applying "${recipe.name}" replaces your ${edits.size} edited field${edits.size === 1 ? "" : "s"} with this recipe's settings.`
        : `Apply "${recipe.name}"? This sets the preset to ${recipe.preset} and applies its settings.`;
    const ok = await confirm({ title: "Apply recipe?", body, confirmLabel: "Apply recipe" });
    if (!alive) return;
    if (!ok) {
      select.value = appliedRecipeId ?? "";
      return;
    }
    applyRecipe(recipe);
  }

  async function onCreate(): Promise<void> {
    if (!schema || !createButton) return;
    createButton.disabled = true;
    const settings: Record<string, string> = Object.fromEntries(edits);
    const body: CreateWorldRequest = { name: worldName.trim(), preset, settings };
    if (seed.trim().length > 0) body.seed = seed.trim();
    try {
      const result = await ctx.api.createWorld(body);
      if (!alive) return;
      if (result.ok) {
        ctx.session.lastWorldId = result.value.world_id;
        ctx.announce(`World "${result.value.name}" created.`);
        await ctx.stack.push(liveScreen(result.value.world_id, ctx));
        return;
      }
      applySeedError(result.message.toLowerCase().includes("seed") ? result.message : null);
      generalErrors = [result.message];
      renderStatus();
      ctx.announce(`Create failed: ${result.message}`);
    } catch (error) {
      generalErrors = [error instanceof Error ? error.message : "create failed"];
      renderStatus();
    }
    updateCreateEnablement();
  }

  function buildPresetFieldset(schemaValue: Schema): HTMLElement {
    const fieldset = el("fieldset", { class: "preset-fieldset" });
    fieldset.append(el("legend", {}, ["Preset"]));
    for (const p of schemaValue.presets) {
      const id = `builder-preset-${p.name}`;
      const radio = el("input", { type: "radio", name: "builder-preset", id, disabled: readOnly }) as HTMLInputElement;
      radio.checked = p.name === preset;
      radio.addEventListener("change", () => {
        if (radio.checked) onPresetChange(p.name);
      });
      presetRadios.set(p.name, radio);
      const label = el("label", { for: id, class: "preset-label" }, [
        el("span", { class: "preset-name" }, [p.name]),
        el("span", { class: "preset-desc" }, [p.description]),
      ]);
      fieldset.append(el("div", { class: "preset-option" }, [radio, label]));
    }
    return fieldset;
  }

  function buildRecipeSelect(): HTMLSelectElement {
    const select = el("select", { disabled: readOnly }) as HTMLSelectElement;
    select.append(el("option", { value: "" }, ["— none —"]));
    for (const r of RECIPES) {
      select.append(el("option", { value: r.id, title: r.description }, [r.name]));
    }
    select.addEventListener("change", () => void onRecipeChange(select));
    recipeSelect = select;
    return select;
  }

  async function loadSchema(screenEl: HTMLElement, loadingMsg: HTMLElement): Promise<void> {
    const result = await ctx.api.schema();
    if (!alive) return;
    loadingMsg.remove();
    if (!result.ok) {
      const retry = button("Retry", () => void loadSchema(screenEl, loadingMsg));
      screenEl.append(
        el("p", { class: "field-error-message", role: "alert" }, [`Could not load the world schema: ${result.message}`]),
        retry,
      );
      return;
    }
    schema = result.value;
    fieldByName = new Map(schema.fields.map((f) => [f.name, f]));
    groups = computeGroups(schema.fields);
    preset = schema.presets[0]?.name ?? "";

    if (readOnly) {
      screenEl.append(
        el("p", { class: "readonly-banner", role: "note" }, [
          "Connected as observer. World creation needs an admin token — every control below is read-only. ",
          "Add an admin token to this profile on the Server screen to create worlds.",
        ]),
      );
    }

    // -- left column --------------------------------------------------
    const nameInput = el("input", { type: "text", maxlength: 200, disabled: readOnly }) as HTMLInputElement;
    nameInput.addEventListener("input", () => {
      worldName = nameInput.value;
      updateCreateEnablement();
    });

    const presetFieldset = buildPresetFieldset(schema);
    const recipeSelectEl = buildRecipeSelect();

    const seedInput = el("input", { type: "text", placeholder: "server default", disabled: readOnly }) as HTMLInputElement;
    seedInput.addEventListener("input", () => {
      seed = seedInput.value;
      schedulePreview();
    });
    const randomiseBtn = button(
      "Randomise",
      () => {
        seed = randomSeedHex();
        seedInput.value = seed;
        schedulePreview();
        ctx.announce(`Seed randomised to ${seed}.`);
      },
      { disabled: readOnly },
    );
    const seedField = field("Seed", seedInput);
    seedErrorEl = el("p", { class: "field-error-message", role: "alert" });
    seedErrorEl.hidden = true;
    seedField.append(seedErrorEl);
    const seedRow = el("div", { class: "seed-row" }, [seedField, randomiseBtn]);

    const leftColumn = el("div", { class: "builder-left" }, [
      field("Name", nameInput),
      presetFieldset,
      field("Recipe", recipeSelectEl),
      seedRow,
    ]);

    // -- main column ----------------------------------------------------
    const searchInput = el("input", { type: "search", placeholder: "Filter fields by name…" }) as HTMLInputElement;
    searchInput.addEventListener("input", () => applySearch(searchInput.value));
    const searchRow = field("Search fields", searchInput);

    statusPanel = el("div", { class: "status-panel", "aria-live": "polite" });
    summaryCount = el("span", { class: "summary-count" }, ["0"]);
    summaryList = el("div", { class: "summary-list" });
    const summaryPanel = el("div", { class: "summary-panel" }, [
      el("h2", {}, ["Changes from preset (", summaryCount, ")"]),
      summaryList,
    ]);

    const groupsContainer = el("div", { class: "builder-groups" });
    for (const [group, fields] of groups) {
      groupsContainer.append(buildGroupSection(group, fields));
    }

    const mainColumn = el("div", { class: "builder-main" }, [searchRow, statusPanel, summaryPanel, groupsContainer]);

    createButton = button("Create World", () => void onCreate(), { variant: "primary", disabled: true });
    const footer = el("div", { class: "builder-footer" }, [createButton]);

    screenEl.append(el("div", { class: "builder-layout" }, [leftColumn, mainColumn]), footer);

    renderSummary();
    renderStatus();
    updateCreateEnablement();
    schedulePreview();
  }

  return {
    id: "builder",
    title: "New World",

    async mount(mountRoot: HTMLElement): Promise<void> {
      root = mountRoot;
      alive = true;

      const heading = el("h1", {}, ["New World"]);
      const backBtn = button("Back", () => void ctx.stack.pop());
      const header = el("div", { class: "screen-header" }, [heading, backBtn]);
      const loadingMsg = el("p", { class: "builder-loading" }, ["Loading schema…"]);
      const screenEl = el("div", { class: "screen builder-screen" }, [header, loadingMsg]);
      root.append(screenEl);

      await loadSchema(screenEl, loadingMsg);
    },

    unmount(): void {
      alive = false;
      if (debounceTimer !== undefined) window.clearTimeout(debounceTimer);
      root = null;
    },

    onKey(event: KeyboardEvent): boolean {
      if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return false;
      const toggles = Array.from(groupSections.values()).map((g) => g.toggle);
      const index = toggles.indexOf(document.activeElement as HTMLButtonElement);
      if (index === -1) return false;
      const delta = event.key === "ArrowDown" ? 1 : -1;
      const next = (index + delta + toggles.length) % toggles.length;
      toggles[next]!.focus();
      return true;
    },
  };
}
