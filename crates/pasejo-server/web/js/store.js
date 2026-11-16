// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// Index versions of a store: their place in the store's chain and their
// sealed contents, which the names key opens.

import { b64, be64, concat, fromUtf8, idBytes, isId, sha256, utf8 } from "./encoding.js";
import { derive, openPadded } from "./crypto.js";
import { ProtocolError } from "./log.js";

/**
 * Checks an index version's place in the store's chain against the last
 * version this browser saw (`pin`: `{version, hash}` or null).
 */
export async function checkChain(envelope, payload, store, pin) {
  if (payload.store !== store) {
    throw new ProtocolError("index of another store");
  }
  if (payload.version === 0 || (payload.version === 1) !== (payload.prev === null)) {
    throw new ProtocolError("index version");
  }
  if (pin) {
    const hash = b64(await sha256(envelope.payload));
    if (payload.version < pin.version) {
      throw new ProtocolError("index went backwards");
    }
    if (payload.version === pin.version && hash !== pin.hash) {
      throw new ProtocolError("index forked");
    }
    if (payload.version === pin.version + 1 && payload.prev !== pin.hash) {
      throw new ProtocolError("index forked");
    }
  }
}

function entries(value) {
  const result = new Map();
  for (const [name, entry] of Object.entries(value ?? {})) {
    if (!isId(entry?.id) || !Number.isSafeInteger(entry.version)) {
      throw new ProtocolError("json");
    }
    result.set(name, { id: entry.id, version: entry.version, hash: entry.hash });
  }
  return result;
}

/** Opens an index version's sealed contents with the epoch's names key. */
export async function openIndex(store, epoch, version, names, sealed) {
  const key = await derive(names, store, utf8("pasejo index v1"));
  const aad = concat(utf8("pasejo index v1"), idBytes(store), be64(epoch), be64(version));
  const plain = JSON.parse(fromUtf8(await openPadded(key, sealed, aad)));
  if (typeof plain?.label !== "string") {
    throw new ProtocolError("json");
  }
  return {
    label: plain.label,
    settings: plain.settings,
    secrets: entries(plain.secrets),
    otp: entries(plain.otp),
  };
}

/** Names of the objects an index lists, by object identifier. */
export function objectNames(index) {
  const names = new Map();
  for (const [name, entry] of index.secrets) {
    names.set(entry.id, { name, kind: "secret" });
  }
  for (const [name, entry] of index.otp) {
    names.set(entry.id, { name, kind: "otp" });
  }
  return names;
}

