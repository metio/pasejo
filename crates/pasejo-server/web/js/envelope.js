// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// Signed envelopes: a JSON payload, kept as the bytes that were signed, with
// Ed25519 signatures under a context label.

import { EncodingError, b64, concat, fromUtf8, isId, unb64, unb64Fixed, utf8 } from "./encoding.js";

export const Context = Object.freeze({
  LogEntry: "log-entry/v1",
  Index: "index/v1",
  Request: "request/v1",
  Token: "token/v1",
  Checkpoint: "checkpoint/v1",
});

/** The bytes a signature covers: `pasejo:<label>\0<payload>`. */
export function message(context, payload) {
  return concat(utf8("pasejo:"), utf8(context), new Uint8Array([0]), payload);
}

// Encodings of the points of small order, with the sign bit cleared. A key
// or signature point among them would let one signature verify for many
// messages, so they are refused, as ed25519-dalek's verify_strict refuses
// them.
const SMALL_ORDER = [
  "0000000000000000000000000000000000000000000000000000000000000000",
  "0100000000000000000000000000000000000000000000000000000000000000",
  "26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc05",
  "c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac037a",
  "ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
  "edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
  "eeffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
];

function smallOrder(point) {
  let text = "";
  for (let index = 0; index < 32; index++) {
    const byte = index === 31 ? point[index] & 0x7f : point[index];
    text += byte.toString(16).padStart(2, "0");
  }
  return SMALL_ORDER.includes(text);
}

// The group order L, little-endian. A signature's scalar must lie below it.
const ORDER = [
  0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10,
];

function canonicalScalar(scalar) {
  for (let index = 31; index >= 0; index--) {
    if (scalar[index] < ORDER[index]) {
      return true;
    }
    if (scalar[index] > ORDER[index]) {
      return false;
    }
  }
  return false;
}

/** Whether `signature` by `key` covers `payload` under `context`. */
export async function verify(context, payload, key, signature) {
  if (key.length !== 32 || signature.length !== 64) {
    return false;
  }
  if (smallOrder(key) || smallOrder(signature.subarray(0, 32))) {
    return false;
  }
  if (!canonicalScalar(signature.subarray(32))) {
    return false;
  }
  try {
    const publicKey = await globalThis.crypto.subtle.importKey(
      "raw",
      key,
      { name: "Ed25519" },
      false,
      ["verify"],
    );
    return await globalThis.crypto.subtle.verify(
      { name: "Ed25519" },
      publicKey,
      signature,
      message(context, payload),
    );
  } catch {
    return false;
  }
}

/** Signs `payload` under `context` with a private Ed25519 CryptoKey. */
export async function sign(context, payload, privateKey) {
  return new Uint8Array(
    await globalThis.crypto.subtle.sign({ name: "Ed25519" }, privateKey, message(context, payload)),
  );
}

/** Parses an envelope from its JSON form, keeping the payload bytes. */
export function parseEnvelope(value) {
  if (typeof value !== "object" || value === null || !Array.isArray(value.signatures)) {
    throw new EncodingError("envelope");
  }
  return {
    payload: unb64(value.payload),
    signatures: value.signatures.map((signature) => {
      if (typeof signature !== "object" || signature === null || !isId(signature.device)) {
        throw new EncodingError("signature");
      }
      return {
        device: signature.device,
        signature: unb64Fixed(signature.signature, 64, "signature"),
      };
    }),
  };
}

export function envelopeJson(envelope) {
  return {
    payload: b64(envelope.payload),
    signatures: envelope.signatures.map((signature) => ({
      device: signature.device,
      signature: b64(signature.signature),
    })),
  };
}

/** Signs `payload` (serialized as JSON) as `device`. */
export async function seal(context, payload, device, privateKey) {
  const bytes = utf8(JSON.stringify(payload));
  return {
    payload: bytes,
    signatures: [{ device, signature: await sign(context, bytes, privateKey) }],
  };
}

/** Adds the signature of `device` to an envelope. */
export async function addSignature(envelope, context, device, privateKey) {
  return {
    payload: envelope.payload,
    signatures: [
      ...envelope.signatures,
      { device, signature: await sign(context, envelope.payload, privateKey) },
    ],
  };
}

/** The payload, parsed as JSON. */
export function open(envelope) {
  try {
    return JSON.parse(fromUtf8(envelope.payload));
  } catch {
    throw new EncodingError("JSON");
  }
}

/** Verifies the signature of `device`, which must be among the signers. */
export async function verifyBy(envelope, context, device, key) {
  const found = envelope.signatures.find((signature) => signature.device === device);
  return found !== undefined && (await verify(context, envelope.payload, key, found.signature));
}
