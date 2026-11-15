-- SPDX-FileCopyrightText: The pasejo Authors
-- SPDX-License-Identifier: AGPL-3.0-or-later

CREATE TABLE users (
    id TEXT PRIMARY KEY,
    issuer TEXT NOT NULL,
    subject TEXT NOT NULL,
    email TEXT NOT NULL,
    name TEXT NOT NULL,
    machine BIGINT NOT NULL,
    active BIGINT NOT NULL,
    created BIGINT NOT NULL,
    UNIQUE (issuer, subject)
);
CREATE INDEX users_email ON users (email);

CREATE TABLE orgs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created BIGINT NOT NULL
);

CREATE TABLE log_entries (
    org TEXT NOT NULL,
    seq BIGINT NOT NULL,
    envelope TEXT NOT NULL,
    hash BLOB NOT NULL,
    time BIGINT NOT NULL,
    PRIMARY KEY (org, seq)
);

CREATE TABLE proposals (
    id TEXT PRIMARY KEY,
    org TEXT NOT NULL,
    envelope TEXT NOT NULL,
    created BIGINT NOT NULL
);

CREATE TABLE join_requests (
    org TEXT NOT NULL,
    user_id TEXT NOT NULL,
    created BIGINT NOT NULL,
    PRIMARY KEY (org, user_id)
);

CREATE TABLE enrollments (
    org TEXT NOT NULL,
    device TEXT NOT NULL,
    user_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    recipient TEXT NOT NULL,
    signing_key BLOB NOT NULL,
    label TEXT NOT NULL,
    requested BIGINT NOT NULL,
    PRIMARY KEY (org, device)
);

CREATE TABLE sessions (
    token_hash BLOB PRIMARY KEY,
    user_id TEXT,
    signing_key BLOB NOT NULL,
    kind TEXT NOT NULL,
    created BIGINT NOT NULL,
    expires BIGINT NOT NULL,
    revoked BIGINT NOT NULL
);
CREATE INDEX sessions_user ON sessions (user_id);

CREATE TABLE device_grants (
    code_hash BLOB PRIMARY KEY,
    user_code TEXT NOT NULL UNIQUE,
    signing_key BLOB NOT NULL,
    label TEXT NOT NULL,
    created BIGINT NOT NULL,
    expires BIGINT NOT NULL,
    user_id TEXT,
    status TEXT NOT NULL,
    last_poll BIGINT NOT NULL
);

CREATE TABLE oidc_flows (
    state TEXT PRIMARY KEY,
    nonce TEXT NOT NULL,
    verifier TEXT NOT NULL,
    purpose TEXT NOT NULL,
    user_code TEXT,
    created BIGINT NOT NULL,
    user_id TEXT,
    approval TEXT
);

CREATE TABLE web_logins (
    cookie_hash BLOB PRIMARY KEY,
    user_id TEXT NOT NULL,
    expires BIGINT NOT NULL
);

CREATE TABLE stores (
    id TEXT PRIMARY KEY,
    org TEXT NOT NULL,
    version BIGINT NOT NULL,
    index_hash BLOB,
    lookup BLOB,
    created BIGINT NOT NULL
);

CREATE TABLE index_versions (
    store TEXT NOT NULL,
    version BIGINT NOT NULL,
    envelope TEXT NOT NULL,
    time BIGINT NOT NULL,
    author TEXT NOT NULL,
    PRIMARY KEY (store, version)
);

CREATE TABLE objects (
    store TEXT NOT NULL,
    id TEXT NOT NULL,
    version BIGINT NOT NULL,
    epoch BIGINT NOT NULL,
    ciphertext BLOB NOT NULL,
    created BIGINT NOT NULL,
    PRIMARY KEY (store, id, version)
);

CREATE TABLE wrappings (
    store TEXT NOT NULL,
    epoch BIGINT NOT NULL,
    device TEXT NOT NULL,
    full_keys BIGINT NOT NULL,
    ciphertext BLOB NOT NULL,
    uploaded_by TEXT NOT NULL,
    PRIMARY KEY (store, epoch, device)
);

CREATE TABLE chain_links (
    store TEXT NOT NULL,
    epoch BIGINT NOT NULL,
    prev_names BLOB NOT NULL,
    prev_data BLOB NOT NULL,
    PRIMARY KEY (store, epoch)
);

CREATE TABLE nonces (
    nonce TEXT PRIMARY KEY,
    expires BIGINT NOT NULL
);

CREATE TABLE audit_events (
    chain TEXT NOT NULL,
    seq BIGINT NOT NULL,
    event TEXT NOT NULL,
    time BIGINT NOT NULL,
    actor_user TEXT,
    store TEXT,
    object TEXT,
    action TEXT NOT NULL,
    PRIMARY KEY (chain, seq)
);
CREATE INDEX audit_user_store ON audit_events (chain, actor_user, store);

CREATE TABLE audit_heads (
    chain TEXT PRIMARY KEY,
    seq BIGINT NOT NULL,
    hash BLOB NOT NULL,
    checkpoint_seq BIGINT NOT NULL,
    checkpoint_time BIGINT NOT NULL
);

CREATE TABLE checkpoints (
    chain TEXT NOT NULL,
    seq BIGINT NOT NULL,
    payload BLOB NOT NULL,
    signature BLOB NOT NULL,
    cosignatures TEXT NOT NULL,
    time BIGINT NOT NULL,
    PRIMARY KEY (chain, seq)
);

CREATE TABLE sink_positions (
    sink TEXT NOT NULL,
    chain TEXT NOT NULL,
    seq BIGINT NOT NULL,
    PRIMARY KEY (sink, chain)
);

CREATE TABLE reviews (
    id TEXT PRIMARY KEY,
    org TEXT NOT NULL,
    store TEXT NOT NULL,
    reviewers TEXT NOT NULL,
    due BIGINT NOT NULL,
    created BIGINT NOT NULL,
    created_by TEXT NOT NULL,
    items TEXT NOT NULL
);

CREATE TABLE scim_tokens (
    id TEXT PRIMARY KEY,
    org TEXT NOT NULL,
    hash BLOB NOT NULL UNIQUE,
    created BIGINT NOT NULL,
    revoked BIGINT NOT NULL
);

CREATE TABLE scim_users (
    org TEXT NOT NULL,
    user_id TEXT NOT NULL,
    external_id TEXT,
    user_name TEXT NOT NULL,
    active BIGINT NOT NULL,
    deleted BIGINT NOT NULL,
    resource TEXT NOT NULL,
    created BIGINT NOT NULL,
    modified BIGINT NOT NULL,
    PRIMARY KEY (org, user_id)
);

CREATE TABLE scim_groups (
    org TEXT NOT NULL,
    id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    external_id TEXT,
    members TEXT NOT NULL,
    deleted BIGINT NOT NULL,
    created BIGINT NOT NULL,
    modified BIGINT NOT NULL,
    PRIMARY KEY (org, id)
);

CREATE TABLE break_glass (
    username TEXT PRIMARY KEY,
    password_hash TEXT NOT NULL,
    totp BLOB NOT NULL,
    failures BIGINT NOT NULL,
    locked_until BIGINT NOT NULL,
    last_step BIGINT NOT NULL,
    disabled BIGINT NOT NULL,
    created BIGINT NOT NULL
);

CREATE TABLE settings (
    name TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
