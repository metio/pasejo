// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

import assert from "node:assert/strict";
import { describe, test } from "node:test";
import { Api } from "../js/api.js";
import { BreakGlass, sessionKey } from "../js/break-glass.js";
import { b64 } from "../js/encoding.js";
import { translator } from "../js/i18n.js";
import { vectors } from "./helpers.js";
import { NOW, fakeServer } from "./server.js";
import { alerts, click, submit, text } from "./tree.js";

const t = translator("en");
const HOUR = 3_600_000;

async function setup(options = {}) {
  const server = await fakeServer(options);
  const key = await sessionKey();
  const api = new Api({ signingKey: key.privateKey, fetch: server.fetch, now: () => NOW });
  let tree = null;
  const page = new BreakGlass({ t, api, key, now: () => NOW, show: (next) => (tree = next) });
  page.start();
  return { server, key, page, view: () => tree };
}

async function signedIn(options) {
  const context = await setup(options);
  await submit(context.view(), { username: "break-glass", password: "correct horse", totp: "123456" });
  assert.equal(context.page.model.screen, "dashboard", alerts(context.view()).join());
  return context;
}

describe("break-glass sign-in", () => {
  test("binds the session to a key generated for this page", async () => {
    const { server, key, page } = await signedIn();
    assert.equal(b64(server.state.sessionKey), b64(key.publicKey));
    assert.equal(page.api.session.user, null);
  });

  test("the session key cannot be exported", async () => {
    const { privateKey } = await sessionKey();
    assert.equal(privateKey.extractable, false);
    await assert.rejects(globalThis.crypto.subtle.exportKey("pkcs8", privateKey));
  });

  test("wrong credentials stay on the form with the server's message", async () => {
    const { page, view } = await setup();
    await submit(view(), { username: "break-glass", password: "wrong", totp: "123456" });
    assert.equal(page.model.screen, "login");
    assert.match(alerts(view()).join(), /invalid credentials/);
  });

  test("lists users and organizations", async () => {
    const { view } = await signedIn();
    const body = text(view());
    assert.ok(body.includes("Grace"));
    assert.ok(body.includes("store"));
    assert.ok(body.includes("Closed"));
  });
});

describe("the emergency window", () => {
  test("opens for the chosen hours and closes again", async () => {
    const { server, view } = await signedIn();
    await submit(view(), { hours: "3" });
    assert.equal(server.state.emergencyUntil, NOW + 3 * HOUR);
    assert.ok(text(view()).includes("Open until"));
    await click(view(), "Close the window");
    assert.equal(server.state.emergencyUntil, null);
    assert.ok(text(view()).includes("Closed"));
  });

  test("refuses more than 24 hours or fractions without asking the server", async () => {
    const { server, view } = await signedIn();
    for (const hours of ["25", "1.5", "-1"]) {
      await submit(view(), { hours });
      assert.match(alerts(view()).join(), /Hours, at most 24/);
    }
    assert.ok(!server.paths().includes("POST /v1/break-glass/emergency"));
  });
});

describe("user actions", () => {
  test("suspends, reactivates and signs out a user", async () => {
    const { server, view } = await signedIn();
    const inRow = (label) => click(findRow(view(), "Grace"), label);
    await inRow("Suspend");
    assert.ok(text(findRow(view(), "Grace")).includes("Suspended"));
    await inRow("Reactivate");
    assert.ok(text(findRow(view(), "Grace")).includes("Active"));
    await inRow("End sessions");
    assert.match(text(view()), /The sessions of Grace have ended\./);
    assert.deepEqual(server.state.decisions, ["suspend", "reactivate", "sessions/revoke"]);
  });

  test("a lost session returns to the sign-in form", async () => {
    const { server, page, view } = await signedIn();
    server.state.token = "expired";
    await click(findRow(view(), "Grace"), "End sessions");
    assert.equal(page.model.screen, "login");
    assert.equal(page.api.session, null);
  });
});

describe("the server audit log", () => {
  test("verifies a chain kept from its first event", async () => {
    const { view } = await signedIn({ events: vectors.audit.events });
    await click(view(), "Load and verify");
    assert.match(text(view()), new RegExp(`Verified ${vectors.audit.events.length} events`));
  });

  test("refuses an altered chain", async () => {
    const events = structuredClone(vectors.audit.events);
    events[0].action = "nothing.happened";
    const { view } = await signedIn({ events });
    await click(view(), "Load and verify");
    assert.match(alerts(view()).join(), /Verification failed.*altered/);
  });

  test("shows a pruned chain without claiming to have verified it", async () => {
    const { view } = await signedIn({ events: vectors.audit.events.slice(2) });
    await click(view(), "Load and verify");
    assert.ok(!text(view()).includes("Verified"));
    assert.ok(text(view()).includes("object.read"));
  });
});

function findRow(tree, name) {
  const stack = [tree];
  while (stack.length) {
    const node = stack.pop();
    if (typeof node !== "object") {
      continue;
    }
    if (node.tag === "tr" && text(node).includes(name)) {
      return node;
    }
    stack.push(...node.children);
  }
  throw new Error(`no row for ${name}`);
}

