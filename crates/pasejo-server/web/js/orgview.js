// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// Rows the console shows, computed from the verified log state, the user
// directory and the store indexes this browser could open. Pure functions:
// they take data and return data.

import { parsePrincipalKey, storeRoleRank } from "./log.js";

/** How a user is named: `machine:NAME` for machines, else name or email. */
export function userDisplay(user) {
  if (user.machine) {
    return `machine:${user.name}`;
  }
  return user.name || user.email || user.id;
}

/** A map from user identifier to display name. */
export function directory(users) {
  return new Map(users.map((user) => [user.id, userDisplay(user)]));
}

function nameOf(names, id) {
  return names.get(id) ?? id;
}

const byText = (key) => (left, right) => key(left).localeCompare(key(right));

export function memberRows(state, users) {
  const names = directory(users);
  const emails = new Map(users.map((user) => [user.id, user.email]));
  return [...state.members]
    .map(([id, role]) => ({
      id,
      name: nameOf(names, id),
      email: emails.get(id) ?? "",
      role,
      devices: [...state.devices.values()].filter((device) => device.active && device.record.user === id).length,
    }))
    .sort(byText((row) => row.name));
}

export function deviceRows(state, users, fingerprints, me) {
  const names = directory(users);
  return [...state.devices.values()]
    .map(({ record, active }) => ({
      id: record.id,
      label: record.label,
      kind: record.kind,
      user: nameOf(names, record.user),
      active,
      fingerprint: fingerprints.get(record.id) ?? "",
      mine: record.id === me,
    }))
    .sort((left, right) => Number(right.active) - Number(left.active) || left.user.localeCompare(right.user));
}

export function teamRows(state, users) {
  const names = directory(users);
  return [...state.teams]
    .map(([id, team]) => ({ id, name: team.name, members: [...team.members].map((user) => nameOf(names, user)).sort() }))
    .sort(byText((row) => row.name));
}

/** How a grant's holder is named. */
export function principalName(principal, state, names) {
  if (principal.user) {
    return nameOf(names, principal.user);
  }
  return state.teams.get(principal.team)?.name ?? principal.team;
}

/** The label of a store, or its identifier when this browser cannot read it. */
export function storeName(store, labels) {
  return labels.get(store) ?? store;
}

export function storeRows(state, users, labels, now) {
  const names = directory(users);
  return [...state.stores]
    .map(([id, store]) => ({
      id,
      label: labels.get(id) ?? null,
      deleted: store.deleted,
      epoch: store.epochs.length > 0 ? store.epochs.length - 1 : null,
      grants: [...store.grants]
        .map(([key, grant]) => {
          const principal = parsePrincipalKey(key);
          return {
            principal,
            name: principalName(principal, state, names),
            team: Boolean(principal.team),
            role: grant.role,
            expires: grant.expires,
            expired: grant.expires !== null && grant.expires <= now,
          };
        })
        .sort((left, right) => storeRoleRank(right.role) - storeRoleRank(left.role) || left.name.localeCompare(right.name)),
    }))
    .sort((left, right) => Number(left.deleted) - Number(right.deleted) || (left.label ?? left.id).localeCompare(right.label ?? right.id));
}

/** Stores whose current epoch gave this device a names-only wrapping. */
export function readableStores(state, device) {
  return [...state.stores]
    .filter(([, store]) => !store.deleted && store.epochs.length > 0 && store.epochs.at(-1).namesOnly.has(device))
    .map(([id]) => id);
}

/** A message key and arguments describing a log entry. */
export function describeEntry(entry, state, names, labels) {
  const body = entry.body;
  const user = (id) => nameOf(names, id);
  const team = (id) => state.teams.get(id)?.name ?? id;
  switch (entry.kind) {
    case "org.settings":
      return { key: "entry-org-settings", args: {} };
    case "member.role":
      return body.role === "none"
        ? { key: "entry-member-remove", args: { user: user(body.user) } }
        : { key: "entry-member-role", args: { user: user(body.user), role: body.role } };
    case "device.add":
      return { key: "entry-device-add", args: { label: body.device.label, user: user(body.device.user) } };
    case "device.revoke":
      return { key: "entry-device-revoke", args: { device: state.devices.get(body.device)?.record.label ?? body.device } };
    case "recovery.set":
      return { key: body.recipient === null ? "entry-recovery-clear" : "entry-recovery-set", args: {} };
    case "team.create":
      return { key: "entry-team-create", args: { name: body.name } };
    case "team.member":
      return { key: body.member ? "entry-team-member-add" : "entry-team-member-remove", args: { user: user(body.user), team: team(body.team) } };
    case "team.delete":
      return { key: "entry-team-delete", args: { team: team(body.team) } };
    case "store.create":
      return { key: "entry-store-create", args: { store: storeName(body.store, labels) } };
    case "store.grant":
      return body.role === "none"
        ? { key: "entry-store-revoke", args: { principal: principalName(body.principal, state, names), store: storeName(body.store, labels) } }
        : {
            key: "entry-store-grant",
            args: { principal: principalName(body.principal, state, names), role: body.role, store: storeName(body.store, labels) },
          };
    case "store.epoch":
      return { key: "entry-store-epoch", args: { store: storeName(body.store, labels) } };
    case "store.delete":
      return { key: "entry-store-delete", args: { store: storeName(body.store, labels) } };
    default:
      return { key: "entry-unknown", args: { kind: entry.kind } };
  }
}

/** A message key and arguments describing a pending action. */
export function describeAction(action, state, names, labels) {
  const user = (id) => nameOf(names, id);
  const store = (id) => storeName(id, labels);
  switch (action.action) {
    case "approve-device":
      return {
        key: "action-approve-device",
        args: { label: action.enrollment.device.label, user: action.enrollment.user_email || action.enrollment.user_name },
      };
    case "join-member":
      return { key: "action-join-member", args: { name: action.name || action.email, email: action.email } };
    case "remove-member":
      return { key: "action-remove-member", args: { user: user(action.user) } };
    case "sync-team":
      return { key: "action-sync-team", args: { name: action.name } };
    case "rotate-epoch":
      return { key: "action-rotate-epoch", args: { store: store(action.store) } };
    case "wrap-keys":
      return { key: "action-wrap-keys", args: { count: action.devices.length, store: store(action.store) } };
    case "revoke-expired":
      return { key: "action-revoke-expired", args: { principal: principalName(action.principal, state, names), store: store(action.store) } };
    case "revoke-grant":
      return { key: "action-revoke-grant", args: { principal: principalName(action.principal, state, names), store: store(action.store) } };
    case "cosign-checkpoint":
      return { key: "action-cosign-checkpoint", args: { seq: action.seq } };
    default:
      return { key: "action-unknown", args: { action: action.action } };
  }
}

/** Audit events with actors, stores and objects named where this browser can. */
export function auditRows(events, names, labels, objects) {
  return [...events].reverse().map((event) => ({
    seq: event.seq,
    time: event.time,
    actor: event.actor_user ? nameOf(names, event.actor_user) : "",
    action: event.action,
    store: event.store ? storeName(event.store, labels) : "",
    object: event.object ? (objects.get(event.object)?.name ?? event.object) : "",
    outcome: event.outcome,
    detail: event.detail ?? "",
  }));
}
