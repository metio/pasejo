// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// A fake pasejo server over the store vectors: the membership log, the
// browser device's wrappings and an index version, all produced by the Rust
// implementation. It checks every request proof and replays every appended
// entry with the console's own log rules, so tests see what a real server
// would refuse.

import { b64, fromUtf8, unb64, utf8 } from "../js/encoding.js";
import { Context, envelopeJson, open, parseEnvelope, verify } from "../js/envelope.js";
import { importPrivate } from "../js/hpke.js";
import { OrgLog } from "../js/log.js";
import { proofMessage } from "../js/request.js";
import { errorResponse, fakeFetch, publicKey, signingKey, vectors } from "./helpers.js";

export const ORG = vectors.store.log.state.org;
export const STORE = vectors.store.store;
export const ADMIN = "a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1";
export const WRITER = "a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2";
export const READER = "a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3";
export const NOW = 1_800_000_000_000;

/** The browser device of the store vectors, as devicekeys.js shapes a device. */
export async function browserDevice() {
  const browser = vectors.store.browser;
  return {
    id: browser.id,
    label: "Firefox",
    signingKey: await signingKey(browser.signing_seed),
    signingPublic: await publicKey(browser.signing_seed),
    hpkeKey: await importPrivate(unb64(browser.hpke_private)),
    hpkePublic: unb64(browser.hpke_public),
  };
}

/** A device of `user` the log does not know, with its record. */
export async function strangerDevice(seed = 30, user = ADMIN) {
  const bytes = new Uint8Array(32).fill(seed);
  const id = seed.toString(16).padStart(2, "0").repeat(16);
  const hpke = await importPrivate(new Uint8Array(32).fill(seed + 1), true);
  const jwk = await globalThis.crypto.subtle.exportKey("jwk", hpke);
  const device = {
    id,
    label: "Chrome",
    signingKey: await signingKey(bytes),
    signingPublic: await publicKey(bytes),
    hpkeKey: hpke,
    hpkePublic: unb64(jwk.x),
  };
  const record = {
    id,
    user,
    kind: "hpke",
    recipient: b64(device.hpkePublic),
    signing_key: b64(device.signingPublic),
    label: "Chrome",
  };
  return { device, record };
}

export function logEnvelopes() {
  const log = vectors.store.log;
  return [log.genesis, ...log.steps.filter((step) => step.result === "ok").map((step) => step.envelope)];
}

export const USERS = [
  { id: ADMIN, name: "Ada", email: "ada@example.com", machine: false, active: true },
  { id: WRITER, name: "Grace", email: "grace@example.com", machine: false, active: true },
  { id: READER, name: "", email: "linus@example.com", machine: false, active: true },
];

const json = (status, body) => new Response(JSON.stringify(body), { status });

/**
 * A fake server. `options` override its state: `cookie` (whether a console
 * sign-in is waiting), `actions`, `reviews`, `rotation`, `events`,
 * `checkpoints`, `serverKey`, `index`, `forbid` (a set of "METHOD /path"
 * answered with 403) and `conflictOnce` (the next append answers conflict).
 */
export async function fakeServer(options = {}) {
  const state = {
    cookie: true,
    actions: [],
    reviews: [],
    proposals: [],
    rotation: { user: WRITER, ended: NOW - 1000, items: [] },
    events: [],
    checkpoints: [],
    serverKey: vectors.audit.server_key,
    index: vectors.store.checks[0].index,
    forbid: new Set(),
    conflictOnce: false,
    enrolled: [],
    decisions: [],
    sessionKey: null,
    token: null,
    emergencyUntil: null,
    users: structuredClone(USERS),
    ...options,
  };
  state.envelopes = logEnvelopes();
  state.log = await OrgLog.replay(state.envelopes.map(parseEnvelope));

  async function authenticate(request) {
    const authorization = request.headers.authorization ?? "";
    if (!state.token || authorization !== `Pasejo ${state.token}`) {
      return errorResponse(401, "unauthorized", "no session");
    }
    const message = await proofMessage(
      request.method,
      request.path + request.query,
      request.headers["pasejo-timestamp"],
      request.headers["pasejo-nonce"],
      utf8(request.body),
      state.token,
    );
    if (!(await verify(Context.Request, message, state.sessionKey, unb64(request.headers["pasejo-proof"])))) {
      return errorResponse(401, "unauthorized", "bad proof");
    }
    return null;
  }

  const routes = {};
  const route = (key, handler, { anonymous = false } = {}) => {
    routes[key] = async (request) => {
      if (state.forbid.has(key)) {
        return errorResponse(403, "forbidden", "forbidden");
      }
      if (!anonymous) {
        const refused = await authenticate(request);
        if (refused) {
          return refused;
        }
      }
      return handler(request, request.body ? JSON.parse(request.body) : undefined);
    };
  };

  route(
    "GET /v1/server",
    () => ({ version: "test", checkpoint_key: state.serverKey, oidc: true, break_glass: true, emergency_until: state.emergencyUntil }),
    { anonymous: true },
  );
  route(
    "POST /v1/auth/web",
    (request, body) => {
      if (!state.cookie) {
        return errorResponse(401, "unauthorized", "no console sign-in");
      }
      state.cookie = false;
      const claim = open(parseEnvelope(body.claim));
      state.sessionKey = unb64(claim.signing_key);
      state.token = `token-${state.sessionKey[0]}`;
      return { token: state.token, expires: NOW + 3_600_000, user: ADMIN };
    },
    { anonymous: true },
  );
  route("POST /v1/auth/logout", () => {
    state.token = null;
    return undefined;
  });
  route("GET /v1/me", () => ({ user: ADMIN, name: "Ada", email: "ada@example.com", machine: false, orgs: [{ id: ORG, name: "store" }] }));
  const activeSession = () =>
    [...state.log.state.devices.values()].some(
      (device) => device.active && b64(device.record.signingKey) === b64(state.sessionKey),
    );
  route(`POST /v1/orgs/${ORG}/enrollments`, (request, body) => {
    if (activeSession()) {
      return errorResponse(400, "bad_request", "this device is already active");
    }
    if (state.log.state.devices.has(body.id) || state.enrolled.some((enrollment) => enrollment.id === body.id)) {
      return errorResponse(409, "conflict", "the device identifier is taken");
    }
    state.enrolled.push(body);
    return json(201, {});
  });
  route(`GET /v1/orgs/${ORG}/log`, (request) => {
    const since = new URLSearchParams(request.query).get("since");
    const from = since === null ? 0 : Number(since) + 1;
    return { entries: state.envelopes.slice(from) };
  });
  route(`POST /v1/orgs/${ORG}/log`, async (request, body) => {
    if (state.conflictOnce) {
      state.conflictOnce = false;
      return errorResponse(409, "conflict", "another entry took the slot");
    }
    try {
      await state.log.append(parseEnvelope(body.entry));
    } catch (error) {
      return errorResponse(400, "invalid", error.message);
    }
    state.envelopes.push(body.entry);
    return { seq: state.envelopes.length - 1 };
  });
  route(`GET /v1/orgs/${ORG}/log/proposals`, () => ({ proposals: state.proposals }));
  route(`POST /v1/orgs/${ORG}/log/proposals`, (request, body) => {
    const proposal = { id: "9".repeat(32), entry: body.entry, created: NOW };
    state.proposals.push(proposal);
    return { id: proposal.id };
  });
  route(`POST /v1/orgs/${ORG}/log/proposals/${"9".repeat(32)}/cosign`, async (request, body) => {
    const proposal = state.proposals.find((candidate) => candidate.id === "9".repeat(32));
    if (!proposal) {
      return errorResponse(404, "not_found", "no such proposal");
    }
    const entry = { payload: proposal.entry.payload, signatures: [...proposal.entry.signatures, body.signature] };
    try {
      await state.log.append(parseEnvelope(entry));
    } catch (error) {
      return errorResponse(400, "invalid", error.message);
    }
    state.envelopes.push(entry);
    state.proposals = state.proposals.filter((candidate) => candidate !== proposal);
    return undefined;
  });
  route(`GET /v1/orgs/${ORG}/users`, () => ({ users: state.users }));
  route(`GET /v1/orgs/${ORG}/stores/${STORE}/keys`, () => vectors.store.keys);
  route(`GET /v1/orgs/${ORG}/stores/${STORE}/index`, () => ({ index: state.index }));
  route(`GET /v1/orgs/${ORG}/actions`, () => ({ actions: state.actions }));
  route(`GET /v1/orgs/${ORG}/reviews`, () => ({ reviews: state.reviews }));
  route(`GET /v1/orgs/${ORG}/stores/${STORE}/rotation`, () => state.rotation);
  route(`GET /v1/orgs/${ORG}/audit`, (request) => {
    const params = new URLSearchParams(request.query);
    const since = Number(params.get("since"));
    const limit = Number(params.get("limit"));
    const events = state.events.filter((event) => event.seq > since).slice(0, limit);
    return { events, next: events.length === limit ? events.at(-1).seq : null };
  });
  route(`GET /v1/orgs/${ORG}/audit/checkpoints`, () => ({ checkpoints: state.checkpoints }));
  route("POST /v1/break-glass/login", (request, body) => {
    if (body.password !== "correct horse" || body.totp !== "123456") {
      return errorResponse(401, "unauthorized", "invalid credentials");
    }
    state.sessionKey = unb64(body.signing_key);
    state.token = "break-glass-token";
    return { token: state.token, expires: NOW + 900_000 };
  }, { anonymous: true });
  route("GET /v1/break-glass/users", () => ({ users: state.users }));
  route("GET /v1/break-glass/orgs", () => ({ orgs: [{ id: ORG, name: "store" }] }));
  route("POST /v1/break-glass/emergency", (request, body) => {
    state.emergencyUntil = body.duration === 0 ? null : NOW + body.duration;
    return { until: state.emergencyUntil };
  });
  route("GET /v1/audit", (request) => {
    const since = Number(new URLSearchParams(request.query).get("since"));
    return { events: state.events.filter((event) => event.seq > since), next: null };
  });
  for (const action of ["suspend", "reactivate", "sessions/revoke"]) {
    route(`POST /v1/break-glass/users/${WRITER}/${action}`, () => {
      state.decisions.push(action);
      const user = state.users.find((candidate) => candidate.id === WRITER);
      if (action !== "sessions/revoke") {
        user.active = action === "reactivate";
      }
      return new Response(null, { status: 204 });
    });
  }

  const statusOf = (device) => {
    const known = state.log.state.devices.get(device);
    const active = Boolean(known?.active);
    const pending = !active && state.enrolled.some((enrollment) => enrollment.id === device);
    return active || pending ? { pending, active } : errorResponse(404, "not_found", "no such enrollment");
  };
  const server = fakeFetch(
    new Proxy(routes, {
      get(target, key) {
        const match = typeof key === "string" && key.match(new RegExp(`^GET /v1/orgs/${ORG}/enrollments/([0-9a-f]{32})$`));
        if (match && !Object.hasOwn(target, key)) {
          route(key, () => statusOf(match[1]));
        }
        return target[key];
      },
    }),
  );
  return {
    ...server,
    state,
    route,
    /** Adds a route answered by `handler` once authenticated. */
    on: (key, handler) => route(key, handler),
    paths: () => server.requests.map((request) => `${request.method} ${request.path}`),
  };
}

/** Decodes a request body that carried a log entry. */
export function postedEntry(request) {
  const body = JSON.parse(request.body);
  return JSON.parse(fromUtf8(unb64(body.entry.payload)));
}

export { envelopeJson };
