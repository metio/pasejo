// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// Starts the console in the page.

import { Api } from "./api.js";
import { Console } from "./app.js";
import { browserName, loadOrCreateDevice } from "./devicekeys.js";
import { chooseLanguage, translator } from "./i18n.js";
import { openStore } from "./kv.js";
import { mount } from "./view.js";
import { storageUnavailable } from "./views.js";

const t = translator(chooseLanguage(navigator.languages));
document.documentElement.lang = t.language;
document.title = t("title");
const root = document.getElementById("app");
const show = (tree) => mount(root, tree, document);

let kv;
try {
  kv = await openStore();
} catch {
  kv = null;
}
if (kv === null) {
  show(storageUnavailable(t));
} else {
  const device = await loadOrCreateDevice(kv, t("device-label", { browser: browserName(navigator.userAgent) }));
  const api = new Api({ signingKey: device.signingKey });
  await new Console({ t, api, kv, device, show }).start();
}
