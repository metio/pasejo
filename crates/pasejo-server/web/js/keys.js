// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// The names keys of a store as a browser device sees them: its own wrapping
// of the current epoch, then the chain backwards, each key checked against
// the commitment its signed epoch entry records.

import { equal, unb64 } from "./encoding.js";
import { namesCommitment, openPreviousNames } from "./crypto.js";
import { ProtocolError } from "./log.js";
import { unwrapNames } from "./wrapping.js";

async function check(log, store, epoch, names) {
  const record = log.epoch(store, epoch);
  if (!record) {
    throw new ProtocolError(`epoch ${epoch} is not in the log`);
  }
  if (!equal(await namesCommitment(store, epoch, names), unb64(record.namesCommit))) {
    throw new ProtocolError("unwrapped key does not match the commitment in the membership log");
  }
}

/**
 * Opens the names keys of `store` from a keys response. Returns a map from
 * epoch to names key, reaching back as far as the chain does.
 */
export async function openNames(log, store, device, keys, privateKey, publicKey) {
  const epochs = log.state.stores.get(store)?.epochs.length ?? 0;
  if (epochs === 0) {
    throw new ProtocolError("the store has no keys yet");
  }
  const current = epochs - 1;
  const wrapping = keys.wrappings.find((candidate) => candidate.epoch === current && candidate.device === device);
  if (!wrapping) {
    throw new ProtocolError("this device holds no wrapping of the store's keys");
  }
  let names = await unwrapNames(store, current, device, privateKey, publicKey, unb64(wrapping.ciphertext));
  await check(log, store, current, names);
  const ring = new Map([[current, names]]);
  for (let epoch = current; epoch > 0; epoch--) {
    const link = keys.chain.find((candidate) => candidate.epoch === epoch);
    if (!link) {
      break;
    }
    names = await openPreviousNames(store, epoch, names, unb64(link.prev_names));
    await check(log, store, epoch - 1, names);
    ring.set(epoch - 1, names);
  }
  return ring;
}
