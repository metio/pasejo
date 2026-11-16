// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

import assert from "node:assert/strict";
import { before, describe, test } from "node:test";
import { plan } from "../js/actions.js";
import { parseEnvelope } from "../js/envelope.js";
import { OrgLog } from "../js/log.js";
import { READER, STORE, WRITER, logEnvelopes, strangerDevice } from "./server.js";

let state;

before(async () => {
  state = (await OrgLog.replay(logEnvelopes().map(parseEnvelope))).state;
});

const enrollmentOf = (record) => ({ device: record, fingerprint: "", requested: 0, user_name: "", user_email: "" });

describe("planning actions", () => {
  test("a device approval needs a person in manual mode", async () => {
    const { record } = await strangerDevice(50, WRITER);
    const planned = plan({ action: "approve-device", enrollment: enrollmentOf(record) }, state);
    assert.equal(planned.type, "approve");
    assert.equal(planned.device.id, record.id);
    assert.equal(planned.fingerprint, undefined);
  });

  test("a device approval is appended directly in automatic mode", async () => {
    const { record } = await strangerDevice(51, WRITER);
    const automatic = { ...state, settings: { ...state.settings, device_approval: "automatic" } };
    const planned = plan({ action: "approve-device", enrollment: enrollmentOf(record) }, automatic);
    assert.equal(planned.type, "append");
    assert.equal(planned.entries[0].kind, "device.add");
  });

  test("a device approval is skipped for non-members and known devices", async () => {
    const { record } = await strangerDevice(52, "a9".repeat(16));
    assert.deepEqual(plan({ action: "approve-device", enrollment: enrollmentOf(record) }, state), {
      type: "skip",
      reason: "action-skip-not-member",
    });
    const known = state.devices.values().next().value.record;
    const json = { ...known, signing_key: Buffer.from(known.signingKey).toString("base64url") };
    assert.deepEqual(plan({ action: "approve-device", enrollment: enrollmentOf(json) }, state), { type: "skip", reason: "action-skip-done" });
  });

  test("a malformed enrollment record is refused", () => {
    assert.throws(() => plan({ action: "approve-device", enrollment: enrollmentOf({ id: "x" }) }, state));
  });

  test("membership changes become role entries", () => {
    assert.deepEqual(plan({ action: "join-member", user: READER, name: "", email: "" }, state).entries, [
      { kind: "member.role", body: { user: READER, role: "member" } },
    ]);
    assert.deepEqual(plan({ action: "remove-member", user: READER }, state).entries, [
      { kind: "member.role", body: { user: READER, role: "none" } },
    ]);
  });

  test("a team sync creates a missing team before changing members", () => {
    const team = "7".repeat(32);
    const planned = plan({ action: "sync-team", team, name: "ops", exists: false, add: [WRITER], remove: [READER] }, state);
    assert.deepEqual(
      planned.entries.map((entry) => [entry.kind, entry.body.member]),
      [["team.create", undefined], ["team.member", true], ["team.member", false]],
    );
    const existing = plan({ action: "sync-team", team, name: "ops", exists: true, add: [], remove: [] }, state);
    assert.deepEqual(existing.entries, []);
  });

  test("grant revocations set the role to none without expiry", () => {
    for (const action of ["revoke-expired", "revoke-grant"]) {
      const planned = plan({ action, store: STORE, principal: { team: "7".repeat(32) }, review: "1".repeat(32) }, state);
      assert.deepEqual(planned.entries, [
        { kind: "store.grant", body: { store: STORE, principal: { team: "7".repeat(32) }, role: "none", expires: null } },
      ]);
    }
  });

  test("checkpoints are co-signed, key actions need full keys, unknown ones are skipped", () => {
    assert.deepEqual(plan({ action: "cosign-checkpoint", seq: 4 }, state), { type: "cosign", seq: 4 });
    assert.deepEqual(plan({ action: "rotate-epoch", store: STORE, epoch: 2 }, state), { type: "needs-keys" });
    assert.deepEqual(plan({ action: "wrap-keys", store: STORE, epoch: 1, devices: [] }, state), { type: "needs-keys" });
    assert.deepEqual(plan({ action: "future" }, state), { type: "skip", reason: "action-skip-unknown" });
  });
});
