// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

import assert from "node:assert/strict";
import { describe, test } from "node:test";
import { Api } from "../js/api.js";
import { Console } from "../js/app.js";
import { orgFingerprint, recordFingerprint } from "../js/device.js";
import { deviceFingerprint } from "../js/devicekeys.js";
import { b64, unb64 } from "../js/encoding.js";
import { translator } from "../js/i18n.js";
import { findAll } from "../js/view.js";
import { memoryStore, vectors } from "./helpers.js";
import { ADMIN, NOW, ORG, READER, STORE, WRITER, browserDevice, fakeServer, postedEntry, strangerDevice } from "./server.js";
import { alerts, buttons, click, submit, text } from "./tree.js";

const t = translator("en");

async function setup(options = {}, { kv = memoryStore(), device } = {}) {
  const server = await fakeServer(options);
  const own = device ?? (await browserDevice());
  const api = new Api({ signingKey: own.signingKey, fetch: server.fetch, now: () => NOW });
  let tree = null;
  const shown = [];
  const app = new Console({
    t,
    api,
    kv,
    device: own,
    now: () => NOW,
    show: (next) => {
      tree = next;
      shown.push(next);
    },
  });
  return { server, api, kv, device: own, app, view: () => tree, shown };
}

/** Starts the console and opens the organization, confirming its fingerprint. */
async function opened(options, extra) {
  const context = await setup(options, extra);
  await context.app.start();
  await click(context.view(), "Open");
  await click(context.view(), "The fingerprints match");
  assert.equal(context.app.model.screen, "org", alerts(context.view()).join());
  return context;
}

const LABEL = vectors.store.index_plain.label;

describe("signing in", () => {
  test("exchanges the console cookie for a session bound to the device key", async () => {
    const { app, server, kv, device, view } = await setup();
    await app.start();
    assert.equal(app.model.screen, "orgs");
    assert.match(text(view()), /Signed in as Ada/);
    assert.deepEqual(b64(server.state.sessionKey), b64(device.signingPublic));
    assert.equal((await kv.get("session")).token, server.state.token);
    assert.deepEqual(server.paths().slice(0, 3), ["GET /v1/server", "POST /v1/auth/web", "GET /v1/me"]);
  });

  test("shows the sign-in link when no console sign-in is waiting", async () => {
    const { app, view } = await setup({ cookie: false });
    await app.start();
    assert.equal(app.model.screen, "sign-in");
    const links = findAll(view(), (node) => node.tag === "a").map((node) => node.attributes.href);
    assert.ok(links.includes("/auth/web/start"));
    assert.ok(links.includes("break-glass.html"));
  });

  test("reuses a stored session that is not about to expire", async () => {
    const kv = memoryStore();
    const first = await setup({}, { kv });
    await first.app.start();
    const second = await setup({ cookie: false }, { kv });
    second.server.state.token = first.server.state.token;
    second.server.state.sessionKey = first.server.state.sessionKey;
    await second.app.start();
    assert.equal(second.app.model.screen, "orgs");
    assert.ok(!second.server.paths().includes("POST /v1/auth/web"));
  });

  test("signs in again instead of reusing a session about to expire", async () => {
    const kv = memoryStore();
    await kv.put("session", { token: "old", expires: NOW + 30_000, user: ADMIN });
    const { app, server } = await setup({}, { kv });
    await app.start();
    assert.ok(server.paths().includes("POST /v1/auth/web"));
    assert.notEqual((await kv.get("session")).token, "old");
  });

  test("a session the server no longer knows leads back to the sign-in screen", async () => {
    const kv = memoryStore();
    await kv.put("session", { token: "revoked", expires: NOW + 3_600_000, user: ADMIN });
    const { app } = await setup({ cookie: false }, { kv });
    await app.start();
    assert.equal(app.model.screen, "sign-in");
    assert.equal(await kv.get("session"), undefined);
  });

  test("pins the server's checkpoint key and refuses a different one", async () => {
    const kv = memoryStore();
    const first = await setup({}, { kv });
    await first.app.start();
    assert.equal(b64(await kv.get("server-key")), vectors.audit.server_key);
    const second = await setup({ serverKey: b64(new Uint8Array(32).fill(9)) }, { kv });
    await second.app.start();
    assert.equal(second.app.model.screen, "failure");
    assert.match(second.app.model.message, /Verification failed.*checkpoint key/);
  });

  test("signing out ends the session on the server and forgets it", async () => {
    const { app, server, kv, view } = await setup();
    await app.start();
    await click(view(), "Sign out");
    assert.equal(app.model.screen, "sign-in");
    assert.equal(server.state.token, null);
    assert.equal(await kv.get("session"), undefined);
  });
});

describe("opening an organization", () => {
  test("asks to compare the organization's fingerprint before trusting it", async () => {
    const { app, server, kv, view } = await setup();
    await app.start();
    await click(view(), "Open");
    assert.equal(app.model.screen, "confirm");
    const expected = orgFingerprint(unb64(server.state.log.genesisHash));
    assert.ok(text(view()).includes(expected));
    assert.equal(await kv.get(`genesis:${ORG}`), undefined);
    await click(view(), "The fingerprints match");
    assert.equal(await kv.get(`genesis:${ORG}`), server.state.log.genesisHash);
  });

  test("cancelling the comparison pins nothing", async () => {
    const { app, kv, view } = await setup();
    await app.start();
    await click(view(), "Open");
    await click(view(), "Cancel");
    assert.equal(app.model.screen, "orgs");
    assert.equal(await kv.get(`genesis:${ORG}`), undefined);
  });

  test("an active device does not enroll again", async () => {
    const { app, server, view, device } = await setup();
    await app.start();
    await click(view(), "Open");
    assert.ok(server.paths().includes(`GET /v1/orgs/${ORG}/enrollments/${device.id}`));
    assert.ok(!server.paths().includes(`POST /v1/orgs/${ORG}/enrollments`));
  });

  test("a new device enrolls before reading the log, and only once", async () => {
    const { device } = await strangerDevice();
    const { app, server, view } = await setup({}, { device });
    await app.start();
    await click(view(), "Open");
    const paths = server.paths();
    assert.ok(paths.indexOf(`POST /v1/orgs/${ORG}/enrollments`) < paths.indexOf(`GET /v1/orgs/${ORG}/log`));
    assert.equal(server.state.enrolled[0].kind, "hpke");
    assert.equal(server.state.enrolled[0].id, device.id);
    await click(view(), "The fingerprints match");
    await click(view(), "Check again");
    assert.equal(server.paths().filter((path) => path === `POST /v1/orgs/${ORG}/enrollments`).length, 1);
  });

  test("refuses a log whose genesis differs from the pinned one", async () => {
    const kv = memoryStore();
    await kv.put(`genesis:${ORG}`, b64(new Uint8Array(32)));
    const { app, view } = await setup({}, { kv });
    await app.start();
    await click(view(), "Open");
    assert.equal(app.model.screen, "failure");
    assert.match(app.model.message, /genesis differs/);
  });

  test("skips the comparison once the genesis is pinned", async () => {
    const kv = memoryStore();
    const first = await opened({}, { kv });
    const { app, server, view } = await setup({ cookie: false }, { kv });
    server.state.token = first.server.state.token;
    server.state.sessionKey = first.server.state.sessionKey;
    await app.start();
    await click(view(), "Open");
    assert.equal(app.model.screen, "org");
  });

  test("a device the log does not know waits for approval with its fingerprint", async () => {
    const { device } = await strangerDevice();
    const { app, view } = await setup({}, { device });
    await app.start();
    await click(view(), "Open");
    await click(view(), "The fingerprints match");
    assert.equal(app.model.screen, "pending");
    const fingerprint = await deviceFingerprint(device);
    assert.ok(text(view()).includes(fingerprint));
    assert.ok(text(view()).includes(`pasejo org approve ${device.id}`));
    await click(view(), "Check again");
    assert.equal(app.model.screen, "pending");
  });
});

describe("the organization", () => {
  test("opens the store's names and pins its index version", async () => {
    const { app, kv } = await opened();
    assert.equal(app.model.labels.get(STORE), LABEL);
    const pin = await kv.get(`pin:${ORG}:${STORE}`);
    assert.equal(pin.version, 1);
    assert.equal(app.model.indexes.get(STORE).secrets.size, Object.keys(vectors.store.index_plain.secrets).length);
  });

  test("refuses an index older than the pinned version and shows no label", async () => {
    const kv = memoryStore();
    await kv.put(`pin:${ORG}:${STORE}`, { version: 2, hash: b64(new Uint8Array(32)) });
    const { app, view } = await opened({}, { kv });
    await click(view(), "Stores");
    assert.equal(app.model.labels.get(STORE), undefined);
    assert.match(alerts(view()).join(), /Verification failed, so nothing is shown: index went backwards/);
    assert.ok(text(view()).includes(STORE));
  });

  test("lists members with names from the directory and their roles", async () => {
    const { view } = await opened();
    const body = text(view());
    assert.ok(body.includes("Ada"));
    assert.ok(body.includes("Grace"));
    assert.ok(body.includes("linus@example.com"));
    assert.ok(body.includes("Admin"));
  });

  test("warns while the organization approves devices automatically", async () => {
    const { app, view } = await opened();
    assert.ok(!text(view()).includes("approves devices automatically"));
    app.model.log.state.settings = { ...app.model.log.state.settings, device_approval: "automatic" };
    app.set({});
    assert.ok(text(view()).includes("This organization approves devices automatically"));
    assert.ok(text(view()).includes("compromised server or identity provider"));
  });

  test("marks this browser among the devices and shows fingerprints", async () => {
    const { app, view, device } = await opened();
    await click(view(), "Devices");
    const body = text(view());
    assert.ok(body.includes("Firefox (this browser)"));
    const record = app.model.log.state.devices.get(device.id).record;
    assert.ok(body.includes(await recordFingerprint(record)));
  });

  test("shows the store's grants by name", async () => {
    const { view } = await opened();
    await click(view(), "Stores");
    const body = text(view());
    assert.ok(body.includes(LABEL));
    assert.ok(body.includes("Grace: Writer"));
    assert.ok(body.includes("linus@example.com: Reader"));
    assert.ok(body.includes("Key epoch 1"));
  });

  test("a failing directory still shows the log, with identifiers", async () => {
    const forbid = new Set([`GET /v1/orgs/${ORG}/users`]);
    const { view } = await opened({ forbid });
    assert.ok(text(view()).includes(WRITER));
    assert.match(alerts(view()).join(), /may not see this/);
  });

  test("shows the rotation list with object names from the index", async () => {
    const object = Object.values(vectors.store.index_plain.secrets)[0].id;
    const name = Object.keys(vectors.store.index_plain.secrets)[0];
    const rotation = { user: WRITER, ended: NOW - 1000, items: [{ id: object, fetched: true, rewritten: false }] };
    const { server, view } = await opened({ rotation });
    await click(view(), "Stores");
    await submit(view(), { user: WRITER });
    const request = server.requests.find((candidate) => candidate.path.endsWith("/rotation"));
    assert.equal(request.query, `?user=${WRITER}`);
    assert.ok(text(view()).includes(`${name} — unchanged, was read`));
  });
});

describe("pending actions", () => {
  test("adds a member who asked to join and reports it done", async () => {
    const user = "a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5";
    const actions = [{ action: "join-member", user, name: "Edsger", email: "edsger@example.com" }];
    const { app, server, view, device } = await opened({ actions });
    await click(view(), "Pending actions");
    assert.ok(text(view()).includes("Add Edsger (edsger@example.com) as a member"));
    server.state.actions = [];
    await click(view(), "Run");
    const posted = server.requests.filter((request) => request.path === `/v1/orgs/${ORG}/log` && request.method === "POST");
    assert.equal(posted.length, 1);
    const entry = postedEntry(posted[0]);
    assert.equal(entry.kind, "member.role");
    assert.deepEqual(entry.body, { user, role: "member" });
    assert.equal(entry.author, device.id);
    assert.equal(server.state.log.state.members.get(user), "member");
    assert.equal(app.model.log.state.members.get(user), "member");
    assert.match(text(view()), /Done\./);
  });

  test("retries once when another entry took the next slot", async () => {
    const actions = [{ action: "remove-member", user: READER }];
    const { server, view } = await opened({ actions, conflictOnce: true });
    await click(view(), "Pending actions");
    await click(view(), "Run");
    assert.equal(server.state.log.state.members.has(READER), false);
  });

  test("approves a device only when the typed fingerprint matches the record", async () => {
    const { record } = await strangerDevice(40, WRITER);
    const enrollment = { device: record, fingerprint: "server supplied", requested: NOW, user_name: "Grace", user_email: "grace@example.com" };
    const { server, view } = await opened({ actions: [{ action: "approve-device", enrollment }] });
    await click(view(), "Pending actions");
    await submit(view(), { fingerprint: "server supplied" });
    assert.match(alerts(view()).join(), /does not match/);
    assert.equal(server.state.log.state.devices.has(record.id), false);

    const fingerprint = await recordFingerprint({ ...record, signingKey: unb64(record.signing_key) });
    await submit(view(), { fingerprint: fingerprint.toUpperCase().replaceAll(" ", "") });
    assert.equal(server.state.log.state.devices.get(record.id)?.active, true);
  });

  test("leaves key actions to a device with full keys", async () => {
    const actions = [{ action: "rotate-epoch", store: STORE, epoch: 2 }];
    const { view } = await opened({ actions });
    await click(view(), "Pending actions");
    assert.ok(text(view()).includes(`Rotate the keys of ${LABEL}`));
    assert.ok(text(view()).includes("pasejo org actions --run"));
    assert.ok(!buttons(view()).some((node) => text(node) === "Run"));
  });

  test("an append the log refuses is reported and not sent", async () => {
    const actions = [{ action: "revoke-grant", store: STORE, principal: { user: "a9a9a9a9a9a9a9a9a9a9a9a9a9a9a9a9" }, review: "1".repeat(32) }];
    const { server, view } = await opened({ actions });
    await click(view(), "Pending actions");
    const before = server.state.envelopes.length;
    await click(view(), "Run");
    assert.equal(server.state.envelopes.length, before);
    assert.equal(alerts(view()).length, 1);
  });

  test("a forbidden action list is reported in its tab", async () => {
    const { view } = await opened({ forbid: new Set([`GET /v1/orgs/${ORG}/actions`]) });
    await click(view(), "Pending actions");
    assert.match(alerts(view()).join(), /may not see this/);
  });
});

describe("access reviews", () => {
  const review = (decision = null) => ({
    id: "7".repeat(32),
    store: STORE,
    reviewers: [ADMIN],
    due: NOW + 86_400_000,
    created: NOW,
    created_by: ADMIN,
    items: [{ principal: { user: WRITER }, role: "writer", decision, decided_by: decision ? ADMIN : null }],
    complete: decision !== null,
  });

  test("a reviewer decides each grant", async () => {
    const { server, view } = await opened({ reviews: [review()] });
    server.on(`POST /v1/orgs/${ORG}/reviews/${"7".repeat(32)}/decisions`, (request, body) => {
      server.state.decisions.push(body);
      server.state.reviews = [review(body.decision)];
      return undefined;
    });
    await click(view(), "Access reviews");
    assert.ok(text(view()).includes(`Review of ${LABEL}`));
    await click(view(), "Revoke");
    assert.deepEqual(server.state.decisions, [{ principal: { user: WRITER }, decision: "revoke" }]);
    assert.ok(text(view()).includes("Revoke by Ada"));
  });

  test("shows that there are none", async () => {
    const { view } = await opened();
    await click(view(), "Access reviews");
    assert.ok(text(view()).includes("No access reviews."));
  });
});

describe("the audit log", () => {
  test("verifies the hash chain before showing events", async () => {
    const { view } = await opened({ events: vectors.audit.events });
    await click(view(), "Audit log");
    await click(view(), "Load and verify");
    assert.match(text(view()), new RegExp(`Verified ${vectors.audit.events.length} events`));
    assert.ok(text(view()).includes("object.read"));
  });

  test("shows nothing of a tampered chain", async () => {
    const events = structuredClone(vectors.audit.events);
    events[2].detail = "changed";
    const { view } = await opened({ events });
    await click(view(), "Audit log");
    await click(view(), "Load and verify");
    assert.match(alerts(view()).join(), /Verification failed/);
    assert.ok(!text(view()).includes("object.read"));
  });

  test("refuses a checkpoint naming a log head this browser never saw", async () => {
    const checkpoints = [{ seq: vectors.audit.checkpoint_payload.seq, checkpoint: vectors.audit.checkpoint }];
    const { view } = await opened({ events: vectors.audit.events, checkpoints });
    await click(view(), "Audit log");
    await click(view(), "Load and verify");
    assert.match(alerts(view()).join(), /log head this client never saw/);
  });

  test("tells members without access who reads it", async () => {
    const { view } = await opened({ forbid: new Set([`GET /v1/orgs/${ORG}/audit/checkpoints`]) });
    await click(view(), "Audit log");
    await click(view(), "Load and verify");
    assert.match(alerts(view()).join(), /Only admins and auditors/);
  });
});

describe("rendering", () => {
  test("every shown tree carries text only, never markup from the server", async () => {
    const actions = [{ action: "join-member", user: "a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5", name: "<img src=x onerror=alert(1)>", email: "x@example.com" }];
    const { view } = await opened({ actions });
    await click(view(), "Pending actions");
    assert.ok(text(view()).includes("<img src=x onerror=alert(1)>"));
    assert.equal(findAll(view(), (node) => node.tag === "img").length, 0);
  });
});
