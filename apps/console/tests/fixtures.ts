// Shared E2E fixtures: connection settings for the real lifesim-server
// started by scripts/run-console-e2e.sh, and two Playwright `test` objects
// that seed a server profile into localStorage (via addInitScript, before
// the page's own scripts run) so every test starts on a cold load with an
// already-connected profile — the console has no query-param connect path
// like apps/observer, profiles.ts's ProfileStore is the only way in.
//
// `adminTest` seeds a profile carrying both tokens (role: admin); `plainTest`
// re-exports the base `test` unmodified for specs that want no profile at
// all (e.g. the title screen's drift-mode background). A third,
// `observerTest`, seeds a profile with the observer token only (role:
// observer) for C3's "no admin controls" case.

import { test as base, expect, type Page } from "@playwright/test";

export const REST_PORT = process.env.LIFESIM_E2E_REST_PORT ?? "8962";
export const WS_PORT = process.env.LIFESIM_E2E_WS_PORT ?? "8963";
export const OBSERVER_TOKEN = process.env.LIFESIM_E2E_OBSERVER_TOKEN ?? "e2e-observer";
export const ADMIN_TOKEN = process.env.LIFESIM_E2E_ADMIN_TOKEN ?? "e2e-admin";

export const REST_BASE = `http://127.0.0.1:${REST_PORT}`;
export const WS_BASE = `ws://127.0.0.1:${WS_PORT}`;

const PROFILES_KEY = "lifesim-console.profiles.v1";
const ACTIVE_KEY = "lifesim-console.activeProfile.v1";

interface SeededProfile {
  id: string;
  name: string;
  restBase: string;
  wsBase: string;
  observerToken: string;
  adminToken?: string;
}

const ADMIN_PROFILE: SeededProfile = {
  id: "e2e-admin-profile",
  name: "E2E Admin",
  restBase: REST_BASE,
  wsBase: WS_BASE,
  observerToken: OBSERVER_TOKEN,
  adminToken: ADMIN_TOKEN,
};

const OBSERVER_PROFILE: SeededProfile = {
  id: "e2e-observer-profile",
  name: "E2E Observer",
  restBase: REST_BASE,
  wsBase: WS_BASE,
  observerToken: OBSERVER_TOKEN,
};

function seedProfileScript(profilesKey: string, activeKey: string, profile: SeededProfile): void {
  window.localStorage.setItem(profilesKey, JSON.stringify({ profiles: [profile] }));
  window.localStorage.setItem(activeKey, profile.id);
}

async function withSeededProfile(page: Page, profile: SeededProfile): Promise<void> {
  await page.addInitScript(
    ({ profilesKey, activeKey, profile: p }) => {
      window.localStorage.setItem(profilesKey, JSON.stringify({ profiles: [p] }));
      window.localStorage.setItem(activeKey, p.id);
    },
    { profilesKey: PROFILES_KEY, activeKey: ACTIVE_KEY, profile },
  );
}

/// A profile with both tokens active on load: session.role resolves to
/// "admin" (main.ts derives role from adminToken presence, not a server
/// round trip).
export const adminTest = base.extend({
  page: async ({ page }, use) => {
    await withSeededProfile(page, ADMIN_PROFILE);
    await use(page);
  },
});

/// A profile with the observer token only: session.role resolves to
/// "observer", so every admin control the app renders conditionally must
/// be absent.
export const observerTest = base.extend({
  page: async ({ page }, use) => {
    await withSeededProfile(page, OBSERVER_PROFILE);
    await use(page);
  },
});

/// No seeded profile at all — a genuinely cold load, used by the title
/// screen's background tests (drift mode only applies when unconnected).
export const plainTest = base;

export { expect, seedProfileScript };

/// Wait for the New World builder's schema-driven form to finish loading
/// (schema fetched, groups rendered) after navigating there.
export async function waitForBuilderReady(page: Page): Promise<void> {
  await expect(page.locator(".builder-groups")).toBeVisible();
  await expect(page.locator(".builder-footer button")).toBeVisible();
}

/// Directly dispatch `input` + `change` on a builder field's control,
/// exactly as a user's edit would (the app listens for `change`), without
/// depending on Playwright's fill() event semantics for <input type=number>
/// and <select>. Requires the field's group to already be expanded (its row
/// mounts into the DOM only once, in builder.ts's ensureGroupRendered).
export async function setBuilderField(page: Page, fieldName: string, value: string): Promise<void> {
  await page.evaluate(
    ({ fieldName, value }) => {
      const row = document.querySelector(`[data-field-name="${fieldName}"]`);
      if (!row) throw new Error(`field row not mounted (expand its group first): ${fieldName}`);
      const control = row.querySelector("input, select") as
        | HTMLInputElement
        | HTMLSelectElement
        | null;
      if (!control) throw new Error(`no control found for field: ${fieldName}`);
      if (control instanceof HTMLInputElement && control.type === "checkbox") {
        control.checked = value === "true";
      } else if (control instanceof HTMLSelectElement) {
        const setter = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, "value")!.set!;
        setter.call(control, value);
      } else {
        const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
        setter.call(control, value);
      }
      control.dispatchEvent(new Event("input", { bubbles: true }));
      control.dispatchEvent(new Event("change", { bubbles: true }));
    },
    { fieldName, value },
  );
}

/// Expand a builder settings group by clicking its toggle (the group's
/// fields only mount into the DOM once expanded).
export async function expandBuilderGroup(page: Page, groupName: string): Promise<void> {
  const toggle = page.locator(".group-toggle", { hasText: groupName }).first();
  if ((await toggle.getAttribute("aria-expanded")) === "true") return;
  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
}

/// Stops (does not delete) every world but world 1 ("primary", the one the
/// server's own flags started). World-creating specs call this in an
/// afterEach so the number of concurrently-ticking worlds stays low for
/// later specs — notably C3's two-second reflection budget, which the
/// server's own tick threads can miss under enough concurrent load (the
/// plan's own risk table: "Sandbox worlds slow the production world").
/// Stopping (not deleting) keeps every created world inspectable and never
/// touches the max-worlds registry bound.
export async function stopNonPrimaryWorlds(): Promise<void> {
  const response = await fetch(`${REST_BASE}/api/worlds`, {
    headers: { Authorization: `Bearer ${OBSERVER_TOKEN}` },
  });
  if (!response.ok) return;
  const worlds = (await response.json()) as { world_id: number; status?: string; paused?: boolean }[];
  for (const world of worlds) {
    if (world.world_id === 1) continue;
    if (world.status === "stopped") continue;
    await fetch(`${REST_BASE}/api/worlds/${world.world_id}/control?action=stop`, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${ADMIN_TOKEN}`,
        "Idempotency-Key": `test-cleanup-stop-${world.world_id}`,
      },
    });
  }
}

/// The server's full field schema, fetched directly (not through the page)
/// so tests have a ground truth to check the builder's rendered fields
/// against.
export async function fetchSchema(): Promise<{
  presets: { name: string; description: string }[];
  fields: { name: string; type: string }[];
  limits: { max_worlds: number; max_cells_x: number; max_cells_y: number };
}> {
  const response = await fetch(`${REST_BASE}/api/schema`, {
    headers: { Authorization: `Bearer ${OBSERVER_TOKEN}` },
  });
  if (!response.ok) throw new Error(`schema fetch failed: ${response.status}`);
  return response.json();
}
