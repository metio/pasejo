// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// HPKE (RFC 9180) in base mode with DHKEM(X25519, HKDF-SHA256),
// HKDF-SHA256 and AES-256-GCM, built from WebCrypto's X25519, HMAC and
// AES-GCM so the console loads no third-party code.

import { concat, utf8 } from "./encoding.js";

const subtle = () => globalThis.crypto.subtle;

const KEM_SUITE = concat(utf8("KEM"), new Uint8Array([0x00, 0x20]));
const HPKE_SUITE = concat(utf8("HPKE"), new Uint8Array([0x00, 0x20, 0x00, 0x01, 0x00, 0x02]));
const VERSION = utf8("HPKE-v1");
const HASH_LEN = 32;

async function hmac(key, data) {
  const material = key.length === 0 ? new Uint8Array(HASH_LEN) : key;
  const imported = await subtle().importKey("raw", material, { name: "HMAC", hash: "SHA-256" }, false, [
    "sign",
  ]);
  return new Uint8Array(await subtle().sign("HMAC", imported, data));
}

async function expand(prk, info, length) {
  const blocks = [];
  let previous = new Uint8Array(0);
  for (let counter = 1; blocks.length * HASH_LEN < length; counter++) {
    previous = await hmac(prk, concat(previous, info, new Uint8Array([counter])));
    blocks.push(previous);
  }
  return concat(...blocks).slice(0, length);
}

function labeledExtract(suite, salt, label, ikm) {
  return hmac(salt, concat(VERSION, suite, utf8(label), ikm));
}

function labeledExpand(suite, prk, label, info, length) {
  const prefix = new Uint8Array([length >> 8, length & 0xff]);
  return expand(prk, concat(prefix, VERSION, suite, utf8(label), info), length);
}

const PKCS8_X25519 = Uint8Array.from([
  0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x6e, 0x04, 0x22, 0x04, 0x20,
]);

/** Imports a raw X25519 private key, for tests and deterministic encapsulation. */
export function importPrivate(raw, extractable = false) {
  return subtle().importKey("pkcs8", concat(PKCS8_X25519, raw), { name: "X25519" }, extractable, [
    "deriveBits",
  ]);
}

async function dh(privateKey, publicRaw) {
  const publicKey = await subtle().importKey("raw", publicRaw, { name: "X25519" }, false, []);
  return new Uint8Array(await subtle().deriveBits({ name: "X25519", public: publicKey }, privateKey, 256));
}

async function sharedSecret(dhResult, enc, recipientPublic) {
  const prk = await labeledExtract(KEM_SUITE, new Uint8Array(0), "eae_prk", dhResult);
  return labeledExpand(KEM_SUITE, prk, "shared_secret", concat(enc, recipientPublic), 32);
}

async function schedule(secret, info) {
  const empty = new Uint8Array(0);
  const pskIdHash = await labeledExtract(HPKE_SUITE, empty, "psk_id_hash", empty);
  const infoHash = await labeledExtract(HPKE_SUITE, empty, "info_hash", info);
  const context = concat(new Uint8Array([0]), pskIdHash, infoHash);
  const prk = await labeledExtract(HPKE_SUITE, secret, "secret", empty);
  return {
    key: await labeledExpand(HPKE_SUITE, prk, "key", context, 32),
    nonce: await labeledExpand(HPKE_SUITE, prk, "base_nonce", context, 12),
  };
}

const BASE_POINT = Uint8Array.from({ length: 32 }, (_, index) => (index === 0 ? 9 : 0));

/** The public key of an X25519 private key: its product with the base point. */
export function publicOf(privateKey) {
  return dh(privateKey, BASE_POINT);
}

/** DeriveKeyPair of RFC 9180 section 7.1.3 for X25519. */
export async function deriveKeyPair(ikm) {
  const prk = await labeledExtract(KEM_SUITE, new Uint8Array(0), "dkp_prk", ikm);
  const raw = await labeledExpand(KEM_SUITE, prk, "sk", new Uint8Array(0), 32);
  const privateKey = await importPrivate(raw);
  return { privateKey, publicKey: await publicOf(privateKey) };
}

/**
 * Seals `plaintext` to `recipientPublic`; the ephemeral key comes from `ikm`
 * (32 random bytes unless given). Returns the encapsulated key and the
 * ciphertext.
 */
export async function seal(recipientPublic, info, aad, plaintext, ikm = globalThis.crypto.getRandomValues(new Uint8Array(32))) {
  const ephemeral = await deriveKeyPair(ikm);
  const enc = ephemeral.publicKey;
  const secret = await sharedSecret(await dh(ephemeral.privateKey, recipientPublic), enc, recipientPublic);
  const { key, nonce } = await schedule(secret, info);
  const aes = await subtle().importKey("raw", key, { name: "AES-GCM" }, false, ["encrypt"]);
  const ciphertext = await subtle().encrypt({ name: "AES-GCM", iv: nonce, additionalData: aad }, aes, plaintext);
  return { enc, ciphertext: new Uint8Array(ciphertext) };
}

/** Opens what `seal` produced with the recipient's private CryptoKey. */
export async function open(privateKey, recipientPublic, enc, info, aad, ciphertext) {
  const secret = await sharedSecret(await dh(privateKey, enc), enc, recipientPublic);
  const { key, nonce } = await schedule(secret, info);
  const aes = await subtle().importKey("raw", key, { name: "AES-GCM" }, false, ["decrypt"]);
  return new Uint8Array(
    await subtle().decrypt({ name: "AES-GCM", iv: nonce, additionalData: aad }, aes, ciphertext),
  );
}
