// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// The audit chain: each event's hash covers its predecessor's hash and its
// fields, and checkpoints carry the server's signature and co-signatures of
// devices.

import { concat, fromUtf8, sha256, unb64, utf8, b64, unb64Fixed } from "./encoding.js";
import { Context, sign, verify } from "./envelope.js";
import { ProtocolError } from "./log.js";

/** The hash of an event, as the server computes it. */
export async function eventHash(event) {
  const fields = {
    seq: event.seq,
    time: event.time,
    org: event.org,
    actor_user: event.actor_user,
    actor_device: event.actor_device,
    session_kind: event.session_kind,
    ip: event.ip,
    user_agent: event.user_agent,
    action: event.action,
    store: event.store,
    object: event.object,
    version: event.version,
    outcome: event.outcome,
    detail: event.detail,
  };
  return b64(await sha256(concat(unb64(event.prev_hash), utf8(JSON.stringify(fields)))));
}

/**
 * Verifies that `events` continue the chain from `start` at `firstSeq` and
 * returns the hash of the last one.
 */
export async function verifyChain(start, firstSeq, events) {
  let previous = start;
  for (const [offset, event] of events.entries()) {
    if (event.seq !== firstSeq + offset) {
      throw new ProtocolError("audit sequence has a gap");
    }
    if (event.prev_hash !== previous) {
      throw new ProtocolError("audit chain is broken");
    }
    if ((await eventHash(event)) !== event.hash) {
      throw new ProtocolError("audit event was altered");
    }
    previous = event.hash;
  }
  return previous;
}

/** Verifies a checkpoint's server signature and returns its payload. */
export async function verifyCheckpoint(checkpoint, serverKey) {
  const payload = unb64(checkpoint.payload);
  const signature = unb64Fixed(checkpoint.server_signature, 64, "signature");
  if (!(await verify(Context.Checkpoint, payload, serverKey, signature))) {
    throw new ProtocolError("the checkpoint is not signed by the server");
  }
  return JSON.parse(fromUtf8(payload));
}

/** Whether `device` co-signed the checkpoint with `key`. */
export async function verifyCosignature(checkpoint, device, key) {
  const found = checkpoint.cosignatures.find((signature) => signature.device === device);
  return (
    found !== undefined &&
    (await verify(Context.Checkpoint, unb64(checkpoint.payload), key, unb64Fixed(found.signature, 64, "signature")))
  );
}

/** The co-signature of `device` for a checkpoint. */
export async function cosign(checkpoint, device, privateKey) {
  return { device, signature: b64(await sign(Context.Checkpoint, unb64(checkpoint.payload), privateKey)) };
}
