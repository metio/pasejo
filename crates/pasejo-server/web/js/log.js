// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// The membership log of an organization, replayed with the rules of the
// Rust implementation in crates/pasejo-protocol/src/log.rs. Both reach the
// same verdict on every entry; the vectors in
// crates/pasejo-protocol/vectors/console.json pin that.

import { b64, fromUtf8, isId, sha256, unb64 } from "./encoding.js";
import { Context, verify, verifyBy } from "./envelope.js";
import { blank, byteLength, parseRecord, recoveryId, validName, validRecipient, validRecord } from "./device.js";
import { isOnCurve } from "./curve.js";

export class Rejection extends Error {
  constructor(code, detail = "") {
    super(detail ? `${code}: ${detail}` : code);
    this.name = "Rejection";
    this.code = code;
    this.detail = detail;
  }
}

/** A verification failure outside the log's own rules. */
export class ProtocolError extends Error {
  constructor(code) {
    super(code);
    this.name = "ProtocolError";
    this.code = code;
  }
}

const ROLES = ["admin", "approver", "auditor", "member"];
const STORE_ROLES = ["reader", "writer", "admin"];

export function storeRoleRank(role) {
  return STORE_ROLES.indexOf(role);
}

export function principalKey(principal) {
  return principal.user ? `user:${principal.user}` : `team:${principal.team}`;
}

export function parsePrincipalKey(key) {
  const [kind, id] = key.split(":");
  return kind === "user" ? { user: id } : { team: id };
}

// ---------------------------------------------------------------------------
// Parsing

function malformed() {
  return new Rejection("Malformed");
}

function object(value) {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw malformed();
  }
  return value;
}

function id(value) {
  if (!isId(value)) {
    throw malformed();
  }
  return value;
}

function uint(value) {
  if (!Number.isSafeInteger(value) || value < 0) {
    throw malformed();
  }
  return value;
}

function bool(value) {
  if (typeof value !== "boolean") {
    throw malformed();
  }
  return value;
}

function string(value) {
  if (typeof value !== "string") {
    throw malformed();
  }
  return value;
}

function hash(value) {
  try {
    if (unb64(value).length === 32) {
      return value;
    }
  } catch {
    // Reported below.
  }
  throw malformed();
}

function optional(value, parse) {
  return value === null || value === undefined ? null : parse(value);
}

function ids(value) {
  if (!Array.isArray(value)) {
    throw malformed();
  }
  return value.map(id);
}

function oneOf(value, choices) {
  if (!choices.includes(value)) {
    throw malformed();
  }
  return value;
}

function settings(value) {
  const settingsObject = object(value);
  return {
    device_approval: oneOf(settingsObject.device_approval, ["manual", "automatic"]),
    two_admin: bool(settingsObject.two_admin),
    auto_join: bool(settingsObject.auto_join),
    offline_reads: settingsObject.offline_reads === undefined ? false : bool(settingsObject.offline_reads),
  };
}

function principal(value) {
  const principalObject = object(value);
  const keys = Object.keys(principalObject);
  if (keys.length !== 1 || (keys[0] !== "user" && keys[0] !== "team")) {
    throw malformed();
  }
  return { [keys[0]]: id(principalObject[keys[0]]) };
}

function record(value) {
  try {
    return parseRecord(value);
  } catch {
    throw malformed();
  }
}

const BODIES = {
  "org.genesis": (body) => ({
    name: string(body.name),
    founder: id(body.founder),
    device: record(body.device),
    settings: settings(body.settings),
  }),
  "org.settings": (body) => ({ settings: settings(body.settings) }),
  "member.role": (body) => ({
    user: id(body.user),
    role: oneOf(body.role, [...ROLES, "none"]),
  }),
  "device.add": (body) => ({ device: record(body.device) }),
  "device.revoke": (body) => ({ device: id(body.device) }),
  "recovery.set": (body) => ({ recipient: optional(body.recipient, string) }),
  "team.create": (body) => ({ team: id(body.team), name: string(body.name) }),
  "team.member": (body) => ({
    team: id(body.team),
    user: id(body.user),
    member: bool(body.member),
  }),
  "team.delete": (body) => ({ team: id(body.team) }),
  "store.create": (body) => ({ store: id(body.store) }),
  "store.grant": (body) => ({
    store: id(body.store),
    principal: principal(body.principal),
    role: oneOf(body.role, [...STORE_ROLES, "none"]),
    expires: optional(body.expires, uint),
  }),
  "store.epoch": (body) => ({
    store: id(body.store),
    epoch: uint(body.epoch),
    names_commit: hash(body.names_commit),
    data_commit: hash(body.data_commit),
    full: ids(body.full),
    names_only: ids(body.names_only),
  }),
  "store.delete": (body) => ({ store: id(body.store) }),
};

/** Parses the payload of a log entry, refusing anything of the wrong shape. */
export function parseEntry(payload) {
  let value;
  try {
    value = JSON.parse(fromUtf8(payload));
  } catch {
    throw malformed();
  }
  const entry = object(value);
  const parse = Object.hasOwn(BODIES, entry.kind) ? BODIES[entry.kind] : undefined;
  if (parse === undefined) {
    throw malformed();
  }
  return {
    org: id(entry.org),
    seq: uint(entry.seq),
    prev: optional(entry.prev, hash),
    time: uint(entry.time),
    author: id(entry.author),
    kind: entry.kind,
    body: parse(object(entry.body)),
  };
}

function recordJson(device) {
  return {
    id: device.id,
    user: device.user,
    kind: device.kind,
    recipient: device.recipient,
    signing_key: b64(device.signingKey),
    label: device.label,
  };
}

/**
 * The JSON object of settings in the wire form, which leaves out
 * `offline_reads` while it is false.
 */
export function settingsJson(settings) {
  const json = { device_approval: settings.device_approval, two_admin: settings.two_admin, auto_join: settings.auto_join };
  if (settings.offline_reads) {
    json.offline_reads = true;
  }
  return json;
}

function bodyJson(kind, body) {
  switch (kind) {
    case "org.genesis":
      return { name: body.name, founder: body.founder, device: recordJson(body.device), settings: settingsJson(body.settings) };
    case "org.settings":
      return { settings: settingsJson(body.settings) };
    case "device.add":
      return { device: recordJson(body.device) };
    case "store.grant": {
      const json = { store: body.store, principal: body.principal, role: body.role };
      if (body.expires !== null && body.expires !== undefined) {
        json.expires = body.expires;
      }
      return json;
    }
    default:
      return body;
  }
}

/** The JSON object of an entry, with its fields in the wire order. */
export function entryJson(entry) {
  return {
    org: entry.org,
    seq: entry.seq,
    prev: entry.prev,
    time: entry.time,
    author: entry.author,
    kind: entry.kind,
    body: bodyJson(entry.kind, entry.body),
  };
}

// ---------------------------------------------------------------------------
// State

/** The active device `device`, if any. */
export function activeDevice(state, device) {
  const found = state.devices.get(device);
  return found?.active ? found.record : undefined;
}

export function devicesOf(state, user) {
  return [...state.devices.values()]
    .filter((device) => device.active && device.record.user === user)
    .map((device) => device.record);
}

export function inTeam(state, team, user) {
  return state.teams.get(team)?.members.has(user) ?? false;
}

function effective(grant, now) {
  return grant.expires === null || grant.expires > now;
}

/**
 * The strongest effective grant `user` holds on `store` at `now`, directly
 * or through a team. Non-members hold nothing.
 */
export function storeRole(state, store, user, now) {
  if (!state.members.has(user)) {
    return undefined;
  }
  const storeState = state.stores.get(store);
  if (!storeState || storeState.deleted) {
    return undefined;
  }
  let best;
  for (const [key, grant] of storeState.grants) {
    if (!effective(grant, now)) {
      continue;
    }
    const holder = parsePrincipalKey(key);
    const applies = holder.user ? holder.user === user : inTeam(state, holder.team, user);
    if (applies && (best === undefined || storeRoleRank(grant.role) > storeRoleRank(best))) {
      best = grant.role;
    }
  }
  return best;
}

export function storeHasAdmin(state, store, now) {
  return [...state.members.keys()].some((user) => storeRole(state, store, user, now) === "admin");
}

export function storeUsers(state, store, now) {
  return [...state.members.keys()].filter((user) => storeRole(state, store, user, now) !== undefined);
}

/**
 * The devices entitled to a store's keys at `now`: age devices of users
 * with a grant and the recovery recipient receive full keys; browser
 * devices of those users and every device of organization admins and
 * auditors receive names only.
 */
export function entitled(state, store, now, recoveryDevice = state.recoveryDevice) {
  const full = new Set();
  const namesOnly = new Set();
  const storeState = state.stores.get(store);
  if (!storeState || storeState.deleted) {
    return { full, namesOnly };
  }
  for (const user of storeUsers(state, store, now)) {
    for (const device of devicesOf(state, user)) {
      (device.kind === "age" ? full : namesOnly).add(device.id);
    }
  }
  if (recoveryDevice) {
    full.add(recoveryDevice);
  }
  for (const [user, role] of state.members) {
    if (role === "admin" || role === "auditor") {
      for (const device of devicesOf(state, user)) {
        if (!full.has(device.id)) {
          namesOnly.add(device.id);
        }
      }
    }
  }
  return { full, namesOnly };
}

function adminCount(state) {
  return [...state.members.values()].filter((role) => role === "admin").length;
}

// ---------------------------------------------------------------------------
// Rules

function requireAdmin(state, user, what) {
  if (state.members.get(user) !== "admin") {
    throw new Rejection("Unauthorized", what);
  }
}

function requireTwo(signers, qualifies) {
  const users = new Set(signers.map(([, user]) => user).filter(qualifies));
  if (users.size < 2) {
    throw new Rejection("SecondAdminRequired");
  }
}

function requireMember(state, user) {
  if (!state.members.has(user)) {
    throw new Rejection("UnknownUser");
  }
}

function liveStore(state, store) {
  const storeState = state.stores.get(store);
  if (!storeState || storeState.deleted) {
    throw new Rejection("UnknownStore");
  }
  return storeState;
}

function isSubset(set, of) {
  return [...set].every((item) => of.has(item));
}

function apply(state, entry, signers, recovery) {
  const author = signers[0][1];
  const now = entry.time;
  const body = entry.body;
  const isAdmin = (user) => state.members.get(user) === "admin";
  switch (entry.kind) {
    case "org.genesis":
      throw new Rejection("Genesis");
    case "org.settings":
      requireAdmin(state, author, "change settings");
      if (state.settings.two_admin || body.settings.two_admin) {
        requireTwo(signers, isAdmin);
      }
      state.settings = { ...body.settings };
      break;
    case "member.role": {
      requireAdmin(state, author, "change roles");
      const current = state.members.get(body.user);
      const role = body.role === "none" ? undefined : body.role;
      if (current === undefined && role === undefined) {
        throw new Rejection("UnknownUser");
      }
      if (state.settings.two_admin && (role === "admin" || current === "admin")) {
        requireTwo(signers, isAdmin);
      }
      if (current === "admin" && role !== "admin" && adminCount(state) === 1) {
        throw new Rejection("LastAdmin");
      }
      if (role !== undefined) {
        state.members.set(body.user, role);
      } else {
        state.members.delete(body.user);
        for (const device of state.devices.values()) {
          if (device.record.user === body.user) {
            device.active = false;
          }
        }
        for (const team of state.teams.values()) {
          team.members.delete(body.user);
        }
        for (const storeState of state.stores.values()) {
          storeState.grants.delete(`user:${body.user}`);
        }
      }
      break;
    }
    case "device.add": {
      const device = body.device;
      if (!validRecord(device) || !isOnCurve(device.signingKey)) {
        throw new Rejection("InvalidDevice");
      }
      requireMember(state, device.user);
      if (state.used.has(device.id)) {
        throw new Rejection("DuplicateIdentifier");
      }
      const role = state.members.get(author);
      if (role !== "admin" && role !== "approver" && author !== device.user) {
        throw new Rejection("Unauthorized", "add devices of other users");
      }
      state.used.add(device.id);
      state.devices.set(device.id, { record: device, active: true });
      break;
    }
    case "device.revoke": {
      const target = activeDevice(state, body.device);
      if (!target) {
        throw new Rejection("UnknownDevice");
      }
      if (target.user !== author) {
        requireAdmin(state, author, "revoke devices of other users");
      }
      state.devices.get(body.device).active = false;
      break;
    }
    case "recovery.set":
      requireAdmin(state, author, "set the recovery recipient");
      if (state.settings.two_admin) {
        requireTwo(signers, isAdmin);
      }
      if (body.recipient !== null && !validRecipient("age", body.recipient)) {
        throw new Rejection("InvalidRecovery");
      }
      state.recovery = body.recipient;
      state.recoveryDevice = recovery;
      break;
    case "team.create":
      requireAdmin(state, author, "create teams");
      if (!validName(body.name)) {
        throw new Rejection("InvalidName");
      }
      if (state.used.has(body.team)) {
        throw new Rejection("DuplicateIdentifier");
      }
      state.used.add(body.team);
      state.teams.set(body.team, { name: body.name, members: new Set() });
      break;
    case "team.member": {
      requireAdmin(state, author, "change teams");
      const team = state.teams.get(body.team);
      if (!team) {
        throw new Rejection("UnknownTeam");
      }
      if (body.member) {
        requireMember(state, body.user);
        team.members.add(body.user);
      } else if (!team.members.delete(body.user)) {
        throw new Rejection("UnknownUser");
      }
      break;
    }
    case "team.delete":
      requireAdmin(state, author, "delete teams");
      if (!state.teams.delete(body.team)) {
        throw new Rejection("UnknownTeam");
      }
      for (const storeState of state.stores.values()) {
        storeState.grants.delete(`team:${body.team}`);
      }
      break;
    case "store.create":
      requireMember(state, author);
      if (state.used.has(body.store)) {
        throw new Rejection("DuplicateIdentifier");
      }
      state.used.add(body.store);
      state.stores.set(body.store, {
        grants: new Map([[`user:${author}`, { role: "admin", expires: null }]]),
        epochs: [],
        deleted: false,
        createdSeq: entry.seq,
      });
      break;
    case "store.grant": {
      const storeState = liveStore(state, body.store);
      const storeAdmin = storeRole(state, body.store, author, now) === "admin";
      const orphanRescue = isAdmin(author) && !storeHasAdmin(state, body.store, now);
      if (!storeAdmin && !orphanRescue) {
        throw new Rejection("Unauthorized", "change grants of this store");
      }
      if (body.principal.user) {
        if (body.role !== "none") {
          requireMember(state, body.principal.user);
        }
      } else if (!state.teams.has(body.principal.team)) {
        throw new Rejection("UnknownTeam");
      }
      if (body.expires !== null && body.expires <= now) {
        throw new Rejection("ExpiryInPast");
      }
      const key = principalKey(body.principal);
      const grants = storeState.grants;
      const adminGrants = [...grants.values()].filter((grant) => grant.role === "admin").length;
      if (grants.get(key)?.role === "admin" && adminGrants === 1 && body.role !== "admin") {
        throw new Rejection("LastStoreAdmin");
      }
      if (body.role !== "none") {
        grants.set(key, { role: body.role, expires: body.expires });
      } else if (!grants.delete(key)) {
        throw new Rejection("UnknownGrant");
      }
      break;
    }
    case "store.epoch": {
      const current = liveStore(state, body.store).epochs.length;
      if (storeRole(state, body.store, author, now) !== "admin") {
        throw new Rejection("Unauthorized", "rotate this store's keys");
      }
      if (body.epoch !== current) {
        throw new Rejection("WrongEpoch");
      }
      const allowed = entitled(state, body.store, now);
      const full = new Set(body.full);
      const namesOnly = new Set(body.names_only);
      const names = new Set([...allowed.full, ...allowed.namesOnly]);
      if (
        !isSubset(full, allowed.full) ||
        !isSubset(namesOnly, names) ||
        [...full].some((device) => namesOnly.has(device))
      ) {
        throw new Rejection("UnentitledRecipient");
      }
      state.stores.get(body.store).epochs.push({
        namesCommit: body.names_commit,
        dataCommit: body.data_commit,
        full,
        namesOnly,
        seq: entry.seq,
        time: entry.time,
      });
      break;
    }
    case "store.delete":
      liveStore(state, body.store);
      if (storeRole(state, body.store, author, now) !== "admin") {
        throw new Rejection("Unauthorized", "delete this store");
      }
      if (state.settings.two_admin) {
        requireTwo(
          signers,
          (user) => isAdmin(user) || storeRole(state, body.store, user, now) === "admin",
        );
      }
      state.stores.get(body.store).deleted = true;
      break;
    default:
      throw malformed();
  }
}

async function checkSignatures(state, envelope, author) {
  if (envelope.signatures[0]?.device !== author) {
    throw new Rejection("AuthorNotFirstSigner");
  }
  const seen = new Set();
  const signers = [];
  for (const signature of envelope.signatures) {
    if (seen.has(signature.device)) {
      throw new Rejection("DuplicateSigner");
    }
    seen.add(signature.device);
    const device = activeDevice(state, signature.device);
    if (!device) {
      throw new Rejection("InactiveSigner");
    }
    if (!(await verify(Context.LogEntry, envelope.payload, device.signingKey, signature.signature))) {
      throw new Rejection("BadSignature");
    }
    signers.push([signature.device, device.user]);
  }
  return signers;
}

function genesisState(entry, hash) {
  const { name, founder, device, settings: orgSettings } = entry.body;
  return {
    org: entry.org,
    name,
    settings: { ...orgSettings },
    members: new Map([[founder, "admin"]]),
    devices: new Map([[device.id, { record: device, active: true }]]),
    recovery: null,
    recoveryDevice: null,
    teams: new Map(),
    stores: new Map(),
    used: new Set([device.id]),
    nextSeq: 1,
    head: hash,
    lastTime: entry.time,
  };
}

/** A replayed log: its entries, their hashes and the resulting state. */
export class OrgLog {
  #entries;
  #parsed;
  #signers;
  #hashes;
  #state;
  // The recovery device identifier of each recovery.set entry, by sequence
  // number; computing it needs the asynchronous SHA-256.
  #recoveryIds = new Map();

  constructor(entries, parsed, signers, hashes, state) {
    this.#entries = entries;
    this.#parsed = parsed;
    this.#signers = signers;
    this.#hashes = hashes;
    this.#state = state;
  }

  /** Starts a log from its genesis entry. */
  static async genesis(envelope) {
    const entry = parseEntry(envelope.payload);
    if (entry.kind !== "org.genesis") {
      throw new Rejection("Genesis");
    }
    const { name, founder, device } = entry.body;
    if (entry.seq !== 0 || entry.prev !== null) {
      throw new Rejection("Genesis");
    }
    if (device.user !== founder || entry.author !== device.id) {
      throw new Rejection("Genesis");
    }
    if (!validRecord(device)) {
      throw new Rejection("InvalidDevice");
    }
    if (blank(name) || byteLength(name) > 200) {
      throw new Rejection("InvalidName");
    }
    if (envelope.signatures.length !== 1 || envelope.signatures[0].device !== device.id) {
      throw new Rejection("AuthorNotFirstSigner");
    }
    if (!(await verify(Context.LogEntry, envelope.payload, device.signingKey, envelope.signatures[0].signature))) {
      throw new Rejection("BadSignature");
    }
    const hash = b64(await sha256(envelope.payload));
    return new OrgLog([envelope], [entry], [[[device.id, founder]]], [hash], genesisState(entry, hash));
  }

  /** Replays a whole log; a failure names the sequence number of the entry. */
  static async replay(envelopes) {
    if (envelopes.length === 0) {
      throw Object.assign(new Rejection("Genesis"), { seq: 0 });
    }
    let log;
    try {
      log = await OrgLog.genesis(envelopes[0]);
    } catch (error) {
      throw Object.assign(error, { seq: 0 });
    }
    for (const envelope of envelopes.slice(1)) {
      const seq = log.state.nextSeq;
      try {
        await log.append(envelope);
      } catch (error) {
        throw Object.assign(error, { seq });
      }
    }
    return log;
  }

  /** An independent copy; appending to it leaves this log unchanged. */
  clone() {
    const copy = new OrgLog(
      [...this.#entries],
      [...this.#parsed],
      [...this.#signers],
      [...this.#hashes],
      structuredClone(this.#state),
    );
    copy.#recoveryIds = new Map(this.#recoveryIds);
    return copy;
  }

  get state() {
    return this.#state;
  }

  get entries() {
    return this.#entries;
  }

  /** The parsed entries, in sequence. */
  get parsed() {
    return this.#parsed;
  }

  get head() {
    return this.#state.head;
  }

  get genesisHash() {
    return this.#hashes[0];
  }

  /**
   * The periods in which the organization allowed offline reads, from the
   * time of the entry that allowed them to the time of the entry that stopped
   * them; `until` is null while they are still allowed.
   */
  offlineReadPeriods() {
    const periods = [];
    for (const entry of this.#parsed) {
      if (entry.kind !== "org.genesis" && entry.kind !== "org.settings") {
        continue;
      }
      const allowed = entry.body.settings.offline_reads;
      const last = periods.at(-1);
      const open = last !== undefined && last.until === null;
      if (allowed && !open) {
        periods.push({ from: entry.time, until: null });
      } else if (!allowed && open) {
        last.until = entry.time;
      }
    }
    return periods;
  }

  /** The sequence number of the entry with hash `hash`, or undefined. */
  seqOf(hash) {
    const index = this.#hashes.indexOf(hash);
    return index === -1 ? undefined : index;
  }

  /** The state after entry `seq`. */
  stateAt(seq) {
    if (!Number.isSafeInteger(seq) || seq < 0 || seq >= this.#entries.length) {
      throw new Rejection("WrongSequence");
    }
    if (seq === this.#entries.length - 1) {
      return this.#state;
    }
    const state = genesisState(this.#parsed[0], this.#hashes[0]);
    for (let index = 1; index <= seq; index++) {
      const entry = this.#parsed[index];
      apply(state, entry, this.#signers[index], this.#recoveryIds.get(index) ?? null);
      state.nextSeq += 1;
      state.head = this.#hashes[index];
      state.lastTime = entry.time;
    }
    return state;
  }

  /**
   * Appends an entry after checking its chain, signatures and the author's
   * rights. A refused entry leaves the log unchanged.
   */
  async append(envelope) {
    const entry = parseEntry(envelope.payload);
    const state = this.#state;
    if (entry.org !== state.org) {
      throw new Rejection("WrongOrganization");
    }
    if (entry.kind === "org.genesis") {
      throw new Rejection("Genesis");
    }
    if (entry.seq !== state.nextSeq) {
      throw new Rejection("WrongSequence");
    }
    if (entry.prev !== state.head) {
      throw new Rejection("WrongPredecessor");
    }
    if (entry.time < state.lastTime) {
      throw new Rejection("TimeWentBackwards");
    }
    const signers = await checkSignatures(state, envelope, entry.author);
    const recovery =
      entry.kind === "recovery.set" && entry.body.recipient !== null
        ? await recoveryId(entry.body.recipient)
        : null;
    const next = structuredClone(state);
    apply(next, entry, signers, recovery);
    const hash = b64(await sha256(envelope.payload));
    next.nextSeq += 1;
    next.head = hash;
    next.lastTime = entry.time;
    if (entry.kind === "recovery.set") {
      this.#recoveryIds.set(this.#entries.length, recovery);
    }
    this.#state = next;
    this.#entries.push(envelope);
    this.#parsed.push(entry);
    this.#signers.push(signers);
    this.#hashes.push(hash);
    return entry;
  }

  /**
   * Checks an index version against the log: its signature by an author
   * device that was active and held a writer or admin grant at the log head
   * the version names, and its epoch.
   */
  async checkIndex(envelope) {
    const payload = parseIndexPayload(envelope.payload);
    const author = envelope.signatures[0]?.device;
    if (author === undefined) {
      throw new ProtocolError("signature");
    }
    const seq = this.seqOf(payload.log_head);
    if (seq === undefined) {
      throw new ProtocolError("index names an unknown log head");
    }
    const state = this.stateAt(seq);
    const device = activeDevice(state, author);
    if (!device) {
      throw new ProtocolError("index author is not an active device");
    }
    if (!(await verifyBy(envelope, Context.Index, author, device.signingKey))) {
      throw new ProtocolError("signature");
    }
    const role = storeRole(state, payload.store, device.user, payload.time);
    if (role !== "writer" && role !== "admin") {
      throw new ProtocolError("index author may not write the store");
    }
    const epochs = state.stores.get(payload.store)?.epochs.length ?? 0;
    if (payload.epoch >= epochs) {
      throw new ProtocolError("index names an unknown epoch");
    }
    return payload;
  }

  /** The epoch record that commits to a store's keys of `epoch`. */
  epoch(store, epoch) {
    return this.#state.stores.get(store)?.epochs[epoch];
  }
}

/** Parses an index payload. */
export function parseIndexPayload(payload) {
  try {
    const value = object(JSON.parse(fromUtf8(payload)));
    return {
      store: id(value.store),
      version: uint(value.version),
      prev: optional(value.prev, hash),
      epoch: uint(value.epoch),
      log_head: hash(value.log_head),
      time: uint(value.time),
      index: unb64(string(value.index)),
      lookup_hash: optional(value.lookup_hash, hash),
      changed: ids(value.changed),
      deleted: ids(value.deleted),
    };
  } catch {
    throw new ProtocolError("json");
  }
}

/** Builds the next entry for `body`; its time never goes below the head's. */
export function nextEntry(log, author, time, kind, body) {
  const state = log.state;
  return {
    org: state.org,
    seq: state.nextSeq,
    prev: state.head,
    time: Math.max(time, state.lastTime),
    author,
    kind,
    body,
  };
}
