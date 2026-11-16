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

/**
 * Verifies an audit trail: every checkpoint's server signature, `check` on
 * each payload, every co-signature against the key `cosigner` returns for
 * its device (undefined refuses it), and the hash chain of `events` from the
 * first kept one, which starts at the zero hash or at a checkpoint.
 */
export async function verifyTrail({ checkpoints: entries, events, serverKey, check, cosigner }) {
  const checkpoints = new Map();
  let cosignatures = 0;
  for (const entry of entries) {
    const payload = await verifyCheckpoint(entry.checkpoint, serverKey);
    check(entry.seq, payload);
    for (const signature of entry.checkpoint.cosignatures) {
      const key = cosigner(signature.device);
      if (!key) {
        throw new ProtocolError(`checkpoint ${entry.seq} carries a co-signature by an unknown device`);
      }
      if (!(await verifyCosignature(entry.checkpoint, signature.device, key))) {
        throw new ProtocolError(`a co-signature of checkpoint ${entry.seq} does not verify`);
      }
      cosignatures += 1;
    }
    checkpoints.set(entry.seq, payload);
  }
  if (events.length === 0) {
    return { checkpoints: checkpoints.size, cosignatures, matched: 0 };
  }
  const first = events[0].seq;
  const start = first === 1 ? b64(new Uint8Array(32)) : checkpoints.get(first - 1)?.hash;
  if (!start) {
    throw new ProtocolError("no checkpoint vouches for the first kept event");
  }
  await verifyChain(start, first, events);
  let matched = 0;
  for (const event of events) {
    const payload = checkpoints.get(event.seq);
    if (payload) {
      if (payload.hash !== event.hash) {
        throw new ProtocolError(`checkpoint ${event.seq} contradicts the events`);
      }
      matched += 1;
    }
  }
  return { checkpoints: checkpoints.size, cosignatures, matched };
}
