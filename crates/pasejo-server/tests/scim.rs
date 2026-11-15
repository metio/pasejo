// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

//! SCIM 2.0 provisioning: token authentication, users, groups, filters,
//! paging, the pending actions provisioning produces, and the limits of
//! what one organization's SCIM client can affect.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{Harness, Member, Reply};
use pasejo_protocol::Id;
use pasejo_protocol::api::{Action, Actions, Me, ScimTokenCreated, ScimTokens};
use pasejo_protocol::log::{RoleChange, Settings};
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

struct Scim<'a> {
    harness: &'a Harness,
    org: Id,
    token: String,
}

impl Scim<'_> {
    async fn send(&self, method: Method, path: &str, body: Option<Value>) -> Reply {
        self.send_as(&self.token, method, path, body).await
    }

    async fn send_as(&self, token: &str, method: Method, path: &str, body: Option<Value>) -> Reply {
        let mut request = self
            .harness
            .http
            .request(
                method,
                format!("{}/scim/v2/{}{path}", self.harness.url, self.org),
            )
            .bearer_auth(token);
        if let Some(body) = body {
            request = request
                .header("content-type", "application/scim+json")
                .body(body.to_string());
        }
        Reply::from(request.send().await.unwrap()).await
    }

    async fn create_user(&self, user_name: &str, email: &str) -> Value {
        let reply = self
            .send(
                Method::POST,
                "/Users",
                Some(json!({
                    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                    "userName": user_name,
                    "externalId": format!("ext-{user_name}"),
                    "name": {"givenName": capitalize(user_name), "familyName": "Example"},
                    "emails": [{"value": email, "primary": true}],
                    "active": true,
                })),
            )
            .await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
        reply.json()
    }

    async fn patch(&self, path: &str, operations: Value) -> Reply {
        self.send(
            Method::PATCH,
            path,
            Some(json!({"schemas": ["urn:ietf:params:scim:api:messages:2.0:PatchOp"], "Operations": operations})),
        )
        .await
    }
}

fn capitalize(text: &str) -> String {
    common::capitalize(text)
}

fn id_of(resource: &Value) -> Id {
    resource["id"].as_str().unwrap().parse().unwrap()
}

async fn setup(harness: &Harness) -> (Member, Id, String) {
    let alice = harness.member("alice").await;
    let org = harness.org(&alice, Settings::default()).await;
    let created: ScimTokenCreated = alice
        .client
        .post_empty(&format!("/v1/orgs/{org}/scim-tokens"))
        .await
        .ok();
    (alice, org, created.token)
}

async fn actions(admin: &Member, org: Id) -> Vec<Action> {
    admin
        .client
        .get(&format!("/v1/orgs/{org}/actions"))
        .await
        .ok::<Actions>()
        .actions
}

fn assert_scim_error(reply: &Reply, status: StatusCode, scim_type: Option<&str>) {
    assert_eq!(reply.status, status, "{}", reply.text());
    let body: Value = reply.json();
    assert_eq!(
        body["schemas"][0],
        "urn:ietf:params:scim:api:messages:2.0:Error"
    );
    assert_eq!(body["status"], status.as_u16().to_string());
    if let Some(scim_type) = scim_type {
        assert_eq!(body["scimType"], scim_type);
    }
}

#[tokio::test]
async fn every_request_needs_a_live_token_of_its_organization() {
    let harness = Harness::start().await;
    let (alice, org, token) = setup(&harness).await;
    let scim = Scim {
        harness: &harness,
        org,
        token: token.clone(),
    };
    for path in [
        "/ServiceProviderConfig",
        "/ResourceTypes",
        "/Schemas",
        "/Users",
        "/Groups",
    ] {
        assert_scim_error(
            &scim.send_as("wrong", Method::GET, path, None).await,
            StatusCode::UNAUTHORIZED,
            None,
        );
        assert_eq!(
            scim.send(Method::GET, path, None).await.status,
            StatusCode::OK,
            "{path}"
        );
    }
    let unauthenticated = harness
        .http
        .get(format!("{}/scim/v2/{org}/Users", harness.url))
        .send()
        .await
        .unwrap();
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);
    // A token of another organization.
    let bob = harness.member("bob").await;
    let other = harness.org(&bob, Settings::default()).await;
    let foreign: ScimTokenCreated = bob
        .client
        .post_empty(&format!("/v1/orgs/{other}/scim-tokens"))
        .await
        .ok();
    assert_scim_error(
        &scim
            .send_as(&foreign.token, Method::GET, "/Users", None)
            .await,
        StatusCode::UNAUTHORIZED,
        None,
    );
    // A revoked token.
    let listed: ScimTokens = alice
        .client
        .get(&format!("/v1/orgs/{org}/scim-tokens"))
        .await
        .ok();
    assert_eq!(listed.tokens.len(), 1);
    let id = listed.tokens[0].id;
    alice
        .client
        .delete(&format!("/v1/orgs/{org}/scim-tokens/{id}"))
        .await;
    let listed: ScimTokens = alice
        .client
        .get(&format!("/v1/orgs/{org}/scim-tokens"))
        .await
        .ok();
    assert!(listed.tokens.is_empty());
    assert_scim_error(
        &scim.send(Method::GET, "/Users", None).await,
        StatusCode::UNAUTHORIZED,
        None,
    );
}

#[tokio::test]
async fn discovery_describes_what_the_server_supports() {
    let harness = Harness::start().await;
    let (_, org, token) = setup(&harness).await;
    let scim = Scim {
        harness: &harness,
        org,
        token,
    };
    let config: Value = scim
        .send(Method::GET, "/ServiceProviderConfig", None)
        .await
        .json();
    assert_eq!(config["patch"]["supported"], true);
    assert_eq!(config["filter"]["supported"], true);
    assert_eq!(config["bulk"]["supported"], false);
    assert_eq!(
        config["authenticationSchemes"][0]["type"],
        "oauthbearertoken"
    );
    let types: Value = scim.send(Method::GET, "/ResourceTypes", None).await.json();
    let names: Vec<&str> = types["Resources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["User", "Group"]);
}

#[tokio::test]
async fn users_are_created_read_listed_and_filtered() {
    let harness = Harness::start().await;
    let (_, org, token) = setup(&harness).await;
    let scim = Scim {
        harness: &harness,
        org,
        token,
    };
    let bob = scim.create_user("bob", "bob@example.com").await;
    assert_eq!(
        bob["schemas"][0],
        "urn:ietf:params:scim:schemas:core:2.0:User"
    );
    assert_eq!(bob["active"], true);
    assert_eq!(bob["meta"]["resourceType"], "User");
    let location = bob["meta"]["location"].as_str().unwrap();
    assert!(
        location.ends_with(&format!("/scim/v2/{org}/Users/{}", id_of(&bob))),
        "{location}"
    );
    let fetched: Value = scim
        .send(Method::GET, &format!("/Users/{}", id_of(&bob)), None)
        .await
        .json();
    assert_eq!(fetched, bob);
    scim.create_user("carol", "carol@example.com").await;
    scim.create_user("dave", "dave@example.com").await;

    let all: Value = scim.send(Method::GET, "/Users", None).await.json();
    assert_eq!(all["totalResults"], 3);
    let page: Value = scim
        .send(Method::GET, "/Users?startIndex=2&count=1", None)
        .await
        .json();
    assert_eq!(
        (
            page["totalResults"].as_u64(),
            page["startIndex"].as_u64(),
            page["itemsPerPage"].as_u64()
        ),
        (Some(3), Some(2), Some(1))
    );
    assert_eq!(page["Resources"][0]["userName"], "carol");

    let by_name: Value = scim
        .send(Method::GET, "/Users?filter=userName%20eq%20%22BOB%22", None)
        .await
        .json();
    assert_eq!(by_name["totalResults"], 1);
    assert_eq!(by_name["Resources"][0]["id"], bob["id"]);
    let by_external: Value = scim
        .send(
            Method::GET,
            "/Users?filter=externalId%20eq%20%22ext-carol%22",
            None,
        )
        .await
        .json();
    assert_eq!(by_external["Resources"][0]["userName"], "carol");
    let none: Value = scim
        .send(
            Method::GET,
            "/Users?filter=userName%20eq%20%22nobody%22",
            None,
        )
        .await
        .json();
    assert_eq!(none["totalResults"], 0);
    assert_scim_error(
        &scim
            .send(Method::GET, "/Users?filter=userName%20co%20%22b%22", None)
            .await,
        StatusCode::BAD_REQUEST,
        Some("invalidFilter"),
    );
    assert_scim_error(
        &scim
            .send(Method::GET, "/Users?filter=title%20eq%20%22x%22", None)
            .await,
        StatusCode::BAD_REQUEST,
        Some("invalidFilter"),
    );
    assert_scim_error(
        &scim
            .send(Method::GET, &format!("/Users/{}", Id::random()), None)
            .await,
        StatusCode::NOT_FOUND,
        None,
    );
    assert_scim_error(
        &scim.send(Method::GET, "/Users/not-an-id", None).await,
        StatusCode::NOT_FOUND,
        None,
    );
}

#[tokio::test]
async fn user_names_are_unique_and_required() {
    let harness = Harness::start().await;
    let (_, org, token) = setup(&harness).await;
    let scim = Scim {
        harness: &harness,
        org,
        token,
    };
    scim.create_user("bob", "bob@example.com").await;
    let duplicate = scim
        .send(
            Method::POST,
            "/Users",
            Some(json!({"userName": "Bob", "emails": [{"value": "other@example.com"}]})),
        )
        .await;
    assert_scim_error(&duplicate, StatusCode::CONFLICT, Some("uniqueness"));
    assert_scim_error(
        &scim
            .send(Method::POST, "/Users", Some(json!({"userName": " "})))
            .await,
        StatusCode::BAD_REQUEST,
        None,
    );
    assert_scim_error(
        &scim
            .send(
                Method::POST,
                "/Users",
                Some(json!({"userName": "x", "active": "maybe"})),
            )
            .await,
        StatusCode::BAD_REQUEST,
        None,
    );
    assert_scim_error(
        &scim
            .send(Method::POST, "/Users", Some(json!("not an object")))
            .await,
        StatusCode::BAD_REQUEST,
        None,
    );
}

#[tokio::test]
async fn provisioned_users_become_join_actions_and_deactivation_removes_them() {
    let harness = Harness::start().await;
    let (alice, org, token) = setup(&harness).await;
    let scim = Scim {
        harness: &harness,
        org,
        token,
    };
    let bob_member = harness.member("bob").await;
    let bob = scim.create_user("bob", "bob@example.com").await;
    // The SCIM user links to the account that signed in with that email.
    assert_eq!(id_of(&bob), bob_member.user());
    let pending = actions(&alice, org).await;
    assert!(pending.iter().any(|action| matches!(action, Action::JoinMember { user, email, .. } if *user == bob_member.user() && email == "bob@example.com")), "{pending:?}");

    harness
        .join(&alice, &bob_member, org, RoleChange::Member)
        .await;
    bob_member
        .client
        .get(&format!("/v1/orgs/{org}/stores"))
        .await
        .ok::<Value>();
    let patched = scim
        .patch(
            &format!("/Users/{}", id_of(&bob)),
            json!([{"op": "replace", "path": "active", "value": false}]),
        )
        .await;
    assert_eq!(patched.json::<Value>()["active"], false);
    // Suspended in this organization at once, before any log entry.
    bob_member
        .client
        .get(&format!("/v1/orgs/{org}/stores"))
        .await
        .expect(StatusCode::FORBIDDEN, "pending_device");
    let pending = actions(&alice, org).await;
    assert!(
        pending.iter().any(
            |action| matches!(action, Action::RemoveMember { user } if *user == bob_member.user())
        ),
        "{pending:?}"
    );
    // Reactivation lifts the suspension.
    scim.patch(
        &format!("/Users/{}", id_of(&bob)),
        json!([{"op": "replace", "value": {"active": "True"}}]),
    )
    .await;
    bob_member
        .client
        .get(&format!("/v1/orgs/{org}/stores"))
        .await
        .ok::<Value>();
}

#[tokio::test]
async fn one_organizations_scim_client_cannot_touch_the_account_elsewhere() {
    let harness = Harness::start().await;
    let (_, org, token) = setup(&harness).await;
    let scim = Scim {
        harness: &harness,
        org,
        token,
    };
    // Bob belongs to his own organization.
    let bob = harness.member("bob").await;
    let home = harness.org(&bob, Settings::default()).await;
    let resource = scim
        .send(
            Method::POST,
            "/Users",
            Some(json!({"userName": "bob", "displayName": "Mallory's Puppet", "emails": [{"value": "bob@example.com"}]})),
        )
        .await
        .json::<Value>();
    let id = id_of(&resource);
    scim.patch(&format!("/Users/{id}"), json!([{"op": "replace", "path": "emails[type eq \"work\"].value", "value": "mallory@example.com"}])).await;
    assert_eq!(
        scim.send(Method::DELETE, &format!("/Users/{id}"), None)
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    // His name, email, sessions and other organization are untouched.
    let me: Me = bob.client.get("/v1/me").await.ok();
    assert_eq!(
        (me.name.as_str(), me.email.as_str()),
        ("Bob", "bob@example.com")
    );
    bob.client
        .get(&format!("/v1/orgs/{home}/stores"))
        .await
        .ok::<Value>();
}

#[tokio::test]
async fn replace_and_patch_rewrite_the_resource() {
    let harness = Harness::start().await;
    let (_, org, token) = setup(&harness).await;
    let scim = Scim {
        harness: &harness,
        org,
        token,
    };
    let bob = scim.create_user("bob", "bob@example.com").await;
    let path = format!("/Users/{}", id_of(&bob));
    let replaced: Value = scim
        .send(
            Method::PUT,
            &path,
            Some(
                json!({"userName": "robert", "displayName": "Robert", "id": "ignored", "meta": {}}),
            ),
        )
        .await
        .json();
    assert_eq!(replaced["userName"], "robert");
    assert_eq!(replaced["id"], bob["id"]);
    assert!(replaced.get("externalId").is_none());
    let patched: Value = scim
        .patch(
            &path,
            json!([
                {"op": "add", "path": "externalId", "value": "ext-9"},
                {"op": "replace", "path": "name.givenName", "value": "Rob"},
                {"op": "remove", "path": "displayName"},
            ]),
        )
        .await
        .json();
    assert_eq!(patched["externalId"], "ext-9");
    assert_eq!(patched["name"]["givenName"], "Rob");
    assert!(patched.get("displayName").is_none());
    assert_scim_error(
        &scim
            .patch(&path, json!([{"op": "remove", "path": "userName"}]))
            .await,
        StatusCode::BAD_REQUEST,
        Some("invalidPath"),
    );
    assert_scim_error(
        &scim
            .patch(&path, json!([{"op": "move", "path": "x"}]))
            .await,
        StatusCode::BAD_REQUEST,
        Some("invalidSyntax"),
    );
    assert_scim_error(
        &scim
            .patch(
                &path,
                json!([{"op": "replace", "path": "roles", "value": []}]),
            )
            .await,
        StatusCode::BAD_REQUEST,
        Some("invalidPath"),
    );
    assert_scim_error(
        &scim
            .send(
                Method::PATCH,
                &path,
                Some(json!({"schemas": ["urn:other"], "Operations": []})),
            )
            .await,
        StatusCode::BAD_REQUEST,
        None,
    );
    // Renaming onto a taken user name conflicts.
    scim.create_user("carol", "carol@example.com").await;
    assert_scim_error(
        &scim
            .patch(
                &path,
                json!([{"op": "replace", "path": "userName", "value": "CAROL"}]),
            )
            .await,
        StatusCode::CONFLICT,
        Some("uniqueness"),
    );
}

#[tokio::test]
async fn deleted_users_disappear_and_can_be_provisioned_again() {
    let harness = Harness::start().await;
    let (_, org, token) = setup(&harness).await;
    let scim = Scim {
        harness: &harness,
        org,
        token,
    };
    let bob = scim.create_user("bob", "bob@example.com").await;
    let path = format!("/Users/{}", id_of(&bob));
    assert_eq!(
        scim.send(Method::DELETE, &path, None).await.status,
        StatusCode::NO_CONTENT
    );
    assert_scim_error(
        &scim.send(Method::GET, &path, None).await,
        StatusCode::NOT_FOUND,
        None,
    );
    assert_scim_error(
        &scim.send(Method::DELETE, &path, None).await,
        StatusCode::NOT_FOUND,
        None,
    );
    let list: Value = scim.send(Method::GET, "/Users", None).await.json();
    assert_eq!(list["totalResults"], 0);
    // The name is free again, and the same account comes back.
    let again = scim.create_user("bob", "bob@example.com").await;
    assert_eq!(again["id"], bob["id"]);
}

#[tokio::test]
async fn groups_become_team_actions() {
    let harness = Harness::start().await;
    let (alice, org, token) = setup(&harness).await;
    let scim = Scim {
        harness: &harness,
        org,
        token,
    };
    let bob = id_of(&scim.create_user("bob", "bob@example.com").await);
    let carol = id_of(&scim.create_user("carol", "carol@example.com").await);
    let group: Value = scim
        .send(
            Method::POST,
            "/Groups",
            Some(json!({"displayName": "ops", "members": [{"value": bob.to_string()}]})),
        )
        .await
        .json();
    assert_eq!(group["members"][0]["value"], bob.to_string());
    let team = id_of(&group);
    // Teams take members once they belong to the organization.
    let pending = actions(&alice, org).await;
    assert!(
        pending.iter().any(|action| matches!(action, Action::SyncTeam { team: t, name, exists: false, add, .. } if *t == team && name == "ops" && add.is_empty())),
        "{pending:?}"
    );
    let bob_member = harness.member("bob").await;
    harness
        .join(&alice, &bob_member, org, RoleChange::Member)
        .await;
    let pending = actions(&alice, org).await;
    assert!(
        pending.iter().any(|action| matches!(action, Action::SyncTeam { team: t, add, .. } if *t == team && add == &vec![bob])),
        "{pending:?}"
    );
    // Members must be users this organization provisioned.
    let stranger = Id::random();
    assert_scim_error(
        &scim
            .send(
                Method::POST,
                "/Groups",
                Some(json!({"displayName": "x", "members": [{"value": stranger.to_string()}]})),
            )
            .await,
        StatusCode::BAD_REQUEST,
        None,
    );
    assert_scim_error(
        &scim
            .send(Method::POST, "/Groups", Some(json!({"displayName": ""})))
            .await,
        StatusCode::BAD_REQUEST,
        None,
    );
    let path = format!("/Groups/{team}");
    let added: Value = scim
        .patch(
            &path,
            json!([{"op": "add", "path": "members", "value": [{"value": carol.to_string()}]}]),
        )
        .await
        .json();
    assert_eq!(added["members"].as_array().unwrap().len(), 2);
    let removed: Value = scim
        .patch(
            &path,
            json!([{"op": "remove", "path": format!("members[value eq \"{bob}\"]")}]),
        )
        .await
        .json();
    assert_eq!(
        removed["members"],
        json!([{"value": carol.to_string(), "type": "User"}])
    );
    let renamed: Value = scim
        .patch(
            &path,
            json!([{"op": "replace", "path": "displayName", "value": "platform"}]),
        )
        .await
        .json();
    assert_eq!(renamed["displayName"], "platform");
    let replaced: Value = scim
        .send(
            Method::PUT,
            &path,
            Some(json!({"displayName": "platform", "members": []})),
        )
        .await
        .json();
    assert_eq!(replaced["members"], json!([]));
    let filtered: Value = scim
        .send(
            Method::GET,
            "/Groups?filter=displayName%20eq%20%22platform%22",
            None,
        )
        .await
        .json();
    assert_eq!(filtered["totalResults"], 1);
    assert_eq!(
        scim.send(Method::DELETE, &path, None).await.status,
        StatusCode::NO_CONTENT
    );
    assert_scim_error(
        &scim.send(Method::GET, &path, None).await,
        StatusCode::NOT_FOUND,
        None,
    );
}

#[tokio::test]
async fn provisioning_is_audited_under_the_token() {
    let harness = Harness::start().await;
    let (_, org, token) = setup(&harness).await;
    let scim = Scim {
        harness: &harness,
        org,
        token,
    };
    let bob = scim.create_user("bob", "bob@example.com").await;
    scim.send(Method::DELETE, &format!("/Users/{}", id_of(&bob)), None)
        .await;
    scim.send_as("wrong", Method::GET, "/Users", None).await;
    let events = pasejo_server::audit::events(&harness.state, &org.to_string(), 0, 1000)
        .await
        .unwrap();
    for action in ["scim.user.create", "scim.user.delete"] {
        let event = events.iter().find(|event| event.action == action).unwrap();
        assert_eq!(event.session_kind.as_deref(), Some("scim"), "{event:?}");
    }
    assert!(
        events
            .iter()
            .any(|event| event.action == "request.refused" && event.outcome == "denied")
    );
}
