// Arrow-key navigation on the Live and Saves screens, per the plan: every
// screen navigates by arrows/enter/escape. title.ts and worlds.ts already
// cover Up/Down menu and card navigation; this file covers live.ts's
// Left/Right top-bar-and-HUD navigation and saves.ts's Up/Down row
// navigation (plus Enter activating a focused row button), added onKey
// handlers following the same pattern as title.ts/worlds.ts.

import { adminTest as test, expect } from "./fixtures";

const WORLD_ID = "1";

/// The visible label of the currently focused control: an aria-label when
/// one is set (the world switcher), else the text of a <label for=id>
/// pointing at it (dom.ts's field() helper — the Speed select), else the
/// element's own text (every button).
async function activeControlLabel(page: import("@playwright/test").Page): Promise<string> {
  return page.evaluate(() => {
    const el = document.activeElement as HTMLElement | null;
    if (!el) return "none";
    const aria = el.getAttribute("aria-label");
    if (aria) return aria;
    if (el.id) {
      const label = document.querySelector(`label[for="${el.id}"]`);
      if (label?.textContent) return label.textContent.trim();
    }
    return (el.textContent ?? "").trim();
  });
}

test.describe("Arrow-key navigation: Live screen", () => {
  test("Left/Right move focus across the top bar and HUD controls, in DOM order", async ({
    page,
  }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "Worlds" }).click();
    const card = page.locator(`[data-world-id="${WORLD_ID}"]`);
    await expect(card).toBeVisible();
    await card.getByRole("button", { name: "View" }).click();
    await expect(page.locator(".live-connection-status")).toContainText("Connected", {
      timeout: 15_000,
    });

    // focusFirst() lands on the first focusable control in DOM order, which
    // is Back (the top bar's first child).
    await expect(page.getByRole("button", { name: "Back" })).toBeFocused();

    const forwardOrder = ["Back", "Switch world", "Saves", "Pause", "Resume", "Speed", "Scientific overlay"];
    expect(await activeControlLabel(page)).toBe(forwardOrder[0]);
    for (let i = 1; i < forwardOrder.length; i += 1) {
      await page.keyboard.press("ArrowRight");
      expect(await activeControlLabel(page)).toBe(forwardOrder[i]);
    }
    // Wraps back around to the first control.
    await page.keyboard.press("ArrowRight");
    expect(await activeControlLabel(page)).toBe(forwardOrder[0]);

    // ArrowLeft reverses, wrapping the other way.
    await page.keyboard.press("ArrowLeft");
    expect(await activeControlLabel(page)).toBe(forwardOrder[forwardOrder.length - 1]);
    await page.keyboard.press("ArrowLeft");
    expect(await activeControlLabel(page)).toBe(forwardOrder[forwardOrder.length - 2]);
  });
});

test.describe("Arrow-key navigation: Saves screen", () => {
  test("Up/Down move focus between save rows' first buttons, Enter activates", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "Worlds" }).click();
    const card = page.locator(`[data-world-id="${WORLD_ID}"]`);
    await expect(card).toBeVisible();
    // Start listening before the click that triggers savesScreen's mount
    // (and its initial GET) — reading a row-count baseline off the DOM
    // before that fetch lands would see the pre-refresh() empty state.
    const initialSavesLoad = page.waitForResponse(
      (response) => response.url().includes(`/worlds/${WORLD_ID}/saves`) && response.request().method() === "GET",
    );
    await card.getByRole("button", { name: "Branch" }).click();
    await expect(page.locator(".screen-header h1")).toContainText("Saves");
    await initialSavesLoad;

    // Ensure at least two rows exist so Up/Down has somewhere to move.
    // "Save now" is a no-op while a save is already in flight (savingNow in
    // saves.ts), so each iteration waits for the row count to actually
    // grow before starting the next — a second click before then would
    // silently do nothing.
    const rows = page.locator(".saves-table tbody tr");
    let expectedRowCount = await rows.count();
    for (let i = 0; i < 2; i += 1) {
      await page.getByRole("button", { name: "Save now" }).click();
      const dialog = page.getByRole("alertdialog");
      await dialog.locator("input[type=text]").fill(`arrow-nav-${i}`);
      await dialog.getByRole("button", { name: "Save now", exact: true }).click();
      await expect(dialog).toBeHidden();
      expectedRowCount += 1;
      await expect(rows).toHaveCount(expectedRowCount, { timeout: 10_000 });
    }

    const firstRowButton = rows.nth(0).getByRole("button", { name: "Verify" });
    const secondRowButton = rows.nth(1).getByRole("button", { name: "Verify" });

    await firstRowButton.focus();
    await expect(firstRowButton).toBeFocused();

    await page.keyboard.press("ArrowDown");
    await expect(secondRowButton).toBeFocused();

    await page.keyboard.press("ArrowUp");
    await expect(firstRowButton).toBeFocused();

    // Enter activates the focused row's first button natively (it's a real
    // <button>) — this opens Verify's confirmation dialog.
    await page.keyboard.press("Enter");
    const verifyDialog = page.getByRole("alertdialog");
    await expect(verifyDialog).toBeVisible();
    await verifyDialog.getByRole("button", { name: "Cancel" }).click();
    await expect(verifyDialog).toBeHidden();
  });
});
