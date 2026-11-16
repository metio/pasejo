// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

import assert from "node:assert/strict";
import { describe, test } from "node:test";
import { fieldValue, findAll, h, mount, render, textOf } from "../js/view.js";

/** Just enough of a DOM to observe what render does. */
function fakeDocument() {
  const element = (tag) => ({
    tag,
    attributes: {},
    properties: {},
    listeners: {},
    children: [],
    setAttribute(name, value) {
      this.attributes[name] = value;
    },
    addEventListener(event, handler) {
      this.listeners[event] = handler;
    },
    appendChild(child) {
      this.children.push(child);
    },
    replaceChildren(...children) {
      this.children = children;
    },
    set disabled(value) {
      this.properties.disabled = value;
    },
    set value(value) {
      this.properties.value = value;
    },
    set open(value) {
      this.properties.open = value;
    },
  });
  return {
    createElement: element,
    createTextNode: (text) => ({ text }),
  };
}

describe("view trees", () => {
  test("flatten children and drop empty ones", () => {
    const node = h("ul", {}, [h("li", {}, "a"), [null, h("li", {}, "b")]], false, undefined, 0);
    assert.equal(node.children.length, 3);
    assert.equal(textOf(node), "ab0");
  });

  test("find nodes depth first", () => {
    const tree = h("div", {}, h("p", { class: "x" }, "one"), h("section", {}, h("p", { class: "x" }, "two")));
    assert.deepEqual(findAll(tree, (node) => node.attributes.class === "x").map(textOf), ["one", "two"]);
  });
});

describe("rendering", () => {
  test("text becomes text nodes, never markup", () => {
    const document = fakeDocument();
    const element = render(h("p", {}, "<script>alert(1)</script>"), document);
    assert.deepEqual(element.children, [{ text: "<script>alert(1)</script>" }]);
  });

  test("sets attributes, properties and listeners", () => {
    const document = fakeDocument();
    const click = () => "clicked";
    const element = render(
      h("button", { type: "button", disabled: true, hidden: false, "aria-current": "page", required: false, on: { click } }, "go"),
      document,
    );
    assert.deepEqual(element.attributes, { type: "button", "aria-current": "page" });
    assert.equal(element.properties.disabled, true);
    assert.equal(element.listeners.click, click);
  });

  test("a true attribute renders empty", () => {
    const element = render(h("input", { autofocus: true }), fakeDocument());
    assert.equal(element.attributes.autofocus, "");
  });

  test("mount replaces the container's contents", () => {
    const document = fakeDocument();
    const container = document.createElement("div");
    container.appendChild({ text: "old" });
    mount(container, h("main", {}, "new"), document);
    assert.equal(container.children.length, 1);
    assert.equal(container.children[0].tag, "main");
  });

  test("reads form controls by name", () => {
    const event = { target: { elements: { user: { value: "a" } } } };
    assert.equal(fieldValue(event, "user"), "a");
    assert.equal(fieldValue(event, "missing"), "");
  });
});
