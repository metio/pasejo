// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// What the console does with the server: sign in, enroll, keep the
// membership log verified, sign changes, read store labels and verify the
// audit trail. Every function takes the API and what it needs explicitly,
// so the views stay independent of the transport.

import { ApiError } from "./api.js";
import { b64, sha256, unb64Fixed } from "./encoding.js";
import { addSignature, Context, envelopeJson, parseEnvelope, seal } from "./envelope.js";
import { claim, enrollmentRequest } from "./devicekeys.js";
import { OrgLog, ProtocolError, Rejection, entryJson, nextEntry, parseEntry } from "./log.js";
import { openNames } from "./keys.js";
import { checkChain, openIndex } from "./store.js";
import { cosign, verifyCheckpoint, verifyTrail } from "./audit.js";

/** Exchanges the single-use console cookie for a session bound to the device key. */
export async function signIn(api, device) {
  const response = await api.anonymous("POST", "/v1/auth/web", {
    claim: await claim(device, "web", "", api.now()),
  });
  api.session = { token: response.token, expires: response.expires, user: response.user };
  return api.session;
}

export async function signOut(api) {
  try {
    await api.post("/v1/auth/logout");
  } finally {
    api.session = null;
  }
}

/**
 * The server's checkpoint key, pinned in `store` on first use. A later
 * change is refused, as the CLI refuses it.
 */
export async function serverKey(api, store) {
  const info = await api.anonymous("GET", "/v1/server");
  const key = unb64Fixed(info.checkpoint_key, 32, "checkpoint key");
  const pinned = await store.get("server-key");
  if (pinned && b64(pinned) !== b64(key)) {
    throw new ProtocolError("the server's checkpoint key differs from the pinned one");
  }
  if (!pinned) {
    await store.put("server-key", key);
  }
  return { key, info };
}

/**
 * The device's enrollment in `org`: `{pending, active}`, or null when the
 * server knows of none.
 */
export async function enrollmentStatus(api, org, device) {
  try {
    return await api.get(`/v1/orgs/${org}/enrollments/${device}`);
  } catch (error) {
    if (error instanceof ApiError && (error.status === 404 || error.status === 403)) {
      return null;
    }
    throw error;
  }
}

/**
 * Enrolls the device unless it is active or waiting for approval already.
 * An enrollment that races with another counts as success.
 */
export async function enroll(api, org, device) {
  const status = await enrollmentStatus(api, org, device.id);
  if (status) {
    return status;
  }
  try {
    await api.post(`/v1/orgs/${org}/enrollments`, enrollmentRequest(device));
  } catch (error) {
    if (!(error instanceof ApiError && error.code === "conflict")) {
      throw error;
    }
  }
  return { pending: true, active: false };
}

/**
 * Fetches and verifies the log of `org`, continuing `known` when given and
 * checking the genesis against `pinnedGenesis` when given.
 */
export async function fetchLog(api, org, known = null, pinnedGenesis = null) {
  const path = known ? `/v1/orgs/${org}/log?since=${known.entries.length - 1}` : `/v1/orgs/${org}/log`;
  const page = await api.get(path);
  const envelopes = page.entries.map(parseEnvelope);
  let log;
  if (known) {
    log = known.clone();
    for (const envelope of envelopes) {
      await log.append(envelope);
    }
  } else {
    log = await OrgLog.replay(envelopes);
  }
  if (log.state.org !== org) {
    throw new ProtocolError("the log belongs to another organization");
  }
  if (pinnedGenesis && log.genesisHash !== pinnedGenesis) {
    throw new ProtocolError("the organization's genesis differs from the pinned one");
  }
  return log;
}

/**
 * Signs and appends an entry. The entry is first appended to a copy of the
 * log, so the console never sends what its own replay refuses; an entry
 * that needs a second admin becomes a proposal instead.
 */
export async function append(api, log, device, kind, body) {
  const entry = nextEntry(log, device.id, api.now(), kind, body);
  const envelope = await seal(Context.LogEntry, entryJson(entry), device.id, device.signingKey);
  const trial = log.clone();
  try {
    await trial.append(envelope);
  } catch (error) {
    if (error instanceof Rejection && error.code === "SecondAdminRequired") {
      const proposal = await api.post(`/v1/orgs/${log.state.org}/log/proposals`, {
        entry: envelopeJson(envelope),
      });
      return { log, proposal };
    }
    throw error;
  }
  await api.post(`/v1/orgs/${log.state.org}/log`, { entry: envelopeJson(envelope) });
  return { log: trial, proposal: null };
}

/** `append`, refetching the log and retrying once when another entry took the slot. */
export async function appendFresh(api, log, device, kind, body) {
  try {
    return await append(api, log, device, kind, body);
  } catch (error) {
    if (error instanceof ApiError && error.code === "conflict") {
      const fresh = await fetchLog(api, log.state.org, log);
      return append(api, fresh, device, kind, body);
    }
    throw error;
  }
}

/** Pending proposals, each with its parsed entry. */
export async function proposals(api, org) {
  const answer = await api.get(`/v1/orgs/${org}/log/proposals`);
  return answer.proposals.map((proposal) => {
    const envelope = parseEnvelope(proposal.entry);
    return { id: proposal.id, created: proposal.created, envelope, entry: parseEntry(envelope.payload) };
  });
}

/**
 * Co-signs a proposal after checking that, with this device's signature,
 * the entry appends to the current log.
 */
export async function cosignProposal(api, log, device, proposal) {
  const signed = await addSignature(proposal.envelope, Context.LogEntry, device.id, device.signingKey);
  await log.clone().append(signed);
  const signature = signed.signatures.at(-1);
  await api.post(`/v1/orgs/${log.state.org}/log/proposals/${proposal.id}/cosign`, {
    signature: { device: signature.device, signature: b64(signature.signature) },
  });
  return fetchLog(api, log.state.org, log);
}

/**
 * Opens a store's current index with the device's names-only keys and
 * returns it with the new pin. `pin` is the last version seen, or null.
 */
export async function loadStore(api, log, device, store, pin) {
  const org = log.state.org;
  const keys = await api.get(`/v1/orgs/${org}/stores/${store}/keys`);
  const ring = await openNames(log, store, device.id, keys, device.hpkeKey, device.hpkePublic);
  const answer = await api.get(`/v1/orgs/${org}/stores/${store}/index`);
  if (!answer.index) {
    return { index: null, pin };
  }
  const envelope = parseEnvelope(answer.index);
  const payload = await log.checkIndex(envelope);
  await checkChain(envelope, payload, store, pin);
  const names = ring.get(payload.epoch);
  if (!names) {
    throw new ProtocolError("this device holds no key of the index's epoch");
  }
  const index = await openIndex(store, payload.epoch, payload.version, names, payload.index);
  return { index, pin: { version: payload.version, hash: b64(await sha256(envelope.payload)) } };
}

const PAGE = 500;

/** Every audit event at `path` after `since`, following the pages. */
export async function auditEvents(api, path, since) {
  const events = [];
  let cursor = since;
  for (;;) {
    const page = await api.get(`${path}?since=${cursor}&limit=${PAGE}`);
    events.push(...page.events);
    if (page.next === null || page.next === undefined || page.next <= cursor) {
      return events;
    }
    cursor = page.next;
  }
}

function checkPayload(org, seq, payload, log) {
  if (payload.org !== org || payload.seq !== seq) {
    throw new ProtocolError(`checkpoint ${seq} names another chain or position`);
  }
  if (!payload.log_head || log.seqOf(payload.log_head) === undefined) {
    throw new ProtocolError(`checkpoint ${seq} names a membership log head this client never saw`);
  }
}

/**
 * Fetches and verifies the audit trail of `org`, as `pasejo audit verify`
 * does: every checkpoint's server signature and co-signatures, and the hash
 * chain of the events from the first kept one.
 */
export async function verifyAudit(api, log, key) {
  const org = log.state.org;
  const { checkpoints } = await api.get(`/v1/orgs/${org}/audit/checkpoints?since=0`);
  const events = await auditEvents(api, `/v1/orgs/${org}/audit`, 0);
  const result = await verifyTrail({
    checkpoints,
    events,
    serverKey: key,
    check: (seq, payload) => checkPayload(org, seq, payload, log),
    cosigner: (device) => log.state.devices.get(device)?.record.signingKey,
  });
  return { events, ...result };
}

/**
 * Co-signs checkpoint `seq` after checking the server's signature and that
 * it names a log head this browser verified.
 */
export async function cosignCheckpoint(api, log, device, key, seq) {
  const org = log.state.org;
  const { checkpoints } = await api.get(`/v1/orgs/${org}/audit/checkpoints?since=${Math.max(0, seq - 1)}`);
  const entry = checkpoints.find((candidate) => candidate.seq === seq);
  if (!entry) {
    throw new ProtocolError(`checkpoint ${seq} is gone`);
  }
  const payload = await verifyCheckpoint(entry.checkpoint, key);
  checkPayload(org, seq, payload, log);
  await api.post(`/v1/orgs/${org}/audit/checkpoints/${seq}/cosign`, {
    signature: await cosign(entry.checkpoint, device.id, device.signingKey),
  });
}
