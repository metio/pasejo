// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// The console's screens as view trees. Each view takes a translator, the
// data it shows and the handlers it calls; none of them reaches into
// another or into the transport.

import { fieldValue, h } from "./view.js";
import { auditRows, describeAction, describeEntry, deviceRows, directory, memberRows, storeRows, teamRows } from "./orgview.js";

function button(label, onClick, attributes = {}) {
  return h("button", { type: "button", ...attributes, on: { click: onClick } }, label);
}

function table(headers, rows) {
  return h(
    "table",
    {},
    h("thead", {}, h("tr", {}, headers.map((header) => h("th", { scope: "col" }, header)))),
    h("tbody", {}, rows),
  );
}

function code(text) {
  return h("code", { class: "code" }, text);
}

export function header(t, signedIn, onSignOut) {
  return h(
    "header",
    { class: "top" },
    h("h1", {}, t("title")),
    signedIn ? h("p", { class: "muted" }, t("signed-in-as", { name: signedIn }), " ", button(t("sign-out"), onSignOut, { class: "link" })) : null,
  );
}

export function loading(t) {
  return h("main", { "aria-busy": "true" }, h("h1", {}, t("title")), h("p", { class: "muted" }, t("loading")));
}

export function failure(t, message, onRetry) {
  return h(
    "main",
    {},
    h("h1", {}, t("title")),
    h("p", { class: "error", role: "alert" }, message),
    onRetry ? button(t("retry"), onRetry) : null,
  );
}

export function storageUnavailable(t) {
  return h("main", {}, h("h1", {}, t("title")), h("p", { class: "error", role: "alert" }, t("storage-unavailable")));
}

export function signIn(t) {
  return h(
    "main",
    {},
    h("h1", {}, t("title")),
    h("h2", {}, t("sign-in-title")),
    h("p", {}, t("sign-in-intro")),
    h("p", {}, h("a", { class: "button", href: "/auth/web/start" }, t("sign-in"))),
    h("p", { class: "muted" }, h("a", { href: "break-glass.html" }, t("break-glass-link"))),
  );
}

export function organizations(t, me, onOpen, onSignOut) {
  return h(
    "main",
    {},
    header(t, me.name || me.email, onSignOut),
    h("h2", {}, t("orgs-title")),
    me.orgs.length === 0
      ? h("p", {}, t("no-orgs"))
      : h(
          "ul",
          { class: "list" },
          me.orgs.map((org) => h("li", {}, h("span", {}, org.name), " ", button(t("org-open"), () => onOpen(org)))),
        ),
  );
}

export function confirmOrganization(t, org, fingerprint, onConfirm, onCancel) {
  return h(
    "main",
    {},
    h("h1", {}, t("title")),
    h("h2", {}, t("confirm-title")),
    h("p", {}, t("confirm-intro", { org: org.name })),
    h("p", {}, code(fingerprint)),
    h("p", { class: "actions" }, button(t("confirm-button"), onConfirm, { class: "primary" }), " ", button(t("cancel"), onCancel)),
  );
}

export function pending(t, org, device, fingerprint, onCheck) {
  return h(
    "main",
    {},
    h("h1", {}, t("title")),
    h("h2", {}, t("pending-title")),
    h("p", {}, t("pending-intro", { org: org.name })),
    h("p", {}, code(fingerprint)),
    h("p", { class: "muted" }, t("pending-cli")),
    h("pre", {}, `pasejo org approve ${device} --fingerprint "${fingerprint}"`),
    button(t("check-again"), onCheck),
  );
}

export const TABS = ["members", "teams", "devices", "stores", "actions", "reviews", "audit"];

function tabs(t, current, onSelect) {
  return h(
    "nav",
    { class: "tabs", "aria-label": t("title") },
    TABS.map((tab) =>
      button(t(`tab-${tab}`), () => onSelect(tab), {
        class: tab === current ? "tab current" : "tab",
        "aria-current": tab === current ? "page" : false,
      }),
    ),
  );
}

const VERIFICATION_ERRORS = new Set(["ProtocolError", "Rejection", "EncodingError", "DecryptionError"]);

/** The message for an error: refused access, failed verification or anything else. */
export function errorText(t, error) {
  if (error.localized) {
    return error.message;
  }
  if (error.status === 403) {
    return t("forbidden");
  }
  if (VERIFICATION_ERRORS.has(error.name)) {
    return t("verification-failed", { message: error.message });
  }
  return t("error", { message: error.message });
}

function sectionError(t, error) {
  return error ? h("p", { class: "error", role: "alert" }, errorText(t, error)) : null;
}

function notice(model) {
  if (!model.notice) {
    return null;
  }
  return h("p", { class: model.notice.error ? "notice error" : "notice ok", role: model.notice.error ? "alert" : "status" }, model.notice.text);
}

function membersTab(t, model) {
  const rows = memberRows(model.log.state, model.users);
  return table(
    [t("col-name"), t("col-email"), t("col-role"), t("col-devices")],
    rows.map((row) => h("tr", {}, h("td", {}, row.name), h("td", {}, row.email), h("td", {}, t(`role-${row.role}`)), h("td", {}, row.devices))),
  );
}

function teamsTab(t, model) {
  const rows = teamRows(model.log.state, model.users);
  if (rows.length === 0) {
    return h("p", {}, t("no-teams"));
  }
  return table(
    [t("col-name"), t("col-members")],
    rows.map((row) => h("tr", {}, h("td", {}, row.name), h("td", {}, row.members.join(", ")))),
  );
}

function devicesTab(t, model) {
  const rows = deviceRows(model.log.state, model.users, model.fingerprints, model.device);
  return table(
    [t("col-label"), t("col-user"), t("col-kind"), t("col-state"), t("col-fingerprint")],
    rows.map((row) =>
      h(
        "tr",
        { class: row.active ? "" : "inactive" },
        h("td", {}, row.label, row.mine ? h("span", { class: "muted" }, ` (${t("this-device")})`) : null),
        h("td", {}, row.user),
        h("td", {}, t(`kind-${row.kind}`)),
        h("td", {}, t(row.active ? "device-active" : "device-revoked")),
        h("td", {}, code(row.fingerprint)),
      ),
    ),
  );
}

function rotationPanel(t, model, store, handlers) {
  const rotation = model.rotation?.store === store ? model.rotation : null;
  const objects = model.objects.get(store) ?? new Map();
  return h(
    "details",
    { open: Boolean(rotation) },
    h("summary", {}, t("rotation-title")),
    h("p", { class: "muted" }, t("rotation-intro")),
    h(
      "form",
      {
        on: {
          submit: (event) => {
            event.preventDefault();
            return handlers.showRotation(store, fieldValue(event, "user"));
          },
        },
      },
      h("label", {}, t("rotation-user"), " ", h("select", { name: "user" }, model.users.map((user) => h("option", { value: user.id, selected: rotation?.user === user.id }, directory([user]).get(user.id))))),
      " ",
      h("button", { type: "submit" }, t("rotation-show")),
    ),
    rotation?.error ? sectionError(t, rotation.error) : null,
    rotation?.result
      ? h(
          "div",
          {},
          h("p", {}, rotation.result.ended === null ? t("rotation-ongoing") : t("rotation-ended", { date: t.date(rotation.result.ended) })),
          rotation.result.items.length === 0
            ? h("p", {}, t("rotation-empty"))
            : h(
                "ul",
                {},
                rotation.result.items.map((item) =>
                  h(
                    "li",
                    {},
                    objects.get(item.id)?.name ?? item.id,
                    " — ",
                    t(item.rewritten ? "rotation-rewritten" : "rotation-pending"),
                    item.fetched ? `, ${t("rotation-fetched")}` : "",
                  ),
                ),
              ),
        )
      : null,
  );
}

function storesTab(t, model, handlers) {
  const rows = storeRows(model.log.state, model.users, model.labels, model.now);
  if (rows.length === 0) {
    return h("p", {}, t("no-stores"));
  }
  return h(
    "div",
    { class: "cards" },
    rows.map((row) => {
      const index = model.indexes.get(row.id);
      return h(
        "section",
        { class: row.deleted ? "card inactive" : "card" },
        h("h3", {}, row.label ?? code(row.id)),
        h(
          "p",
          { class: "muted" },
          row.deleted ? t("store-deleted") : row.epoch === null ? t("store-no-keys") : t("store-epoch", { epoch: row.epoch }),
          index ? ` · ${t("store-entries", { secrets: index.secrets.size, otp: index.otp.size })}` : "",
        ),
        row.label === null && !row.deleted ? h("p", { class: "muted" }, t("store-unreadable")) : null,
        model.storeErrors.get(row.id) ? sectionError(t, model.storeErrors.get(row.id)) : null,
        h(
          "ul",
          { class: "grants" },
          row.grants.map((grant) =>
            h(
              "li",
              { class: grant.expired ? "inactive" : "" },
              grant.team ? t("team-label", { name: grant.name }) : grant.name,
              ": ",
              t(`store-role-${grant.role}`),
              grant.expires === null ? "" : ` (${t(grant.expired ? "grant-expired" : "grant-until", { date: t.date(grant.expires) })})`,
            ),
          ),
        ),
        row.deleted ? null : rotationPanel(t, model, row.id, handlers),
      );
    }),
  );
}

function entryText(t, description) {
  const args = { ...description.args };
  if (args.role) {
    args.role = t(`role-${args.role}`) === `role-${args.role}` ? t(`store-role-${args.role}`) : t(`role-${args.role}`);
  }
  return t(description.key, args);
}

function actionsTab(t, model, handlers) {
  const names = directory(model.users);
  const state = model.log.state;
  const items = model.actions ?? [];
  return h(
    "div",
    {},
    h("p", { class: "muted" }, t("actions-intro")),
    sectionError(t, model.actionsError),
    items.length === 0 && !model.actionsError ? h("p", {}, t("no-actions")) : null,
    h(
      "ul",
      { class: "list" },
      items.map((item, index) => {
        const planned = item.plan;
        return h(
          "li",
          {},
          h("p", {}, entryText(t, describeAction(item.action, state, names, model.labels))),
          planned.type === "approve"
            ? h(
                "form",
                {
                  on: {
                    submit: (event) => {
                      event.preventDefault();
                      return handlers.runAction(index, fieldValue(event, "fingerprint"));
                    },
                  },
                },
                h("label", {}, t("approve-fingerprint"), " ", h("input", { name: "fingerprint", autocomplete: "off", spellcheck: "false", required: true })),
                " ",
                h("button", { type: "submit" }, t("approve-button")),
              )
            : null,
          planned.type === "append" || planned.type === "cosign" ? button(t("action-run"), () => handlers.runAction(index)) : null,
          planned.type === "needs-keys" ? h("p", { class: "muted" }, t("action-needs-keys")) : null,
          planned.type === "skip" ? h("p", { class: "muted" }, t(planned.reason)) : null,
        );
      }),
    ),
    model.proposals?.length
      ? h(
          "section",
          {},
          h("h3", {}, t("proposals-title")),
          h(
            "ul",
            { class: "list" },
            model.proposals.map((proposal) =>
              h(
                "li",
                {},
                h("p", {}, entryText(t, describeEntry(proposal.entry, state, names, model.labels))),
                button(t("proposal-cosign"), () => handlers.cosign(proposal.id)),
              ),
            ),
          ),
        )
      : null,
  );
}

function reviewsTab(t, model, handlers) {
  const names = directory(model.users);
  const state = model.log.state;
  if (model.reviewsError) {
    return sectionError(t, model.reviewsError);
  }
  const reviews = model.reviews ?? [];
  if (reviews.length === 0) {
    return h("p", {}, t("no-reviews"));
  }
  return h(
    "div",
    {},
    reviews.map((review) => {
      const reviewer = review.reviewers.includes(model.user) && !review.complete;
      return h(
        "section",
        { class: "card" },
        h("h3", {}, t("review-title", { store: model.labels.get(review.store) ?? review.store, date: t.day(review.due) })),
        h("p", { class: "muted" }, t(review.complete ? "review-complete" : "review-open")),
        table(
          [t("col-principal"), t("col-role"), t("col-decision")],
          review.items.map((item) => {
            const holder = item.principal.user ? (names.get(item.principal.user) ?? item.principal.user) : t("team-label", { name: state.teams.get(item.principal.team)?.name ?? item.principal.team });
            return h(
              "tr",
              {},
              h("td", {}, holder),
              h("td", {}, t(`store-role-${item.role}`)),
              h(
                "td",
                {},
                item.decision
                  ? t("decision-by", { decision: t(`decision-${item.decision}`), user: names.get(item.decided_by) ?? item.decided_by ?? "" })
                  : reviewer
                    ? [
                        button(t("decision-keep"), () => handlers.decide(review.id, item.principal, "keep")),
                        " ",
                        button(t("decision-revoke"), () => handlers.decide(review.id, item.principal, "revoke"), { class: "danger" }),
                      ]
                    : t("decision-pending"),
              ),
            );
          }),
        ),
      );
    }),
  );
}

function auditTab(t, model, handlers) {
  const audit = model.audit;
  const names = directory(model.users);
  const objects = new Map([...model.objects.values()].flatMap((map) => [...map]));
  return h(
    "div",
    {},
    button(t("audit-load"), handlers.loadAudit, { disabled: audit.state === "loading" }),
    audit.state === "loading" ? h("p", { class: "muted" }, t("loading")) : null,
    audit.state === "error"
      ? h("p", { class: "error", role: "alert" }, audit.error.status === 403 ? t("audit-forbidden") : errorText(t, audit.error))
      : null,
    audit.state === "done"
      ? h(
          "div",
          {},
          h("p", { class: "ok" }, t("audit-verified", { events: audit.result.events.length, checkpoints: audit.result.checkpoints, cosignatures: audit.result.cosignatures })),
          ...model.log.offlineReadPeriods().map((period) =>
            h(
              "p",
              { class: "notice" },
              period.until === null
                ? t("audit-offline-reads-open", { from: t.date(period.from) })
                : t("audit-offline-reads-closed", { from: t.date(period.from), until: t.date(period.until) }),
            ),
          ),
          audit.result.events.length === 0
            ? h("p", {}, t("audit-empty"))
            : table(
                [t("col-time"), t("col-actor"), t("col-action"), t("col-store"), t("col-object"), t("col-outcome"), t("col-detail")],
                auditRows(audit.result.events, names, model.labels, objects).map((row) =>
                  h(
                    "tr",
                    {},
                    h("td", {}, t.date(row.time)),
                    h("td", {}, row.actor),
                    h("td", {}, code(row.action)),
                    h("td", {}, row.store),
                    h("td", {}, row.object),
                    h("td", {}, row.outcome),
                    h("td", {}, row.detail),
                  ),
                ),
              ),
        )
      : null,
  );
}

const TAB_VIEWS = {
  members: membersTab,
  teams: teamsTab,
  devices: devicesTab,
  stores: storesTab,
  actions: actionsTab,
  reviews: reviewsTab,
  audit: auditTab,
};

export function organization(t, model, handlers) {
  return h(
    "main",
    { class: "wide" },
    header(t, model.userName, handlers.signOut),
    h("p", {}, button(`← ${t("orgs-title")}`, handlers.back, { class: "link" })),
    h("h2", {}, model.log.state.name),
    model.log.state.settings.device_approval === "automatic" ? h("p", { class: "notice error" }, t("org-automatic-approval")) : null,
    model.log.state.settings.offline_reads ? h("p", { class: "notice error" }, t("org-offline-reads")) : null,
    sectionError(t, model.usersError),
    tabs(t, model.tab, handlers.selectTab),
    notice(model),
    h("section", { class: "panel" }, TAB_VIEWS[model.tab](t, model, handlers)),
  );
}
