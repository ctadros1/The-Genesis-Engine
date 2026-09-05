// The screen stack: every screen in the console (title, server, worlds,
// builder, live, saves, and whatever else lands later) implements Screen and
// is pushed/popped/replaced on one ScreenStack. This is the contract the
// per-screen agents build against — keep it stable.

import type { ApiClient } from "./api";
import type { Profile, ProfileStore } from "./profiles";

export interface Screen {
  /// Stable identifier, e.g. "title", "worlds". Used for logging/tests only.
  id: string;
  /// Human-readable title announced in #live-status on mount.
  title: string;
  /// Build the screen's DOM under `root` (the stack clears `root` before
  /// calling mount, so a screen owns the whole element while mounted).
  mount(root: HTMLElement, ctx: AppContext): void | Promise<void>;
  /// Release timers, sockets, and listeners. Called whenever this screen
  /// stops being the top of the stack (pop or replace) — one screen is
  /// mounted at a time, so a push above it also unmounts it first.
  unmount(): void;
  /// Handle a keydown event while this screen is on top. Return true to mark
  /// it handled (the stack calls preventDefault()); return false or omit the
  /// method to let the stack's own bindings (currently none globally) or the
  /// browser default apply.
  onKey?(event: KeyboardEvent): boolean;
}

export interface Session {
  profile?: Profile;
  role?: "observer" | "admin";
  lastWorldId?: number;
}

export interface AppContext {
  stack: ScreenStack;
  api: ApiClient;
  profiles: ProfileStore;
  session: Session;
  /// Speak text through the visually-hidden #live-status region.
  announce(text: string): void;
  /// True when the viewer has requested reduced motion
  /// (prefers-reduced-motion: reduce). Screens must not animate when true.
  reducedMotion(): boolean;
  /// Record `worldId` as the last-viewed world for this session (updates
  /// `session.lastWorldId`) and, when a profile is active, persist it
  /// beside the profile store so a fresh boot's Continue targets it too.
  /// Call this at every point the console treats a world as "the one being
  /// looked at": the worlds screen's View and Branch actions, the live
  /// screen's world switcher, the builder's Create, and the saves screen's
  /// Branch.
  rememberWorld(worldId: number): void;
}

const FOCUSABLE_SELECTOR =
  'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])';

export class ScreenStack {
  private screens: Screen[] = [];
  /// The screen currently mounted in `root`, if any. Tracked separately
  /// from `top()` so mountTop() can always find and release whatever is
  /// live before mounting the next screen, regardless of which stack
  /// operation (push/pop/replace) got it there.
  private mounted: Screen | null = null;

  constructor(
    private root: HTMLElement,
    private ctx: AppContext,
  ) {}

  top(): Screen | null {
    return this.screens.length > 0 ? this.screens[this.screens.length - 1]! : null;
  }

  async push(screen: Screen): Promise<void> {
    this.screens.push(screen);
    await this.mountTop();
  }

  /// No-ops at the root: the title screen (or whatever was pushed first)
  /// stays on the stack so Escape can be bound globally without ever
  /// emptying the screen container.
  async pop(): Promise<void> {
    if (this.screens.length <= 1) return;
    this.screens.pop();
    await this.mountTop();
  }

  async replace(screen: Screen): Promise<void> {
    this.screens.pop();
    this.screens.push(screen);
    await this.mountTop();
  }

  /// Forward a keydown to the top screen. Returns true when the screen
  /// claimed it (main.ts uses this to decide whether to preventDefault).
  dispatchKey(event: KeyboardEvent): boolean {
    const top = this.top();
    if (!top?.onKey) return false;
    return top.onKey(event);
  }

  /// Unmount whatever is currently mounted, then mount the new top (if
  /// any). Centralizing the unmount here — rather than in push/pop/replace
  /// individually — is what guarantees at most one screen is ever mounted:
  /// a push above the current top now releases it exactly like a pop or
  /// replace would.
  private async mountTop(): Promise<void> {
    if (this.mounted) {
      this.mounted.unmount();
      this.mounted = null;
    }
    const screen = this.top();
    this.root.innerHTML = "";
    if (!screen) return;
    await screen.mount(this.root, this.ctx);
    this.mounted = screen;
    this.ctx.announce(screen.title);
    focusFirst(this.root);
  }
}

function focusFirst(root: HTMLElement): void {
  const candidate = root.querySelector<HTMLElement>(FOCUSABLE_SELECTOR);
  if (candidate) {
    candidate.focus();
    return;
  }
  root.setAttribute("tabindex", "-1");
  root.focus();
}
