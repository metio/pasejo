// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::fmt::Write as _;
use std::net::IpAddr;
use std::path::Path;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{Context, bail};
use ed25519_dalek::SigningKey;
use hkdf::Hkdf;
use pasejo_protocol::api::ErrorCode;
use pasejo_protocol::crypto::Key;
use pasejo_protocol::encoding::unb64;
use pasejo_protocol::envelope::Envelope;
use pasejo_protocol::log::OrgLog;
use pasejo_protocol::{Hash, Id, Millis};
use sha2::Sha256;
use sqlx::Row;
use tokio::sync::{Notify, RwLock};
use zeroize::Zeroizing;

use crate::clock::Clock;
use crate::config::Config;
use crate::db::{self, Db};
use crate::error::{ApiError, ApiResult, is_unique_violation};
use crate::jwt::JwksCache;

/// Keys derived from the server's master key.
pub struct ServerKeys {
    pub checkpoint: SigningKey,
    pub totp: Key,
}

impl ServerKeys {
    #[must_use]
    pub fn derive(master: &[u8; 32]) -> Self {
        let hkdf = Hkdf::<Sha256>::new(None, master);
        let mut checkpoint = Zeroizing::new([0u8; 32]);
        let mut totp = [0u8; 32];
        // Both outputs are 32 bytes, far below HKDF's limit.
        let _ = hkdf.expand(b"pasejo-server checkpoint v1", checkpoint.as_mut());
        let _ = hkdf.expand(b"pasejo-server totp v1", &mut totp);
        Self {
            checkpoint: SigningKey::from_bytes(&checkpoint),
            totp: Key(totp),
        }
    }

    /// Reads the master key file: 32 bytes as base64url text.
    ///
    /// # Errors
    ///
    /// Fails when the file is missing or does not hold 32 bytes.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let text = Zeroizing::new(
            std::fs::read_to_string(path)
                .with_context(|| format!("cannot read master key {}", path.display()))?,
        );
        let bytes = Zeroizing::new(
            unb64(text.trim()).map_err(|_| anyhow::anyhow!("master key is not base64url"))?,
        );
        let Ok(master) = <[u8; 32]>::try_from(bytes.as_slice()) else {
            bail!("master key must hold 32 bytes");
        };
        Ok(Self::derive(&master))
    }
}

/// Who is behind a request, for audit events.
#[derive(Debug, Clone, Default)]
pub struct Actor {
    pub user: Option<Id>,
    pub device: Option<Id>,
    pub session_kind: Option<String>,
}

/// Per-request context shared between middleware, extractors and handlers.
#[derive(Debug)]
pub struct RequestCtx {
    pub ip: IpAddr,
    pub user_agent: Option<String>,
    actor: Mutex<Actor>,
    audited: AtomicBool,
}

impl RequestCtx {
    #[must_use]
    pub fn new(ip: IpAddr, user_agent: Option<String>) -> Self {
        Self {
            ip,
            user_agent,
            actor: Mutex::default(),
            audited: AtomicBool::new(false),
        }
    }

    #[must_use]
    pub fn actor(&self) -> Actor {
        self.actor
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    pub fn set_user(&self, user: Option<Id>, session_kind: &str) {
        let mut actor = self
            .actor
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        actor.user = user;
        actor.session_kind = Some(session_kind.to_owned());
    }

    pub fn set_device(&self, device: Option<Id>) {
        self.actor
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .device = device;
    }

    pub fn mark_audited(&self) {
        self.audited.store(true, Ordering::Relaxed);
    }

    #[must_use]
    pub fn audited(&self) -> bool {
        self.audited.load(Ordering::Relaxed)
    }
}

/// Counters exported at `/metrics`.
#[derive(Debug, Default)]
pub struct Metrics {
    pub requests: [AtomicU64; 5],
    pub audit_events: AtomicU64,
    pub refusals: AtomicU64,
    pub commits: AtomicU64,
    pub object_reads: AtomicU64,
}

impl Metrics {
    pub fn request(&self, status: u16) {
        let class = usize::from((status / 100).clamp(1, 5)) - 1;
        self.requests[class].fetch_add(1, Ordering::Relaxed);
    }

    #[must_use]
    pub fn render(&self) -> String {
        let mut text = String::from(
            "# HELP pasejo_requests_total HTTP requests by status class.\n# TYPE pasejo_requests_total counter\n",
        );
        for (index, counter) in self.requests.iter().enumerate() {
            let _ = writeln!(
                text,
                "pasejo_requests_total{{class=\"{}xx\"}} {}",
                index + 1,
                counter.load(Ordering::Relaxed)
            );
        }
        for (name, help, counter) in [
            (
                "pasejo_audit_events_total",
                "Audit events recorded.",
                &self.audit_events,
            ),
            ("pasejo_refusals_total", "Refused requests.", &self.refusals),
            ("pasejo_commits_total", "Accepted commits.", &self.commits),
            (
                "pasejo_object_reads_total",
                "Secret objects served.",
                &self.object_reads,
            ),
        ] {
            let _ = writeln!(
                text,
                "# HELP {name} {help}\n# TYPE {name} counter\n{name} {}",
                counter.load(Ordering::Relaxed)
            );
        }
        text
    }
}

type Windows = HashMap<(&'static str, IpAddr), (Millis, u32)>;

/// Fixed-window request counting per bucket and address.
#[derive(Debug, Default)]
pub struct RateLimiter {
    windows: Mutex<Windows>,
}

impl RateLimiter {
    /// Counts a request and tells whether it stays within `limit` per
    /// minute.
    pub fn allow(&self, bucket: &'static str, address: IpAddr, limit: u32, now: Millis) -> bool {
        let mut windows = self
            .windows
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if windows.len() > 100_000 {
            windows.retain(|_, (start, _)| now.saturating_sub(*start) < 60_000);
        }
        let entry = windows.entry((bucket, address)).or_insert((now, 0));
        if now.saturating_sub(entry.0) >= 60_000 {
            *entry = (now, 0);
        }
        entry.1 += 1;
        let allowed = entry.1 <= limit;
        drop(windows);
        allowed
    }
}

/// Replayed membership logs, refreshed when the stored head moves.
#[derive(Default)]
pub struct LogCache {
    logs: RwLock<HashMap<Id, Arc<OrgLog>>>,
    appends: Mutex<HashMap<Id, Arc<tokio::sync::Mutex<()>>>>,
}

impl LogCache {
    /// The organization's current log.
    ///
    /// # Errors
    ///
    /// Fails with `unknown_org` when it has no log.
    pub async fn get(&self, db: &Db, org: Id) -> ApiResult<Arc<OrgLog>> {
        let stored: Option<i64> =
            sqlx::query(db.sql("SELECT MAX(seq) AS seq FROM log_entries WHERE org = $1"))
                .bind(org.to_string())
                .fetch_one(db.pool())
                .await?
                .try_get("seq")?;
        let Some(stored) = stored.map(db::to_u64) else {
            return Err(ApiError::new(ErrorCode::UnknownOrg, "unknown organization"));
        };
        let cached = self.logs.read().await.get(&org).cloned();
        if let Some(log) = &cached
            && log.state().next_seq == stored + 1
        {
            return Ok(Arc::clone(log));
        }
        let after = cached
            .as_ref()
            .map_or(-1, |log| db::to_i64(log.state().next_seq) - 1);
        let rows = sqlx::query(
            db.sql("SELECT envelope FROM log_entries WHERE org = $1 AND seq > $2 ORDER BY seq"),
        )
        .bind(org.to_string())
        .bind(after)
        .fetch_all(db.pool())
        .await?;
        let mut envelopes = Vec::with_capacity(rows.len());
        for row in rows {
            let text: String = row.try_get("envelope")?;
            envelopes.push(serde_json::from_str::<Envelope>(&text).map_err(anyhow::Error::from)?);
        }
        let log = match cached {
            Some(log) => {
                let mut log = (*log).clone();
                for envelope in envelopes {
                    log.append(envelope).map_err(|rejection| {
                        anyhow::anyhow!("stored log of {org} does not replay: {rejection}")
                    })?;
                }
                log
            }
            None => OrgLog::replay(envelopes).map_err(|(seq, rejection)| {
                anyhow::anyhow!("stored log of {org} does not replay at {seq}: {rejection}")
            })?,
        };
        let log = Arc::new(log);
        self.logs.write().await.insert(org, Arc::clone(&log));
        Ok(log)
    }

    fn append_lock(&self, org: Id) -> Arc<tokio::sync::Mutex<()>> {
        let mut locks = self
            .appends
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Arc::clone(locks.entry(org).or_default())
    }

    /// Appends an entry after replaying it against the current log.
    ///
    /// # Errors
    ///
    /// Fails with `rejected` when replay refuses the entry and with
    /// `conflict` when another server appended first.
    pub async fn append(
        &self,
        db: &Db,
        org: Id,
        envelope: Envelope,
    ) -> ApiResult<(Arc<OrgLog>, u64, Hash)> {
        let lock = self.append_lock(org);
        let _guard = lock.lock().await;
        let current = self.get(db, org).await?;
        let mut log = (*current).clone();
        let entry = log.append(envelope.clone())?;
        let hash = envelope.hash();
        let result = sqlx::query(db.sql(
            "INSERT INTO log_entries (org, seq, envelope, hash, time) VALUES ($1, $2, $3, $4, $5)",
        ))
        .bind(org.to_string())
        .bind(db::to_i64(entry.seq))
        .bind(serde_json::to_string(&envelope).map_err(anyhow::Error::from)?)
        .bind(hash.0.to_vec())
        .bind(db::to_i64(entry.time))
        .execute(db.pool())
        .await;
        match result {
            Ok(_) => {}
            Err(error) if is_unique_violation(&error) => {
                return Err(ApiError::new(ErrorCode::Conflict, "the log moved on"));
            }
            Err(error) => return Err(error.into()),
        }
        let log = Arc::new(log);
        self.logs.write().await.insert(org, Arc::clone(&log));
        Ok((log, entry.seq, hash))
    }

    /// Stores a genesis entry for a new organization.
    ///
    /// # Errors
    ///
    /// Fails when the genesis does not verify or the organization exists.
    pub async fn create(&self, db: &Db, envelope: Envelope, now: Millis) -> ApiResult<Arc<OrgLog>> {
        let log = OrgLog::genesis(envelope.clone())?;
        let org = log.state().org;
        let mut transaction = db.begin().await?;
        let inserted =
            sqlx::query(db.sql("INSERT INTO orgs (id, name, created) VALUES ($1, $2, $3)"))
                .bind(org.to_string())
                .bind(log.state().name.clone())
                .bind(db::to_i64(now))
                .execute(&mut *transaction)
                .await;
        match inserted {
            Ok(_) => {}
            Err(error) if is_unique_violation(&error) => {
                return Err(ApiError::new(
                    ErrorCode::Conflict,
                    "the organization exists",
                ));
            }
            Err(error) => return Err(error.into()),
        }
        sqlx::query(db.sql(
            "INSERT INTO log_entries (org, seq, envelope, hash, time) VALUES ($1, 0, $2, $3, $4)",
        ))
        .bind(org.to_string())
        .bind(serde_json::to_string(&envelope).map_err(anyhow::Error::from)?)
        .bind(envelope.hash().0.to_vec())
        .bind(db::to_i64(log.state().last_time))
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        let log = Arc::new(log);
        self.logs.write().await.insert(org, Arc::clone(&log));
        Ok(log)
    }
}

/// Everything a request handler needs.
pub struct Inner {
    pub config: Config,
    pub db: Db,
    pub clock: Arc<Clock>,
    pub keys: ServerKeys,
    pub logs: LogCache,
    pub audit: crate::audit::Audit,
    pub limiter: RateLimiter,
    pub metrics: Metrics,
    pub http: reqwest::Client,
    pub oidc: Option<crate::oidc::Oidc>,
    pub workload: Vec<(crate::config::WorkloadIssuer, JwksCache)>,
    pub webhook_wake: Notify,
}

#[derive(Clone)]
pub struct AppState(pub Arc<Inner>);

impl std::ops::Deref for AppState {
    type Target = Inner;

    fn deref(&self) -> &Inner {
        &self.0
    }
}

impl AppState {
    /// Builds the state from a configuration and an opened database.
    ///
    /// # Errors
    ///
    /// Fails when a client or secret file cannot be set up.
    pub fn new(
        config: Config,
        db: Db,
        keys: ServerKeys,
        clock: Arc<Clock>,
    ) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        let oidc = config
            .oidc
            .as_ref()
            .map(|oidc| crate::oidc::Oidc::new(oidc, &config, http.clone()))
            .transpose()?;
        let workload = config
            .workload
            .iter()
            .map(|issuer| {
                (
                    issuer.clone(),
                    JwksCache::new(
                        issuer.jwks_url.clone(),
                        issuer.jwks_file.clone(),
                        http.clone(),
                    ),
                )
            })
            .collect();
        let audit = crate::audit::Audit::new(&config.audit);
        Ok(Self(Arc::new(Inner {
            config,
            db,
            clock,
            keys,
            logs: LogCache::default(),
            audit,
            limiter: RateLimiter::default(),
            metrics: Metrics::default(),
            http,
            oidc,
            workload,
            webhook_wake: Notify::new(),
        })))
    }

    #[must_use]
    pub fn now(&self) -> Millis {
        self.clock.now()
    }
}

/// Parses an identifier from a path segment.
///
/// # Errors
///
/// Fails with `not_found` for a malformed identifier.
pub fn parse_id(text: &str) -> ApiResult<Id> {
    Id::from_str(text).map_err(|_| ApiError::not_found("no such resource"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_keys_are_deterministic_and_distinct() {
        let first = ServerKeys::derive(&[7; 32]);
        let second = ServerKeys::derive(&[7; 32]);
        let other = ServerKeys::derive(&[8; 32]);
        assert_eq!(first.checkpoint.to_bytes(), second.checkpoint.to_bytes());
        assert_eq!(first.totp.0, second.totp.0);
        assert_ne!(first.checkpoint.to_bytes(), other.checkpoint.to_bytes());
        assert_ne!(first.checkpoint.to_bytes(), first.totp.0);
    }

    #[test]
    fn master_key_files_must_hold_32_bytes() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("key");
        std::fs::write(
            &path,
            format!("{}\n", pasejo_protocol::encoding::b64(&[1; 32])),
        )
        .unwrap();
        assert!(ServerKeys::load(&path).is_ok());
        std::fs::write(&path, pasejo_protocol::encoding::b64(&[1; 16])).unwrap();
        assert!(ServerKeys::load(&path).is_err());
        std::fs::write(&path, "not base64!").unwrap();
        assert!(ServerKeys::load(&path).is_err());
        assert!(ServerKeys::load(&directory.path().join("missing")).is_err());
    }

    #[test]
    fn rate_limits_reset_each_minute() {
        let limiter = RateLimiter::default();
        let address: IpAddr = "192.0.2.1".parse().unwrap();
        assert!(limiter.allow("auth", address, 2, 0));
        assert!(limiter.allow("auth", address, 2, 1));
        assert!(!limiter.allow("auth", address, 2, 2));
        assert!(limiter.allow("other", address, 2, 2));
        assert!(limiter.allow("auth", "192.0.2.2".parse().unwrap(), 2, 2));
        assert!(limiter.allow("auth", address, 2, 60_000));
    }

    #[test]
    fn metrics_render_in_prometheus_text() {
        let metrics = Metrics::default();
        metrics.request(200);
        metrics.request(404);
        metrics.request(404);
        let text = metrics.render();
        assert!(text.contains("pasejo_requests_total{class=\"2xx\"} 1"));
        assert!(text.contains("pasejo_requests_total{class=\"4xx\"} 2"));
        assert!(text.contains("# TYPE pasejo_audit_events_total counter"));
    }
}
