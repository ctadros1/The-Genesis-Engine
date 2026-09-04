// The Server screen: profiles (name, REST base, WS base, tokens) kept in
// localStorage via ProfileStore, a Test button that proves a profile's
// tokens actually work before committing to it, and Connect, which swaps
// the app's live ApiClient, updates the session, and returns to the title.
//
// Tokens never leave this screen except as an Authorization header: they
// are not logged, not put in a URL, and password inputs never echo them
// in a way a screen reader or screenshot would leak (type="password").

import { ApiClient } from "../api";
import type { Profile } from "../profiles";
import type { AppContext, Screen } from "../screens";
import { button, el, field } from "../ui/dom";
import { confirm as confirmDialog } from "../ui/dialog";
import "./server.css";

interface FormValues {
  name: string;
  restBase: string;
  wsBase: string;
  observerToken: string;
  adminToken: string;
}

type TestKind = "neutral" | "ok" | "fail";

export function serverScreen(ctx: AppContext): Screen {
  let root: HTMLElement | null = null;
  let disposed = false;

  // Null editingId means the form describes a not-yet-saved profile.
  let editingId: string | null = null;

  let profileListEl!: HTMLUListElement;
  let formHeadingEl!: HTMLHeadingElement;
  let cancelEditButton!: HTMLButtonElement;
  let formErrorEl!: HTMLParagraphElement;
  let testResultEl!: HTMLDivElement;

  let nameInput!: HTMLInputElement;
  let restInput!: HTMLInputElement;
  let wsInput!: HTMLInputElement;
  let observerTokenInput!: HTMLInputElement;
  let adminTokenInput!: HTMLInputElement;

  // --- form helpers ----------------------------------------------------

  function readForm(): FormValues {
    return {
      name: nameInput.value.trim(),
      restBase: restInput.value.trim(),
      wsBase: wsInput.value.trim(),
      observerToken: observerTokenInput.value.trim(),
      adminToken: adminTokenInput.value.trim(),
    };
  }

  function validateForm(values: FormValues): string[] {
    const errors: string[] = [];
    if (values.name.length === 0) errors.push("Name is required.");
    if (values.restBase.length === 0) errors.push("REST base is required.");
    if (values.wsBase.length === 0) errors.push("WS base is required.");
    if (values.observerToken.length === 0) errors.push("Observer token is required.");
    return errors;
  }

  function toProfile(values: FormValues, id: string): Profile {
    const profile: Profile = {
      id,
      name: values.name,
      restBase: values.restBase,
      wsBase: values.wsBase,
      observerToken: values.observerToken,
    };
    if (values.adminToken.length > 0) profile.adminToken = values.adminToken;
    return profile;
  }

  function showFormError(message: string | null): void {
    formErrorEl.textContent = message ?? "";
    formErrorEl.hidden = message === null;
  }

  function updateFormHeading(): void {
    formHeadingEl.textContent = editingId ? "Edit profile" : "Add profile";
    cancelEditButton.hidden = editingId === null;
  }

  function resetForm(): void {
    editingId = null;
    nameInput.value = "";
    restInput.value = "";
    wsInput.value = "";
    observerTokenInput.value = "";
    adminTokenInput.value = "";
    updateFormHeading();
    showFormError(null);
    testResultEl.hidden = true;
    testResultEl.replaceChildren();
  }

  function loadIntoForm(profile: Profile): void {
    editingId = profile.id;
    nameInput.value = profile.name;
    restInput.value = profile.restBase;
    wsInput.value = profile.wsBase;
    observerTokenInput.value = profile.observerToken;
    adminTokenInput.value = profile.adminToken ?? "";
    updateFormHeading();
    showFormError(null);
    testResultEl.hidden = true;
    testResultEl.replaceChildren();
    nameInput.focus();
  }

  function roleBadge(role: "observer" | "admin"): HTMLElement {
    return el("span", { class: `badge badge-role-${role}` }, [role.toUpperCase()]);
  }

  function setTestResult(text: string, kind: TestKind, role?: "observer" | "admin"): void {
    const kindClass = kind === "ok" ? "badge-ok" : kind === "fail" ? "badge-fail" : "badge-neutral";
    const kindLabel = kind === "ok" ? "OK" : kind === "fail" ? "FAILED" : "TESTING";
    const line: (Node | string)[] = [el("span", { class: `badge ${kindClass}` }, [kindLabel]), text];
    if (role) line.push(roleBadge(role));
    testResultEl.hidden = false;
    testResultEl.replaceChildren(el("div", { class: "test-result-line", role: "status" }, line));
  }

  // --- list -------------------------------------------------------------

  function refreshProfileList(): void {
    const profiles = ctx.profiles.list();
    const activeId = ctx.profiles.activeId();
    if (profiles.length === 0) {
      profileListEl.replaceChildren(
        el("li", { class: "profile-empty" }, ["No saved profiles yet — use Add profile below."]),
      );
      return;
    }
    profileListEl.replaceChildren(
      ...profiles.map((profile) => {
        const isActive = profile.id === activeId;
        const nameParts: (Node | string)[] = [profile.name];
        if (isActive) nameParts.push(el("span", { class: "badge badge-ok" }, ["ACTIVE"]));
        return el("li", { class: isActive ? "profile-row is-active" : "profile-row" }, [
          el("div", { class: "profile-info" }, [
            el("span", { class: "profile-name" }, nameParts),
            el("span", { class: "profile-detail" }, [`REST: ${profile.restBase}`]),
            el("span", { class: "profile-detail" }, [`WS: ${profile.wsBase}`]),
          ]),
          el("div", { class: "profile-actions" }, [
            button("Connect", () => void connectToProfile(profile), { title: `Connect to ${profile.name}` }),
            button("Edit", () => loadIntoForm(profile), { title: `Edit ${profile.name}` }),
            button("Remove", () => void removeProfile(profile), {
              variant: "danger",
              title: `Remove ${profile.name}`,
            }),
          ]),
        ]);
      }),
    );
  }

  async function removeProfile(profile: Profile): Promise<void> {
    const confirmed = await confirmDialog({
      title: `Remove ${profile.name}?`,
      body: `This deletes the saved profile "${profile.name}" from this browser. This cannot be undone.`,
      confirmLabel: "Remove",
      danger: true,
    });
    if (!confirmed || disposed) return;
    ctx.profiles.remove(profile.id);
    if (editingId === profile.id) resetForm();
    refreshProfileList();
    ctx.announce(`Removed profile ${profile.name}`);
  }

  // --- test ---------------------------------------------------------------

  async function runTest(): Promise<void> {
    const values = readForm();
    const errors = validateForm(values);
    if (errors.length > 0) {
      setTestResult(errors.join(" "), "fail");
      return;
    }
    setTestResult("Testing connection…", "neutral");
    const client = new ApiClient(toProfile(values, editingId ?? "draft"));
    const health = await client.health();
    if (disposed) return;
    if (!health.ok) {
      setTestResult(`Health check failed (status ${health.status}): ${health.message}`, "fail");
      ctx.announce("Connection test failed");
      return;
    }
    const worlds = await client.listWorlds();
    if (disposed) return;
    const worldsText = worlds.ok
      ? `${worlds.value.length} world${worlds.value.length === 1 ? "" : "s"} visible`
      : `worlds unavailable: ${worlds.message}`;

    let role: "observer" | "admin" = "observer";
    let adminNote = "";
    if (values.adminToken.length > 0) {
      // Mutation-free admin proof: GET /api/audit is admin-only and reads
      // nothing sensitive back — it either 200s (admin accepted) or
      // 401/403s (fall back to observer), never mutates state.
      const audit = await client.audit();
      if (disposed) return;
      if (audit.ok) {
        role = "admin";
      } else {
        adminNote = ` Admin token rejected (status ${audit.status}) — falling back to observer.`;
      }
    }
    setTestResult(`Connected. ${worldsText}.${adminNote}`, "ok", role);
    ctx.announce(`Connection test succeeded as ${role}`);
  }

  // --- connect --------------------------------------------------------------

  async function connectToProfile(profile: Profile): Promise<void> {
    ctx.profiles.setActive(profile.id);
    const client = new ApiClient(profile);
    ctx.api = client;
    ctx.session.profile = profile;
    ctx.session.role = undefined;
    ctx.session.lastWorldId = undefined;

    const health = await client.health();
    if (disposed) return;
    if (!health.ok) {
      refreshProfileList();
      ctx.announce(`Connected profile set to ${profile.name}, but the health check failed: ${health.message}`);
      void ctx.stack.pop();
      return;
    }

    let role: "observer" | "admin" = "observer";
    if (profile.adminToken && profile.adminToken.length > 0) {
      const audit = await client.audit();
      if (disposed) return;
      if (audit.ok) role = "admin";
    }
    ctx.session.role = role;

    const worlds = await client.listWorlds();
    if (disposed) return;
    if (worlds.ok) {
      const first = worlds.value[0];
      if (first) ctx.session.lastWorldId = first.world_id;
    }

    refreshProfileList();
    ctx.announce(`Connected to ${profile.name} as ${role}`);
    void ctx.stack.pop();
  }

  async function onFormConnect(): Promise<void> {
    const values = readForm();
    const errors = validateForm(values);
    if (errors.length > 0) {
      showFormError(errors.join(" "));
      return;
    }
    showFormError(null);
    const id = editingId ?? crypto.randomUUID();
    const profile = toProfile(values, id);
    ctx.profiles.save(profile);
    editingId = id;
    refreshProfileList();
    await connectToProfile(profile);
  }

  function onFormSave(): void {
    const values = readForm();
    const errors = validateForm(values);
    if (errors.length > 0) {
      showFormError(errors.join(" "));
      return;
    }
    showFormError(null);
    const id = editingId ?? crypto.randomUUID();
    const profile = toProfile(values, id);
    ctx.profiles.save(profile);
    editingId = id;
    updateFormHeading();
    refreshProfileList();
    ctx.announce(`Saved profile ${profile.name}`);
  }

  // --- mount / unmount ----------------------------------------------------

  function buildDom(): HTMLElement {
    profileListEl = el("ul", { class: "profile-list", "aria-label": "Saved server profiles" }) as HTMLUListElement;

    const listSection = el("section", { class: "server-section" }, [
      el("div", { class: "server-section-header" }, [
        el("h2", {}, ["Saved profiles"]),
        button("Add profile", () => resetForm()),
      ]),
      profileListEl,
    ]);

    nameInput = el("input", { type: "text", autocomplete: "off" }) as HTMLInputElement;
    restInput = el("input", {
      type: "text",
      autocomplete: "off",
      placeholder: "http://127.0.0.1:8940",
    }) as HTMLInputElement;
    wsInput = el("input", {
      type: "text",
      autocomplete: "off",
      placeholder: "ws://127.0.0.1:8941",
    }) as HTMLInputElement;
    observerTokenInput = el("input", { type: "password", autocomplete: "off" }) as HTMLInputElement;
    adminTokenInput = el("input", { type: "password", autocomplete: "off" }) as HTMLInputElement;

    formHeadingEl = el("h2", {}, ["Add profile"]) as HTMLHeadingElement;
    cancelEditButton = button("Cancel edit", () => resetForm(), { title: "Discard changes and start a new profile" });
    cancelEditButton.hidden = true;

    formErrorEl = el("p", { class: "field-error-message", role: "alert" }, []) as HTMLParagraphElement;
    formErrorEl.hidden = true;

    testResultEl = el("div", { class: "test-result", hidden: true }) as HTMLDivElement;

    const formSection = el("section", { class: "server-section" }, [
      el("div", { class: "server-form" }, [
        el("div", { class: "server-section-header" }, [formHeadingEl, cancelEditButton]),
        field("Name", nameInput),
        el("div", { class: "form-row" }, [field("REST base", restInput), field("WS base", wsInput)]),
        el("div", { class: "form-row" }, [
          field("Observer token", observerTokenInput),
          field("Admin token (optional)", adminTokenInput),
        ]),
        formErrorEl,
        el("div", { class: "form-actions" }, [
          button("Test", () => void runTest(), { title: "Check health, worlds and the admin token" }),
          button("Save", () => onFormSave()),
          button("Connect", () => void onFormConnect(), { variant: "primary" }),
        ]),
        testResultEl,
      ]),
    ]);

    return el("div", { class: "screen server-screen" }, [
      el("div", { class: "screen-header" }, [el("h1", {}, ["Server"]), button("Back", () => void ctx.stack.pop())]),
      el("p", {}, [
        "Profiles are stored in this browser only. Tokens are sent as an Authorization header and never appear in a log or a URL.",
      ]),
      listSection,
      formSection,
    ]);
  }

  return {
    id: "server",
    title: "Server",

    mount(mountRoot: HTMLElement): void {
      root = mountRoot;
      disposed = false;
      root.append(buildDom());
      refreshProfileList();
      updateFormHeading();
    },

    unmount(): void {
      disposed = true;
      root = null;
    },

    onKey(event: KeyboardEvent): boolean {
      if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return false;
      if (!root) return false;
      const active = document.activeElement;
      if (!(active instanceof HTMLElement) || !active.matches(".profile-list button")) return false;
      const buttons = Array.from(root.querySelectorAll<HTMLButtonElement>(".profile-list button"));
      if (buttons.length === 0) return false;
      const currentIndex = buttons.indexOf(active as HTMLButtonElement);
      const delta = event.key === "ArrowDown" ? 1 : -1;
      const nextIndex = (currentIndex + delta + buttons.length) % buttons.length;
      buttons[nextIndex]!.focus();
      return true;
    },
  };
}
