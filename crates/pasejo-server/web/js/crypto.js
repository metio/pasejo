// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// Symmetric cryptography of stores: HKDF-SHA256 key derivation, AES-256-GCM
// with the nonce in front, Padme padding and the key commitments of epochs.

import { EncodingError, be64, concat, idBytes, random, sha256, utf8 } from "./encoding.js";

const subtle = () => globalThis.crypto.subtle;

export const NONCE_LEN = 12;

export class DecryptionError extends Error {
  constructor() {
    super("decryption failed");
    this.name = "DecryptionError";
  }
}

/** HKDF-SHA256 with the store identifier as salt. */
export async function derive(ikm, store, info) {
  const key = await subtle().importKey("raw", ikm, "HKDF", false, ["deriveBits"]);
  return new Uint8Array(
    await subtle().deriveBits({ name: "HKDF", hash: "SHA-256", salt: idBytes(store), info }, key, 256),
  );
}

async function aesKey(key, usage) {
  return subtle().importKey("raw", key, { name: "AES-GCM" }, false, [usage]);
}

/** AES-256-GCM; the result is the nonce followed by ciphertext and tag. */
export async function seal(key, plaintext, aad, nonce = random(NONCE_LEN)) {
  const ciphertext = await subtle().encrypt(
    { name: "AES-GCM", iv: nonce, additionalData: aad },
    await aesKey(key, "encrypt"),
    plaintext,
  );
  return concat(nonce, new Uint8Array(ciphertext));
}

export async function open(key, sealed, aad) {
  if (sealed.length < NONCE_LEN) {
    throw new DecryptionError();
  }
  try {
    return new Uint8Array(
      await subtle().decrypt(
        { name: "AES-GCM", iv: sealed.subarray(0, NONCE_LEN), additionalData: aad },
        await aesKey(key, "decrypt"),
        sealed.subarray(NONCE_LEN),
      ),
    );
  } catch {
    throw new DecryptionError();
  }
}

/** The Padme length for `length` bytes, which leaks O(log log length) bits. */
export function padme(length) {
  if (length < 2) {
    return length;
  }
  const exponent = Math.floor(Math.log2(length));
  const adjusted = 2 ** exponent > length ? exponent - 1 : exponent;
  const bits = Math.floor(Math.log2(adjusted)) + 1;
  const mask = 2 ** (adjusted - bits) - 1;
  const total = length + mask;
  return total - (total % (mask + 1));
}

/** The smallest padded plaintext, so typical secrets all look the same size. */
export const MIN_PADDED = 1024;

/** The padded length for `length` bytes: the floor or the Padme length. */
export function paddedLength(length) {
  return Math.max(MIN_PADDED, padme(length));
}

/** A 4-byte big-endian length, the plaintext and zeros up to the padded length. */
export function pad(plaintext) {
  if (plaintext.length > 0xffffffff) {
    throw new EncodingError("padding");
  }
  const padded = new Uint8Array(paddedLength(plaintext.length + 4));
  new DataView(padded.buffer).setUint32(0, plaintext.length);
  padded.set(plaintext, 4);
  return padded;
}

export function unpad(padded) {
  if (padded.length < 4) {
    throw new EncodingError("padding");
  }
  const length = new DataView(padded.buffer, padded.byteOffset, 4).getUint32(0);
  if (length > padded.length - 4 || paddedLength(length + 4) !== padded.length) {
    throw new EncodingError("padding");
  }
  if (padded.subarray(4 + length).some((byte) => byte !== 0)) {
    throw new EncodingError("padding");
  }
  return padded.slice(4, 4 + length);
}

export async function sealPadded(key, plaintext, aad, nonce) {
  return seal(key, pad(plaintext), aad, nonce);
}

export async function openPadded(key, sealed, aad) {
  return unpad(await open(key, sealed, aad));
}

async function commitment(label, store, epoch, key) {
  return sha256(concat(utf8(label), idBytes(store), be64(epoch), key));
}

export function namesCommitment(store, epoch, key) {
  return commitment("pasejo names commit v1", store, epoch, key);
}

export function dataCommitment(store, epoch, key) {
  return commitment("pasejo data commit v1", store, epoch, key);
}

/**
 * Opens the names key of epoch `epoch - 1` from the chain link of `epoch`.
 * Browser devices hold names keys only, so the data half stays sealed.
 */
export async function openPreviousNames(store, epoch, names, prevNames) {
  const aad = concat(utf8("pasejo previous v1"), idBytes(store), be64(epoch));
  const key = await derive(names, store, utf8("pasejo previous names v1"));
  const previous = await open(key, prevNames, aad);
  if (previous.length !== 32) {
    throw new DecryptionError();
  }
  return previous;
}
