// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// Refusal paths of the protocol layer: what the console must not accept.

import assert from "node:assert/strict";
import { describe, test } from "node:test";
import { cosign, verifyChain, verifyCheckpoint, verifyCosignature } from "../js/audit.js";
import { DecryptionError, open, openPadded, pad, seal, sealPadded, unpad } from "../js/crypto.js";
import { isOnCurve } from "../js/curve.js";
import { blank, fingerprintMatches, hasControl, parseRecord, validName, validRecipient } from "../js/device.js";
import { b64, concat, unb64 } from "../js/encoding.js";
import { Context, addSignature, open as openEnvelope, parseEnvelope, seal as sealEnvelope, verify, verifyBy } from "../js/envelope.js";
import * as hpke from "../js/hpke.js";
import { unwrapNames, wrapNames } from "../js/wrapping.js";
import { publicKey, signingKey, vectors } from "./helpers.js";

const seed = (byte) => new Uint8Array(32).fill(byte);
const payload = new TextEncoder().encode("payload");

describe("signatures", () => {
  test("verify under the right context only", async () => {
    const key = await signingKey(seed(1));
    const envelope = await sealEnvelope(Context.LogEntry, { a: 1 }, "1".repeat(32), key);
    const raw = await publicKey(seed(1));
    assert.equal(await verifyBy(envelope, Context.LogEntry, "1".repeat(32), raw), true);
    assert.equal(await verifyBy(envelope, Context.Index, "1".repeat(32), raw), false);
    assert.equal(await verifyBy(envelope, Context.LogEntry, "2".repeat(32), raw), false);
    assert.equal(await verifyBy(envelope, Context.LogEntry, "1".repeat(32), await publicKey(seed(2))), false);
  });

  test("refuse a non-canonical scalar", async () => {
    const key = await signingKey(seed(3));
    const envelope = await sealEnvelope(Context.Token, { x: 1 }, "1".repeat(32), key);
    const signature = Uint8Array.from(envelope.signatures[0].signature);
    // Adding the group order L to S leaves the equation intact but makes S non-canonical.
    const order = [0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14];
    let carry = 0;
    for (let index = 0; index < 32; index++) {
      const sum = signature[32 + index] + (order[index] ?? 0) + (index === 31 ? 0x10 : 0) + carry;
      signature[32 + index] = sum & 0xff;
      carry = sum >> 8;
    }
    assert.equal(await verify(Context.Token, envelope.payload, await publicKey(seed(3)), signature), false);
  });

  test("refuse small-order keys and R points, and wrong lengths", async () => {
    const identity = new Uint8Array(32);
    identity[0] = 1;
    const signature = new Uint8Array(64);
    signature.set(identity, 0);
    assert.equal(await verify(Context.Token, payload, identity, signature), false);
    const realKey = await publicKey(seed(4));
    assert.equal(await verify(Context.Token, payload, realKey, signature), false);
    assert.equal(await verify(Context.Token, payload, realKey.subarray(1), new Uint8Array(64)), false);
    assert.equal(await verify(Context.Token, payload, realKey, new Uint8Array(63)), false);
  });

  test("a second signature leaves the payload bytes untouched", async () => {
    const first = await sealEnvelope(Context.LogEntry, { b: 2, a: 1 }, "1".repeat(32), await signingKey(seed(5)));
    const both = await addSignature(first, Context.LogEntry, "2".repeat(32), await signingKey(seed(6)));
    assert.equal(b64(both.payload), b64(first.payload));
    assert.equal(both.signatures.length, 2);
    assert.deepEqual(openEnvelope(both), { b: 2, a: 1 });
  });

  test("envelopes with malformed fields are refused", () => {
    assert.throws(() => parseEnvelope(null));
    assert.throws(() => parseEnvelope({ payload: "e30", signatures: {} }));
    assert.throws(() => parseEnvelope({ payload: "e30=", signatures: [] }));
    assert.throws(() => parseEnvelope({ payload: "e30", signatures: [{ device: "X".repeat(32), signature: b64(new Uint8Array(64)) }] }));
    assert.throws(() => parseEnvelope({ payload: "e30", signatures: [{ device: "1".repeat(32), signature: b64(new Uint8Array(63)) }] }));
    assert.throws(() => openEnvelope({ payload: new Uint8Array([0xff]), signatures: [] }));
  });
});

describe("device records", () => {
  test("public keys must lie on the curve", async () => {
    assert.equal(isOnCurve(await publicKey(seed(7))), true);
    const notOnCurve = new Uint8Array(32);
    notOnCurve[0] = 2;
    assert.equal(isOnCurve(notOnCurve), false);
    assert.equal(isOnCurve(new Uint8Array(31)), false);
  });

  test("names follow Rust's whitespace and control rules", () => {
    assert.equal(blank(" \t\u{3000}"), true);
    assert.equal(blank("\u{feff}"), false);
    assert.equal(hasControl("a\x85b"), true);
    assert.equal(validName("ops"), true);
    assert.equal(validName("   "), false);
    assert.equal(validName("a\nb"), false);
    assert.equal(validName("é".repeat(100)), true);
    assert.equal(validName("é".repeat(101)), false);
  });

  test("recipients match their kind", () => {
    assert.equal(validRecipient("hpke", b64(new Uint8Array(32))), true);
    assert.equal(validRecipient("hpke", b64(new Uint8Array(31))), false);
    assert.equal(validRecipient("age", "age1abc"), true);
    assert.equal(validRecipient("age", "ssh-ed25519 AAAA"), true);
    assert.equal(validRecipient("age", "pgp"), false);
    assert.equal(validRecipient("age", "age1\u{e9}"), false);
    assert.equal(validRecipient("ssh", "age1abc"), false);
  });

  test("records with malformed fields are refused", () => {
    const record = { id: "1".repeat(32), user: "2".repeat(32), kind: "hpke", recipient: "x", signing_key: b64(new Uint8Array(32)), label: "l" };
    assert.equal(parseRecord(record).signingKey.length, 32);
    for (const change of [{ id: "1" }, { user: null }, { kind: "pgp" }, { recipient: 1 }, { label: null }, { signing_key: b64(new Uint8Array(31)) }]) {
      assert.throws(() => parseRecord({ ...record, ...change }), JSON.stringify(change));
    }
  });

  test("typed fingerprints ignore spacing and case, never match empty", () => {
    assert.equal(fingerprintMatches(" ABCD 1234 ", "abcd1234"), true);
    assert.equal(fingerprintMatches("abcd1235", "abcd1234"), false);
    assert.equal(fingerprintMatches("   ", ""), false);
  });
});

describe("symmetric encryption", () => {
  const key = new Uint8Array(32).fill(8);
  const aad = new Uint8Array([1, 2, 3]);

  test("opens what it sealed and refuses changes", async () => {
    const sealed = await seal(key, payload, aad);
    assert.deepEqual(await open(key, sealed, aad), payload);
    await assert.rejects(open(key, sealed, new Uint8Array([1, 2])), DecryptionError);
    const flipped = Uint8Array.from(sealed);
    flipped[flipped.length - 1] ^= 1;
    await assert.rejects(open(key, flipped, aad), DecryptionError);
    await assert.rejects(open(new Uint8Array(32), sealed, aad), DecryptionError);
    await assert.rejects(open(key, sealed.subarray(0, 5), aad), DecryptionError);
  });

  test("padding round-trips and refuses malformed padding", async () => {
    for (const length of [0, 1, 31, 32, 33, 1000]) {
      const plain = new Uint8Array(length).fill(5);
      assert.deepEqual(unpad(pad(plain)), plain);
    }
    assert.throws(() => unpad(new Uint8Array(3)));
    const sealed = await sealPadded(key, payload, aad);
    assert.deepEqual(await openPadded(key, sealed, aad), payload);
  });
});

describe("browser wrappings", () => {
  const store = "5".repeat(32);
  const device = "6".repeat(32);
  const names = new Uint8Array(32).fill(9);

  async function keys() {
    const { privateKey, publicKey: raw } = await hpke.deriveKeyPair(new Uint8Array(32).fill(1));
    return { privateKey, raw };
  }

  test("open only for the store, epoch and device they were sealed for", async () => {
    const { privateKey, raw } = await keys();
    const wrapped = await wrapNames(store, 3, device, raw, names);
    assert.deepEqual(await unwrapNames(store, 3, device, privateKey, raw, wrapped), names);
    await assert.rejects(unwrapNames(store, 2, device, privateKey, raw, wrapped), DecryptionError);
    await assert.rejects(unwrapNames("7".repeat(32), 3, device, privateKey, raw, wrapped), DecryptionError);
    await assert.rejects(unwrapNames(store, 3, "8".repeat(32), privateKey, raw, wrapped), DecryptionError);
    await assert.rejects(unwrapNames(store, 3, device, privateKey, raw, wrapped.subarray(0, 20)), DecryptionError);
  });

  async function sealedPlain(plain) {
    const { privateKey, raw } = await keys();
    const aad = concat(unb64(b64(Uint8Array.from(Buffer.from(store, "hex")))), new Uint8Array(8), Uint8Array.from(Buffer.from(device, "hex")));
    const { enc, ciphertext } = await hpke.seal(raw, new TextEncoder().encode("pasejo wrapping v1"), aad, new TextEncoder().encode(JSON.stringify(plain)));
    return { privateKey, raw, wrapped: concat(enc, ciphertext) };
  }

  test("refuse a wrapping that carries a data key", async () => {
    const { privateKey, raw, wrapped } = await sealedPlain({ store, epoch: 0, names_key: b64(names), data_key: b64(names) });
    await assert.rejects(unwrapNames(store, 0, device, privateKey, raw, wrapped), DecryptionError);
  });

  test("refuse a wrapping whose contents name another store or epoch", async () => {
    for (const plain of [
      { store: "9".repeat(32), epoch: 0, names_key: b64(names), data_key: null },
      { store, epoch: 1, names_key: b64(names), data_key: null },
      { store, epoch: 0, names_key: b64(new Uint8Array(16)), data_key: null },
      { store, epoch: 0, names_key: "!", data_key: null },
    ]) {
      const { privateKey, raw, wrapped } = await sealedPlain(plain);
      await assert.rejects(unwrapNames(store, 0, device, privateKey, raw, wrapped), DecryptionError, JSON.stringify(plain));
    }
  });
});

describe("audit checkpoints", () => {
  test("refuse a checkpoint not signed by the pinned server key", async () => {
    await assert.rejects(verifyCheckpoint(vectors.audit.checkpoint, await publicKey(seed(9))), /not signed by the server/);
    const payload = await verifyCheckpoint(vectors.audit.checkpoint, unb64(vectors.audit.server_key));
    assert.equal(payload.seq, vectors.audit.checkpoint_payload.seq);
  });

  test("co-signatures verify for their device only", async () => {
    const key = await signingKey(seed(10));
    const signature = await cosign(vectors.audit.checkpoint, "a".repeat(32), key);
    const checkpoint = { ...vectors.audit.checkpoint, cosignatures: [signature] };
    assert.equal(await verifyCosignature(checkpoint, "a".repeat(32), await publicKey(seed(10))), true);
    assert.equal(await verifyCosignature(checkpoint, "b".repeat(32), await publicKey(seed(10))), false);
    assert.equal(await verifyCosignature(checkpoint, "a".repeat(32), await publicKey(seed(11))), false);
  });

  test("chains refuse gaps, broken links and altered events", async () => {
    const events = vectors.audit.events;
    const start = vectors.audit.start;
    assert.equal(await verifyChain(start, 1, events), events.at(-1).hash);
    await assert.rejects(verifyChain(start, 2, events), /gap/);
    await assert.rejects(verifyChain(start, 1, [events[0], events[2]]), /gap/);
    await assert.rejects(verifyChain(b64(new Uint8Array(32).fill(1)), 1, events), /broken/);
    const altered = structuredClone(events);
    altered[1].outcome = "denied";
    await assert.rejects(verifyChain(start, 1, altered), /altered/);
  });
});
