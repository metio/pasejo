// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// Wrappings of a store's epoch keys for browser devices. A browser device
// receives the names key only; a wrapping that carries a data key is
// refused, so script in the console never holds one.

import { b64, be64, concat, fromUtf8, idBytes, unb64, utf8 } from "./encoding.js";
import { DecryptionError } from "./crypto.js";
import * as hpke from "./hpke.js";

const INFO = utf8("pasejo wrapping v1");
const ENCAPPED_LEN = 32;

function aad(store, epoch, device) {
  return concat(idBytes(store), be64(epoch), idBytes(device));
}

/** Opens a names-only wrapping and returns the names key. */
export async function unwrapNames(store, epoch, device, privateKey, publicKey, wrapped) {
  if (wrapped.length < ENCAPPED_LEN) {
    throw new DecryptionError();
  }
  let plain;
  try {
    const bytes = await hpke.open(
      privateKey,
      publicKey,
      wrapped.subarray(0, ENCAPPED_LEN),
      INFO,
      aad(store, epoch, device),
      wrapped.subarray(ENCAPPED_LEN),
    );
    plain = JSON.parse(fromUtf8(bytes));
  } catch {
    throw new DecryptionError();
  }
  if (plain?.store !== store || plain.epoch !== epoch) {
    throw new DecryptionError();
  }
  if (plain.data_key !== null && plain.data_key !== undefined) {
    throw new DecryptionError();
  }
  let names;
  try {
    names = unb64(plain.names_key);
  } catch {
    throw new DecryptionError();
  }
  if (names.length !== 32) {
    throw new DecryptionError();
  }
  return names;
}

/** Wraps a names key for a browser device. */
export async function wrapNames(store, epoch, device, publicKey, names, ikm) {
  const plaintext = utf8(JSON.stringify({ store, epoch, names_key: b64(names), data_key: null }));
  const { enc, ciphertext } = await hpke.seal(publicKey, INFO, aad(store, epoch, device), plaintext, ikm);
  return concat(enc, ciphertext);
}
