// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

//! SCIM 2.0 provisioning (RFC 7643, RFC 7644) of users and groups.
//!
//! Users become organization members and groups become teams through
//! pending actions, since only devices sign membership-log entries. A SCIM
//! client acts on its own organization only: deprovisioning suspends the
//! user there at once (`auth::scim_suspended`), and leaves the account, its
//! name and its sessions elsewhere alone.

use std::collections::{BTreeSet, HashMap};
use std::str::FromStr;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Extension, Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use pasejo_protocol::Id;
use pasejo_protocol::encoding::sha256;
use serde_json::{Map, Value, json};
use sqlx::Row;

use crate::audit::{self, Event};
use crate::db::{self, to_i64};
use crate::state::{AppState, RequestCtx};
use crate::time::rfc3339;
use crate::users;

const USER_SCHEMA: &str = "urn:ietf:params:scim:schemas:core:2.0:User";
const GROUP_SCHEMA: &str = "urn:ietf:params:scim:schemas:core:2.0:Group";
const LIST_SCHEMA: &str = "urn:ietf:params:scim:api:messages:2.0:ListResponse";
const ERROR_SCHEMA: &str = "urn:ietf:params:scim:api:messages:2.0:Error";
const PATCH_SCHEMA: &str = "urn:ietf:params:scim:api:messages:2.0:PatchOp";

/// A SCIM error response.
#[derive(Debug)]
pub struct ScimError {
    status: StatusCode,
    scim_type: Option<&'static str>,
    detail: String,
}

impl ScimError {
    fn new(status: StatusCode, scim_type: Option<&'static str>, detail: impl Into<String>) -> Self {
        Self {
            status,
            scim_type,
            detail: detail.into(),
        }
    }

    fn bad(detail: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, Some("invalidValue"), detail)
    }

    fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND, None, "resource not found")
    }
}

impl From<sqlx::Error> for ScimError {
    fn from(error: sqlx::Error) -> Self {
        tracing::error!(%error, "database error");
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, None, "internal error")
    }
}

impl From<anyhow::Error> for ScimError {
    fn from(error: anyhow::Error) -> Self {
        tracing::error!(error = %format!("{error:#}"), "internal error");
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, None, "internal error")
    }
}

impl From<crate::error::ApiError> for ScimError {
    fn from(error: crate::error::ApiError) -> Self {
        Self::new(
            StatusCode::from_u16(error.code.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            None,
            error.message,
        )
    }
}

impl From<serde_json::Error> for ScimError {
    fn from(error: serde_json::Error) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            Some("invalidSyntax"),
            error.to_string(),
        )
    }
}

fn scim(status: StatusCode, body: &Value) -> Response {
    let mut response = (status, body.to_string()).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/scim+json"),
    );
    response
}

impl IntoResponse for ScimError {
    fn into_response(self) -> Response {
        let mut body = json!({
            "schemas": [ERROR_SCHEMA],
            "status": self.status.as_u16().to_string(),
            "detail": self.detail,
        });
        if let Some(scim_type) = self.scim_type {
            body["scimType"] = json!(scim_type);
        }
        scim(self.status, &body)
    }
}

type ScimResult<T> = Result<T, ScimError>;

/// Authenticates the bearer token for `org` and returns the token's id.
async fn authenticate(
    state: &AppState,
    ctx: &RequestCtx,
    org: &str,
    headers: &HeaderMap,
) -> ScimResult<(Id, Id)> {
    let unauthorized = || ScimError::new(StatusCode::UNAUTHORIZED, None, "invalid token");
    let org = Id::from_str(org).map_err(|_| unauthorized())?;
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or_else(unauthorized)?;
    let row = sqlx::query(
        state
            .db
            .sql("SELECT id FROM scim_tokens WHERE hash = $1 AND org = $2 AND revoked = 0"),
    )
    .bind(sha256(token.as_bytes()).to_vec())
    .bind(org.to_string())
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(unauthorized)?;
    ctx.set_user(None, "scim");
    Ok((org, db::id(&row, "id")?))
}

async fn record(
    state: &AppState,
    ctx: &RequestCtx,
    org: Id,
    token: Id,
    action: &str,
    detail: String,
) -> ScimResult<()> {
    audit::record(
        state,
        Some(ctx),
        Event::ok(action)
            .org(org)
            .detail(format!("token {token}: {detail}")),
    )
    .await?;
    Ok(())
}

/// `GET /scim/v2/{org}/ServiceProviderConfig`
pub async fn service_provider_config(
    State(state): State<AppState>,
    Extension(ctx): Extension<Arc<RequestCtx>>,
    Path(org): Path<String>,
    headers: HeaderMap,
) -> ScimResult<Response> {
    authenticate(&state, &ctx, &org, &headers).await?;
    Ok(scim(
        StatusCode::OK,
        &json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:ServiceProviderConfig"],
            "patch": {"supported": true},
            "bulk": {"supported": false, "maxOperations": 0, "maxPayloadSize": 0},
            "filter": {"supported": true, "maxResults": 200},
            "changePassword": {"supported": false},
            "sort": {"supported": false},
            "etag": {"supported": false},
            "authenticationSchemes": [{"type": "oauthbearertoken", "name": "Bearer token", "description": "An organization SCIM token"}]
        }),
    ))
}

/// `GET /scim/v2/{org}/ResourceTypes`
pub async fn resource_types(
    State(state): State<AppState>,
    Extension(ctx): Extension<Arc<RequestCtx>>,
    Path(org): Path<String>,
    headers: HeaderMap,
) -> ScimResult<Response> {
    authenticate(&state, &ctx, &org, &headers).await?;
    let resources = json!([
        {"schemas": ["urn:ietf:params:scim:schemas:core:2.0:ResourceType"], "id": "User", "name": "User", "endpoint": "/Users", "schema": USER_SCHEMA},
        {"schemas": ["urn:ietf:params:scim:schemas:core:2.0:ResourceType"], "id": "Group", "name": "Group", "endpoint": "/Groups", "schema": GROUP_SCHEMA}
    ]);
    Ok(scim(
        StatusCode::OK,
        &list_response(resources.as_array().map_or(&[][..], Vec::as_slice), 1),
    ))
}

/// `GET /scim/v2/{org}/Schemas`
pub async fn schemas(
    State(state): State<AppState>,
    Extension(ctx): Extension<Arc<RequestCtx>>,
    Path(org): Path<String>,
    headers: HeaderMap,
) -> ScimResult<Response> {
    authenticate(&state, &ctx, &org, &headers).await?;
    let attribute = |name: &str, kind: &str, required: bool| json!({"name": name, "type": kind, "multiValued": false, "required": required, "mutability": "readWrite", "returned": "default", "uniqueness": if name == "userName" { "server" } else { "none" }});
    let resources = vec![
        json!({"schemas": ["urn:ietf:params:scim:schemas:core:2.0:Schema"], "id": USER_SCHEMA, "name": "User", "attributes": [
            attribute("userName", "string", true), attribute("displayName", "string", false),
            attribute("active", "boolean", false), attribute("externalId", "string", false),
            {"name": "emails", "type": "complex", "multiValued": true, "required": false, "mutability": "readWrite", "returned": "default"},
            {"name": "name", "type": "complex", "multiValued": false, "required": false, "mutability": "readWrite", "returned": "default"}
        ]}),
        json!({"schemas": ["urn:ietf:params:scim:schemas:core:2.0:Schema"], "id": GROUP_SCHEMA, "name": "Group", "attributes": [
            attribute("displayName", "string", true), attribute("externalId", "string", false),
            {"name": "members", "type": "complex", "multiValued": true, "required": false, "mutability": "readWrite", "returned": "default"}
        ]}),
    ];
    Ok(scim(StatusCode::OK, &list_response(&resources, 1)))
}

fn list_response(resources: &[Value], start: usize) -> Value {
    json!({
        "schemas": [LIST_SCHEMA],
        "totalResults": resources.len() + start - 1,
        "startIndex": start,
        "itemsPerPage": resources.len(),
        "Resources": resources,
    })
}

/// A parsed `attribute eq "value"` filter.
#[derive(Debug, PartialEq, Eq)]
pub struct Filter {
    pub attribute: String,
    pub value: String,
}

/// Parses the `eq` filters SCIM clients send for lookups.
///
/// # Errors
///
/// Fails for any other filter.
pub fn parse_filter(text: &str) -> Result<Filter, String> {
    let text = text.trim();
    let mut parts = text.splitn(3, char::is_whitespace);
    let attribute = parts.next().unwrap_or_default();
    let operator = parts.next().unwrap_or_default();
    let value = parts.next().unwrap_or_default().trim();
    if !operator.eq_ignore_ascii_case("eq") {
        return Err(format!("unsupported filter {text:?}"));
    }
    let value: String =
        serde_json::from_str(value).map_err(|_| format!("unsupported filter value in {text:?}"))?;
    Ok(Filter {
        attribute: attribute.to_ascii_lowercase(),
        value,
    })
}

fn paging(query: &HashMap<String, String>) -> (usize, usize) {
    let start = query
        .get("startIndex")
        .and_then(|start| start.parse::<usize>().ok())
        .unwrap_or(1)
        .max(1);
    let count = query
        .get("count")
        .and_then(|count| count.parse::<usize>().ok())
        .unwrap_or(100)
        .min(200);
    (start, count)
}

fn page(resources: Vec<Value>, query: &HashMap<String, String>) -> Value {
    let total = resources.len();
    let (start, count) = paging(query);
    let resources: Vec<Value> = resources.into_iter().skip(start - 1).take(count).collect();
    json!({
        "schemas": [LIST_SCHEMA],
        "totalResults": total,
        "startIndex": start,
        "itemsPerPage": resources.len(),
        "Resources": resources,
    })
}

fn string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn boolean(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(flag) => Some(*flag),
        Value::String(text) if text.eq_ignore_ascii_case("true") => Some(true),
        Value::String(text) if text.eq_ignore_ascii_case("false") => Some(false),
        _ => None,
    }
}

/// The parts of a SCIM user the server uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserFields {
    pub user_name: String,
    pub external_id: Option<String>,
    pub email: String,
    pub display_name: String,
    pub active: bool,
}

/// Reads and checks a user resource.
///
/// # Errors
///
/// Fails without a user name or with malformed attributes.
pub fn user_fields(resource: &Value) -> ScimResult<UserFields> {
    let user_name = string(resource, "userName")
        .filter(|name| !name.trim().is_empty())
        .ok_or_else(|| ScimError::bad("userName is required"))?;
    let emails = resource
        .get("emails")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let primary = emails
        .iter()
        .find(|email| email.get("primary").and_then(boolean).unwrap_or(false))
        .or_else(|| emails.first())
        .and_then(|email| string(email, "value"));
    let email = primary
        .or_else(|| user_name.contains('@').then(|| user_name.clone()))
        .unwrap_or_default();
    let name = resource.get("name").cloned().unwrap_or(Value::Null);
    let display_name = string(resource, "displayName")
        .or_else(|| string(&name, "formatted"))
        .or_else(|| {
            let given = string(&name, "givenName").unwrap_or_default();
            let family = string(&name, "familyName").unwrap_or_default();
            let joined = format!("{given} {family}").trim().to_owned();
            (!joined.is_empty()).then_some(joined)
        })
        .unwrap_or_else(|| user_name.clone());
    let active = match resource.get("active") {
        None | Some(Value::Null) => true,
        Some(value) => boolean(value).ok_or_else(|| ScimError::bad("active must be a boolean"))?,
    };
    Ok(UserFields {
        user_name,
        external_id: string(resource, "externalId"),
        email,
        display_name,
        active,
    })
}

fn clean(resource: &Value) -> Value {
    let mut resource = resource.as_object().cloned().unwrap_or_default();
    for key in ["id", "meta", "schemas"] {
        resource.remove(key);
    }
    Value::Object(resource)
}

fn user_resource(state: &AppState, org: Id, row: &sqlx::any::AnyRow) -> ScimResult<Value> {
    let id = db::id(row, "user_id")?;
    let text: String = row.try_get("resource")?;
    let mut resource: Value = serde_json::from_str(&text)?;
    let object = resource
        .as_object_mut()
        .ok_or_else(|| ScimError::bad("stored resource"))?;
    object.insert("schemas".into(), json!([USER_SCHEMA]));
    object.insert("id".into(), json!(id.to_string()));
    object.insert("active".into(), json!(db::uint(row, "active")? != 0));
    object.insert(
        "meta".into(),
        json!({
            "resourceType": "User",
            "created": rfc3339(db::uint(row, "created")?),
            "lastModified": rfc3339(db::uint(row, "modified")?),
            "location": state.config.url(&format!("/scim/v2/{org}/Users/{id}")),
        }),
    );
    Ok(resource)
}

async fn find_user(state: &AppState, org: Id, id: &str) -> ScimResult<sqlx::any::AnyRow> {
    let id = Id::from_str(id).map_err(|_| ScimError::not_found())?;
    sqlx::query(state.db.sql("SELECT user_id, external_id, user_name, active, resource, created, modified FROM scim_users WHERE org = $1 AND user_id = $2 AND deleted = 0"))
        .bind(org.to_string())
        .bind(id.to_string())
        .fetch_optional(state.db.pool())
        .await?
        .ok_or_else(ScimError::not_found)
}

/// `GET /scim/v2/{org}/Users`
pub async fn list_users(
    State(state): State<AppState>,
    Extension(ctx): Extension<Arc<RequestCtx>>,
    Path(org): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> ScimResult<Response> {
    let (org, _) = authenticate(&state, &ctx, &org, &headers).await?;
    let filter = query
        .get("filter")
        .map(|filter| parse_filter(filter))
        .transpose()
        .map_err(|detail| ScimError::new(StatusCode::BAD_REQUEST, Some("invalidFilter"), detail))?;
    let rows = sqlx::query(state.db.sql("SELECT user_id, external_id, user_name, active, resource, created, modified FROM scim_users WHERE org = $1 AND deleted = 0 ORDER BY created"))
        .bind(org.to_string())
        .fetch_all(state.db.pool())
        .await?;
    let mut resources = Vec::new();
    for row in &rows {
        let resource = user_resource(&state, org, row)?;
        let keep = match &filter {
            None => true,
            Some(filter) => {
                let fields = user_fields(&resource)?;
                match filter.attribute.as_str() {
                    "username" => fields.user_name.eq_ignore_ascii_case(&filter.value),
                    "externalid" => fields.external_id.as_deref() == Some(filter.value.as_str()),
                    "displayname" => fields.display_name == filter.value,
                    _ => {
                        return Err(ScimError::new(
                            StatusCode::BAD_REQUEST,
                            Some("invalidFilter"),
                            "unsupported filter attribute",
                        ));
                    }
                }
            }
        };
        if keep {
            resources.push(resource);
        }
    }
    Ok(scim(StatusCode::OK, &page(resources, &query)))
}

/// `GET /scim/v2/{org}/Users/{id}`
pub async fn get_user(
    State(state): State<AppState>,
    Extension(ctx): Extension<Arc<RequestCtx>>,
    Path((org, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ScimResult<Response> {
    let (org, _) = authenticate(&state, &ctx, &org, &headers).await?;
    let row = find_user(&state, org, &id).await?;
    Ok(scim(StatusCode::OK, &user_resource(&state, org, &row)?))
}

async fn user_name_taken(
    state: &AppState,
    org: Id,
    user_name: &str,
    except: Option<Id>,
) -> ScimResult<bool> {
    let rows = sqlx::query(
        state
            .db
            .sql("SELECT user_id, user_name FROM scim_users WHERE org = $1 AND deleted = 0"),
    )
    .bind(org.to_string())
    .fetch_all(state.db.pool())
    .await?;
    for row in rows {
        let name: String = row.try_get("user_name")?;
        if name.eq_ignore_ascii_case(user_name) && Some(db::id(&row, "user_id")?) != except {
            return Ok(true);
        }
    }
    Ok(false)
}

/// `POST /scim/v2/{org}/Users`
pub async fn create_user(
    State(state): State<AppState>,
    Extension(ctx): Extension<Arc<RequestCtx>>,
    Path(org): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> ScimResult<Response> {
    let (org, token) = authenticate(&state, &ctx, &org, &headers).await?;
    let resource = clean(&serde_json::from_slice(&body)?);
    let fields = user_fields(&resource)?;
    if user_name_taken(&state, org, &fields.user_name, None).await? {
        return Err(ScimError::new(
            StatusCode::CONFLICT,
            Some("uniqueness"),
            "userName is taken",
        ));
    }
    let existing =
        if fields.email.is_empty() {
            None
        } else {
            sqlx::query(state.db.sql(
                "SELECT id FROM users WHERE email = $1 AND machine = 0 ORDER BY created LIMIT 1",
            ))
            .bind(fields.email.clone())
            .fetch_optional(state.db.pool())
            .await?
            .map(|row| db::id(&row, "id"))
            .transpose()?
        };
    let user = match existing {
        Some(user) => user,
        None => users::provision(&state, &fields.email, &fields.display_name).await?,
    };
    let now = state.now();
    let inserted = sqlx::query(state.db.sql("INSERT INTO scim_users (org, user_id, external_id, user_name, active, deleted, resource, created, modified) VALUES ($1, $2, $3, $4, $5, 0, $6, $7, $7)"))
        .bind(org.to_string())
        .bind(user.to_string())
        .bind(fields.external_id.clone())
        .bind(fields.user_name.clone())
        .bind(i64::from(fields.active))
        .bind(resource.to_string())
        .bind(to_i64(now))
        .execute(state.db.pool())
        .await;
    match inserted {
        Ok(_) => {}
        Err(error) if crate::error::is_unique_violation(&error) => {
            // A deleted SCIM record of the same user is revived.
            sqlx::query(state.db.sql("UPDATE scim_users SET external_id = $1, user_name = $2, active = $3, deleted = 0, resource = $4, modified = $5 WHERE org = $6 AND user_id = $7"))
                .bind(fields.external_id.clone())
                .bind(fields.user_name.clone())
                .bind(i64::from(fields.active))
                .bind(resource.to_string())
                .bind(to_i64(now))
                .bind(org.to_string())
                .bind(user.to_string())
                .execute(state.db.pool())
                .await?;
        }
        Err(error) => return Err(error.into()),
    }
    record(
        &state,
        &ctx,
        org,
        token,
        "scim.user.create",
        format!("user {user}"),
    )
    .await?;
    let row = find_user(&state, org, &user.to_string()).await?;
    Ok(scim(
        StatusCode::CREATED,
        &user_resource(&state, org, &row)?,
    ))
}

async fn store_user(
    state: &AppState,
    org: Id,
    user: Id,
    resource: &Value,
) -> ScimResult<UserFields> {
    let fields = user_fields(resource)?;
    if user_name_taken(state, org, &fields.user_name, Some(user)).await? {
        return Err(ScimError::new(
            StatusCode::CONFLICT,
            Some("uniqueness"),
            "userName is taken",
        ));
    }
    sqlx::query(state.db.sql("UPDATE scim_users SET external_id = $1, user_name = $2, active = $3, resource = $4, modified = $5 WHERE org = $6 AND user_id = $7"))
        .bind(fields.external_id.clone())
        .bind(fields.user_name.clone())
        .bind(i64::from(fields.active))
        .bind(resource.to_string())
        .bind(to_i64(state.now()))
        .bind(org.to_string())
        .bind(user.to_string())
        .execute(state.db.pool())
        .await?;
    Ok(fields)
}

/// `PUT /scim/v2/{org}/Users/{id}`
pub async fn replace_user(
    State(state): State<AppState>,
    Extension(ctx): Extension<Arc<RequestCtx>>,
    Path((org, id)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> ScimResult<Response> {
    let (org, token) = authenticate(&state, &ctx, &org, &headers).await?;
    let row = find_user(&state, org, &id).await?;
    let user = db::id(&row, "user_id")?;
    let resource = clean(&serde_json::from_slice(&body)?);
    let fields = store_user(&state, org, user, &resource).await?;
    record(
        &state,
        &ctx,
        org,
        token,
        "scim.user.replace",
        format!("user {user}, active {}", fields.active),
    )
    .await?;
    let row = find_user(&state, org, &id).await?;
    Ok(scim(StatusCode::OK, &user_resource(&state, org, &row)?))
}

fn operations(body: &Value) -> ScimResult<Vec<Value>> {
    let schemas_ok = body
        .get("schemas")
        .and_then(Value::as_array)
        .is_none_or(|schemas| schemas.iter().any(|schema| schema == PATCH_SCHEMA));
    if !schemas_ok {
        return Err(ScimError::bad("not a PatchOp request"));
    }
    body.get("Operations")
        .or_else(|| body.get("operations"))
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| ScimError::bad("Operations are required"))
}

/// Applies one PATCH operation to a user resource.
///
/// # Errors
///
/// Fails for unsupported paths and operations.
pub fn patch_user(resource: &mut Map<String, Value>, operation: &Value) -> ScimResult<()> {
    let op = string(operation, "op")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let value = operation.get("value").cloned().unwrap_or(Value::Null);
    let path = string(operation, "path");
    match (op.as_str(), path.as_deref()) {
        ("add" | "replace", None) => {
            let object = value
                .as_object()
                .ok_or_else(|| ScimError::bad("value must be an object"))?;
            for (key, value) in object {
                set_user_path(resource, key, value.clone())?;
            }
        }
        ("add" | "replace", Some(path)) => set_user_path(resource, path, value)?,
        ("remove", Some(path)) => match path {
            "externalId" | "displayName" | "emails" | "name" => {
                resource.remove(path);
            }
            _ => {
                return Err(ScimError::new(
                    StatusCode::BAD_REQUEST,
                    Some("invalidPath"),
                    format!("cannot remove {path}"),
                ));
            }
        },
        _ => {
            return Err(ScimError::new(
                StatusCode::BAD_REQUEST,
                Some("invalidSyntax"),
                format!("unsupported operation {op}"),
            ));
        }
    }
    Ok(())
}

fn set_user_path(resource: &mut Map<String, Value>, path: &str, value: Value) -> ScimResult<()> {
    let lower = path.to_ascii_lowercase();
    match lower.as_str() {
        "active" => {
            let flag = boolean(&value).ok_or_else(|| ScimError::bad("active must be a boolean"))?;
            resource.insert("active".into(), Value::Bool(flag));
        }
        "username" => {
            resource.insert("userName".into(), value);
        }
        "displayname" => {
            resource.insert("displayName".into(), value);
        }
        "externalid" => {
            resource.insert("externalId".into(), value);
        }
        "emails" => {
            resource.insert("emails".into(), value);
        }
        "name" => {
            resource.insert("name".into(), value);
        }
        "name.givenname" | "name.familyname" | "name.formatted" => {
            let key = match lower.as_str() {
                "name.givenname" => "givenName",
                "name.familyname" => "familyName",
                _ => "formatted",
            };
            let name = resource.entry("name").or_insert_with(|| json!({}));
            if let Some(name) = name.as_object_mut() {
                name.insert(key.into(), value);
            }
        }
        _ if lower.starts_with("emails[") && lower.ends_with("].value") => {
            resource.insert("emails".into(), json!([{"value": value, "primary": true}]));
        }
        _ => {
            return Err(ScimError::new(
                StatusCode::BAD_REQUEST,
                Some("invalidPath"),
                format!("unsupported path {path}"),
            ));
        }
    }
    Ok(())
}

/// `PATCH /scim/v2/{org}/Users/{id}`
pub async fn patch_user_handler(
    State(state): State<AppState>,
    Extension(ctx): Extension<Arc<RequestCtx>>,
    Path((org, id)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> ScimResult<Response> {
    let (org, token) = authenticate(&state, &ctx, &org, &headers).await?;
    let row = find_user(&state, org, &id).await?;
    let user = db::id(&row, "user_id")?;
    let text: String = row.try_get("resource")?;
    let mut resource: Map<String, Value> = serde_json::from_str(&text)?;
    resource.insert("active".into(), Value::Bool(db::uint(&row, "active")? != 0));
    for operation in operations(&serde_json::from_slice(&body)?)? {
        patch_user(&mut resource, &operation)?;
    }
    let fields = store_user(&state, org, user, &Value::Object(resource)).await?;
    record(
        &state,
        &ctx,
        org,
        token,
        "scim.user.patch",
        format!("user {user}, active {}", fields.active),
    )
    .await?;
    let row = find_user(&state, org, &id).await?;
    Ok(scim(StatusCode::OK, &user_resource(&state, org, &row)?))
}

/// `DELETE /scim/v2/{org}/Users/{id}`
pub async fn delete_user(
    State(state): State<AppState>,
    Extension(ctx): Extension<Arc<RequestCtx>>,
    Path((org, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ScimResult<StatusCode> {
    let (org, token) = authenticate(&state, &ctx, &org, &headers).await?;
    let row = find_user(&state, org, &id).await?;
    let user = db::id(&row, "user_id")?;
    sqlx::query(state.db.sql("UPDATE scim_users SET deleted = 1, active = 0, modified = $1 WHERE org = $2 AND user_id = $3"))
        .bind(to_i64(state.now()))
        .bind(org.to_string())
        .bind(user.to_string())
        .execute(state.db.pool())
        .await?;
    record(
        &state,
        &ctx,
        org,
        token,
        "scim.user.delete",
        format!("user {user}"),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn org_scim_users(state: &AppState, org: Id) -> ScimResult<BTreeSet<Id>> {
    sqlx::query(
        state
            .db
            .sql("SELECT user_id FROM scim_users WHERE org = $1 AND deleted = 0"),
    )
    .bind(org.to_string())
    .fetch_all(state.db.pool())
    .await?
    .iter()
    .map(|row| db::id(row, "user_id").map_err(ScimError::from))
    .collect()
}

fn member_ids(value: &Value, known: &BTreeSet<Id>) -> ScimResult<Vec<Id>> {
    let members = match value {
        Value::Null => return Ok(Vec::new()),
        Value::Array(members) => members.clone(),
        Value::Object(_) => vec![value.clone()],
        _ => return Err(ScimError::bad("members must be a list")),
    };
    members
        .iter()
        .map(|member| {
            let id = string(member, "value").and_then(|id| Id::from_str(&id).ok());
            match id {
                Some(id) if known.contains(&id) => Ok(id),
                _ => Err(ScimError::bad(
                    "a member is not a user of this organization",
                )),
            }
        })
        .collect()
}

fn group_resource(state: &AppState, org: Id, row: &sqlx::any::AnyRow) -> ScimResult<Value> {
    let id = db::id(row, "id")?;
    let members: String = row.try_get("members")?;
    let members: Vec<Id> = serde_json::from_str(&members)?;
    let mut resource = json!({
        "schemas": [GROUP_SCHEMA],
        "id": id.to_string(),
        "displayName": row.try_get::<String, _>("display_name")?,
        "members": members.iter().map(|member| json!({"value": member.to_string(), "type": "User"})).collect::<Vec<_>>(),
        "meta": {
            "resourceType": "Group",
            "created": rfc3339(db::uint(row, "created")?),
            "lastModified": rfc3339(db::uint(row, "modified")?),
            "location": state.config.url(&format!("/scim/v2/{org}/Groups/{id}")),
        },
    });
    if let Some(external) = row.try_get::<Option<String>, _>("external_id")? {
        resource["externalId"] = json!(external);
    }
    Ok(resource)
}

async fn find_group(state: &AppState, org: Id, id: &str) -> ScimResult<sqlx::any::AnyRow> {
    let id = Id::from_str(id).map_err(|_| ScimError::not_found())?;
    sqlx::query(state.db.sql("SELECT id, display_name, external_id, members, created, modified FROM scim_groups WHERE org = $1 AND id = $2 AND deleted = 0"))
        .bind(org.to_string())
        .bind(id.to_string())
        .fetch_optional(state.db.pool())
        .await?
        .ok_or_else(ScimError::not_found)
}

/// `GET /scim/v2/{org}/Groups`
pub async fn list_groups(
    State(state): State<AppState>,
    Extension(ctx): Extension<Arc<RequestCtx>>,
    Path(org): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> ScimResult<Response> {
    let (org, _) = authenticate(&state, &ctx, &org, &headers).await?;
    let filter = query
        .get("filter")
        .map(|filter| parse_filter(filter))
        .transpose()
        .map_err(|detail| ScimError::new(StatusCode::BAD_REQUEST, Some("invalidFilter"), detail))?;
    let rows = sqlx::query(state.db.sql("SELECT id, display_name, external_id, members, created, modified FROM scim_groups WHERE org = $1 AND deleted = 0 ORDER BY created"))
        .bind(org.to_string())
        .fetch_all(state.db.pool())
        .await?;
    let mut resources = Vec::new();
    for row in &rows {
        let resource = group_resource(&state, org, row)?;
        let keep = match &filter {
            None => true,
            Some(filter) => match filter.attribute.as_str() {
                "displayname" => {
                    string(&resource, "displayName").as_deref() == Some(filter.value.as_str())
                }
                "externalid" => {
                    string(&resource, "externalId").as_deref() == Some(filter.value.as_str())
                }
                _ => {
                    return Err(ScimError::new(
                        StatusCode::BAD_REQUEST,
                        Some("invalidFilter"),
                        "unsupported filter attribute",
                    ));
                }
            },
        };
        if keep {
            resources.push(resource);
        }
    }
    Ok(scim(StatusCode::OK, &page(resources, &query)))
}

/// `GET /scim/v2/{org}/Groups/{id}`
pub async fn get_group(
    State(state): State<AppState>,
    Extension(ctx): Extension<Arc<RequestCtx>>,
    Path((org, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ScimResult<Response> {
    let (org, _) = authenticate(&state, &ctx, &org, &headers).await?;
    let row = find_group(&state, org, &id).await?;
    Ok(scim(StatusCode::OK, &group_resource(&state, org, &row)?))
}

/// `POST /scim/v2/{org}/Groups`
pub async fn create_group(
    State(state): State<AppState>,
    Extension(ctx): Extension<Arc<RequestCtx>>,
    Path(org): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> ScimResult<Response> {
    let (org, token) = authenticate(&state, &ctx, &org, &headers).await?;
    let resource: Value = serde_json::from_slice(&body)?;
    let name = string(&resource, "displayName")
        .filter(|name| !name.trim().is_empty() && name.len() <= 200)
        .ok_or_else(|| ScimError::bad("displayName is required"))?;
    let known = org_scim_users(&state, org).await?;
    let members = member_ids(resource.get("members").unwrap_or(&Value::Null), &known)?;
    let id = Id::random();
    let now = to_i64(state.now());
    sqlx::query(state.db.sql("INSERT INTO scim_groups (org, id, display_name, external_id, members, deleted, created, modified) VALUES ($1, $2, $3, $4, $5, 0, $6, $6)"))
        .bind(org.to_string())
        .bind(id.to_string())
        .bind(name)
        .bind(string(&resource, "externalId"))
        .bind(serde_json::to_string(&members)?)
        .bind(now)
        .execute(state.db.pool())
        .await?;
    record(
        &state,
        &ctx,
        org,
        token,
        "scim.group.create",
        format!("group {id}"),
    )
    .await?;
    let row = find_group(&state, org, &id.to_string()).await?;
    Ok(scim(
        StatusCode::CREATED,
        &group_resource(&state, org, &row)?,
    ))
}

async fn store_group(
    state: &AppState,
    org: Id,
    id: Id,
    name: &str,
    external: Option<String>,
    members: &[Id],
) -> ScimResult<()> {
    if name.trim().is_empty() || name.len() > 200 {
        return Err(ScimError::bad("displayName is required"));
    }
    let unique: BTreeSet<Id> = members.iter().copied().collect();
    sqlx::query(state.db.sql("UPDATE scim_groups SET display_name = $1, external_id = $2, members = $3, modified = $4 WHERE org = $5 AND id = $6"))
        .bind(name.to_owned())
        .bind(external)
        .bind(serde_json::to_string(&unique.into_iter().collect::<Vec<_>>())?)
        .bind(to_i64(state.now()))
        .bind(org.to_string())
        .bind(id.to_string())
        .execute(state.db.pool())
        .await?;
    Ok(())
}

/// `PUT /scim/v2/{org}/Groups/{id}`
pub async fn replace_group(
    State(state): State<AppState>,
    Extension(ctx): Extension<Arc<RequestCtx>>,
    Path((org, id)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> ScimResult<Response> {
    let (org, token) = authenticate(&state, &ctx, &org, &headers).await?;
    let row = find_group(&state, org, &id).await?;
    let group = db::id(&row, "id")?;
    let resource: Value = serde_json::from_slice(&body)?;
    let known = org_scim_users(&state, org).await?;
    let members = member_ids(resource.get("members").unwrap_or(&Value::Null), &known)?;
    let name = string(&resource, "displayName").unwrap_or_default();
    store_group(
        &state,
        org,
        group,
        &name,
        string(&resource, "externalId"),
        &members,
    )
    .await?;
    record(
        &state,
        &ctx,
        org,
        token,
        "scim.group.replace",
        format!("group {group}"),
    )
    .await?;
    let row = find_group(&state, org, &id).await?;
    Ok(scim(StatusCode::OK, &group_resource(&state, org, &row)?))
}

/// The member a path like `members[value eq "id"]` names.
fn member_in_path(path: &str) -> Option<Id> {
    let inner = path.strip_prefix("members[")?.strip_suffix(']')?;
    let filter = parse_filter(inner).ok()?;
    (filter.attribute == "value")
        .then(|| Id::from_str(&filter.value).ok())
        .flatten()
}

/// `PATCH /scim/v2/{org}/Groups/{id}`
pub async fn patch_group(
    State(state): State<AppState>,
    Extension(ctx): Extension<Arc<RequestCtx>>,
    Path((org, id)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> ScimResult<Response> {
    let (org, token) = authenticate(&state, &ctx, &org, &headers).await?;
    let row = find_group(&state, org, &id).await?;
    let group = db::id(&row, "id")?;
    let mut name: String = row.try_get("display_name")?;
    let mut external: Option<String> = row.try_get("external_id")?;
    let text: String = row.try_get("members")?;
    let mut members: Vec<Id> = serde_json::from_str(&text)?;
    let known = org_scim_users(&state, org).await?;
    for operation in operations(&serde_json::from_slice(&body)?)? {
        let op = string(&operation, "op")
            .unwrap_or_default()
            .to_ascii_lowercase();
        let value = operation.get("value").cloned().unwrap_or(Value::Null);
        let path = string(&operation, "path");
        match (op.as_str(), path.as_deref()) {
            ("add", Some(path)) if path.eq_ignore_ascii_case("members") => {
                members.extend(member_ids(&value, &known)?);
            }
            ("replace", Some(path)) if path.eq_ignore_ascii_case("members") => {
                members = member_ids(&value, &known)?;
            }
            ("remove", Some(path)) if path.eq_ignore_ascii_case("members") => {
                if value.is_null() {
                    members.clear();
                } else {
                    let gone: BTreeSet<Id> =
                        member_ids(&value, &BTreeSet::from_iter(members.iter().copied()))?
                            .into_iter()
                            .collect();
                    members.retain(|member| !gone.contains(member));
                }
            }
            ("remove", Some(path)) if member_in_path(path).is_some() => {
                let gone = member_in_path(path);
                members.retain(|member| Some(*member) != gone);
            }
            ("add" | "replace", Some(path)) if path.eq_ignore_ascii_case("displayName") => {
                value.as_str().unwrap_or_default().clone_into(&mut name);
            }
            ("add" | "replace", Some(path)) if path.eq_ignore_ascii_case("externalId") => {
                external = value.as_str().map(ToOwned::to_owned);
            }
            ("add" | "replace", None) => {
                if let Some(display) = string(&value, "displayName") {
                    name = display;
                }
                if let Some(external_id) = string(&value, "externalId") {
                    external = Some(external_id);
                }
                if let Some(list) = value.get("members") {
                    let listed = member_ids(list, &known)?;
                    if op == "add" {
                        members.extend(listed);
                    } else {
                        members = listed;
                    }
                }
            }
            _ => {
                return Err(ScimError::new(
                    StatusCode::BAD_REQUEST,
                    Some("invalidPath"),
                    format!("unsupported operation {op}"),
                ));
            }
        }
    }
    store_group(&state, org, group, &name, external, &members).await?;
    record(
        &state,
        &ctx,
        org,
        token,
        "scim.group.patch",
        format!("group {group}"),
    )
    .await?;
    let row = find_group(&state, org, &id).await?;
    Ok(scim(StatusCode::OK, &group_resource(&state, org, &row)?))
}

/// `DELETE /scim/v2/{org}/Groups/{id}`
pub async fn delete_group(
    State(state): State<AppState>,
    Extension(ctx): Extension<Arc<RequestCtx>>,
    Path((org, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ScimResult<StatusCode> {
    let (org, token) = authenticate(&state, &ctx, &org, &headers).await?;
    let row = find_group(&state, org, &id).await?;
    let group = db::id(&row, "id")?;
    sqlx::query(
        state
            .db
            .sql("UPDATE scim_groups SET deleted = 1, modified = $1 WHERE org = $2 AND id = $3"),
    )
    .bind(to_i64(state.now()))
    .bind(org.to_string())
    .bind(group.to_string())
    .execute(state.db.pool())
    .await?;
    record(
        &state,
        &ctx,
        org,
        token,
        "scim.group.delete",
        format!("group {group}"),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eq_filters_parse_and_others_do_not() {
        assert_eq!(
            parse_filter(r#"userName eq "alice@example.com""#).unwrap(),
            Filter {
                attribute: "username".into(),
                value: "alice@example.com".into()
            }
        );
        assert_eq!(parse_filter(r#"externalId EQ "a b""#).unwrap().value, "a b");
        assert!(parse_filter(r#"userName co "a""#).is_err());
        assert!(parse_filter("userName eq alice").is_err());
    }

    #[test]
    fn user_fields_fall_back_sensibly() {
        let fields = user_fields(&json!({
            "userName": "alice",
            "name": {"givenName": "Alice", "familyName": "Liddell"},
            "emails": [{"value": "work@x", "primary": false}, {"value": "main@x", "primary": true}]
        }))
        .unwrap();
        assert_eq!(fields.email, "main@x");
        assert_eq!(fields.display_name, "Alice Liddell");
        assert!(fields.active);
        let minimal = user_fields(&json!({"userName": "bob@x", "active": "False"})).unwrap();
        assert_eq!(minimal.email, "bob@x");
        assert!(!minimal.active);
        assert!(user_fields(&json!({"userName": ""})).is_err());
        assert!(user_fields(&json!({"userName": "x", "active": 3})).is_err());
    }

    #[test]
    fn user_patches_cover_the_common_provider_shapes() {
        let mut resource = Map::new();
        resource.insert("userName".into(), json!("alice"));
        patch_user(
            &mut resource,
            &json!({"op": "replace", "path": "active", "value": "False"}),
        )
        .unwrap();
        assert_eq!(resource["active"], json!(false));
        patch_user(
            &mut resource,
            &json!({"op": "Replace", "value": {"active": true, "displayName": "A"}}),
        )
        .unwrap();
        assert_eq!(resource["active"], json!(true));
        assert_eq!(resource["displayName"], json!("A"));
        patch_user(
            &mut resource,
            &json!({"op": "add", "path": "emails[type eq \"work\"].value", "value": "a@x"}),
        )
        .unwrap();
        assert_eq!(
            user_fields(&Value::Object(resource.clone())).unwrap().email,
            "a@x"
        );
        patch_user(
            &mut resource,
            &json!({"op": "replace", "path": "name.givenName", "value": "Al"}),
        )
        .unwrap();
        assert_eq!(resource["name"]["givenName"], json!("Al"));
        assert!(
            patch_user(
                &mut resource,
                &json!({"op": "replace", "path": "password", "value": "x"})
            )
            .is_err()
        );
        assert!(patch_user(&mut resource, &json!({"op": "move", "path": "active"})).is_err());
    }

    #[test]
    fn member_paths_name_one_member() {
        let id = Id([3; 16]);
        assert_eq!(
            member_in_path(&format!("members[value eq \"{id}\"]")),
            Some(id)
        );
        assert_eq!(member_in_path("members[display eq \"x\"]"), None);
        assert_eq!(member_in_path("members"), None);
    }

    #[test]
    fn pages_slice_with_one_based_indices() {
        let resources: Vec<Value> = (0..5).map(|index| json!(index)).collect();
        let mut query = HashMap::new();
        query.insert("startIndex".into(), "2".into());
        query.insert("count".into(), "2".into());
        let page = page(resources, &query);
        assert_eq!(page["totalResults"], json!(5));
        assert_eq!(page["Resources"], json!([1, 2]));
    }
}
