// C2: the New World builder is generated from the server's schema. Every
// field the schema returns must render an input; editing one must update
// the previewed config hash; an invalid value must show the server's
// message beside the field and disable Create; and the changes summary
// must equal exactly what gets sent to POST /api/worlds.

import {
  adminTest as test,
  expect,
  expandBuilderGroup,
  fetchSchema,
  setBuilderField,
  stopNonPrimaryWorlds,
  waitForBuilderReady,
} from "./fixtures";

test.describe("C2: the builder is the schema", () => {
  test.afterEach(async () => {
    // Only the "changes summary" test actually creates a world; harmless
    // no-op for the others. See c1-title-to-live.spec.ts's afterEach for why.
    await stopNonPrimaryWorlds();
  });

  test("every schema field renders an input", async ({ page }) => {
    const schema = await fetchSchema();

    await page.goto("/");
    await page.getByRole("button", { name: "New World" }).click();
    await waitForBuilderReady(page);

    // Expand every settings group so its fields mount (builder.ts only
    // renders a group's rows once expanded).
    const toggles = page.locator(".group-toggle");
    const groupCount = await toggles.count();
    expect(groupCount).toBeGreaterThan(0);
    for (let i = 0; i < groupCount; i += 1) {
      await toggles.nth(i).click();
    }

    const rendered = await page.evaluate(() =>
      Array.from(document.querySelectorAll<HTMLElement>("[data-field-name]")).map((row) => ({
        name: row.getAttribute("data-field-name"),
        hasControl: row.querySelector("input, select") !== null,
      })),
    );

    expect(rendered.length).toBe(schema.fields.length);
    const byName = new Map(rendered.map((r) => [r.name, r.hasControl]));
    for (const field of schema.fields) {
      expect(byName.has(field.name), `missing rendered row for field "${field.name}"`).toBe(true);
      expect(byName.get(field.name), `field "${field.name}"'s row has no input/select`).toBe(true);
    }
  });

  test("editing a field updates the previewed config hash", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "New World" }).click();
    await waitForBuilderReady(page);

    await expect(page.locator(".status-panel .config-hash")).toBeVisible();
    const before = await page.locator(".status-panel .config-hash").textContent();

    await expandBuilderGroup(page, "general");
    await setBuilderField(page, "growth_rate_q16_per_s", "4000");

    await expect(async () => {
      const after = await page.locator(".status-panel .config-hash").textContent();
      expect(after).not.toBe(before);
    }).toPass({ timeout: 5_000 });

    await expect(
      page.locator('[data-field-name="growth_rate_q16_per_s"] .edited-badge'),
    ).toBeVisible();
    await expect(page.locator(".status-panel .status-badge")).toHaveText("Valid");
  });

  test("an invalid value shows the server's message beside the field and disables Create", async ({
    page,
  }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "New World" }).click();
    await waitForBuilderReady(page);
    await page.locator(".builder-left input[type=text]").first().fill("Invalid World");

    // cells_x's server-side bound is reported via /api/schema's limits
    // (max_cells_x); 99999 is comfortably over it.
    await expandBuilderGroup(page, "general");
    await setBuilderField(page, "cells_x", "99999");

    const errorEl = page.locator('[data-field-name="cells_x"] .field-error-message');
    await expect(errorEl).toBeVisible();
    await expect(errorEl).toContainText("cells_x");
    await expect(page.getByRole("button", { name: "Create World" })).toBeDisabled();
  });

  test("the changes summary equals exactly what is sent to the server", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "New World" }).click();
    await waitForBuilderReady(page);
    await page.locator(".builder-left input[type=text]").first().fill("Summary World");

    await expandBuilderGroup(page, "general");
    await setBuilderField(page, "growth_rate_q16_per_s", "5000");
    await setBuilderField(page, "reproduction_enabled", "false");

    const createButton = page.getByRole("button", { name: "Create World" });
    await expect(createButton).toBeEnabled();

    const summaryRows = await page.evaluate(() =>
      Array.from(document.querySelectorAll<HTMLElement>(".summary-list .summary-row")).map((row) => {
        const name = row.querySelector(".summary-name")!.textContent!.trim();
        const arrow = row.querySelector(".summary-arrow")!.textContent!.trim();
        const to = arrow.split("→").pop()!.trim();
        return [name, to] as const;
      }),
    );
    expect(summaryRows.length).toBe(2);
    const summaryMap = Object.fromEntries(summaryRows);

    let capturedSettings: Record<string, string> | null = null;
    await page.route("**/api/worlds", async (route) => {
      if (route.request().method() === "POST") {
        const body = route.request().postDataJSON() as { settings: Record<string, string> };
        capturedSettings = body.settings;
      }
      await route.continue();
    });

    await createButton.click();
    await expect(page.locator(".live-connection-status")).toContainText("Connected", {
      timeout: 15_000,
    });

    expect(capturedSettings).not.toBeNull();
    const sent = capturedSettings as unknown as Record<string, string>;
    expect(Object.keys(sent).sort()).toEqual(Object.keys(summaryMap).sort());
    for (const [name, to] of Object.entries(summaryMap)) {
      expect(sent[name]).toBe(to);
    }
  });
});
