// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

import { readFileSync } from "node:fs";
import { concat, unb64 } from "../js/encoding.js";

const VECTORS = new URL("../../../pasejo-protocol/vectors/console.json", import.meta.url);

/** The vectors the Rust test in crates/pasejo-protocol/tests/vectors.rs verifies too. */
export const vectors = JSON.parse(readFileSync(VECTORS, "utf8"));

const PKCS8_ED25519 = Uint8Array.from([
  0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20,
]);

/** An Ed25519 private CryptoKey from a 32-byte seed. */
export function signingKey(seed) {
  const bytes = typeof seed === "string" ? unb64(seed) : seed;
  return globalThis.crypto.subtle.importKey("pkcs8", concat(PKCS8_ED25519, bytes), { name: "Ed25519" }, false, [
    "sign",
  ]);
}

/** The raw public key of a seed. */
export async function publicKey(seed) {
  const bytes = typeof seed === "string" ? unb64(seed) : seed;
  const key = await globalThis.crypto.subtle.importKey(
    "pkcs8",
    concat(PKCS8_ED25519, bytes),
    { name: "Ed25519" },
    true,
    ["sign"],
  );
  const jwk = await globalThis.crypto.subtle.exportKey("jwk", key);
  return unb64(jwk.x);
}

/** A key-value store over a Map, with the interface of kv.js. */
export function memoryStore() {
  const values = new Map();
  return {
    values,
    get: async (key) => values.get(key),
    put: async (key, value) => void values.set(key, value),
    delete: async (key) => void values.delete(key),
  };
}

/**
 * A fetch that routes requests to `handlers` keyed by "METHOD /path" (the
 * path without its query) and records every request.
 */
export function fakeFetch(handlers) {
  const requests = [];
  const fetch = async (url, init = {}) => {
    const parsed = new URL(url, "http://console.test");
    const method = init.method ?? "GET";
    const body = init.body ? new TextDecoder().decode(init.body) : "";
    const request = { method, path: parsed.pathname, query: parsed.search, headers: init.headers ?? {}, body };
    requests.push(request);
    const handler = handlers[`${method} ${parsed.pathname}`];
    if (!handler) {
      return new Response(JSON.stringify({ code: "not_found", message: `no handler for ${method} ${parsed.pathname}` }), {
        status: 404,
      });
    }
    const answer = await handler(request);
    if (answer instanceof Response) {
      return answer;
    }
    return new Response(answer === undefined ? "" : JSON.stringify(answer), { status: 200 });
  };
  return { fetch, requests };
}

export function errorResponse(status, code, message = code) {
  return new Response(JSON.stringify({ code, message }), { status });
}
