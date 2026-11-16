// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { describe, test } from "node:test";
import { MESSAGES, chooseLanguage, translator } from "../js/i18n.js";

const placeholders = (text) => [...text.matchAll(/\{(\w+)\}/g)].map((match) => match[1]).sort();
const english = MESSAGES.en;

describe("messages", () => {
  test("ships the CLI's eight languages", () => {
    assert.deepEqual(Object.keys(MESSAGES).sort(), ["de", "en", "es", "fr", "it", "ja", "ko", "zh"]);
  });

  for (const [language, messages] of Object.entries(MESSAGES)) {
    test(`${language} defines exactly the English keys with the same placeholders`, () => {
      assert.deepEqual(Object.keys(messages).sort(), Object.keys(english).sort());
      for (const [key, text] of Object.entries(messages)) {
        assert.equal(typeof text, "string", key);
        assert.notEqual(text.trim(), "", key);
        assert.deepEqual(placeholders(text), placeholders(english[key]), `${language} ${key}`);
      }
    });
  }

  test("every key the code names literally exists in English", () => {
    const directory = new URL("../js/", import.meta.url);
    const sources = readdirSync(directory)
      .filter((name) => name.endsWith(".js"))
      .map((name) => readFileSync(new URL(name, directory), "utf8"))
      .join("\n");
    const literal = [...sources.matchAll(/\bt\("([a-z0-9-]+)"/g)].map((match) => match[1]);
    const keyed = [...sources.matchAll(/\bkey: "([a-z0-9-]+)"/g)].map((match) => match[1]);
    const reasons = [...sources.matchAll(/reason: "([a-z0-9-]+)"/g)].map((match) => match[1]);
    const missing = [...literal, ...keyed, ...reasons].filter((key) => !Object.hasOwn(english, key));
    assert.deepEqual(missing, []);
    assert.ok(literal.length > 80);
  });

  test("every role, kind, tab and decision has a message", () => {
    for (const role of ["admin", "approver", "auditor", "member"]) {
      assert.ok(english[`role-${role}`]);
    }
    for (const role of ["reader", "writer", "admin"]) {
      assert.ok(english[`store-role-${role}`]);
    }
    for (const kind of ["age", "hpke"]) {
      assert.ok(english[`kind-${kind}`]);
    }
    for (const tab of ["members", "teams", "devices", "stores", "actions", "reviews", "audit"]) {
      assert.ok(english[`tab-${tab}`]);
    }
    for (const decision of ["keep", "revoke", "pending"]) {
      assert.ok(english[`decision-${decision}`]);
    }
  });
});

describe("choosing a language", () => {
  test("takes the first supported preference by its base language", () => {
    assert.equal(chooseLanguage(["pt-BR", "de-AT", "fr"]), "de");
    assert.equal(chooseLanguage(["zh_TW"]), "zh");
    assert.equal(chooseLanguage(["JA"]), "ja");
  });

  test("falls back to English", () => {
    assert.equal(chooseLanguage(["pt", "nl"]), "en");
    assert.equal(chooseLanguage([]), "en");
    assert.equal(chooseLanguage(undefined), "en");
    assert.equal(chooseLanguage(["__proto__"]), "en");
  });
});

describe("translating", () => {
  test("fills placeholders and leaves unknown ones visible", () => {
    const t = translator("de");
    assert.equal(t("signed-in-as", { name: "Ada" }), "Angemeldet als Ada");
    assert.equal(t("signed-in-as"), "Angemeldet als {name}");
  });

  test("an argument cannot inject a further placeholder", () => {
    const t = translator("en");
    assert.equal(t("action-join-member", { name: "{email}", email: "x" }), "Add {email} (x) as a member");
  });

  test("an unknown key shows the key", () => {
    assert.equal(translator("fr")("no-such-key"), "no-such-key");
  });

  test("an unknown language translates as English", () => {
    const t = translator("pt");
    assert.equal(t("sign-in"), "Sign in");
  });

  test("formats dates in the chosen language", () => {
    const t = translator("de");
    const day = t.day(Date.UTC(2026, 10, 16, 12));
    assert.match(day, /2026/);
    assert.equal(t.language, "de");
  });
});
