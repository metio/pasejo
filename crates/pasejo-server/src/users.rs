// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The server's user records: people identified by OIDC issuer and
//! subject, users provisioned by SCIM before their first sign-in, and
//! machines named by workload rules.

use pasejo_protocol::Id;
use pasejo_protocol::api::UserSummary;
use sqlx::Row;

use crate::db::{self, to_i64};
use crate::error::ApiResult;
use crate::oidc::Identity;
use crate::state::AppState;

/// Issuer of SCIM-provisioned users who have not signed in yet.
pub const UNLINKED: &str = "";
/// Issuer of machine users; their subject is the machine name.
pub const MACHINE: &str = "machine";

/// Finds or creates the user an OIDC identity signs in as. A SCIM user with
/// the same verified email and no linked identity yet becomes that user.
///
/// # Errors
///
/// Fails when the database does.
pub async fn sign_in(state: &AppState, identity: &Identity) -> ApiResult<Id> {
    let db = &state.db;
    if let Some(row) =
        sqlx::query(db.sql("SELECT id FROM users WHERE issuer = $1 AND subject = $2"))
            .bind(identity.issuer.clone())
            .bind(identity.subject.clone())
            .fetch_optional(db.pool())
            .await?
    {
        let user = db::id(&row, "id")?;
        sqlx::query(db.sql("UPDATE users SET email = $1, name = $2 WHERE id = $3"))
            .bind(identity.email.clone())
            .bind(identity.name.clone())
            .bind(user.to_string())
            .execute(db.pool())
            .await?;
        return Ok(user);
    }
    if !identity.email.is_empty()
        && let Some(row) =
            sqlx::query(db.sql(
                "SELECT id FROM users WHERE issuer = $1 AND email = $2 ORDER BY created LIMIT 1",
            ))
            .bind(UNLINKED)
            .bind(identity.email.clone())
            .fetch_optional(db.pool())
            .await?
    {
        let user = db::id(&row, "id")?;
        sqlx::query(db.sql(
            "UPDATE users SET issuer = $1, subject = $2, name = $3 WHERE id = $4 AND issuer = $5",
        ))
        .bind(identity.issuer.clone())
        .bind(identity.subject.clone())
        .bind(identity.name.clone())
        .bind(user.to_string())
        .bind(UNLINKED)
        .execute(db.pool())
        .await?;
        return Ok(user);
    }
    insert(
        state,
        &identity.issuer,
        &identity.subject,
        &identity.email,
        &identity.name,
        false,
    )
    .await
}

/// Finds or creates the machine user `name`.
///
/// # Errors
///
/// Fails when the database does.
pub async fn machine(state: &AppState, name: &str) -> ApiResult<Id> {
    let db = &state.db;
    if let Some(row) =
        sqlx::query(db.sql("SELECT id FROM users WHERE issuer = $1 AND subject = $2"))
            .bind(MACHINE)
            .bind(name.to_owned())
            .fetch_optional(db.pool())
            .await?
    {
        return Ok(db::id(&row, "id")?);
    }
    insert(state, MACHINE, name, "", name, true).await
}

/// Creates an unlinked user for SCIM.
///
/// # Errors
///
/// Fails when the database does.
pub async fn provision(state: &AppState, email: &str, name: &str) -> ApiResult<Id> {
    let subject = Id::random().to_string();
    insert(state, UNLINKED, &subject, email, name, false).await
}

async fn insert(
    state: &AppState,
    issuer: &str,
    subject: &str,
    email: &str,
    name: &str,
    machine: bool,
) -> ApiResult<Id> {
    let id = Id::random();
    sqlx::query(state.db.sql("INSERT INTO users (id, issuer, subject, email, name, machine, active, created) VALUES ($1, $2, $3, $4, $5, $6, 1, $7)"))
        .bind(id.to_string())
        .bind(issuer.to_owned())
        .bind(subject.to_owned())
        .bind(email.to_owned())
        .bind(name.to_owned())
        .bind(i64::from(machine))
        .bind(to_i64(state.now()))
        .execute(state.db.pool())
        .await?;
    Ok(id)
}

/// Updates a user's email and display name.
///
/// # Errors
///
/// Fails when the database does.
pub async fn rename(state: &AppState, user: Id, email: &str, name: &str) -> ApiResult<()> {
    sqlx::query(
        state
            .db
            .sql("UPDATE users SET email = $1, name = $2 WHERE id = $3"),
    )
    .bind(email.to_owned())
    .bind(name.to_owned())
    .bind(user.to_string())
    .execute(state.db.pool())
    .await?;
    Ok(())
}

/// Sets whether a user may sign in.
///
/// # Errors
///
/// Fails when the database does.
pub async fn set_active(state: &AppState, user: Id, active: bool) -> ApiResult<bool> {
    Ok(
        sqlx::query(state.db.sql("UPDATE users SET active = $1 WHERE id = $2"))
            .bind(i64::from(active))
            .bind(user.to_string())
            .execute(state.db.pool())
            .await?
            .rows_affected()
            > 0,
    )
}

/// One user's summary.
///
/// # Errors
///
/// Fails when the database does.
pub async fn summary(state: &AppState, user: Id) -> ApiResult<Option<UserSummary>> {
    let row = sqlx::query(
        state
            .db
            .sql("SELECT id, name, email, machine, active FROM users WHERE id = $1"),
    )
    .bind(user.to_string())
    .fetch_optional(state.db.pool())
    .await?;
    row.map(|row| to_summary(&row)).transpose()
}

/// Summaries of `users`, skipping unknown ones.
///
/// # Errors
///
/// Fails when the database does.
pub async fn summaries(
    state: &AppState,
    users: impl IntoIterator<Item = Id>,
) -> ApiResult<Vec<UserSummary>> {
    let mut summaries = Vec::new();
    for user in users {
        if let Some(summary) = summary(state, user).await? {
            summaries.push(summary);
        }
    }
    Ok(summaries)
}

/// Every user.
///
/// # Errors
///
/// Fails when the database does.
pub async fn all(state: &AppState) -> ApiResult<Vec<UserSummary>> {
    let rows = sqlx::query("SELECT id, name, email, machine, active FROM users ORDER BY created")
        .fetch_all(state.db.pool())
        .await?;
    rows.iter().map(to_summary).collect()
}

fn to_summary(row: &sqlx::any::AnyRow) -> ApiResult<UserSummary> {
    Ok(UserSummary {
        id: db::id(row, "id")?,
        name: row.try_get("name")?,
        email: row.try_get("email")?,
        machine: db::uint(row, "machine")? != 0,
        active: db::uint(row, "active")? != 0,
    })
}
