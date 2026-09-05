// Server connection profiles: name, REST/WS bases and tokens, kept in
// localStorage under one key. All storage access is wrapped in try/catch so
// a blocked or unavailable store (private browsing, disabled storage) still
// yields a usable, empty store rather than throwing during boot.

const STORAGE_KEY = "lifesim-console.profiles.v1";
const ACTIVE_KEY = "lifesim-console.activeProfile.v1";
const LAST_WORLD_KEY = "lifesim-console.lastWorld.v1";

export interface Profile {
  id: string;
  name: string;
  restBase: string;
  wsBase: string;
  observerToken: string;
  adminToken?: string;
}

interface StoredShape {
  profiles: Profile[];
  activeId: string | null;
}

function readRaw(): StoredShape {
  try {
    const text = window.localStorage.getItem(STORAGE_KEY);
    if (!text) return { profiles: [], activeId: readActiveId() };
    const parsed = JSON.parse(text) as unknown;
    if (!parsed || typeof parsed !== "object" || !Array.isArray((parsed as StoredShape).profiles)) {
      return { profiles: [], activeId: readActiveId() };
    }
    return { profiles: (parsed as StoredShape).profiles, activeId: readActiveId() };
  } catch {
    return { profiles: [], activeId: null };
  }
}

function readActiveId(): string | null {
  try {
    return window.localStorage.getItem(ACTIVE_KEY);
  } catch {
    return null;
  }
}

function writeRaw(profiles: Profile[]): void {
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify({ profiles }));
  } catch {
    // Storage blocked: profile changes for this session are not persisted.
  }
}

function writeActiveId(id: string | null): void {
  try {
    if (id === null) window.localStorage.removeItem(ACTIVE_KEY);
    else window.localStorage.setItem(ACTIVE_KEY, id);
  } catch {
    // Storage blocked: the active profile choice is not persisted.
  }
}

/// The last-viewed world per profile, kept beside the profile list under
/// its own key (rather than inside a Profile itself, which is round-
/// tripped through the Server screen's edit form and would otherwise need
/// to preserve a field it never displays). Keyed by profile id so switching
/// profiles never leaks one profile's last world into another's Continue.
function readLastWorldMap(): Record<string, number> {
  try {
    const text = window.localStorage.getItem(LAST_WORLD_KEY);
    if (!text) return {};
    const parsed = JSON.parse(text) as unknown;
    return parsed && typeof parsed === "object" ? (parsed as Record<string, number>) : {};
  } catch {
    return {};
  }
}

export function getLastWorldId(profileId: string): number | null {
  const value = readLastWorldMap()[profileId];
  return typeof value === "number" ? value : null;
}

export function setLastWorldId(profileId: string, worldId: number): void {
  try {
    const map = readLastWorldMap();
    map[profileId] = worldId;
    window.localStorage.setItem(LAST_WORLD_KEY, JSON.stringify(map));
  } catch {
    // Storage blocked: the last-viewed world for this profile is not
    // persisted — Continue falls back to the first listed world on the
    // next boot, same as if nothing had ever been viewed.
  }
}

export class ProfileStore {
  list(): Profile[] {
    return readRaw().profiles;
  }

  get(id: string): Profile | null {
    return this.list().find((profile) => profile.id === id) ?? null;
  }

  save(profile: Profile): void {
    const profiles = this.list();
    const index = profiles.findIndex((existing) => existing.id === profile.id);
    if (index >= 0) profiles[index] = profile;
    else profiles.push(profile);
    writeRaw(profiles);
  }

  remove(id: string): void {
    writeRaw(this.list().filter((profile) => profile.id !== id));
    if (readActiveId() === id) writeActiveId(null);
  }

  activeId(): string | null {
    return readActiveId();
  }

  setActive(id: string | null): void {
    writeActiveId(id);
  }

  active(): Profile | null {
    const id = this.activeId();
    if (id === null) return null;
    return this.get(id);
  }
}
