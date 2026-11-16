// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// Starts the break-glass page.

import { Api } from "./api.js";
import { BreakGlass, sessionKey } from "./break-glass.js";
import { chooseLanguage, translator } from "./i18n.js";
import { mount } from "./view.js";

const t = translator(chooseLanguage(navigator.languages));
document.documentElement.lang = t.language;
document.title = t("bg-title");
const root = document.getElementById("app");
const key = await sessionKey();
const api = new Api({ signingKey: key.privateKey });
new BreakGlass({ t, api, key, show: (tree) => mount(root, tree, document) }).start();
