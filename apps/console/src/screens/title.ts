// The Title screen: the name, a live pixel background, the status strip
// (active profile, role, connection) and the six-entry main menu. Keep the
// exported factory name and signature — screens.ts and main.ts depend on
// `titleScreen(ctx)` staying a zero-argument-besides-ctx factory.

import { confirm as confirmDialog } from "../ui/dialog";
import { button, el } from "../ui/dom";
import type { AppContext, Screen } from "../screens";
import { builderScreen } from "./builder";
import { liveScreen } from "./live";
import { savesScreen } from "./saves";
import { serverScreen } from "./server";
import { TitleBackground } from "./title/background";
import "./title.css";
import { worldsScreen } from "./worlds";

const ADR_REFERENCE = "ADR-0039: Multi-World Server And The Console";

interface MenuEntry {
  label: string;
  disabled?: boolean;
  disabledReason?: string;
  activate: () => void;
}

function buildStatusStrip(ctx: AppContext): HTMLElement {
  const profile = ctx.session.profile;
  const role = ctx.session.role;
  const connected = role !== undefined;

  const items: (Node | string)[] = [
    el("span", { class: "title-status-item" }, [`Profile: ${profile ? profile.name : "none"}`]),
    el("span", { class: `badge ${connected ? "badge-ok" : "badge-neutral"}` }, [
      connected ? "CONNECTED" : "NOT CONNECTED",
    ]),
  ];
  if (role) items.push(el("span", { class: `badge badge-role-${role}` }, [role.toUpperCase()]));

  return el("div", { class: "title-status", role: "status" }, items);
}

async function showAbout(): Promise<void> {
  await confirmDialog({
    title: "About Genesis Engine",
    body: `A private laboratory for artificial life. ${ADR_REFERENCE}.`,
    confirmLabel: "Close",
  });
}

export function titleScreen(ctx: AppContext): Screen {
  let root: HTMLElement | null = null;
  let background: TitleBackground | null = null;

  return {
    id: "title",
    title: "Genesis Engine",

    mount(mountRoot: HTMLElement): void {
      root = mountRoot;

      const backgroundHost = el("div", { class: "title-background" });

      const heading = el("h1", { class: "title-heading" }, ["GENESIS ENGINE"]);
      const subtitle = el("p", { class: "title-subtitle" }, [
        "Connect to a server, then watch, build and branch worlds of artificial life.",
      ]);
      const header = el("header", { class: "title-header" }, [heading, subtitle, buildStatusStrip(ctx)]);

      const continueWorldId = ctx.session.lastWorldId;
      const entries: MenuEntry[] = [
        {
          label: "Continue",
          disabled: continueWorldId === undefined,
          disabledReason: "No world to continue yet — open Worlds or start a New World first.",
          activate: () => {
            if (continueWorldId !== undefined) void ctx.stack.push(liveScreen(continueWorldId, ctx));
          },
        },
        { label: "Worlds", activate: () => void ctx.stack.push(worldsScreen(ctx)) },
        { label: "New World", activate: () => void ctx.stack.push(builderScreen(ctx)) },
        { label: "Load Save", activate: () => void ctx.stack.push(savesScreen(ctx)) },
        { label: "Server", activate: () => void ctx.stack.push(serverScreen(ctx)) },
        { label: "About", activate: () => void showAbout() },
      ];

      const list = el("ul", { class: "menu-list" });
      for (const entry of entries) {
        const item = button(entry.label, entry.activate, {
          disabled: entry.disabled,
          title: entry.disabled ? entry.disabledReason : undefined,
        });
        list.append(el("li", {}, [item]));
      }
      const nav = el("nav", { class: "title-menu-panel", "aria-label": "Main menu" }, [list]);

      const content = el("div", { class: "title-content" }, [header, nav]);
      root.append(el("div", { class: "screen title-screen" }, [backgroundHost, content]));

      // Mount the background after it is attached to the live DOM so it can
      // read a real container size.
      background = new TitleBackground(ctx);
      background.mount(backgroundHost);
    },

    unmount(): void {
      background?.unmount();
      background = null;
      root = null;
    },

    onKey(event: KeyboardEvent): boolean {
      if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return false;
      if (!root) return false;
      const buttons = Array.from(root.querySelectorAll<HTMLButtonElement>(".menu-list button"));
      if (buttons.length === 0) return false;
      const currentIndex = buttons.indexOf(document.activeElement as HTMLButtonElement);
      const delta = event.key === "ArrowDown" ? 1 : -1;
      // Skip disabled entries (Continue, before a world exists) rather than
      // landing on one a screen reader and a click could never activate.
      let nextIndex = currentIndex;
      for (let step = 0; step < buttons.length; step += 1) {
        nextIndex = (nextIndex + delta + buttons.length) % buttons.length;
        if (!buttons[nextIndex]!.disabled) break;
      }
      buttons[nextIndex]!.focus();
      return true;
    },
  };
}
