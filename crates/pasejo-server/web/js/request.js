// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// Request proofs: every authenticated request is signed with the device key
// over its method, path, time, nonce, body hash and token hash.

import { b64, random, sha256, utf8 } from "./encoding.js";
import { Context, sign } from "./envelope.js";

export const AUTHORIZATION_SCHEME = "Pasejo";

/** The bytes a request proof signs. */
export async function proofMessage(method, pathAndQuery, timestamp, nonce, body, token) {
  return utf8(
    [
      method.toUpperCase(),
      pathAndQuery,
      String(timestamp),
      nonce,
      b64(await sha256(body)),
      b64(await sha256(utf8(token))),
    ].join("\n"),
  );
}

/** The headers that authenticate a request. */
export async function prove(method, pathAndQuery, timestamp, body, token, privateKey, nonce = b64(random(16))) {
  const signature = await sign(
    Context.Request,
    await proofMessage(method, pathAndQuery, timestamp, nonce, body, token),
    privateKey,
  );
  return {
    authorization: `${AUTHORIZATION_SCHEME} ${token}`,
    "pasejo-timestamp": String(timestamp),
    "pasejo-nonce": nonce,
    "pasejo-proof": b64(signature),
  };
}
