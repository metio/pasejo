// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// Views are plain trees built with `h` and turned into DOM nodes by
// `render`. Text always becomes text nodes, never markup, so nothing the
// server or another member wrote can inject script.

/** An element node: `h("p", {class: "muted"}, "text", child)`. */
export function h(tag, attributes = {}, ...children) {
  return { tag, attributes, children: children.flat(Infinity).filter((child) => child !== null && child !== undefined && child !== false) };
}

const PROPERTIES = new Set(["disabled", "checked", "required", "selected", "value", "open"]);

/** Builds DOM nodes from a tree. */
export function render(node, document) {
  if (typeof node === "string" || typeof node === "number") {
    return document.createTextNode(String(node));
  }
  const element = document.createElement(node.tag);
  for (const [name, value] of Object.entries(node.attributes)) {
    if (name === "on") {
      for (const [event, handler] of Object.entries(value)) {
        element.addEventListener(event, handler);
      }
    } else if (PROPERTIES.has(name)) {
      element[name] = value;
    } else if (value !== false && value !== null && value !== undefined) {
      element.setAttribute(name, value === true ? "" : String(value));
    }
  }
  for (const child of node.children) {
    element.appendChild(render(child, document));
  }
  return element;
}

/** The value of the form control `name` in the form a submit event came from. */
export function fieldValue(event, name) {
  return event.target.elements[name]?.value ?? "";
}

/** Replaces the contents of `container` with the rendered tree. */
export function mount(container, node, document) {
  container.replaceChildren(render(node, document));
}

/** The text of a tree, for tests and accessible names. */
export function textOf(node) {
  if (typeof node === "string" || typeof node === "number") {
    return String(node);
  }
  return node.children.map(textOf).join("");
}

/** Every node of a tree matching `predicate`, depth first. */
export function findAll(node, predicate) {
  if (typeof node !== "object") {
    return [];
  }
  const own = predicate(node) ? [node] : [];
  return [...own, ...node.children.flatMap((child) => findAll(child, predicate))];
}
