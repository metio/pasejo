// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The Kubernetes bridge against a real server: enrollment through a
//! workload token, approval, reads by store label or identifier with lines
//! and fields, the status codes external-secrets-operator acts on, and the
//! tenant label in the audit trail. Client calls run in `block_in_place`
//! so the server keeps serving on the other runtime workers.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::too_many_lines)]

mod common;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::Harness;
use ed25519_dalek::SigningKey;
use pasejo_bridge::bridge::{Bridge, Failure, Part};
use pasejo_bridge::config::Settings;
use pasejo_bridge::device::Device as BridgeDevice;
use pasejo_bridge::http::{AppState, router};
use pasejo_client::session::Poll;
use pasejo_client::store::Changes;
use pasejo_client::{Client, Clock, DeviceIdentity};
use pasejo_protocol::Id;
use pasejo_protocol::api::ErrorCode;
use pasejo_protocol::device::DeviceKind;
use pasejo_protocol::encoding::b64;
use pasejo_protocol::log::{
    Body, OrgLog, Principal, RoleChange, Settings as OrgSettings, StoreRoleChange,
};
use pasejo_server::audit;
use rand_core::OsRng;
use reqwest::StatusCode;
use serde_json::Value;
use tempfile::TempDir;

fn sync<T>(work: impl FnOnce() -> T) -> T {
    tokio::task::block_in_place(work)
}

struct Admin {
    client: Client,
    identity: age::x25519::Identity,
}

impl Admin {
    fn identities(&self) -> Vec<Box<dyn age::Identity>> {
        vec![Box::new(self.identity.clone())]
    }
}

fn clock(harness: &Harness) -> Clock {
    let clock = Arc::clone(&harness.state.clock);
    Arc::new(move || clock.now())
}

async fn admin(harness: &Harness) -> Admin {
    let identity = age::x25519::Identity::generate();
    let device = DeviceIdentity {
        id: Id::random(),
        signing_key: SigningKey::generate(&mut OsRng),
        kind: DeviceKind::Age,
        recipient: identity.to_public().to_string(),
        label: "laptop".into(),
    };
    let mut client = Client::new(
        &harness.url,
        device,
        clock(harness),
        Duration::from_secs(20),
    );
    let authorization = sync(|| client.start_login()).unwrap();
    harness.idp.set_user("alice", "alice@example.com", "Alice");
    let page = harness
        .follow_to_callback(&authorization.verification_uri_complete)
        .await;
    assert_eq!(page.status, StatusCode::OK, "{}", page.text());
    let approval = common::between(&page.text(), "name=\"approval\" value=\"", "\"").unwrap();
    assert_eq!(
        harness.approve(&approval, "approve").await.status,
        StatusCode::OK
    );
    match sync(|| client.poll_login(&authorization)).unwrap() {
        Poll::Done(_) => {}
        other => panic!("{other:?}"),
    }
    Admin { client, identity }
}

/// Alice's organization with a store labelled `prod` holding three secrets,
/// and a bridge configured for it but not approved yet.
struct World {
    harness: Harness,
    alice: Admin,
    log: OrgLog,
    store: Id,
    dir: TempDir,
    device_id: Id,
    settings_text: String,
    url: String,
    team_a: String,
    team_b: String,
}

async fn world() -> World {
    let harness = Harness::start().await;
    let alice = admin(&harness).await;
    let org = Id::random();
    let mut log = sync(|| alice.client.create_org(org, "Acme", OrgSettings::default())).unwrap();
    let (ring, snapshot) = sync(|| alice.client.create_store(&mut log, "prod")).unwrap();
    let mut changes = Changes::default();
    changes.secrets.insert(
        "db".into(),
        Some("hunter2\nuser:   app\nhost: db.internal".into()),
    );
    changes.secrets.insert("api/key".into(), Some("k3y".into()));
    sync(|| alice.client.commit(&log, &ring, &snapshot, &changes)).unwrap();

    let dir = tempfile::tempdir().unwrap();
    let device = BridgeDevice::generate();
    let device_id = device.id;
    device.create(&dir.path().join("device.json")).unwrap();
    std::fs::write(
        dir.path().join("token"),
        harness.workload_jwt("system:serviceaccount:ci:bridge", "pasejo-server", 600),
    )
    .unwrap();
    let (team_a, hash_a) = pasejo_bridge::credential();
    let (team_b, hash_b) = pasejo_bridge::credential();
    let settings_text = format!(
        "server = \"{}\"\norg = \"{org}\"\ngenesis = \"{}\"\ndevice_file = \"{}\"\nworkload_token_file = \"{}\"\nlabel = \"cluster-1\"\ntimeout_seconds = 5\n[[credentials]]\ntenant = \"team-a\"\ntoken_sha256 = \"{hash_a}\"\n[[credentials]]\ntenant = \"team-b\"\ntoken_sha256 = \"{hash_b}\"\n",
        harness.url,
        b64(&log.genesis_hash().0),
        dir.path().join("device.json").display(),
        dir.path().join("token").display(),
    );
    let settings = Settings::parse(&settings_text).unwrap();
    let bridge = Bridge::new(
        &settings,
        BridgeDevice::load(&settings.config.device_file).unwrap(),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let app = router(AppState {
        bridge: Arc::new(Mutex::new(bridge)),
        credentials: Arc::new(settings.credentials),
    });
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    World {
        harness,
        alice,
        log,
        store: snapshot.store,
        dir,
        device_id,
        settings_text,
        url,
        team_a: team_a.to_string(),
        team_b: team_b.to_string(),
    }
}

impl World {
    async fn get(&self, path: &str, token: Option<&str>) -> (StatusCode, Value) {
        let mut request = self.harness.http.get(format!("{}{path}", self.url));
        if let Some(token) = token {
            request = request.bearer_auth(token);
        }
        let response = request.send().await.unwrap();
        let status = response.status();
        assert_eq!(
            response
                .headers()
                .get("cache-control")
                .and_then(|value| value.to_str().ok()),
            Some("no-store"),
            "{path}"
        );
        (status, response.json().await.unwrap())
    }

    async fn value(&self, path: &str) -> String {
        let (status, body) = self.get(path, Some(&self.team_a)).await;
        assert_eq!(status, StatusCode::OK, "{path}: {body}");
        body["value"].as_str().unwrap().to_owned()
    }

    /// Alice makes the bridge's machine user a member, approves its
    /// device, grants it `prod` and wraps the keys for it.
    fn approve(&mut self) {
        let org = self.log.state().org;
        let enrollment = self
            .alice
            .client
            .enrollments(org)
            .unwrap()
            .into_iter()
            .find(|enrollment| enrollment.device.id == self.device_id)
            .expect("the bridge enrolled itself");
        assert_eq!(enrollment.device.label, "cluster-1");
        assert!(enrollment.user_email.is_empty());
        assert_eq!(enrollment.user_name, "ci");
        let machine = enrollment.device.user;
        self.alice
            .client
            .append_fresh(
                &mut self.log,
                Body::MemberRole {
                    user: machine,
                    role: RoleChange::Member,
                },
            )
            .unwrap();
        self.alice
            .client
            .append_fresh(
                &mut self.log,
                Body::DeviceAdd {
                    device: enrollment.device,
                },
            )
            .unwrap();
        self.grant(StoreRoleChange::Reader);
    }

    fn machine(&self) -> Id {
        let org = self.log.state().org;
        self.alice
            .client
            .users(org)
            .unwrap()
            .into_iter()
            .find(|user| user.machine)
            .unwrap()
            .id
    }

    fn grant(&mut self, role: StoreRoleChange) {
        let machine = self.machine();
        self.alice
            .client
            .grant(
                &mut self.log,
                self.store,
                Principal::User(machine),
                role,
                None,
            )
            .unwrap();
        self.alice
            .client
            .run_actions(&mut self.log, &self.alice.identities(), &[0; 32])
            .unwrap();
    }

    async fn tenant_reads(&self) -> Vec<String> {
        let org = self.log.state().org.to_string();
        audit::events(&self.harness.state, &org, 0, 10_000)
            .await
            .unwrap()
            .into_iter()
            .filter(|event| event.action == "object.read")
            .filter_map(|event| event.detail)
            .collect()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_bridge_enrolls_waits_for_approval_and_then_serves_values() {
    let mut world = world().await;
    assert_eq!(world.get("/healthz", None).await.0, StatusCode::OK);

    // Credentials come first, whatever the device's state.
    let (status, body) = world.get("/v1/secrets/prod/db", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
    let (status, _) = world
        .get("/v1/secrets/prod/db", Some("pasejo-bridge-wrong"))
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // An unapproved device answers 503 and enrolls itself.
    let (status, body) = world.get("/v1/secrets/prod/db", Some(&world.team_a)).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    assert!(
        body["error"]
            .as_str()
            .unwrap()
            .contains("waits for approval"),
        "{body}"
    );
    let (status, _) = world.get("/readyz", None).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    // Asking again keeps the single enrollment.
    let (status, _) = world.get("/v1/secrets/prod/db", Some(&world.team_a)).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    let pending = sync(|| world.alice.client.enrollments(world.log.state().org)).unwrap();
    assert_eq!(pending.len(), 1);

    sync(|| world.approve());
    let (status, body) = world.get("/readyz", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    assert_eq!(
        world.value("/v1/secrets/prod/db").await,
        "hunter2\nuser:   app\nhost: db.internal"
    );
    assert_eq!(world.value("/v1/secrets/prod/db?line=1").await, "hunter2");
    assert_eq!(
        world.value("/v1/secrets/prod/db?line=-1").await,
        "host: db.internal"
    );
    assert_eq!(world.value("/v1/secrets/prod/db?field=user").await, "app");
    assert_eq!(world.value("/v1/secrets/prod/api/key").await, "k3y");
    assert_eq!(
        world
            .value(&format!("/v1/secrets/{}/db?field=host", world.store))
            .await,
        "db.internal"
    );

    let (status, _) = world
        .get("/v1/secrets/prod/db?field=password", Some(&world.team_a))
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _) = world
        .get("/v1/secrets/prod/db?line=9", Some(&world.team_a))
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    for bad in [
        "line=0",
        "line=x",
        "line=1&field=user",
        "colour=red",
        "field=",
    ] {
        let (status, body) = world
            .get(&format!("/v1/secrets/prod/db?{bad}"), Some(&world.team_a))
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}: {body}");
    }

    // Only a name absent from the verified index is gone.
    let (status, _) = world
        .get("/v1/secrets/prod/missing", Some(&world.team_a))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, body) = world
        .get("/v1/secrets/staging/db", Some(&world.team_a))
        .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    assert!(body["error"].as_str().unwrap().contains("'staging'"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn every_read_is_audited_under_its_tenant() {
    let mut world = world().await;
    let _ = world.get("/v1/secrets/prod/db", Some(&world.team_a)).await;
    sync(|| world.approve());
    world.value("/v1/secrets/prod/db").await;
    let (status, _) = world
        .get("/v1/secrets/prod/api/key", Some(&world.team_b))
        .await;
    assert_eq!(status, StatusCode::OK);
    let reads = world.tenant_reads().await;
    assert_eq!(reads, vec!["tenant team-a", "tenant team-b"], "{reads:?}");
    // A name absent from the index never reaches the server.
    let _ = world
        .get("/v1/secrets/prod/missing", Some(&world.team_b))
        .await;
    assert_eq!(world.tenant_reads().await.len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn losing_the_grant_makes_reads_unavailable_and_never_not_found() {
    let mut world = world().await;
    let _ = world.get("/v1/secrets/prod/db", Some(&world.team_a)).await;
    sync(|| world.approve());
    world.value("/v1/secrets/prod/db").await;
    sync(|| world.grant(StoreRoleChange::None));
    for path in [
        "/v1/secrets/prod/db",
        &format!("/v1/secrets/{}/db", world.store),
    ] {
        let (status, body) = world.get(path, Some(&world.team_a)).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{path}: {body}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_unreachable_server_or_unreadable_token_is_unavailable() {
    let world = world().await;
    let unreachable = world
        .settings_text
        .replace(&world.harness.url, "http://127.0.0.1:9");
    let settings = Settings::parse(&unreachable).unwrap();
    let mut bridge = Bridge::new(
        &settings,
        BridgeDevice::load(&settings.config.device_file).unwrap(),
    );
    let failure = sync(|| bridge.read("prod", "db", &Part::All, "team-a")).unwrap_err();
    assert!(matches!(failure, Failure::Unavailable(_)), "{failure:?}");

    std::fs::remove_file(world.dir.path().join("token")).unwrap();
    let settings = Settings::parse(&world.settings_text).unwrap();
    let mut bridge = Bridge::new(
        &settings,
        BridgeDevice::load(&settings.config.device_file).unwrap(),
    );
    let Failure::Unavailable(message) =
        sync(|| bridge.read("prod", "db", &Part::All, "team-a")).unwrap_err()
    else {
        panic!("expected unavailable");
    };
    assert!(message.contains("workload token"), "{message}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn only_machine_sessions_name_a_tenant_and_labels_are_checked() {
    let mut world = world().await;
    let _ = world.get("/v1/secrets/prod/db", Some(&world.team_a)).await;
    sync(|| world.approve());

    let org = world.log.state().org;
    let ring = sync(|| {
        world
            .alice
            .client
            .keys(&world.log, world.store, &world.alice.identities())
    })
    .unwrap();
    let snapshot = sync(|| world.alice.client.index(&world.log, &ring, None)).unwrap();
    let names = [(pasejo_client::store::Kind::Secret, "db".to_owned())];
    let person = sync(|| {
        world
            .alice
            .client
            .objects_for(&world.log, &ring, &snapshot, &names, Some("team-a"))
    })
    .unwrap_err();
    assert!(person.is(ErrorCode::BadRequest), "{person}");

    // The bridge's own device, signed in as the machine, with a malformed
    // label.
    let device = BridgeDevice::load(&world.dir.path().join("device.json")).unwrap();
    let mut machine = Client::new(
        &world.harness.url,
        device.identity("cluster-1"),
        clock(&world.harness),
        Duration::from_secs(20),
    );
    let token = std::fs::read_to_string(world.dir.path().join("token")).unwrap();
    sync(|| machine.workload_login(&token)).unwrap();
    let log = sync(|| machine.fetch_log(org, None, Some(world.log.genesis_hash()))).unwrap();
    let ring = sync(|| machine.keys(&log, world.store, &device.identities())).unwrap();
    let snapshot = sync(|| machine.index(&log, &ring, None)).unwrap();
    for bad in ["", "team a", "ä", &"x".repeat(129)] {
        let error =
            sync(|| machine.objects_for(&log, &ring, &snapshot, &names, Some(bad))).unwrap_err();
        assert!(error.is(ErrorCode::BadRequest), "{bad:?}: {error}");
    }
    let values =
        sync(|| machine.objects_for(&log, &ring, &snapshot, &names, Some("ns/team-c"))).unwrap();
    assert_eq!(values.len(), 1);
    assert!(
        world
            .tenant_reads()
            .await
            .contains(&"tenant ns/team-c".to_owned())
    );
}
