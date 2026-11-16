// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// The break-glass page: a local account for server operators while single
// sign-on is down. Its session is bound to a signing key generated for this
// page load and kept only in memory, so reloading the page signs out.

import { ApiError } from "./api.js";
import { b64 } from "./encoding.js";
import { verifyChain } from "./audit.js";
import { auditEvents } from "./session.js";
import { userDisplay } from "./orgview.js";
import { fieldValue, h } from "./view.js";
import { errorText } from "./views.js";

const HOUR = 3_600_000;
export const MAX_HOURS = 24;

/** A fresh, non-extractable signing key for one break-glass session. */
export async function sessionKey() {
  const pair = await globalThis.crypto.subtle.generateKey({ name: "Ed25519" }, false, ["sign", "verify"]);
  return {
    privateKey: pair.privateKey,
    publicKey: new Uint8Array(await globalThis.crypto.subtle.exportKey("raw", pair.publicKey)),
  };
}

function field(label, name, attributes = {}) {
  return h("p", {}, h("label", {}, label, h("br", {}), h("input", { name, required: true, ...attributes })));
}

function button(label, onClick, attributes = {}) {
  return h("button", { type: "button", ...attributes, on: { click: onClick } }, label);
}

export function loginView(t, model, onLogin) {
  return h(
    "main",
    {},
    h("h1", {}, t("bg-title")),
    h("p", {}, t("bg-intro")),
    model.notice ? h("p", { class: "error", role: "alert" }, model.notice) : null,
    h(
      "form",
      {
        on: {
          submit: (event) => {
            event.preventDefault();
            return onLogin(fieldValue(event, "username"), fieldValue(event, "password"), fieldValue(event, "totp"));
          },
        },
      },
      field(t("bg-username"), "username", { autocomplete: "username", value: model.username ?? "break-glass" }),
      field(t("bg-password"), "password", { type: "password", autocomplete: "current-password" }),
      field(t("bg-totp"), "totp", { inputmode: "numeric", autocomplete: "one-time-code", pattern: "[0-9]{6,8}" }),
      h("button", { type: "submit", class: "primary", disabled: model.busy === true }, t("bg-login")),
    ),
    h("p", { class: "muted" }, h("a", { href: "./" }, t("bg-back"))),
  );
}

export function dashboardView(t, model, handlers) {
  const open = model.emergencyUntil !== null && model.emergencyUntil > model.now;
  return h(
    "main",
    { class: "wide" },
    h("h1", {}, t("bg-title")),
    model.notice ? h("p", { class: model.noticeError ? "notice error" : "notice ok", role: model.noticeError ? "alert" : "status" }, model.notice) : null,
    h(
      "section",
      { class: "card" },
      h("h2", {}, t("bg-emergency-title")),
      h("p", { class: "muted" }, t("bg-emergency-intro")),
      h("p", {}, open ? t("bg-emergency-open", { date: t.date(model.emergencyUntil) }) : t("bg-emergency-closed")),
      h(
        "form",
        {
          on: {
            submit: (event) => {
              event.preventDefault();
              return handlers.openWindow(Number(fieldValue(event, "hours")));
            },
          },
        },
        h("label", {}, t("bg-hours"), " ", h("input", { name: "hours", type: "number", min: "1", max: String(MAX_HOURS), value: "1", required: true })),
        " ",
        h("button", { type: "submit" }, t("bg-open")),
        " ",
        open ? button(t("bg-close"), handlers.closeWindow, { class: "danger" }) : null,
      ),
    ),
    h(
      "section",
      { class: "card" },
      h("h2", {}, t("bg-users")),
      h(
        "table",
        {},
        h("thead", {}, h("tr", {}, h("th", { scope: "col" }, t("col-name")), h("th", { scope: "col" }, t("col-email")), h("th", { scope: "col" }, t("col-state")), h("th", { scope: "col" }, t("col-action")))),
        h(
          "tbody",
          {},
          model.users.map((user) =>
            h(
              "tr",
              { class: user.active ? "" : "inactive" },
              h("td", {}, userDisplay(user)),
              h("td", {}, user.email),
              h("td", {}, t(user.active ? "bg-user-active" : "bg-user-suspended")),
              h(
                "td",
                {},
                user.active
                  ? button(t("bg-suspend"), () => handlers.suspend(user), { class: "danger" })
                  : button(t("bg-reactivate"), () => handlers.reactivate(user)),
                " ",
                button(t("bg-end-sessions"), () => handlers.endSessions(user)),
              ),
            ),
          ),
        ),
      ),
    ),
    h("section", { class: "card" }, h("h2", {}, t("bg-orgs")), h("ul", {}, model.orgs.map((org) => h("li", {}, org.name, " ", h("code", { class: "code" }, org.id))))),
    h(
      "section",
      { class: "card" },
      h("h2", {}, t("bg-audit")),
      button(t("audit-load"), handlers.loadAudit),
      model.audit
        ? h(
            "div",
            {},
            model.audit.verified
              ? h("p", { class: "ok" }, t("audit-verified", { events: model.audit.events.length, checkpoints: 0, cosignatures: 0 }))
              : null,
            model.audit.events.length === 0 ? h("p", {}, t("audit-empty")) : null,
            h(
              "table",
              {},
              h("thead", {}, h("tr", {}, [t("col-time"), t("col-action"), t("col-outcome"), t("col-detail")].map((header) => h("th", { scope: "col" }, header)))),
              h(
                "tbody",
                {},
                [...model.audit.events].reverse().map((event) =>
                  h("tr", {}, h("td", {}, t.date(event.time)), h("td", {}, h("code", { class: "code" }, event.action)), h("td", {}, event.outcome), h("td", {}, event.detail ?? "")),
                ),
              ),
            ),
          )
        : null,
    ),
    h("p", { class: "muted" }, h("a", { href: "./" }, t("bg-back"))),
  );
}

export class BreakGlass {
  constructor({ t, api, key, show, now = () => Date.now() }) {
    this.t = t;
    this.api = api;
    this.key = key;
    this.show = show;
    this.now = now;
    this.model = { screen: "login" };
    this.handlers = {
      openWindow: (hours) => this.emergency(hours),
      closeWindow: () => this.emergency(0),
      suspend: (user) => this.userAction(user, "suspend"),
      reactivate: (user) => this.userAction(user, "reactivate"),
      endSessions: (user) => this.userAction(user, "sessions/revoke"),
      loadAudit: () => this.loadAudit(),
    };
  }

  set(changes) {
    Object.assign(this.model, changes);
    this.show(this.view());
  }

  view() {
    return this.model.screen === "login"
      ? loginView(this.t, this.model, (username, password, totp) => this.login(username, password, totp))
      : dashboardView(this.t, this.model, this.handlers);
  }

  start() {
    this.set({});
  }

  async login(username, password, totp) {
    this.set({ busy: true, notice: null, username });
    try {
      const answer = await this.api.anonymous("POST", "/v1/break-glass/login", {
        username,
        password,
        totp,
        signing_key: b64(this.key.publicKey),
      });
      this.api.session = { token: answer.token, expires: answer.expires, user: null };
      await this.load();
    } catch (error) {
      this.set({ screen: "login", busy: false, notice: errorText(this.t, error) });
    }
  }

  async load() {
    const [server, { users }, { orgs }] = await Promise.all([
      this.api.anonymous("GET", "/v1/server"),
      this.api.get("/v1/break-glass/users"),
      this.api.get("/v1/break-glass/orgs"),
    ]);
    this.set({ screen: "dashboard", busy: false, emergencyUntil: server.emergency_until ?? null, users, orgs, now: this.now(), audit: this.model.audit ?? null });
  }

  /** Runs `step`; a lost session returns to the login form, other errors become the notice. */
  async attempt(step) {
    try {
      const notice = await step();
      this.set({ notice: notice ?? null, noticeError: false });
    } catch (error) {
      if (error instanceof ApiError && error.status === 401) {
        this.api.session = null;
        this.model = { screen: "login" };
        this.set({ notice: errorText(this.t, error) });
        return;
      }
      this.set({ notice: errorText(this.t, error), noticeError: true });
    }
  }

  emergency(hours) {
    return this.attempt(async () => {
      if (!(Number.isInteger(hours) && hours >= 0 && hours <= MAX_HOURS)) {
        throw Object.assign(new Error(this.t("bg-hours")), { localized: true });
      }
      const state = await this.api.post("/v1/break-glass/emergency", { duration: hours * HOUR });
      this.set({ emergencyUntil: state.until ?? null, now: this.now() });
      return null;
    });
  }

  userAction(user, action) {
    return this.attempt(async () => {
      await this.api.post(`/v1/break-glass/users/${user.id}/${action}`);
      await this.load();
      return action === "sessions/revoke" ? this.t("bg-sessions-ended", { name: userDisplay(user) }) : null;
    });
  }

  loadAudit() {
    return this.attempt(async () => {
      const events = await auditEvents(this.api, "/v1/audit", 0);
      // The API serves no checkpoints of the server chain, so only a chain
      // kept from its first event can be verified here.
      const verified = events.length > 0 && events[0].seq === 1;
      if (verified) {
        await verifyChain(b64(new Uint8Array(32)), 1, events);
      }
      this.set({ audit: { events, verified } });
      return null;
    });
  }
}
