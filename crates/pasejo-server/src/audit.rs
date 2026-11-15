// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The hash-chained audit log. Each organization has a chain of its own,
//! and events outside any organization go to the server's chain.

use std::fmt::Write as _;
use std::str::FromStr;
use std::sync::atomic::Ordering;
use std::time::Duration;

use hmac::{Hmac, Mac};
use pasejo_protocol::checkpoint::{AuditEvent, Checkpoint, CheckpointPayload};
use pasejo_protocol::encoding::{Bytes, Fixed};
use pasejo_protocol::envelope::{Context, Signature, sign};
use pasejo_protocol::{Hash, Id, Millis};
use sha2::Sha256;
use sqlx::Row;

use crate::db::{self, to_i64};
use crate::error::ApiResult;
use crate::state::{Actor, AppState, RequestCtx};

pub const SERVER_CHAIN: &str = "server";

/// The chain an organization's events belong to.
#[must_use]
pub fn chain(org: Option<Id>) -> String {
    org.map_or_else(|| SERVER_CHAIN.to_owned(), |org| org.to_string())
}

/// An event before the chain assigns its sequence number and hashes.
#[derive(Debug, Clone, Default)]
pub struct Event {
    pub org: Option<Id>,
    pub action: String,
    pub store: Option<Id>,
    pub object: Option<Id>,
    pub version: Option<u64>,
    pub outcome: &'static str,
    pub detail: Option<String>,
    /// The actor when no request carries one, such as SCIM changes.
    pub user: Option<Id>,
}

impl Event {
    pub fn ok(action: impl Into<String>) -> Self {
        Self {
            action: action.into(),
            outcome: "ok",
            ..Self::default()
        }
    }

    pub fn denied(action: impl Into<String>) -> Self {
        Self {
            action: action.into(),
            outcome: "denied",
            ..Self::default()
        }
    }

    #[must_use]
    pub const fn org(mut self, org: Id) -> Self {
        self.org = Some(org);
        self
    }

    #[must_use]
    pub const fn store(mut self, store: Id) -> Self {
        self.store = Some(store);
        self
    }

    #[must_use]
    pub const fn object(mut self, object: Id, version: u64) -> Self {
        self.object = Some(object);
        self.version = Some(version);
        self
    }

    #[must_use]
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    #[must_use]
    pub const fn user(mut self, user: Id) -> Self {
        self.user = Some(user);
        self
    }
}

pub struct Audit {
    lock: tokio::sync::Mutex<()>,
    stdout: bool,
    checkpoint_every: u64,
}

impl Audit {
    #[must_use]
    pub fn new(config: &crate::config::Audit) -> Self {
        Self {
            lock: tokio::sync::Mutex::new(()),
            stdout: config.stdout,
            checkpoint_every: config.checkpoint_every,
        }
    }
}

/// Records an event for the request behind `ctx`, or for no request.
///
/// # Errors
///
/// Fails when the event cannot be stored; callers must then not serve
/// what the event describes.
#[allow(clippy::too_many_lines)]
pub async fn record(
    state: &AppState,
    ctx: Option<&RequestCtx>,
    event: Event,
) -> ApiResult<AuditEvent> {
    let actor = ctx.map_or_else(Actor::default, RequestCtx::actor);
    let log_head = match event.org {
        Some(org) => state
            .logs
            .get(&state.db, org)
            .await
            .ok()
            .map(|log| log.head()),
        None => None,
    };
    let chain = chain(event.org);
    let db = &state.db;
    let _guard = state.audit.lock.lock().await;
    let mut transaction = db.begin().await?;
    let head = sqlx::query(if db.is_sqlite() {
        db.sql("SELECT seq, hash, checkpoint_seq FROM audit_heads WHERE chain = $1")
    } else {
        "SELECT seq, hash, checkpoint_seq FROM audit_heads WHERE chain = $1 FOR UPDATE"
    })
    .bind(&chain)
    .fetch_optional(&mut *transaction)
    .await?;
    let (seq, prev_hash, checkpoint_seq) = match &head {
        Some(row) => (
            db::uint(row, "seq")? + 1,
            Hash(db::bytes32(row, "hash")?),
            db::uint(row, "checkpoint_seq")?,
        ),
        None => (1, Hash::default(), 0),
    };
    let now = state.now();
    let mut audit_event = AuditEvent {
        seq,
        time: now,
        org: event.org,
        actor_user: event.user.or(actor.user),
        actor_device: actor.device,
        session_kind: actor.session_kind,
        ip: ctx.map(|ctx| ctx.ip.to_string()),
        user_agent: ctx.and_then(|ctx| ctx.user_agent.clone()),
        action: event.action,
        store: event.store,
        object: event.object,
        version: event.version,
        outcome: event.outcome.to_owned(),
        detail: event.detail,
        prev_hash,
        hash: Hash::default(),
    };
    audit_event.hash = audit_event.compute_hash();
    sqlx::query(db.sql("INSERT INTO audit_events (chain, seq, event, time, actor_user, store, object, action) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"))
        .bind(&chain)
        .bind(to_i64(seq))
        .bind(serde_json::to_string(&audit_event)?)
        .bind(to_i64(now))
        .bind(audit_event.actor_user.map(|user| user.to_string()))
        .bind(audit_event.store.map(|store| store.to_string()))
        .bind(audit_event.object.map(|object| object.to_string()))
        .bind(audit_event.action.clone())
        .execute(&mut *transaction)
        .await?;
    if head.is_some() {
        sqlx::query(db.sql("UPDATE audit_heads SET seq = $1, hash = $2 WHERE chain = $3"))
            .bind(to_i64(seq))
            .bind(audit_event.hash.0.to_vec())
            .bind(&chain)
            .execute(&mut *transaction)
            .await?;
    } else {
        sqlx::query(db.sql("INSERT INTO audit_heads (chain, seq, hash, checkpoint_seq, checkpoint_time) VALUES ($1, $2, $3, 0, $4)"))
            .bind(&chain)
            .bind(to_i64(seq))
            .bind(audit_event.hash.0.to_vec())
            .bind(to_i64(now))
            .execute(&mut *transaction)
            .await?;
    }
    if seq - checkpoint_seq >= state.audit.checkpoint_every {
        write_checkpoint(
            state,
            &mut transaction,
            &chain,
            event.org,
            seq,
            audit_event.hash,
            log_head,
            now,
        )
        .await?;
    }
    transaction.commit().await?;
    state.metrics.audit_events.fetch_add(1, Ordering::Relaxed);
    if audit_event.outcome != "ok" {
        state.metrics.refusals.fetch_add(1, Ordering::Relaxed);
    }
    if let Some(ctx) = ctx {
        ctx.mark_audited();
    }
    if state.audit.stdout
        && let Ok(line) = serde_json::to_string(&audit_event)
    {
        println!("{line}");
    }
    if audit_event.action == "break-glass.login" && audit_event.outcome == "ok" {
        state.webhook_wake.notify_one();
    }
    Ok(audit_event)
}

#[allow(clippy::too_many_arguments)]
async fn write_checkpoint(
    state: &AppState,
    transaction: &mut sqlx::Transaction<'_, sqlx::Any>,
    chain: &str,
    org: Option<Id>,
    seq: u64,
    hash: Hash,
    log_head: Option<Hash>,
    now: Millis,
) -> ApiResult<()> {
    let db = &state.db;
    let payload = serde_json::to_vec(&CheckpointPayload {
        org,
        seq,
        hash,
        log_head,
        time: now,
    })?;
    let signature = sign(Context::Checkpoint, &payload, &state.keys.checkpoint);
    sqlx::query(db.sql("INSERT INTO checkpoints (chain, seq, payload, signature, cosignatures, time) VALUES ($1, $2, $3, $4, '[]', $5)"))
        .bind(chain.to_owned())
        .bind(to_i64(seq))
        .bind(payload)
        .bind(signature.to_vec())
        .bind(to_i64(now))
        .execute(&mut **transaction)
        .await?;
    sqlx::query(
        db.sql("UPDATE audit_heads SET checkpoint_seq = $1, checkpoint_time = $2 WHERE chain = $3"),
    )
    .bind(to_i64(seq))
    .bind(to_i64(now))
    .bind(chain.to_owned())
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

/// Writes a checkpoint for every chain with events newer than its last
/// checkpoint and older than the configured interval.
///
/// # Errors
///
/// Fails when the database does.
pub async fn checkpoint_due(state: &AppState) -> ApiResult<usize> {
    let db = &state.db;
    let now = state.now();
    let interval = state.config.audit.checkpoint_interval_seconds * 1_000;
    let rows =
        sqlx::query(db.sql(
            "SELECT chain FROM audit_heads WHERE seq > checkpoint_seq AND checkpoint_time <= $1",
        ))
        .bind(to_i64(now.saturating_sub(interval)))
        .fetch_all(db.pool())
        .await?;
    let mut written = 0;
    for row in rows {
        let chain: String = row.try_get("chain")?;
        let org = Id::from_str(&chain).ok();
        let log_head = match org {
            Some(org) => state.logs.get(db, org).await.ok().map(|log| log.head()),
            None => None,
        };
        let _guard = state.audit.lock.lock().await;
        let mut transaction = db.begin().await?;
        let head = sqlx::query(
            db.sql("SELECT seq, hash, checkpoint_seq FROM audit_heads WHERE chain = $1"),
        )
        .bind(&chain)
        .fetch_one(&mut *transaction)
        .await?;
        let seq = db::uint(&head, "seq")?;
        if seq > db::uint(&head, "checkpoint_seq")? {
            let hash = Hash(db::bytes32(&head, "hash")?);
            write_checkpoint(
                state,
                &mut transaction,
                &chain,
                org,
                seq,
                hash,
                log_head,
                now,
            )
            .await?;
            written += 1;
        }
        transaction.commit().await?;
    }
    Ok(written)
}

/// Events of a chain after `since`, at most `limit`.
///
/// # Errors
///
/// Fails when the database does.
pub async fn events(
    state: &AppState,
    chain: &str,
    since: u64,
    limit: u64,
) -> ApiResult<Vec<AuditEvent>> {
    let db = &state.db;
    let rows =
        sqlx::query(db.sql(
            "SELECT event FROM audit_events WHERE chain = $1 AND seq > $2 ORDER BY seq LIMIT $3",
        ))
        .bind(chain.to_owned())
        .bind(to_i64(since))
        .bind(to_i64(limit))
        .fetch_all(db.pool())
        .await?;
    rows.iter()
        .map(|row| {
            let text: String = row.try_get("event")?;
            Ok(serde_json::from_str(&text)?)
        })
        .collect()
}

/// Checkpoints of a chain after `since`.
///
/// # Errors
///
/// Fails when the database does.
pub async fn checkpoints(
    state: &AppState,
    chain: &str,
    since: u64,
) -> ApiResult<Vec<(u64, Checkpoint)>> {
    let db = &state.db;
    let rows = sqlx::query(db.sql("SELECT seq, payload, signature, cosignatures FROM checkpoints WHERE chain = $1 AND seq > $2 ORDER BY seq"))
        .bind(chain.to_owned())
        .bind(to_i64(since))
        .fetch_all(db.pool())
        .await?;
    rows.iter()
        .map(|row| {
            let signature: Vec<u8> = row.try_get("signature")?;
            let cosignatures: String = row.try_get("cosignatures")?;
            Ok((
                db::uint(row, "seq")?,
                Checkpoint {
                    payload: Bytes(row.try_get("payload")?),
                    server_signature: Fixed(
                        signature
                            .try_into()
                            .map_err(|_| anyhow::anyhow!("malformed checkpoint signature"))?,
                    ),
                    cosignatures: serde_json::from_str(&cosignatures)?,
                },
            ))
        })
        .collect()
}

/// Adds a co-signature to a checkpoint after verifying it.
///
/// # Errors
///
/// Fails when the checkpoint is unknown or the signature does not verify.
pub async fn cosign(
    state: &AppState,
    chain: &str,
    seq: u64,
    signature: Signature,
    key: &[u8; 32],
) -> ApiResult<()> {
    let db = &state.db;
    let _guard = state.audit.lock.lock().await;
    let mut transaction = db.begin().await?;
    let row = sqlx::query(
        db.sql("SELECT payload, cosignatures FROM checkpoints WHERE chain = $1 AND seq = $2"),
    )
    .bind(chain.to_owned())
    .bind(to_i64(seq))
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(|| crate::error::ApiError::not_found("no such checkpoint"))?;
    let payload: Vec<u8> = row.try_get("payload")?;
    pasejo_protocol::envelope::verify(Context::Checkpoint, &payload, key, &signature.signature.0)?;
    let text: String = row.try_get("cosignatures")?;
    let mut cosignatures: Vec<Signature> = serde_json::from_str(&text)?;
    cosignatures.retain(|existing| existing.device != signature.device);
    cosignatures.push(signature);
    sqlx::query(db.sql("UPDATE checkpoints SET cosignatures = $1 WHERE chain = $2 AND seq = $3"))
        .bind(serde_json::to_string(&cosignatures)?)
        .bind(chain.to_owned())
        .bind(to_i64(seq))
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    Ok(())
}

/// Removes events older than `before`, keeping every event after the last
/// checkpoint written before the cut so the rest stays verifiable.
///
/// # Errors
///
/// Fails when the database does.
pub async fn prune(state: &AppState, before: Millis) -> ApiResult<u64> {
    let db = &state.db;
    let _guard = state.audit.lock.lock().await;
    let chains = sqlx::query("SELECT chain FROM audit_heads")
        .fetch_all(db.pool())
        .await?;
    let mut removed = 0;
    for row in chains {
        let chain: String = row.try_get("chain")?;
        let cut = sqlx::query(
            db.sql("SELECT MAX(seq) AS seq FROM checkpoints WHERE chain = $1 AND time < $2"),
        )
        .bind(&chain)
        .bind(to_i64(before))
        .fetch_one(db.pool())
        .await?
        .try_get::<Option<i64>, _>("seq")?;
        let Some(cut) = cut else { continue };
        let mut transaction = db.begin().await?;
        removed += sqlx::query(
            db.sql("DELETE FROM audit_events WHERE chain = $1 AND seq <= $2 AND time < $3"),
        )
        .bind(&chain)
        .bind(cut)
        .bind(to_i64(before))
        .execute(&mut *transaction)
        .await?
        .rows_affected();
        sqlx::query(db.sql("DELETE FROM checkpoints WHERE chain = $1 AND seq < $2"))
            .bind(&chain)
            .bind(cut)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
    }
    Ok(removed)
}

type HmacSha256 = Hmac<Sha256>;

/// The webhook signature header value over `timestamp.body`.
#[must_use]
pub fn webhook_signature(secret: &[u8], timestamp: Millis, body: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret)
        .unwrap_or_else(|_| unreachable!("HMAC takes any key length"));
    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(body);
    let digest = mac.finalize().into_bytes();
    let mut text = String::from("sha256=");
    for byte in digest {
        let _ = write!(text, "{byte:02x}");
    }
    text
}

/// Delivers one batch per chain to the webhook and advances its position.
///
/// # Errors
///
/// Fails when the webhook refuses a batch; positions stay where they were.
pub async fn deliver_webhook(state: &AppState, secret: &[u8]) -> anyhow::Result<usize> {
    let Some(webhook) = &state.config.audit.webhook else {
        return Ok(0);
    };
    let db = &state.db;
    let chains = sqlx::query("SELECT chain, seq FROM audit_heads")
        .fetch_all(db.pool())
        .await?;
    let mut delivered = 0;
    for row in chains {
        let chain: String = row.try_get("chain")?;
        let position = sqlx::query(
            db.sql("SELECT seq FROM sink_positions WHERE sink = 'webhook' AND chain = $1"),
        )
        .bind(&chain)
        .fetch_optional(db.pool())
        .await?
        .map(|row| db::uint(&row, "seq"))
        .transpose()?;
        let since = position.unwrap_or(0);
        let batch = events(state, &chain, since, webhook.batch as u64)
            .await
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        let Some(last) = batch.last().map(|event| event.seq) else {
            continue;
        };
        let alert = batch
            .iter()
            .any(|event| event.action == "break-glass.login" && event.outcome == "ok");
        let body = serde_json::to_vec(
            &serde_json::json!({"chain": chain, "alert": alert, "events": batch}),
        )?;
        let timestamp = state.now();
        let response = state
            .http
            .post(webhook.url.clone())
            .header("content-type", "application/json")
            .header("pasejo-timestamp", timestamp.to_string())
            .header(
                "pasejo-signature",
                webhook_signature(secret, timestamp, &body),
            )
            .body(body)
            .send()
            .await?;
        if !response.status().is_success() {
            anyhow::bail!("webhook answered {}", response.status());
        }
        if position.is_some() {
            sqlx::query(
                db.sql("UPDATE sink_positions SET seq = $1 WHERE sink = 'webhook' AND chain = $2"),
            )
            .bind(to_i64(last))
            .bind(&chain)
            .execute(db.pool())
            .await?;
        } else {
            sqlx::query(
                db.sql("INSERT INTO sink_positions (sink, chain, seq) VALUES ('webhook', $1, $2)"),
            )
            .bind(&chain)
            .bind(to_i64(last))
            .execute(db.pool())
            .await?;
        }
        delivered += batch.len();
    }
    Ok(delivered)
}

/// Runs webhook delivery until the process ends, backing off on failure.
pub async fn webhook_loop(state: AppState) {
    let Some(webhook) = state.config.audit.webhook.clone() else {
        return;
    };
    let secret = match std::fs::read(&webhook.secret_file) {
        Ok(secret) => zeroize::Zeroizing::new(secret.trim_ascii().to_vec()),
        Err(error) => {
            tracing::error!(%error, "cannot read the webhook secret; audit webhook disabled");
            return;
        }
    };
    let mut backoff = Duration::from_secs(1);
    loop {
        match deliver_webhook(&state, &secret).await {
            Ok(delivered) => {
                backoff = Duration::from_secs(1);
                if delivered == 0 {
                    tokio::select! {
                        () = state.webhook_wake.notified() => {}
                        () = tokio::time::sleep(Duration::from_secs(5)) => {}
                    }
                }
            }
            Err(error) => {
                tracing::warn!(error = %format!("{error:#}"), "audit webhook delivery failed");
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(Duration::from_secs(300));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webhook_signatures_cover_timestamp_and_body() {
        let first = webhook_signature(b"secret", 1, b"{}");
        assert!(first.starts_with("sha256="));
        assert_eq!(first.len(), 7 + 64);
        assert_ne!(first, webhook_signature(b"secret", 2, b"{}"));
        assert_ne!(first, webhook_signature(b"secret", 1, b"{ }"));
        assert_ne!(first, webhook_signature(b"other", 1, b"{}"));
    }

    #[test]
    fn chains_are_named_after_organizations() {
        let org = Id([1; 16]);
        assert_eq!(chain(Some(org)), org.to_string());
        assert_eq!(chain(None), SERVER_CHAIN);
    }
}
