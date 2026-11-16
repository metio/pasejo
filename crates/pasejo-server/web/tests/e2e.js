// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// Drives the console against a running server. crates/pasejo-server/tests/console.rs
// starts the server, prepares an organization and passes everything this
// script needs in PASEJO_CONSOLE_E2E; the script fails with a non-zero exit
// code when the console misbehaves.

import assert from "node:assert/strict";
import { Api } from "../js/api.js";
import { Console } from "../js/app.js";
import { BreakGlass, sessionKey } from "../js/break-glass.js";
import { parseRecord, recordFingerprint } from "../js/device.js";
import { unb64 } from "../js/encoding.js";
import { importPrivate } from "../js/hpke.js";
import { translator } from "../js/i18n.js";
import { memoryStore, publicKey, signingKey } from "./helpers.js";
import { alerts, click, submit, text } from "./tree.js";

const config = JSON.parse(process.env.PASEJO_CONSOLE_E2E);
const device = {
  id: config.device.id,
  label: "Firefox console",
  signingKey: await signingKey(config.device.signing_seed),
  signingPublic: await publicKey(config.device.signing_seed),
  hpkeKey: await importPrivate(unb64(config.device.hpke_private)),
  hpkePublic: unb64(config.device.hpke_public),
};

// Node's fetch keeps no cookies, so the single-use console cookie is added
// to the one request the server reads it on.
const fetchWithCookie = (url, init) =>
  new URL(url).pathname === "/v1/auth/web"
    ? fetch(url, { ...init, headers: { ...init.headers, cookie: config.cookie } })
    : fetch(url, init);

const api = new Api({ base: config.url, signingKey: device.signingKey, fetch: fetchWithCookie });
let tree = null;
const app = new Console({ t: translator("en"), api, kv: memoryStore(), device, show: (next) => (tree = next) });
const step = (name) => process.stdout.write(`${name}\n`);
const expectScreen = (screen) => assert.equal(app.model.screen, screen, `${app.model.message ?? ""} ${alerts(tree).join(" ")}`);

await app.start();
expectScreen("orgs");
assert.ok(text(tree).includes("Signed in as Alice"));
step("signed in");

await click(tree, "Open");
expectScreen("confirm");
await click(tree, "The fingerprints match");
expectScreen("org");
assert.equal(app.model.labels.get(config.store), config.label);
assert.equal(app.model.indexes.get(config.store).secrets.size, 1);
step("opened the organization and the store's names");

await click(tree, "Members");
assert.ok(text(tree).includes("Bob"));
await click(tree, "Devices");
assert.ok(text(tree).includes("Firefox console (this browser)"));

await click(tree, "Pending actions");
const approval = app.model.actions.find((item) => item.action.action === "approve-device");
assert.ok(approval, JSON.stringify(app.model.actions));
assert.equal(approval.plan.type, "approve");
await submit(tree, { fingerprint: "0000 0000" });
assert.match(alerts(tree).join(), /does not match/);
const fingerprint = await recordFingerprint(parseRecord(approval.action.enrollment.device));
await submit(tree, { fingerprint });
assert.ok(app.model.log.state.devices.get(config.approve)?.active, alerts(tree).join());
assert.match(text(tree), /Done\./);
step("approved a device after comparing its fingerprint");

await click(tree, "Stores");
await submit(tree, { user: config.bob });
assert.equal(app.model.rotation.error, null, String(app.model.rotation.error?.message));
step("read a rotation list");

await click(tree, "Audit log");
await click(tree, "Load and verify");
assert.equal(app.model.audit.state, "done", alerts(tree).join());
assert.ok(app.model.audit.result.events.length > 0);
assert.ok(app.model.audit.result.events.some((event) => event.action === "log.append" || event.action.startsWith("log")));
step("verified the audit chain");

await click(tree, "Sign out");
expectScreen("sign-in");
step("signed out");

const breakGlassKey = await sessionKey();
const breakGlassApi = new Api({ base: config.url, signingKey: breakGlassKey.privateKey });
const pins = memoryStore();
let breakGlassTree = null;
const page = new BreakGlass({
  t: translator("en"),
  api: breakGlassApi,
  key: breakGlassKey,
  kv: pins,
  show: (next) => (breakGlassTree = next),
});
page.start();
await submit(breakGlassTree, config.break_glass);
assert.equal(page.model.screen, "dashboard", alerts(breakGlassTree).join(" "));
await click(breakGlassTree, "Load and verify");
assert.ok(page.model.audit, alerts(breakGlassTree).join(" "));
assert.ok(page.model.audit.checkpoints > 0, JSON.stringify(page.model.audit.checkpoints));
assert.ok(page.model.audit.events.some((event) => event.action === "break-glass.login"));
assert.match(text(breakGlassTree), /Verified \d+ events, \d+ checkpoints/);
assert.ok(text(breakGlassTree).includes("pinned in this browser"));
assert.ok(await pins.get("server-key"));
step("verified the server audit chain");
