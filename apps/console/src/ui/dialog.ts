// Modal confirmation dialog. Every admin action in the plan (pause, resume,
// stop, delete, branch, ...) routes through this so the confirmation UX is
// identical everywhere: focus trap, Escape cancels, Enter confirms, and
// focus returns to whatever triggered the dialog when it closes.

import { button, el } from "./dom";

export interface ConfirmOptions {
  title: string;
  body: string;
  confirmLabel: string;
  danger?: boolean;
}

const FOCUSABLE_SELECTOR =
  'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])';

export function confirm(options: ConfirmOptions): Promise<boolean> {
  return new Promise((resolve) => {
    const returnFocusTo = document.activeElement as HTMLElement | null;

    const backdrop = el("div", { class: "dialog-backdrop" });
    const heading = el("h2", { id: "dialog-title" }, [options.title]);
    const body = el("p", { id: "dialog-body" }, [options.body]);

    const cleanup = (result: boolean) => {
      document.removeEventListener("keydown", onKeyDown, true);
      backdrop.remove();
      returnFocusTo?.focus();
      resolve(result);
    };

    const cancelButton = button("Cancel", () => cleanup(false));
    const confirmButton = button(options.confirmLabel, () => cleanup(true), {
      variant: options.danger ? "danger" : "primary",
    });

    const dialog = el(
      "div",
      {
        class: "dialog",
        role: "alertdialog",
        "aria-modal": "true",
        "aria-labelledby": "dialog-title",
        "aria-describedby": "dialog-body",
      },
      [heading, body, el("div", { class: "dialog-actions" }, [cancelButton, confirmButton])],
    );
    backdrop.append(dialog);

    function onKeyDown(event: KeyboardEvent): void {
      if (event.key === "Escape") {
        event.preventDefault();
        cleanup(false);
        return;
      }
      if (event.key === "Enter") {
        event.preventDefault();
        cleanup(true);
        return;
      }
      if (event.key !== "Tab") return;
      const focusable = Array.from(dialog.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR));
      if (focusable.length === 0) return;
      const first = focusable[0]!;
      const last = focusable[focusable.length - 1]!;
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    }

    document.addEventListener("keydown", onKeyDown, true);
    document.body.append(backdrop);
    confirmButton.focus();
  });
}
