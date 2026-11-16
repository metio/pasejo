// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// Cross-language parity: the same vectors file is verified by the Rust
// implementation, so every assertion here pins the console to it.

import assert from "node:assert/strict";
import { describe, test } from "node:test";
import { b64, unb64 } from "../js/encoding.js";
import { parseEnvelope, Context, sign } from "../js/envelope.js";
import { OrgLog, entitled, entryJson, parseEntry, settingsJson } from "../js/log.js";
import { dataCommitment, derive, namesCommitment, padme } from "../js/crypto.js";
import { fingerprint, orgFingerprint, recoveryId, parseRecord } from "../js/device.js";
import { importPrivate } from "../js/hpke.js";
import { unwrapNames, wrapNames } from "../js/wrapping.js";
import { openNames } from "../js/keys.js";
import { checkChain, openIndex } from "../js/store.js";
import { eventHash, verifyChain, verifyCheckpoint, verifyCosignature } from "../js/audit.js";
import { proofMessage } from "../js/request.js";
import { vectors, publicKey, signingKey } from "./helpers.js";
import { claim } from "../js/devicekeys.js";

const sorted = (set) => [...set].sort();

/** The state summary of crates/pasejo-protocol/tests/vectors.rs. */
function summary(state) {
  const sortedObject = (entries) => Object.fromEntries(entries.sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)));
  return {
    org: state.org,
    name: state.name,
    settings: settingsJson(state.settings),
    members: sortedObject([...state.members]),
    devices: sortedObject(
      [...state.devices].map(([id, device]) => [
        id,
        { active: device.active, user: device.record.user, kind: device.record.kind },
      ]),
    ),
    recovery: state.recovery,
    teams: sortedObject([...state.teams].map(([id, team]) => [id, { name: team.name, members: sorted(team.members) }])),
    stores: sortedObject(
      [...state.stores].map(([id, store]) => {
        const rights = entitled(state, id, state.lastTime);
        return [
          id,
          {
            deleted: store.deleted,
            created_seq: store.createdSeq,
            grants: sortedObject([...store.grants].map(([key, grant]) => [key, { role: grant.role, expires: grant.expires }])),
            epochs: store.epochs.map((epoch) => ({
              names_commit: epoch.namesCommit,
              data_commit: epoch.dataCommit,
              full: sorted(epoch.full),
              names_only: sorted(epoch.namesOnly),
              seq: epoch.seq,
              time: epoch.time,
            })),
            entitled: { full: sorted(rights.full), names_only: sorted(rights.namesOnly) },
          },
        ];
      }),
    ),
    next_seq: state.nextSeq,
    head: state.head,
    last_time: state.lastTime,
  };
}

async function verdict(promise) {
  try {
    await promise;
    return "ok";
  } catch (error) {
    return error.code ?? error.message;
  }
}

async function replay(scenario) {
  const log = await OrgLog.genesis(parseEnvelope(scenario.genesis));
  for (const step of scenario.steps) {
    assert.equal(await verdict(log.append(parseEnvelope(step.envelope))), step.result, `${scenario.name}: ${step.note}`);
  }
  return log;
}

describe("membership log scenarios", () => {
  for (const scenario of vectors.scenarios) {
    test(`${scenario.name}: every verdict and the final state match`, async () => {
      const log = await replay(scenario);
      assert.deepEqual(summary(log.state), scenario.state);
    });

    test(`${scenario.name}: entries re-serialize to the signed bytes`, () => {
      for (const step of scenario.steps.filter((candidate) => candidate.result === "ok" && candidate.serialized)) {
        const envelope = parseEnvelope(step.envelope);
        const text = new TextDecoder().decode(envelope.payload);
        assert.equal(JSON.stringify(entryJson(parseEntry(envelope.payload))), text, step.note);
      }
    });

    test(`${scenario.name}: replay reaches the state of appending and reconstructs earlier states`, async () => {
      const accepted = [
        scenario.genesis,
        ...scenario.steps.filter((step) => step.result === "ok").map((step) => step.envelope),
      ].map(parseEnvelope);
      const log = await OrgLog.replay(accepted);
      assert.deepEqual(summary(log.state), scenario.state);
      for (let seq = 0; seq < accepted.length; seq++) {
        const prefix = await OrgLog.replay(accepted.slice(0, seq + 1));
        assert.deepEqual(summary(log.stateAt(seq)), summary(prefix.state), `state after ${seq}`);
      }
    });
  }
});

describe("geneses", () => {
  for (const genesis of vectors.geneses) {
    test(genesis.name, async () => {
      assert.equal(await verdict(OrgLog.genesis(parseEnvelope(genesis.genesis))), genesis.result);
    });
  }
});

describe("store keys and indexes", () => {
  const store = vectors.store;

  test("the browser's wrappings open to the committed names keys and the chain reaches back", async () => {
    const log = await replay(store.log);
    const privateKey = await importPrivate(unb64(store.browser.hpke_private));
    const ring = await openNames(log, store.store, store.browser.id, store.keys, privateKey, unb64(store.browser.hpke_public));
    assert.deepEqual([...ring.keys()].sort(), [0, 1]);
    assert.equal(b64(ring.get(0)), store.names_keys[0]);
    assert.equal(b64(ring.get(1)), store.names_keys[1]);
  });

  for (const check of store.checks) {
    test(`index check: ${check.name}`, async () => {
      const log = await replay(store.log);
      const envelope = parseEnvelope(check.index);
      const result = await verdict(
        (async () => {
          const payload = await log.checkIndex(envelope);
          await checkChain(envelope, payload, store.store, check.pin);
          const index = await openIndex(store.store, payload.epoch, payload.version, unb64(store.names_keys[payload.epoch]), payload.index);
          assert.equal(index.label, store.index_plain.label);
          assert.deepEqual(Object.fromEntries(index.secrets), store.index_plain.secrets);
          assert.deepEqual(Object.fromEntries(index.otp), store.index_plain.otp);
          assert.deepEqual(index.settings, store.index_plain.settings);
        })(),
      );
      assert.equal(result, check.result);
    });
  }
});

describe("audit", () => {
  const audit = vectors.audit;

  test("event hashes match, including escapes and non-ASCII details", async () => {
    for (const event of audit.events) {
      assert.equal(await eventHash(event), event.hash, JSON.stringify(event.detail));
    }
  });

  test("the chain verifies and ends at the checkpoint's hash", async () => {
    const end = await verifyChain(audit.start, audit.first_seq, audit.events);
    const payload = await verifyCheckpoint(audit.checkpoint, unb64(audit.server_key));
    assert.deepEqual(payload, audit.checkpoint_payload);
    assert.equal(payload.hash, end);
    assert.ok(await verifyCosignature(audit.checkpoint, audit.cosigner.device, unb64(audit.cosigner.key)));
  });
});

test("request proofs: the message and signature match", async () => {
  const request = vectors.request;
  const message = await proofMessage(request.method, request.path, request.timestamp, request.nonce, unb64(request.body), request.token);
  assert.equal(b64(message), request.message);
  assert.equal(b64(await sign(Context.Request, message, await signingKey(request.seed))), request.signature);
});

test("session claims: the sealed envelope matches byte for byte", async () => {
  const { claim: fields, device, seed, envelope } = vectors.claim;
  const browser = { id: device, signingKey: await signingKey(seed), signingPublic: await publicKey(seed) };
  assert.deepEqual(await claim(browser, fields.purpose, fields.value, fields.time), envelope);
});

test("padding lengths match", () => {
  for (const [length, padded] of vectors.padding) {
    assert.equal(padme(length), padded, String(length));
  }
});

test("key derivation and commitments match", async () => {
  const vector = vectors.derive;
  for (const derived of vector.derived) {
    assert.equal(b64(await derive(unb64(vector.ikm), vector.store, unb64(derived.info))), derived.okm);
  }
  assert.equal(b64(await namesCommitment(vector.store, 7, unb64(vector.names_key))), vector.names_commitment);
  assert.equal(b64(await dataCommitment(vector.store, 7, unb64(vector.names_key))), vector.data_commitment);
});

test("HPKE: a wrapping from the same ephemeral key matches byte for byte and opens", async () => {
  const vector = vectors.hpke;
  const wrapped = await wrapNames(vector.store, vector.epoch, vector.device, unb64(vector.public), unb64(vector.names_key), unb64(vector.ikm));
  assert.equal(b64(wrapped), vector.wrapped);
  const privateKey = await importPrivate(unb64(vector.private));
  const names = await unwrapNames(vector.store, vector.epoch, vector.device, privateKey, unb64(vector.public), unb64(vector.wrapped));
  assert.equal(b64(names), vector.names_key);
});

test("fingerprints and the recovery identifier match", async () => {
  const vector = vectors.fingerprints;
  const record = parseRecord(vector.device);
  assert.equal(await fingerprint(record.kind, record.recipient, record.signingKey), vector.fingerprint);
  assert.equal(orgFingerprint(unb64(vector.genesis)), vector.org_fingerprint);
  assert.equal(await recoveryId(vector.recovery), vector.recovery_id);
});
