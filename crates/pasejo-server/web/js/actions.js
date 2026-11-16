// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// Pending actions the server computed, turned into what the console does
// for them. The console holds names keys only, so actions that need a
// store's data keys are left to a device with full keys.

import { parseRecord } from "./device.js";

/**
 * What running `action` means, given the current log state:
 * - `{type: "append", entries}`: log entries to sign, in order;
 * - `{type: "approve", device}`: a device to add once a person compared
 *   its fingerprint, which the console computes from the record itself;
 * - `{type: "cosign", seq}`: an audit checkpoint to co-sign;
 * - `{type: "needs-keys"}`: needs a device with full keys;
 * - `{type: "skip", reason}`: nothing to do, with a message key.
 */
export function plan(action, state) {
  switch (action.action) {
    case "approve-device": {
      const device = parseRecord(action.enrollment.device);
      if (!state.members.has(device.user)) {
        return { type: "skip", reason: "action-skip-not-member" };
      }
      if (state.devices.has(device.id)) {
        return { type: "skip", reason: "action-skip-done" };
      }
      return state.settings.device_approval === "automatic"
        ? { type: "append", entries: [{ kind: "device.add", body: { device } }] }
        : { type: "approve", device };
    }
    case "join-member":
      return { type: "append", entries: [{ kind: "member.role", body: { user: action.user, role: "member" } }] };
    case "remove-member":
      return { type: "append", entries: [{ kind: "member.role", body: { user: action.user, role: "none" } }] };
    case "sync-team": {
      const entries = [];
      if (!action.exists) {
        entries.push({ kind: "team.create", body: { team: action.team, name: action.name } });
      }
      for (const user of action.add) {
        entries.push({ kind: "team.member", body: { team: action.team, user, member: true } });
      }
      for (const user of action.remove) {
        entries.push({ kind: "team.member", body: { team: action.team, user, member: false } });
      }
      return { type: "append", entries };
    }
    case "revoke-expired":
    case "revoke-grant":
      return {
        type: "append",
        entries: [
          { kind: "store.grant", body: { store: action.store, principal: action.principal, role: "none", expires: null } },
        ],
      };
    case "cosign-checkpoint":
      return { type: "cosign", seq: action.seq };
    case "rotate-epoch":
    case "wrap-keys":
      return { type: "needs-keys" };
    default:
      return { type: "skip", reason: "action-skip-unknown" };
  }
}
