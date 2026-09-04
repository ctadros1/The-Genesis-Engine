// Small DOM-building helpers shared by every screen. No framework: screens
// build real elements and wire real listeners.

type Attrs = Record<string, string | number | boolean | undefined>;

export function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  attrs: Attrs = {},
  children: (Node | string)[] = [],
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  for (const [key, value] of Object.entries(attrs)) {
    if (value === undefined || value === false) continue;
    if (value === true) {
      node.setAttribute(key, "");
      continue;
    }
    if (key === "class") node.className = String(value);
    else node.setAttribute(key, String(value));
  }
  for (const child of children) {
    node.append(typeof child === "string" ? document.createTextNode(child) : child);
  }
  return node;
}

export interface ButtonOptions {
  variant?: "primary" | "danger";
  disabled?: boolean;
  title?: string;
}

export function button(label: string, onClick: () => void, opts: ButtonOptions = {}): HTMLButtonElement {
  const node = el("button", {
    type: "button",
    class: opts.variant ? `btn btn-${opts.variant}` : "btn",
    disabled: opts.disabled,
    title: opts.title,
  });
  node.textContent = label;
  node.addEventListener("click", onClick);
  return node;
}

/// Wraps `input` with a <label>, connected by a generated id when the input
/// has none, so screen-reader users get the association without every call
/// site inventing an id.
export function field(label: string, input: HTMLElement): HTMLElement {
  if (!input.id) input.id = `field-${Math.random().toString(36).slice(2, 10)}`;
  const labelNode = el("label", { for: input.id, class: "field-label" }, [label]);
  return el("div", { class: "field" }, [labelNode, input]);
}
