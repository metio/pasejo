// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// The browser's device keys: an Ed25519 key that signs requests and log
// entries, and an X25519 key names-only wrappings are sealed to. Both
// private keys are generated non-extractable, so script can use them but
// never read them, and they are kept in IndexedDB as CryptoKey objects.

import { b64, randomId } from "./encoding.js";
import { fingerprint } from "./device.js";
import { Context, seal, envelopeJson } from "./envelope.js";

const subtle = () => globalThis.crypto.subtle;

export async function createDevice(label) {
  const signing = await subtle().generateKey({ name: "Ed25519" }, false, ["sign", "verify"]);
  const hpke = await subtle().generateKey({ name: "X25519" }, false, ["deriveBits"]);
  return {
    id: randomId(),
    label,
    signingKey: signing.privateKey,
    signingPublic: new Uint8Array(await subtle().exportKey("raw", signing.publicKey)),
    hpkeKey: hpke.privateKey,
    hpkePublic: new Uint8Array(await subtle().exportKey("raw", hpke.publicKey)),
  };
}

/** The device kept in `store` under `name`, created on first use. */
export async function loadOrCreateDevice(store, label, name = "device") {
  const existing = await store.get(name);
  if (existing) {
    return existing;
  }
  const device = await createDevice(label);
  await store.put(name, device);
  return device;
}

export function enrollmentRequest(device) {
  return { id: device.id, kind: "hpke", recipient: b64(device.hpkePublic), label: device.label };
}

export function deviceFingerprint(device) {
  return fingerprint("hpke", b64(device.hpkePublic), device.signingPublic);
}

/** A signed session claim for `purpose`. */
export async function claim(device, purpose, value, time) {
  const payload = { purpose, value, signing_key: b64(device.signingPublic), time };
  return envelopeJson(await seal(Context.Token, payload, device.id, device.signingKey));
}

/** A label naming the browser, such as "Firefox". */
export function browserName(userAgent) {
  const names = [
    [/Edg\//, "Edge"],
    [/OPR\//, "Opera"],
    [/Vivaldi\//, "Vivaldi"],
    [/Firefox\//, "Firefox"],
    [/Chrome\//, "Chrome"],
    [/Safari\//, "Safari"],
  ];
  return names.find(([pattern]) => pattern.test(userAgent))?.[1] ?? "Browser";
}
