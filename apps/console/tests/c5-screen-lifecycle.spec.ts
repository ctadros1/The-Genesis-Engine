// C5 (regression): the ScreenStack contract at screens.ts:20-22 says one
// screen is mounted at a time, so a push above it also unmounts it first.
// This suite catches the case the C1-C4 specs cannot: they each do a single
// page.goto per test, so a stack bug that only shows up across repeated
// navigation within one page load is invisible to them.
//
// Two probes, matching the two ways the bug showed up in practice:
//  - a screen with a poll timer (Worlds) must stop polling once something
//    is pushed over it, not just once it is popped;
//  - a screen with a live resource (Title's background WebSocket) must not
//    accumulate a second live copy of itself across repeated push/pop.

import type { Page, WebSocket as PwWebSocket } from "@playwright/test";
import { WS_BASE, adminTest, expect, stopNonPrimaryWorlds } from "./fixtures";

const WORLDS_POLL_MS = 2000;

/// Every request the page has issued to `/api/worlds` (exact path, no
/// query) since this was called, as an array that grows live — callers
/// snapshot `.length` at a point in time rather than re-attaching listeners.
function trackWorldsListRequests(page: Page): { count: () => number } {
  let count = 0;
  page.on("request", (request) => {
    const url = new URL(request.url());
    if (url.pathname === "/api/worlds") count += 1;
  });
  return { count: () => count };
}

/// Every app WebSocket (connections to WS_BASE — the real lifesim-server)
/// currently open on the page, live-updated as sockets open and close.
/// Excludes Vite's own dev-server HMR socket, which stays open for the
/// whole test on a different origin and would otherwise inflate every
/// count by one regardless of what the app itself is doing.
function trackOpenSockets(page: Page): { size: () => number } {
  const open = new Set<PwWebSocket>();
  page.on("websocket", (ws) => {
    if (!ws.url().startsWith(WS_BASE)) return;
    open.add(ws);
    ws.on("close", () => open.delete(ws));
  });
  return { size: () => open.size };
}

adminTest.describe("C5: screen lifecycle — unmount on navigate-away", () => {
  adminTest.afterEach(async () => {
    await stopNonPrimaryWorlds();
  });

  adminTest(
    "pushing Saves over Worlds stops the Worlds poll timer",
    async ({ page }) => {
      await page.goto("/");
      await page.getByRole("button", { name: "Worlds" }).click();

      const card = page.locator('[data-world-id="1"]');
      await expect(card).toBeVisible();
      // Let the first poll tick land so the timer is definitely running
      // before we measure the "after leaving" window.
      await page.waitForTimeout(WORLDS_POLL_MS + 500);

      await card.getByRole("button", { name: "Branch" }).click();
      await expect(page.locator(".screen-header h1")).toContainText("Saves");

      // Measure a fresh window entirely after navigating away. With the
      // stack bug, Worlds's setInterval is still alive underneath Saves and
      // fires roughly every WORLDS_POLL_MS; three-plus polls fit in this
      // window, so any request at all here is a leak.
      const worldsRequests = trackWorldsListRequests(page);
      await page.waitForTimeout(WORLDS_POLL_MS * 3 + 500);
      expect(worldsRequests.count()).toBe(0);
    },
  );

  adminTest(
    "three round trips from Title to Server never accumulate open sockets",
    async ({ page }) => {
      const sockets = trackOpenSockets(page);

      await page.goto("/");
      await expect(page.locator(".title-menu-panel")).toBeVisible();
      // world 1 exists (server-started), so the title screen's background
      // opens a live WebSocket on mount — session.lastWorldId is set by
      // main.ts's boot() before the title screen ever mounts.
      await expect.poll(() => sockets.size()).toBe(1);

      for (let trip = 0; trip < 3; trip += 1) {
        await page.getByRole("button", { name: "Server" }).click();
        await expect(page.locator(".screen-header h1")).toContainText("Server");
        // The title screen (and its socket) must be gone now, not merely
        // hidden underneath Server.
        await expect.poll(() => sockets.size()).toBe(0);

        await page.getByRole("button", { name: "Back" }).click();
        await expect(page.locator(".title-menu-panel")).toBeVisible();
        // Back on Title: exactly one live socket again, never a second one
        // stacked on top of a first that was never released.
        await expect.poll(() => sockets.size()).toBe(1);
      }
    },
  );
});

// Worlds screen: the population sparkline is fed from a per-card metrics
// WebSocket (planning/console-and-multi-world-server.md screen 3: "a
// sparkline from the metrics stream"), not from the 2s REST poll. These
// reuse trackOpenSockets from above rather than installing a second
// page-level WebSocket wrapper.
adminTest.describe("C5: Worlds screen — per-card metrics sockets", () => {
  adminTest.afterEach(async () => {
    await stopNonPrimaryWorlds();
  });

  adminTest("leaving the Worlds screen closes its metrics sockets", async ({ page }) => {
    const sockets = trackOpenSockets(page);

    await page.goto("/");
    await expect(page.locator(".title-menu-panel")).toBeVisible();
    // Title's own background preview socket for the last-viewed world.
    await expect.poll(() => sockets.size()).toBe(1);

    await page.getByRole("button", { name: "Worlds" }).click();
    const cards = page.locator(".card");
    await expect(cards.first()).toBeVisible();
    const cardCount = await cards.count();
    // A stopped world's socket is opened and then immediately closed by
    // the server itself (a 410, per crates/sim-server/src/stream.rs) — it
    // never has metrics to stream, so only running/paused cards keep a
    // live socket. That is still "never more sockets than cards": the
    // sparkline just falls back to the poll for a stopped world's card.
    const stoppedCount = await page.locator(".card .status-badge--stopped").count();
    const expectedOpenSockets = cardCount - stoppedCount;
    await expect.poll(() => sockets.size()).toBe(expectedOpenSockets);

    await page.getByRole("button", { name: "Back" }).click();
    await expect(page.locator(".title-menu-panel")).toBeVisible();
    // Every one of the Worlds screen's sockets is gone; Title's own
    // background socket is the only one left.
    await expect.poll(() => sockets.size()).toBe(1);
  });

  adminTest(
    "a card's sparkline updates from the metrics stream, faster than the 2s poll",
    async ({ page }) => {
      await page.goto("/");
      await page.getByRole("button", { name: "Worlds" }).click();
      const card = page.locator('[data-world-id="1"]');
      await expect(card).toBeVisible();
      const text = card.locator(".sparkline-wrap .visually-hidden");
      await expect(text).not.toHaveText("Population trend: not enough samples yet.", {
        timeout: 5_000,
      });

      // A pure 2s REST poll cannot have produced a second data point yet
      // this soon after the card first rendered — this window only closes
      // if the metrics socket (broadcasting at 1 sample/s once subscribed)
      // is what is actually feeding the sparkline.
      const before = await text.textContent();
      await expect(async () => {
        const after = await text.textContent();
        expect(after).not.toBe(before);
      }).toPass({ timeout: 1_800, intervals: [100] });
    },
  );
});
