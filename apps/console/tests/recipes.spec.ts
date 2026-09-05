// Every shipped recipe is one campaign's base copied verbatim, so every
// recipe must preview valid against the server's schema, and applying one
// in the builder must show the valid hash with exactly its settings listed.

import { RECIPES } from "../src/recipes";
import { OBSERVER_TOKEN, REST_BASE, adminTest as test, expect, fetchSchema, waitForBuilderReady } from "./fixtures";

test.describe("Recipes: every research regime previews valid", () => {
  for (const recipe of RECIPES) {
    test(`recipe "${recipe.name}" previews valid on the server`, async () => {
      const response = await fetch(`${REST_BASE}/api/schema/preview`, {
        method: "POST",
        headers: { Authorization: `Bearer ${OBSERVER_TOKEN}`, "Content-Type": "application/json" },
        body: JSON.stringify({ preset: recipe.preset, seed: "0x11", settings: recipe.settings }),
      });
      expect(response.status).toBe(200);
      const preview = (await response.json()) as { valid: boolean; errors: string[]; config_hash: string };
      expect(preview.errors, recipe.id).toEqual([]);
      expect(preview.valid, recipe.id).toBe(true);
      expect(preview.config_hash).toMatch(/^0x[0-9a-f]{16}$/);
    });
  }

  test("applying the artifacts recipe in the builder lists exactly its settings and previews valid", async ({ page }) => {
    const recipe = RECIPES.find((r) => r.id === "artifacts-phase12");
    expect(recipe).toBeDefined();
    // The builder lists a change only where the recipe's value differs from
    // the preset's default; a campaign base repeats many defaults verbatim.
    const schema = (await fetchSchema()) as { fields: { name: string; defaults?: Record<string, string> }[] };
    const defaults = new Map(schema.fields.map((f) => [f.name, f.defaults?.[recipe!.preset]]));
    const differing = Object.entries(recipe!.settings).filter(([name, value]) => defaults.get(name) !== value).length;
    expect(differing).toBeGreaterThan(0);
    await page.goto("/");
    await page.getByRole("button", { name: "New World" }).click();
    await waitForBuilderReady(page);
    const select = page.getByLabel("Recipe");
    await select.selectOption(recipe!.id);
    await page.getByRole("button", { name: "Apply recipe" }).click();
    await expect(page.getByText(/Changes from preset \(\d+\)/)).toContainText(
      `Changes from preset (${differing})`,
      { timeout: 10_000 },
    );
    await expect(page.getByText("Valid", { exact: true })).toBeVisible({ timeout: 10_000 });
  });
});
