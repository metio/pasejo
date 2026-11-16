// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// The console follows the browser's language preference among the CLI's
// eight languages and falls back to English for anything missing.

import de from "./messages/de.js";
import en from "./messages/en.js";
import es from "./messages/es.js";
import fr from "./messages/fr.js";
import it from "./messages/it.js";
import ja from "./messages/ja.js";
import ko from "./messages/ko.js";
import zh from "./messages/zh.js";

export const MESSAGES = { en, de, es, fr, it, ja, ko, zh };

/** The first supported language among `preferences`, else English. */
export function chooseLanguage(preferences) {
  for (const preference of preferences ?? []) {
    const base = String(preference).toLowerCase().split(/[-_]/)[0];
    if (Object.hasOwn(MESSAGES, base)) {
      return base;
    }
  }
  return "en";
}

/** A translator for `language`: `t(key, {name: value})`. */
export function translator(language) {
  const messages = MESSAGES[language] ?? en;
  const t = (key, args = {}) => {
    const template = messages[key] ?? en[key] ?? key;
    return template.replace(/\{(\w+)\}/g, (match, name) => (Object.hasOwn(args, name) ? String(args[name]) : match));
  };
  t.language = language;
  t.date = (millis) =>
    new Intl.DateTimeFormat(language, { dateStyle: "medium", timeStyle: "short" }).format(new Date(millis));
  t.day = (millis) => new Intl.DateTimeFormat(language, { dateStyle: "medium" }).format(new Date(millis));
  return t;
}
