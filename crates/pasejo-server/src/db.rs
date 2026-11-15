// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Database access through sqlx's `Any` driver. SQL is written once with
//! `$n` placeholders and rewritten to `?n` for SQLite.

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Mutex, OnceLock};

use anyhow::{Context, bail};
use pasejo_protocol::Id;
use sqlx::any::{AnyPoolOptions, AnyRow};
use sqlx::{AnyPool, Executor, Row};

const SQLITE_MIGRATIONS: &[(i64, &str)] =
    &[(1, include_str!("../migrations/sqlite/0001_initial.sql"))];
const POSTGRES_MIGRATIONS: &[(i64, &str)] =
    &[(1, include_str!("../migrations/postgres/0001_initial.sql"))];

#[derive(Debug, Clone)]
pub struct Db {
    pool: AnyPool,
    sqlite: bool,
}

impl Db {
    /// Connects to `url`, a `sqlite:` or `postgres:` URL.
    ///
    /// # Errors
    ///
    /// Fails for other schemes or when the database is unreachable.
    pub async fn connect(url: &str) -> anyhow::Result<Self> {
        sqlx::any::install_default_drivers();
        let sqlite = if url.starts_with("sqlite:") {
            true
        } else if url.starts_with("postgres:") || url.starts_with("postgresql:") {
            false
        } else {
            bail!("database_url must start with sqlite: or postgres:");
        };
        let pool = AnyPoolOptions::new()
            .max_connections(if sqlite { 4 } else { 16 })
            .after_connect(move |connection, _| {
                Box::pin(async move {
                    if sqlite {
                        connection.execute("PRAGMA busy_timeout = 10000").await?;
                        connection.execute("PRAGMA journal_mode = WAL").await?;
                    }
                    Ok(())
                })
            })
            .connect(url)
            .await
            .context("cannot connect to the database")?;
        Ok(Self { pool, sqlite })
    }

    #[must_use]
    pub const fn pool(&self) -> &AnyPool {
        &self.pool
    }

    /// Begins a transaction. On SQLite it is `BEGIN IMMEDIATE`, which takes
    /// the write lock up front: a deferred transaction that reads first
    /// fails with `SQLITE_BUSY_SNAPSHOT` when another writer commits in
    /// between, and the busy timeout does not retry that.
    ///
    /// # Errors
    ///
    /// Fails when no connection is available.
    pub async fn begin(&self) -> sqlx::Result<sqlx::Transaction<'static, sqlx::Any>> {
        if self.sqlite {
            self.pool.begin_with("BEGIN IMMEDIATE").await
        } else {
            self.pool.begin().await
        }
    }

    #[must_use]
    pub const fn is_sqlite(&self) -> bool {
        self.sqlite
    }

    /// The statement for this backend.
    #[must_use]
    pub fn sql(&self, statement: &'static str) -> &'static str {
        if self.sqlite {
            sqlite_statement(statement)
        } else {
            statement
        }
    }

    /// `FOR UPDATE` on PostgreSQL; SQLite serializes writers anyway.
    #[must_use]
    pub const fn for_update(&self) -> &'static str {
        if self.sqlite { "" } else { " FOR UPDATE" }
    }

    /// Applies outstanding migrations.
    ///
    /// # Errors
    ///
    /// Fails when a migration does not apply.
    pub async fn migrate(&self) -> anyhow::Result<()> {
        let mut connection = self.pool.acquire().await?;
        if !self.sqlite {
            connection
                .execute("SELECT pg_advisory_lock(7351803951)")
                .await?;
        }
        connection
            .execute(
                "CREATE TABLE IF NOT EXISTS schema_migrations (version BIGINT PRIMARY KEY, applied BIGINT NOT NULL)",
            )
            .await?;
        let migrations = if self.sqlite {
            SQLITE_MIGRATIONS
        } else {
            POSTGRES_MIGRATIONS
        };
        let result = async {
            for (version, text) in migrations {
                let applied = sqlx::query(
                    self.sql("SELECT version FROM schema_migrations WHERE version = $1"),
                )
                .bind(*version)
                .fetch_optional(&mut *connection)
                .await?;
                if applied.is_some() {
                    continue;
                }
                let mut transaction = sqlx::Connection::begin(&mut *connection).await?;
                sqlx::raw_sql(text).execute(&mut *transaction).await?;
                sqlx::query(
                    self.sql("INSERT INTO schema_migrations (version, applied) VALUES ($1, $2)"),
                )
                .bind(*version)
                .bind(0_i64)
                .execute(&mut *transaction)
                .await?;
                transaction.commit().await?;
                tracing::info!(version, "applied migration");
            }
            anyhow::Ok(())
        }
        .await;
        if !self.sqlite {
            connection
                .execute("SELECT pg_advisory_unlock(7351803951)")
                .await?;
        }
        result
    }

    /// Checks that the database answers.
    ///
    /// # Errors
    ///
    /// Fails when it does not.
    pub async fn ping(&self) -> anyhow::Result<()> {
        sqlx::query("SELECT 1").fetch_one(&self.pool).await?;
        Ok(())
    }
}

fn sqlite_statement(statement: &'static str) -> &'static str {
    static CACHE: OnceLock<Mutex<HashMap<&'static str, &'static str>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Mutex::default);
    let mut cache = cache
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    cache
        .entry(statement)
        .or_insert_with(|| Box::leak(rewrite_placeholders(statement).into_boxed_str()))
}

/// Rewrites `$n` placeholders to `?n`.
#[must_use]
pub fn rewrite_placeholders(statement: &str) -> String {
    let mut rewritten = String::with_capacity(statement.len());
    let mut characters = statement.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '$' && characters.peek().is_some_and(char::is_ascii_digit) {
            rewritten.push('?');
        } else {
            rewritten.push(character);
        }
    }
    rewritten
}

/// Converts a non-negative database integer.
#[must_use]
pub fn to_u64(value: i64) -> u64 {
    u64::try_from(value).unwrap_or(0)
}

/// Converts an unsigned value for storage. Values stay below 2^53.
#[must_use]
pub fn to_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

/// Reads an identifier column.
///
/// # Errors
///
/// Fails when the column is missing or malformed.
pub fn id(row: &AnyRow, column: &str) -> anyhow::Result<Id> {
    let text: String = row.try_get(column)?;
    Id::from_str(&text).map_err(|_| anyhow::anyhow!("malformed identifier in {column}"))
}

/// Reads an optional identifier column.
///
/// # Errors
///
/// Fails when the column is missing or malformed.
pub fn optional_id(row: &AnyRow, column: &str) -> anyhow::Result<Option<Id>> {
    let text: Option<String> = row.try_get(column)?;
    text.map(|text| {
        Id::from_str(&text).map_err(|_| anyhow::anyhow!("malformed identifier in {column}"))
    })
    .transpose()
}

/// Reads an unsigned integer column.
///
/// # Errors
///
/// Fails when the column is missing.
pub fn uint(row: &AnyRow, column: &str) -> anyhow::Result<u64> {
    Ok(to_u64(row.try_get::<i64, _>(column)?))
}

/// Reads a 32-byte column.
///
/// # Errors
///
/// Fails when the column is missing or has another length.
pub fn bytes32(row: &AnyRow, column: &str) -> anyhow::Result<[u8; 32]> {
    let bytes: Vec<u8> = row.try_get(column)?;
    bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("{column} is not 32 bytes"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_are_rewritten_for_sqlite() {
        assert_eq!(
            rewrite_placeholders("SELECT a FROM t WHERE b = $1 AND c = $12"),
            "SELECT a FROM t WHERE b = ?1 AND c = ?12"
        );
        assert_eq!(rewrite_placeholders("SELECT '$' || x"), "SELECT '$' || x");
    }

    #[tokio::test]
    async fn sqlite_databases_migrate_twice_without_harm() {
        let directory = tempfile::tempdir().unwrap();
        let url = format!(
            "sqlite://{}?mode=rwc",
            directory.path().join("db").display()
        );
        let db = Db::connect(&url).await.unwrap();
        db.migrate().await.unwrap();
        db.migrate().await.unwrap();
        db.ping().await.unwrap();
        assert!(db.is_sqlite());
        assert_eq!(db.sql("SELECT $1"), "SELECT ?1");
    }

    #[tokio::test]
    async fn other_schemes_are_refused() {
        assert!(Db::connect("mysql://localhost/x").await.is_err());
    }
}
