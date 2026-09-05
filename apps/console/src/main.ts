// Boot: load profiles, attempt to auto-connect to the last profile, push
// the Title screen, then install global keyboard handling (forwarded to
// the top of the stack) and the reduced-motion query helper.

import "./style.css";
import { ApiClient } from "./api";
import { ProfileStore, getLastWorldId, setLastWorldId, type Profile } from "./profiles";
import type { AppContext, Session } from "./screens";
import { ScreenStack } from "./screens";
import { titleScreen } from "./screens/title";

const EMPTY_PROFILE: Profile = {
  id: "__none__",
  name: "(no profile)",
  restBase: "",
  wsBase: "",
  observerToken: "",
};

function mustGet(id: string): HTMLElement {
  const element = document.getElementById(id);
  if (!element) throw new Error(`missing #${id}`);
  return element;
}

const screenRoot = mustGet("screen");
const liveStatus = mustGet("live-status");

function announce(text: string): void {
  // Clearing first forces assistive tech to re-announce identical text
  // (e.g. returning to a screen with the same title).
  liveStatus.textContent = "";
  window.setTimeout(() => {
    liveStatus.textContent = text;
  }, 30);
}

function reducedMotion(): boolean {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

const profiles = new ProfileStore();
const session: Session = {};

const activeProfile = profiles.active();
if (activeProfile) session.profile = activeProfile;

const api = new ApiClient(activeProfile ?? EMPTY_PROFILE);

/// Sets session.lastWorldId and, when a profile is connected, persists it
/// beside the profile store — reads ctx.session.profile at call time
/// (not the `activeProfile` captured at boot) so it stays correct across
/// the Server screen's connectToProfile swapping the active profile
/// mid-session.
function rememberWorld(worldId: number): void {
  session.lastWorldId = worldId;
  const profileId = session.profile?.id;
  if (profileId) setLastWorldId(profileId, worldId);
}

const ctx: AppContext = {
  // `stack` is assigned right after construction below — AppContext and
  // ScreenStack are mutually referential (the stack needs ctx to mount
  // screens, ctx needs the stack for screens to navigate).
  stack: undefined as unknown as ScreenStack,
  api,
  profiles,
  session,
  announce,
  reducedMotion,
  rememberWorld,
};
const stack = new ScreenStack(screenRoot, ctx);
ctx.stack = stack;

document.addEventListener("keydown", (event) => {
  if (event.key === "Escape") {
    event.preventDefault();
    void stack.pop();
    return;
  }
  if (stack.dispatchKey(event)) {
    event.preventDefault();
  }
});

async function boot(): Promise<void> {
  if (activeProfile) {
    const health = await api.health();
    if (health.ok) {
      const worlds = await api.listWorlds();
      session.role = activeProfile.adminToken ? "admin" : "observer";
      if (worlds.ok && worlds.value.length > 0) {
        // Continue targets the last-viewed world, persisted per profile,
        // as long as it is still listed by the server; otherwise the
        // first listed world (never persisted here — only an explicit
        // view/create/branch counts as "viewing" a world for next time).
        const persisted = getLastWorldId(activeProfile.id);
        const stillListed = persisted !== null && worlds.value.some((world) => world.world_id === persisted);
        session.lastWorldId = stillListed ? persisted! : worlds.value[0]!.world_id;
      }
    }
  }
  await stack.push(titleScreen(ctx));
}

void boot();
