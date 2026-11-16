// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

import assert from "node:assert/strict";
import { describe, test } from "node:test";
import { Api } from "../js/api.js";
import { BreakGlass, sessionKey } from "../js/break-glass.js";
import { b64, utf8 } from "../js/encoding.js";
import { Context, sign } from "../js/envelope.js";
import { translator } from "../js/i18n.js";
import { memoryStore, signingKey, vectors } from "./helpers.js";
import { NOW, fakeServer } from "./server.js";
import { alerts, click, submit, text } from "./tree.js";

const t = translator("en");
const HOUR = 3_600_000;

async function setup(options = {}) {
  const server = await fakeServer(options);
  const key = await sessionKey();
  const api = new Api({ signingKey: key.privateKey, fetch: server.fetch, now: () => NOW });
  let tree = null;
  const page = new BreakGlass({ t, api, key, kv: options.kv ?? null, now: () => NOW, show: (next) => (tree = next) });
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

const SERVER_SEED = new Uint8Array(32).fill(0x5e);

/** A checkpoint of the server chain at `seq`, signed with `seed`. */
async function serverCheckpoint(seq, hash, { seed = SERVER_SEED, org = null, logHead = null } = {}) {
  const payload = utf8(JSON.stringify({ org, seq, hash, log_head: logHead, time: NOW }));
  return {
    seq,
    checkpoint: {
      payload: b64(payload),
      server_signature: b64(await sign(Context.Checkpoint, payload, await signingKey(seed))),
      cosignatures: [],
    },
  };
}

const EVENTS = vectors.audit.events;

describe("the server audit log", () => {
  test("verifies a chain kept from its first event", async () => {
    const { view } = await signedIn({ events: EVENTS });
    await click(view(), "Load and verify");
    assert.match(text(view()), new RegExp(`Verified ${EVENTS.length} events, 0 checkpoints`));
  });

  test("verifies a pruned chain from the checkpoint before its first event", async () => {
    const checkpoints = [await serverCheckpoint(2, EVENTS[1].hash), await serverCheckpoint(4, EVENTS[3].hash)];
    const { view } = await signedIn({ events: EVENTS.slice(2), serverCheckpoints: checkpoints });
    await click(view(), "Load and verify");
    assert.match(text(view()), new RegExp(`Verified ${EVENTS.length - 2} events, 2 checkpoints`));
  });

  test("refuses a pruned chain that no checkpoint vouches for", async () => {
    const { view } = await signedIn({ events: EVENTS.slice(2) });
    await click(view(), "Load and verify");
    assert.match(alerts(view()).join(), /Verification failed.*no checkpoint vouches/);
    assert.ok(!text(view()).includes("Verified"));
  });

  test("refuses an altered chain", async () => {
    const events = structuredClone(EVENTS);
    events[0].action = "nothing.happened";
    const { view } = await signedIn({ events });
    await click(view(), "Load and verify");
    assert.match(alerts(view()).join(), /Verification failed.*altered/);
  });

  test("refuses a checkpoint that contradicts the events", async () => {
    const { view } = await signedIn({ events: EVENTS, serverCheckpoints: [await serverCheckpoint(3, EVENTS[1].hash)] });
    await click(view(), "Load and verify");
    assert.match(alerts(view()).join(), /Verification failed.*contradicts/);
  });

  test("refuses a checkpoint the server did not sign", async () => {
    const forged = await serverCheckpoint(2, EVENTS[1].hash, { seed: new Uint8Array(32).fill(9) });
    const { view } = await signedIn({ events: EVENTS, serverCheckpoints: [forged] });
    await click(view(), "Load and verify");
    assert.match(alerts(view()).join(), /Verification failed.*not signed by the server/);
  });

  test("refuses a checkpoint of an organization's chain", async () => {
    const orgs = await serverCheckpoint(2, EVENTS[1].hash, { org: "0f".repeat(16), logHead: EVENTS[0].hash });
    const { view } = await signedIn({ events: EVENTS, serverCheckpoints: [orgs] });
    await click(view(), "Load and verify");
    assert.match(alerts(view()).join(), /Verification failed.*another chain/);
  });

  test("refuses a co-signed checkpoint, since no device co-signs the server chain", async () => {
    const checkpoint = await serverCheckpoint(2, EVENTS[1].hash);
    checkpoint.checkpoint.cosignatures = vectors.audit.checkpoint.cosignatures;
    const { view } = await signedIn({ events: EVENTS, serverCheckpoints: [checkpoint] });
    await click(view(), "Load and verify");
    assert.match(alerts(view()).join(), /Verification failed.*unknown device/);
  });

  test("shows the unpinned checkpoint key when the browser keeps no data", async () => {
    const { view } = await signedIn({ events: EVENTS });
    await click(view(), "Load and verify");
    assert.ok(text(view()).includes(`keeps no data, so the server's checkpoint key is not pinned. Compare ${vectors.audit.server_key}`));
  });

  test("pins the checkpoint key on first use and refuses another key later", async () => {
    const kv = memoryStore();
    const first = await signedIn({ events: EVENTS, kv });
    await click(first.view(), "Load and verify");
    assert.ok(text(first.view()).includes(`pinned in this browser: ${vectors.audit.server_key}`));
    assert.equal(b64(await kv.get("server-key")), vectors.audit.server_key);
    const second = await signedIn({ events: EVENTS, kv, serverKey: b64(new Uint8Array(32).fill(1)) });
    await click(second.view(), "Load and verify");
    assert.match(alerts(second.view()).join(), /Verification failed.*differs from the pinned one/);
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

