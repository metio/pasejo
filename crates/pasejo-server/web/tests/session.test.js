// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

import assert from "node:assert/strict";
import { describe, test } from "node:test";
import { Api, ApiError } from "../js/api.js";
import { browserName, createDevice, deviceFingerprint, enrollmentRequest, loadOrCreateDevice } from "../js/devicekeys.js";
import { recordFingerprint } from "../js/device.js";
import { b64, unb64, utf8 } from "../js/encoding.js";
import { Context, verify } from "../js/envelope.js";
import { proofMessage } from "../js/request.js";
import { append, appendFresh, auditEvents, cosignProposal, fetchLog, proposals, serverKey } from "../js/session.js";
import { errorResponse, fakeFetch, memoryStore, signingKey } from "./helpers.js";
import { ADMIN, NOW, ORG, READER, WRITER, browserDevice, fakeServer } from "./server.js";

async function signedApi(server, device) {
  const api = new Api({ signingKey: device.signingKey, fetch: server.fetch, now: () => NOW });
  server.state.sessionKey = device.signingPublic;
  server.state.token = "token";
  api.session = { token: "token", expires: NOW + 1000, user: ADMIN };
  return api;
}

describe("the API", () => {
  test("signs the exact method, path, query and body bytes", async () => {
    const device = await browserDevice();
    const { fetch, requests } = fakeFetch({ "POST /v1/x": () => ({ ok: true }) });
    const api = new Api({ signingKey: device.signingKey, fetch, now: () => 1234 });
    api.session = { token: "secret", expires: 0, user: null };
    assert.deepEqual(await api.post("/v1/x?a=1", { b: "ü" }), { ok: true });
    const [request] = requests;
    assert.equal(request.headers.authorization, "Pasejo secret");
    assert.equal(request.headers["pasejo-timestamp"], "1234");
    assert.equal(request.headers["content-type"], "application/json");
    const message = await proofMessage("POST", "/v1/x?a=1", "1234", request.headers["pasejo-nonce"], utf8(request.body), "secret");
    assert.equal(await verify(Context.Request, message, device.signingPublic, unb64(request.headers["pasejo-proof"])), true);
    assert.equal(request.body, '{"b":"ü"}');
  });

  test("uses a fresh nonce per request", async () => {
    const device = await browserDevice();
    const { fetch, requests } = fakeFetch({ "GET /v1/me": () => ({}) });
    const api = new Api({ signingKey: device.signingKey, fetch });
    api.session = { token: "t", expires: 0, user: null };
    await api.get("/v1/me");
    await api.get("/v1/me");
    assert.notEqual(requests[0].headers["pasejo-nonce"], requests[1].headers["pasejo-nonce"]);
  });

  test("refuses to send an authenticated request without a session", async () => {
    const { fetch, requests } = fakeFetch({});
    const api = new Api({ signingKey: null, fetch });
    await assert.rejects(api.get("/v1/me"), (error) => error instanceof ApiError && error.status === 401);
    assert.equal(requests.length, 0);
  });

  test("anonymous requests carry no credentials", async () => {
    const { fetch, requests } = fakeFetch({ "GET /v1/server": () => ({}) });
    const api = new Api({ signingKey: null, fetch });
    api.session = { token: "t" };
    await api.anonymous("GET", "/v1/server");
    assert.equal(requests[0].headers.authorization, undefined);
  });

  test("reports server errors with their code, and other failures plainly", async () => {
    const { fetch } = fakeFetch({
      "GET /v1/a": () => new Response(JSON.stringify({ code: "conflict", message: "taken", current_version: 4 }), { status: 409 }),
      "GET /v1/b": () => new Response("<html>bad gateway</html>", { status: 502 }),
      "GET /v1/c": () => new Response("not json", { status: 200 }),
      "GET /v1/d": () => new Response(null, { status: 204 }),
    });
    const api = new Api({ signingKey: null, fetch });
    await assert.rejects(api.anonymous("GET", "/v1/a"), { status: 409, code: "conflict", message: "taken", currentVersion: 4 });
    await assert.rejects(api.anonymous("GET", "/v1/b"), { status: 502, code: null, message: "<html>bad gateway</html>" });
    await assert.rejects(api.anonymous("GET", "/v1/c"), { code: "malformed" });
    assert.equal(await api.anonymous("GET", "/v1/d"), null);
  });

  test("a network failure is reported as unavailable", async () => {
    const api = new Api({ signingKey: null, fetch: async () => { throw new TypeError("offline"); } });
    await assert.rejects(api.anonymous("GET", "/v1/server"), { status: 0, code: "unavailable" });
  });

  test("never follows redirects and never caches", async () => {
    let init;
    const api = new Api({ signingKey: null, fetch: async (url, options) => { init = options; return new Response("{}"); } });
    await api.anonymous("GET", "/v1/server");
    assert.equal(init.redirect, "error");
    assert.equal(init.cache, "no-store");
    assert.equal(init.credentials, "same-origin");
  });
});

describe("device keys", () => {
  test("private keys cannot be exported", async () => {
    const device = await createDevice("Firefox console");
    assert.equal(device.signingKey.extractable, false);
    assert.equal(device.hpkeKey.extractable, false);
    await assert.rejects(globalThis.crypto.subtle.exportKey("pkcs8", device.signingKey));
    await assert.rejects(globalThis.crypto.subtle.exportKey("pkcs8", device.hpkeKey));
    assert.equal(device.signingPublic.length, 32);
    assert.equal(device.hpkePublic.length, 32);
    assert.match(device.id, /^[0-9a-f]{32}$/);
  });

  test("are created once and then reused", async () => {
    const kv = memoryStore();
    const first = await loadOrCreateDevice(kv, "one");
    const second = await loadOrCreateDevice(kv, "two");
    assert.equal(second.id, first.id);
    assert.equal(second.label, "one");
  });

  test("enroll as an hpke device whose fingerprint matches its record", async () => {
    const device = await createDevice("Chrome console");
    const request = enrollmentRequest(device);
    assert.deepEqual(request, { id: device.id, kind: "hpke", recipient: b64(device.hpkePublic), label: "Chrome console" });
    const record = { ...request, user: ADMIN, signingKey: device.signingPublic };
    assert.equal(await deviceFingerprint(device), await recordFingerprint(record));
  });

  test("name the browser from its user agent", () => {
    assert.equal(browserName("Mozilla/5.0 (X11; Linux x86_64; rv:145.0) Gecko/20100101 Firefox/145.0"), "Firefox");
    assert.equal(browserName("Mozilla/5.0 AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0 Safari/537.36 Edg/142.0"), "Edge");
    assert.equal(browserName("Mozilla/5.0 AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0 Safari/537.36"), "Chrome");
    assert.equal(browserName("Mozilla/5.0 (Macintosh) AppleWebKit/605.1.15 Version/18.0 Safari/605.1.15"), "Safari");
    assert.equal(browserName("curl/8"), "Browser");
  });
});

describe("the server key", () => {
  test("is pinned on first use", async () => {
    const server = await fakeServer();
    const api = new Api({ signingKey: null, fetch: server.fetch });
    const kv = memoryStore();
    const first = await serverKey(api, kv);
    assert.equal(b64(await kv.get("server-key")), b64(first.key));
    server.state.serverKey = b64(new Uint8Array(32).fill(1));
    await assert.rejects(serverKey(api, kv), /differs from the pinned one/);
  });
});

describe("the membership log", () => {
  test("continues a known log from its last entry", async () => {
    const server = await fakeServer();
    const api = await signedApi(server, await browserDevice());
    const log = await fetchLog(api, ORG);
    const again = await fetchLog(api, ORG, log);
    assert.equal(again.head, log.head);
    const continued = server.requests.at(-1);
    assert.equal(continued.query, `?since=${log.entries.length - 1}`);
  });

  test("refuses a log of another organization", async () => {
    const server = await fakeServer();
    const api = await signedApi(server, await browserDevice());
    const other = "e".repeat(32);
    server.on(`GET /v1/orgs/${other}/log`, () => ({ entries: server.state.envelopes }));
    await assert.rejects(fetchLog(api, other), /another organization/);
  });

  test("refuses an entry its own replay refuses, without sending it", async () => {
    const server = await fakeServer();
    const api = await signedApi(server, await browserDevice());
    const log = await fetchLog(api, ORG);
    await assert.rejects(append(api, log, await browserDevice(), "member.role", { user: ADMIN, role: "member" }));
    assert.ok(!server.paths().includes(`POST /v1/orgs/${ORG}/log`));
  });

  test("refetches once on conflict and gives up on a second", async () => {
    const server = await fakeServer({ conflictOnce: true });
    const device = await browserDevice();
    const api = await signedApi(server, device);
    const log = await fetchLog(api, ORG);
    const { log: after } = await appendFresh(api, log, device, "member.role", { user: READER, role: "auditor" });
    assert.equal(after.state.members.get(READER), "auditor");
    server.on(`POST /v1/orgs/${ORG}/log`, () => errorResponse(409, "conflict", "taken"));
    await assert.rejects(appendFresh(api, after, device, "member.role", { user: READER, role: "member" }), { code: "conflict" });
  });

  test("settings entries open and close the periods of offline reads", async () => {
    const server = await fakeServer();
    const device = await browserDevice();
    const api = await signedApi(server, device);
    let log = await fetchLog(api, ORG);
    assert.equal(log.state.settings.offline_reads, false);
    assert.deepEqual(log.offlineReadPeriods(), []);
    const settings = { ...log.state.settings, offline_reads: true };
    ({ log } = await append(api, log, device, "org.settings", { settings }));
    const opened = log.parsed.at(-1).time;
    assert.equal(log.state.settings.offline_reads, true);
    assert.equal(server.state.log.state.settings.offline_reads, true);
    assert.deepEqual(log.offlineReadPeriods(), [{ from: opened, until: null }]);
    ({ log } = await append(api, log, device, "org.settings", { settings: { ...settings, auto_join: true } }));
    assert.deepEqual(log.offlineReadPeriods(), [{ from: opened, until: null }]);
    ({ log } = await append(api, log, device, "org.settings", { settings: { ...settings, offline_reads: false } }));
    assert.deepEqual(log.offlineReadPeriods(), [{ from: opened, until: log.parsed.at(-1).time }]);
    assert.equal(server.state.log.offlineReadPeriods().length, 1);
  });

  test("an entry needing a second admin becomes a proposal that another admin co-signs", async () => {
    const server = await fakeServer();
    const device = await browserDevice();
    const api = await signedApi(server, device);
    let log = await fetchLog(api, ORG);
    ({ log } = await append(api, log, device, "member.role", { user: WRITER, role: "admin" }));
    const settings = { device_approval: "manual", two_admin: true, auto_join: false };

    const result = await append(api, log, device, "org.settings", { settings });
    assert.notEqual(result.proposal, null);
    assert.equal(result.log, log);
    assert.equal(server.state.log.state.settings.two_admin, false);

    const [proposal, ...rest] = await proposals(api, ORG);
    assert.deepEqual(rest, []);
    assert.equal(proposal.entry.kind, "org.settings");

    const reader = { id: "03".repeat(16), signingKey: await signingKey(new Uint8Array(32).fill(3)) };
    const posts = () => server.paths().filter((path) => path.endsWith("/cosign")).length;
    await assert.rejects(cosignProposal(api, log, reader, proposal));
    assert.equal(posts(), 0);

    const writer = { id: "02".repeat(16), signingKey: await signingKey(new Uint8Array(32).fill(2)) };
    const after = await cosignProposal(api, log, writer, proposal);
    assert.equal(posts(), 1);
    assert.equal(after.state.settings.two_admin, true);
    assert.equal(server.state.log.state.settings.two_admin, true);
  });
});

describe("audit pages", () => {
  test("follow the cursor until a short page", async () => {
    const events = Array.from({ length: 1203 }, (_, index) => ({ seq: index + 1 }));
    const server = await fakeServer({ events });
    const api = await signedApi(server, await browserDevice());
    const all = await auditEvents(api, `/v1/orgs/${ORG}/audit`, 0);
    assert.equal(all.length, 1203);
    assert.deepEqual(
      server.requests.filter((request) => request.path.endsWith("/audit")).map((request) => request.query),
      ["?since=0&limit=500", "?since=500&limit=500", "?since=1000&limit=500"],
    );
  });
});
