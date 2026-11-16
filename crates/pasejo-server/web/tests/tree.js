// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// Helpers that read and operate view trees the way a person would: by the
// text they see.

import { findAll, textOf } from "../js/view.js";

export const buttons = (tree) => findAll(tree, (node) => node.tag === "button");

/** The button whose text is `label`; fails when there is none or several. */
export function buttonNamed(tree, label) {
  const found = buttons(tree).filter((node) => textOf(node) === label);
  if (found.length !== 1) {
    throw new Error(`expected one button "${label}", found ${found.length} among ${buttons(tree).map(textOf).join(" | ")}`);
  }
  return found[0];
}

/** Clicks the button labelled `label` and returns what its handler returns. */
export function click(tree, label) {
  return buttonNamed(tree, label).attributes.on.click();
}

/** Submits the `index`th form of the tree with the given control values. */
export function submit(tree, values, index = 0) {
  const forms = findAll(tree, (node) => node.tag === "form");
  const form = forms[index];
  if (!form) {
    throw new Error(`no form ${index}`);
  }
  let prevented = false;
  const elements = Object.fromEntries(Object.entries(values).map(([name, value]) => [name, { value }]));
  const result = form.attributes.on.submit({ preventDefault: () => (prevented = true), target: { elements } });
  if (!prevented) {
    throw new Error("the form did not prevent the default submission");
  }
  return result;
}

export const text = textOf;

export const rows = (tree) => findAll(tree, (node) => node.tag === "tr").slice(0);

export const alerts = (tree) => findAll(tree, (node) => node.attributes.role === "alert").map(textOf);
