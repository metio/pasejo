// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// The console's controller: it holds one model, changes it in response to
// the server and the person, and hands the current view tree to `show`.
// Transport, storage, time and rendering are passed in, so tests drive it
// without a browser.

import { ApiError } from "./api.js";
import { plan } from "./actions.js";
import { fingerprintMatches, orgFingerprint, recordFingerprint } from "./device.js";
import { deviceFingerprint } from "./devicekeys.js";
import { unb64 } from "./encoding.js";
import { activeDevice } from "./log.js";
import { readableStores, userDisplay } from "./orgview.js";
import { objectNames } from "./store.js";
import {
  appendFresh,
  cosignCheckpoint,
  cosignProposal,
  enroll,
  fetchLog,
  loadStore,
  proposals,
  serverKey,
  signIn,
  signOut,
  verifyAudit,
} from "./session.js";
import * as views from "./views.js";

/** Sessions closer than this to their expiry are not reused. */
const SESSION_MARGIN = 60_000;

const unauthorized = (error) => error instanceof ApiError && error.status === 401;

export class Console {
  constructor({ t, api, kv, device, show, now = () => Date.now() }) {
    this.t = t;
    this.api = api;
    this.kv = kv;
    this.device = device;
    this.show = show;
    this.now = now;
    this.model = { screen: "loading" };
    this.handlers = {
      signOut: () => this.signOut(),
      back: () => this.organizations(),
      selectTab: (tab) => this.selectTab(tab),
      runAction: (index, typed) => this.runAction(index, typed),
      cosign: (id) => this.cosign(id),
      decide: (review, principal, decision) => this.decide(review, principal, decision),
      showRotation: (store, user) => this.showRotation(store, user),
      loadAudit: () => this.loadAudit(),
    };
  }

  set(changes) {
    Object.assign(this.model, changes);
    this.show(this.view());
  }

  view() {
    const { t, model } = this;
    switch (model.screen) {
      case "loading":
        return views.loading(t);
      case "failure":
        return views.failure(t, model.message, () => this.start());
      case "sign-in":
        return views.signIn(t);
      case "orgs":
        return views.organizations(t, model.me, (org) => this.openOrg(org), () => this.signOut());
      case "confirm":
        return views.confirmOrganization(
          t,
          model.org,
          model.orgFingerprint,
          () => this.confirmOrg(),
          () => this.organizations(),
        );
      case "pending":
        return views.pending(t, model.org, this.device.id, model.deviceFingerprint, () => this.openOrg(model.org));
      case "org":
        return views.organization(t, model, this.handlers);
      default:
        throw new Error(`unknown screen ${model.screen}`);
    }
  }

  /** Runs `step`, turning a lost session into the sign-in screen and anything else into a failure. */
  async guard(step) {
    try {
      return await step();
    } catch (error) {
      if (unauthorized(error)) {
        await this.dropSession();
        this.set({ screen: "sign-in" });
      } else {
        this.set({ screen: "failure", message: views.errorText(this.t, error) });
      }
      return undefined;
    }
  }

  async dropSession() {
    this.api.session = null;
    await this.kv.delete("session");
  }

  /** Reuses a stored session, else exchanges a console sign-in cookie for one. */
  async session() {
    const stored = await this.kv.get("session");
    if (stored && stored.expires > this.now() + SESSION_MARGIN) {
      this.api.session = stored;
      return;
    }
    await signIn(this.api, this.device);
    await this.kv.put("session", { ...this.api.session });
  }

  start() {
    this.set({ screen: "loading" });
    return this.guard(async () => {
      this.serverKey = (await serverKey(this.api, this.kv)).key;
      await this.session();
      await this.organizations();
    });
  }

  organizations() {
    return this.guard(async () => {
      const me = await this.api.get("/v1/me");
      this.me = me;
      this.model = { screen: "orgs", me };
      this.set({});
    });
  }

  signOut() {
    return this.guard(async () => {
      try {
        await signOut(this.api);
      } finally {
        await this.dropSession();
      }
      this.model = { screen: "sign-in" };
      this.set({});
    });
  }

  openOrg(org) {
    this.set({ screen: "loading" });
    return this.guard(async () => {
      await enroll(this.api, org.id, this.device);
      const pinned = (await this.kv.get(`genesis:${org.id}`)) ?? null;
      const log = await fetchLog(this.api, org.id, null, pinned);
      if (pinned === null) {
        this.set({ screen: "confirm", org, log, orgFingerprint: orgFingerprint(unb64(log.genesisHash)) });
        return;
      }
      await this.enterOrg(org, log);
    });
  }

  confirmOrg() {
    const { org, log } = this.model;
    return this.guard(async () => {
      await this.kv.put(`genesis:${org.id}`, log.genesisHash);
      await this.enterOrg(org, log);
    });
  }

  async enterOrg(org, log) {
    if (!activeDevice(log.state, this.device.id)) {
      this.set({ screen: "pending", org, deviceFingerprint: await deviceFingerprint(this.device) });
      return;
    }
    const base = {
      screen: "org",
      org,
      log,
      tab: "members",
      device: this.device.id,
      user: this.api.session.user,
      userName: this.me ? userDisplay({ ...this.me, id: this.me.user }) : "",
      now: this.now(),
      notice: null,
      actions: null,
      actionsError: null,
      proposals: [],
      reviews: null,
      reviewsError: null,
      rotation: null,
      audit: { state: "idle" },
    };
    const [directory, stores, fingerprints] = await Promise.all([
      this.users(org.id),
      this.stores(org.id, log),
      this.fingerprints(log),
    ]);
    this.model = { ...base, ...directory, ...stores, fingerprints };
    this.set({});
  }

  async users(org) {
    try {
      const { users } = await this.api.get(`/v1/orgs/${org}/users`);
      return { users, usersError: null };
    } catch (error) {
      if (unauthorized(error)) {
        throw error;
      }
      return { users: [], usersError: error };
    }
  }

  async fingerprints(log) {
    const entries = await Promise.all(
      [...log.state.devices.values()].map(async ({ record }) => [record.id, await recordFingerprint(record)]),
    );
    return new Map(entries);
  }

  /**
   * Opens the index of every store this browser holds names keys for. A
   * store that fails verification is reported on its own and contributes
   * no label.
   */
  async stores(org, log) {
    const labels = new Map();
    const indexes = new Map();
    const objects = new Map();
    const storeErrors = new Map();
    for (const store of readableStores(log.state, this.device.id)) {
      const pinKey = `pin:${org}:${store}`;
      try {
        const pin = (await this.kv.get(pinKey)) ?? null;
        const opened = await loadStore(this.api, log, this.device, store, pin);
        if (opened.index) {
          await this.kv.put(pinKey, opened.pin);
          labels.set(store, opened.index.label);
          indexes.set(store, opened.index);
          objects.set(store, objectNames(opened.index));
        }
      } catch (error) {
        if (unauthorized(error)) {
          throw error;
        }
        storeErrors.set(store, error);
      }
    }
    return { labels, indexes, objects, storeErrors };
  }

  selectTab(tab) {
    this.set({ tab, notice: null });
    if (tab === "actions" && this.model.actions === null) {
      return this.loadActions();
    }
    if (tab === "reviews" && this.model.reviews === null) {
      return this.loadReviews();
    }
    return Promise.resolve();
  }

  loadActions() {
    const org = this.model.org.id;
    return this.guard(async () => {
      const [listed, pendingProposals] = await Promise.allSettled([
        this.api.get(`/v1/orgs/${org}/actions`),
        proposals(this.api, org),
      ]);
      for (const result of [listed, pendingProposals]) {
        if (result.status === "rejected" && unauthorized(result.reason)) {
          throw result.reason;
        }
      }
      this.set({
        actions:
          listed.status === "fulfilled"
            ? listed.value.actions.map((action) => ({ action, plan: plan(action, this.model.log.state) }))
            : [],
        actionsError: listed.status === "rejected" ? listed.reason : null,
        proposals: pendingProposals.status === "fulfilled" ? pendingProposals.value : [],
      });
    });
  }

  /** Runs `step` and reports its outcome as the notice; a lost session still signs out. */
  async attempt(step) {
    try {
      const text = await step();
      if (text) {
        this.set({ notice: { text, error: false } });
      }
    } catch (error) {
      if (unauthorized(error)) {
        await this.dropSession();
        this.set({ screen: "sign-in" });
        return;
      }
      this.set({ notice: { text: views.errorText(this.t, error), error: true } });
    }
  }

  async refreshLog() {
    const log = await fetchLog(this.api, this.model.org.id, this.model.log);
    this.set({ log });
    return log;
  }

  runAction(index, typed) {
    const item = this.model.actions?.[index];
    if (!item) {
      return Promise.resolve();
    }
    return this.attempt(async () => {
      let log = await this.refreshLog();
      const planned = plan(item.action, log.state);
      let proposed = false;
      if (planned.type === "approve") {
        if (!fingerprintMatches(typed ?? "", await recordFingerprint(planned.device))) {
          throw Object.assign(new Error(this.t("approve-mismatch")), { localized: true });
        }
        const result = await appendFresh(this.api, log, this.device, "device.add", { device: planned.device });
        log = result.log;
        proposed = result.proposal !== null;
      } else if (planned.type === "append") {
        for (const entry of planned.entries) {
          const result = await appendFresh(this.api, log, this.device, entry.kind, entry.body);
          log = result.log;
          proposed ||= result.proposal !== null;
        }
      } else if (planned.type === "cosign") {
        await cosignCheckpoint(this.api, log, this.device, this.serverKey, planned.seq);
      } else {
        return null;
      }
      this.set({ log, fingerprints: await this.fingerprints(log) });
      await this.loadActions();
      return this.t(proposed ? "action-proposed" : "action-done");
    });
  }

  cosign(id) {
    const proposal = this.model.proposals.find((candidate) => candidate.id === id);
    if (!proposal) {
      return Promise.resolve();
    }
    return this.attempt(async () => {
      const current = await this.refreshLog();
      const log = await cosignProposal(this.api, current, this.device, proposal);
      this.set({ log });
      await this.loadActions();
      return this.t("action-done");
    });
  }

  loadReviews() {
    const org = this.model.org.id;
    return this.guard(async () => {
      try {
        const { reviews } = await this.api.get(`/v1/orgs/${org}/reviews`);
        this.set({ reviews, reviewsError: null });
      } catch (error) {
        if (unauthorized(error)) {
          throw error;
        }
        this.set({ reviews: [], reviewsError: error });
      }
    });
  }

  decide(review, principal, decision) {
    const org = this.model.org.id;
    return this.attempt(async () => {
      await this.api.post(`/v1/orgs/${org}/reviews/${review}/decisions`, { principal, decision });
      await this.loadReviews();
      return this.t("action-done");
    });
  }

  showRotation(store, user) {
    const org = this.model.org.id;
    return this.guard(async () => {
      try {
        const result = await this.api.get(`/v1/orgs/${org}/stores/${store}/rotation?user=${encodeURIComponent(user)}`);
        this.set({ rotation: { store, user, result, error: null } });
      } catch (error) {
        if (unauthorized(error)) {
          throw error;
        }
        this.set({ rotation: { store, user, result: null, error } });
      }
    });
  }

  loadAudit() {
    this.set({ audit: { state: "loading" } });
    return this.guard(async () => {
      try {
        const log = await this.refreshLog();
        const result = await verifyAudit(this.api, log, this.serverKey);
        this.set({ audit: { state: "done", result } });
      } catch (error) {
        if (unauthorized(error)) {
          throw error;
        }
        this.set({ audit: { state: "error", error } });
      }
    });
  }
}
