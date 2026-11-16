// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// A small key-value store over IndexedDB. Values are structured clones, so
// non-extractable CryptoKeys stay non-extractable when stored.

export function openStore(name = "pasejo-console", indexedDB = globalThis.indexedDB) {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(name, 1);
    request.onupgradeneeded = () => request.result.createObjectStore("values");
    request.onerror = () => reject(request.error);
    request.onsuccess = () => {
      const database = request.result;
      const run = (mode, action) =>
        new Promise((done, fail) => {
          const transaction = database.transaction("values", mode);
          const operation = action(transaction.objectStore("values"));
          transaction.oncomplete = () => done(operation.result);
          transaction.onerror = () => fail(transaction.error);
        });
      resolve({
        get: (key) => run("readonly", (values) => values.get(key)),
        put: (key, value) => run("readwrite", (values) => values.put(value, key)),
        delete: (key) => run("readwrite", (values) => values.delete(key)),
      });
    };
  });
}

