// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// The server's HTTP API. Authenticated requests carry a proof signed with
// the device key, so a session token is useless without the key that
// claimed it.

import { fromUtf8, utf8 } from "./encoding.js";
import { prove } from "./request.js";

export class ApiError extends Error {
  constructor(status, code, message, currentVersion = null) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.code = code;
    this.currentVersion = currentVersion;
  }
}

function apiError(status, bytes) {
  try {
    const body = JSON.parse(fromUtf8(bytes));
    if (typeof body?.code === "string" && typeof body.message === "string") {
      return new ApiError(status, body.code, body.message, body.current_version ?? null);
    }
  } catch {
    // Not an error body; reported with its text below.
  }
  let text = "";
  try {
    text = fromUtf8(bytes).slice(0, 200);
  } catch {
    text = `HTTP ${status}`;
  }
  return new ApiError(status, null, text || `HTTP ${status}`);
}

export class Api {
  /**
   * `signingKey` is the device's private Ed25519 CryptoKey; `fetch` and
   * `now` are injectable for tests.
   */
  constructor({ base = "", signingKey, fetch = globalThis.fetch.bind(globalThis), now = () => Date.now() }) {
    this.base = base;
    this.signingKey = signingKey;
    this.fetch = fetch;
    this.now = now;
    this.session = null;
  }

  async request(method, path, body, { authenticated = true } = {}) {
    const bytes = body === undefined ? new Uint8Array(0) : utf8(JSON.stringify(body));
    const headers = {};
    if (body !== undefined) {
      headers["content-type"] = "application/json";
    }
    if (authenticated) {
      if (!this.session) {
        throw new ApiError(401, "unauthorized", "no session");
      }
      Object.assign(headers, await prove(method, path, this.now(), bytes, this.session.token, this.signingKey));
    }
    let response;
    try {
      response = await this.fetch(this.base + path, {
        method,
        headers,
        body: body === undefined ? undefined : bytes,
        credentials: "same-origin",
        cache: "no-store",
        redirect: "error",
      });
    } catch (error) {
      throw new ApiError(0, "unavailable", error.message);
    }
    const answer = new Uint8Array(await response.arrayBuffer());
    if (!response.ok) {
      throw apiError(response.status, answer);
    }
    if (answer.length === 0) {
      return null;
    }
    try {
      return JSON.parse(fromUtf8(answer));
    } catch {
      throw new ApiError(response.status, "malformed", "malformed answer");
    }
  }

  get(path) {
    return this.request("GET", path);
  }

  post(path, body) {
    return this.request("POST", path, body);
  }

  delete(path) {
    return this.request("DELETE", path);
  }

  anonymous(method, path, body) {
    return this.request(method, path, body, { authenticated: false });
  }
}
