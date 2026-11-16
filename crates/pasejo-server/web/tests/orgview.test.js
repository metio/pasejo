// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

import assert from "node:assert/strict";
import { before, describe, test } from "node:test";
import { parseEnvelope } from "../js/envelope.js";
import { OrgLog } from "../js/log.js";
import {
  auditRows,
  describeAction,
  describeEntry,
  deviceRows,
  directory,
  memberRows,
  principalName,
  readableStores,
  storeName,
  storeRows,
  teamRows,
  userDisplay,
} from "../js/orgview.js";
import { vectors } from "./helpers.js";
import { ADMIN, READER, STORE, USERS, WRITER, logEnvelopes } from "./server.js";

let state;
let teams;

const replay = async (scenario) =>
  (await OrgLog.replay([scenario.genesis, ...scenario.steps.filter((step) => step.result === "ok").map((step) => step.envelope)].map(parseEnvelope))).state;

before(async () => {
  state = (await OrgLog.replay(logEnvelopes().map(parseEnvelope))).state;
  teams = await replay(vectors.scenarios.find((scenario) => Object.keys(scenario.state.teams).length > 0));
});

describe("names", () => {
  test("machines are marked, people named by name then email", () => {
    assert.equal(userDisplay({ id: "x", name: "ci", email: "", machine: true }), "machine:ci");
    assert.equal(userDisplay({ id: "x", name: "", email: "a@b", machine: false }), "a@b");
    assert.equal(userDisplay({ id: "x", name: "", email: "", machine: false }), "x");
    assert.equal(directory(USERS).get(READER), "linus@example.com");
  });

  test("stores are named by label when readable", () => {
    assert.equal(storeName(STORE, new Map([[STORE, "prod"]])), "prod");
    assert.equal(storeName(STORE, new Map()), STORE);
  });

  test("teams are named by the log, users by the directory", () => {
    const team = [...teams.teams.keys()][0];
    assert.equal(principalName({ team }, teams, new Map()), teams.teams.get(team).name);
    assert.equal(principalName({ team: "f".repeat(32) }, teams, new Map()), "f".repeat(32));
    assert.equal(principalName({ user: ADMIN }, state, directory(USERS)), "Ada");
  });
});

describe("rows", () => {
  test("members are sorted by name with their active devices counted", () => {
    const rows = memberRows(state, USERS);
    assert.deepEqual(rows.map((row) => row.name), ["Ada", "Grace", "linus@example.com"]);
    assert.equal(rows[0].role, "admin");
    assert.equal(rows[0].devices, 2);
  });

  test("members missing from the directory show their identifier", () => {
    const rows = memberRows(state, []);
    assert.ok(rows.some((row) => row.name === WRITER && row.email === ""));
  });

  test("devices list active ones first and mark this browser", () => {
    const browser = vectors.store.browser.id;
    const rows = deviceRows(state, USERS, new Map([[browser, "abcd"]]), browser);
    assert.equal(rows.length, 4);
    const mine = rows.find((row) => row.mine);
    assert.equal(mine.id, browser);
    assert.equal(mine.kind, "hpke");
    assert.equal(mine.fingerprint, "abcd");
    assert.ok(rows.every((row) => row.active));
  });

  test("teams list their members by name", () => {
    const rows = teamRows(teams, []);
    assert.equal(rows.length, teams.teams.size);
    assert.ok(rows.length > 0);
    for (const row of rows) {
      assert.deepEqual(row.members, [...row.members].sort());
    }
  });

  test("stores list grants by rank, mark expired ones and show the epoch", () => {
    const [row] = storeRows(state, USERS, new Map([[STORE, "prod"]]), state.lastTime);
    assert.equal(row.label, "prod");
    assert.equal(row.epoch, 1);
    assert.deepEqual(row.grants.map((grant) => [grant.name, grant.role]), [
      ["Ada", "admin"],
      ["Grace", "writer"],
      ["linus@example.com", "reader"],
    ]);

    const expiring = structuredClone(state);
    expiring.stores.get(STORE).grants.get(`user:${READER}`).expires = 10;
    const [later] = storeRows(expiring, USERS, new Map(), 11);
    assert.equal(later.label, null);
    assert.equal(later.grants.find((grant) => grant.name === "linus@example.com").expired, true);
    assert.equal(later.grants.find((grant) => grant.name === "Grace").expired, false);
  });

  test("deleted stores sort last", () => {
    const copy = structuredClone(state);
    copy.stores.set("0".repeat(32), { ...copy.stores.get(STORE), deleted: true });
    const rows = storeRows(copy, USERS, new Map(), copy.lastTime);
    assert.equal(rows.at(-1).deleted, true);
  });

  test("only stores whose current epoch wraps names for this device are readable", () => {
    assert.deepEqual(readableStores(state, vectors.store.browser.id), [STORE]);
    assert.deepEqual(readableStores(state, ADMIN), []);
    const copy = structuredClone(state);
    copy.stores.get(STORE).deleted = true;
    assert.deepEqual(readableStores(copy, vectors.store.browser.id), []);
  });

  test("audit rows are newest first with names resolved where possible", () => {
    const events = [
      { seq: 1, time: 1, actor_user: ADMIN, action: "object.read", store: STORE, object: "o1", outcome: "ok", detail: null },
      { seq: 2, time: 2, actor_user: null, action: "auth.oidc", store: null, object: null, outcome: "denied", detail: "bad" },
    ];
    const rows = auditRows(events, directory(USERS), new Map([[STORE, "prod"]]), new Map([["o1", { name: "db/password" }]]));
    assert.deepEqual(rows.map((row) => row.seq), [2, 1]);
    assert.deepEqual([rows[1].actor, rows[1].store, rows[1].object], ["Ada", "prod", "db/password"]);
    assert.deepEqual([rows[0].actor, rows[0].store, rows[0].object, rows[0].detail], ["", "", "", "bad"]);
  });
});

describe("descriptions", () => {
  const names = directory(USERS);
  const labels = new Map([[STORE, "prod"]]);

  test("every entry kind of the vectors has a description with resolved names", () => {
    for (const scenario of vectors.scenarios) {
      for (const step of scenario.steps) {
        if (step.result !== "ok") {
          continue;
        }
        const entry = JSON.parse(Buffer.from(step.envelope.payload, "base64url").toString());
        const description = describeEntry(entry, state, names, labels);
        assert.notEqual(description.key, "entry-unknown", entry.kind);
      }
    }
  });

  test("role changes to none read as removals", () => {
    assert.deepEqual(describeEntry({ kind: "member.role", body: { user: WRITER, role: "none" } }, state, names, labels), {
      key: "entry-member-remove",
      args: { user: "Grace" },
    });
    assert.equal(
      describeEntry({ kind: "store.grant", body: { store: STORE, principal: { user: WRITER }, role: "none" } }, state, names, labels).key,
      "entry-store-revoke",
    );
  });

  test("an unknown entry kind is named, not hidden", () => {
    assert.deepEqual(describeEntry({ kind: "org.future", body: {} }, state, names, labels), { key: "entry-unknown", args: { kind: "org.future" } });
  });

  test("every action kind has a description", () => {
    const enrollment = { device: { label: "Chrome" }, user_email: "grace@example.com", user_name: "Grace" };
    const actions = [
      { action: "approve-device", enrollment },
      { action: "join-member", user: "a5".repeat(16), name: "", email: "e@example.com" },
      { action: "remove-member", user: WRITER },
      { action: "sync-team", team: "7".repeat(32), name: "ops", exists: false, add: [], remove: [] },
      { action: "rotate-epoch", store: STORE, epoch: 2 },
      { action: "wrap-keys", store: STORE, epoch: 1, devices: ["1".repeat(32), "2".repeat(32)] },
      { action: "revoke-expired", store: STORE, principal: { user: READER } },
      { action: "revoke-grant", store: STORE, principal: { user: READER }, review: "3".repeat(32) },
      { action: "cosign-checkpoint", seq: 7 },
    ];
    const keys = actions.map((action) => describeAction(action, state, names, labels));
    assert.ok(keys.every((description) => description.key !== "action-unknown"));
    assert.deepEqual(keys[1].args, { name: "e@example.com", email: "e@example.com" });
    assert.deepEqual(keys[5].args, { count: 2, store: "prod" });
    assert.equal(describeAction({ action: "future" }, state, names, labels).key, "action-unknown");
  });
});
