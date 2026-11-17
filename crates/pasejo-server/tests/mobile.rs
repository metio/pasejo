// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The apps' Rust core, `pasejo-mobile`, against the real server: sign-in
//! through the system browser, enrollment, unlocking, reads, autofill and
//! the checks that refuse what a dishonest server returns.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::too_many_lines)]

mod common;

use std::sync::Arc;
use std::time::Duration;

use common::Harness;
use ed25519_dalek::SigningKey;
use pasejo_client::session::Poll;
use pasejo_client::store::Changes;
use pasejo_client::transport::Clock;
use pasejo_client::{Client, DeviceIdentity};
use pasejo_mobile::{
    AppRequest, Device, DeviceSigner, DomainVerifier, FillRequest, KeyAgreement, MobileError,
    generate_secret, software_agreement, software_signer,
};
use pasejo_protocol::Id;
use pasejo_protocol::device::DeviceKind;
use pasejo_protocol::encoding::b64;
use pasejo_protocol::log::{Body, OrgLog, Settings};
use pasejo_protocol::store::OtpParams;
use rand_core::OsRng;
use reqwest::StatusCode;

const REDIRECT: &str = "wtf.metio.pasejo:/auth";
const CERT: &str = "14:6D:E9:83:C5:73:06:50:D8:EE:B9:95:2F:34:FC:64:16:A0:83:42:E6:1D:BE:A8:8A:04:96:B2:3F:CF:44:E5";

fn sync<T>(work: impl FnOnce() -> T) -> T {
    tokio::task::block_in_place(work)
}

fn clock(harness: &Harness) -> Clock {
    let clock = Arc::clone(&harness.state.clock);
    Arc::new(move || clock.now())
}

struct Laptop {
    client: Client,
    identity: age::x25519::Identity,
}

impl Laptop {
    fn identities(&self) -> Vec<Box<dyn age::Identity>> {
        vec![Box::new(self.identity.clone())]
    }
}

async fn laptop(harness: &Harness, subject: &str) -> Laptop {
    let identity = age::x25519::Identity::generate();
    let mut client = Client::new(
        &harness.url,
        DeviceIdentity {
            id: Id::random(),
            signer: Arc::new(SigningKey::generate(&mut OsRng)),
            kind: DeviceKind::Age,
            recipient: identity.to_public().to_string(),
            label: "laptop".into(),
        },
        clock(harness),
        Duration::from_secs(20),
    );
    let authorization = sync(|| client.start_login()).unwrap();
    harness.idp.set_user(
        subject,
        &format!("{subject}@example.com"),
        &common::capitalize(subject),
    );
    let page = harness
        .follow_to_callback(&authorization.verification_uri_complete)
        .await;
    assert_eq!(page.status, StatusCode::OK, "{}", page.text());
    let approval = common::between(&page.text(), "name=\"approval\" value=\"", "\"").unwrap();
    assert_eq!(
        harness.approve(&approval, "approve").await.status,
        StatusCode::OK
    );
    assert!(matches!(
        sync(|| client.poll_login(&authorization)).unwrap(),
        Poll::Done(_)
    ));
    Laptop { client, identity }
}

fn totp() -> OtpParams {
    OtpParams {
        secret: "JBSWY3DPEHPK3PXP".into(),
        otp_type: "totp".into(),
        algorithm: "sha1".into(),
        digits: 6,
        period: 30,
        counter: 0,
        skew: 1,
    }
}

struct Phone {
    device: Arc<Device>,
    signer: Arc<dyn DeviceSigner>,
    agreement: Arc<dyn KeyAgreement>,
}

fn phone(harness: &Harness) -> Phone {
    let signer = software_signer(generate_secret()).unwrap();
    let agreement = software_agreement(generate_secret()).unwrap();
    let device = Device::create(
        harness.url.clone(),
        "phone".into(),
        Arc::clone(&signer),
        agreement.public_key(),
        vec![],
    )
    .unwrap();
    Phone {
        device,
        signer,
        agreement,
    }
}

async fn sign_in(harness: &Harness, phone: &Phone, subject: &str) {
    let url = phone.device.start_sign_in(REDIRECT.into());
    harness.idp.set_user(
        subject,
        &format!("{subject}@example.com"),
        &common::capitalize(subject),
    );
    let redirect = harness.follow_to_callback(&url).await.location().unwrap();
    assert!(redirect.starts_with(REDIRECT), "{redirect}");
    sync(|| phone.device.finish_sign_in(redirect)).unwrap();
}

/// Alice's laptop founds an organization with one store, and her phone
/// signs in, enrolls, is approved and receives the store's keys.
struct World {
    harness: Harness,
    alice: Laptop,
    log: OrgLog,
    org: Id,
    store: Id,
    phone: Phone,
}

async fn world() -> World {
    let harness = Harness::start().await;
    let alice = laptop(&harness, "alice").await;
    let org = Id::random();
    let mut log = sync(|| alice.client.create_org(org, "Acme", Settings::default())).unwrap();
    let (ring, snapshot) = sync(|| alice.client.create_store(&mut log, "Work")).unwrap();
    let mut changes = Changes::default();
    changes.secrets.insert(
        "example".into(),
        Some("hunter2\nurl: https://example.com\nuser: alice".into()),
    );
    changes.secrets.insert(
        "login".into(),
        Some("pw2\nurl: https://login.example.com\nuser: alice2".into()),
    );
    changes.secrets.insert(
        "mail".into(),
        Some(format!(
            "mailpw\nurl: androidapp://com.example.mail\nandroid-cert: {CERT}\nuser: bob"
        )),
    );
    changes.secrets.insert(
        "webmail".into(),
        Some("webpw\nurl: androidapp://com.example.webmail\nurl: https://login.webmail.net\nuser: carol".into()),
    );
    changes.secrets.insert(
        "hidden".into(),
        Some("nope\nurl: https://example.com\npasejo-fill: never".into()),
    );
    changes.otp.insert("example".into(), Some(totp()));
    sync(|| alice.client.commit(&log, &ring, &snapshot, &changes)).unwrap();

    let phone = phone(&harness);
    sync(|| phone.device.connect(None)).unwrap();
    sign_in(&harness, &phone, "alice").await;
    let genesis = log.genesis_hash().to_string();
    let enrollment = sync(|| phone.device.join(org.to_string(), genesis)).unwrap();
    assert!(enrollment.pending && !enrollment.active);
    approve(&alice, &mut log, &phone);
    World {
        store: ring.store,
        harness,
        alice,
        log,
        org,
        phone,
    }
}

/// Approves the phone's enrollment and wraps the store keys to it.
fn approve(alice: &Laptop, log: &mut OrgLog, phone: &Phone) {
    let org = log.state().org;
    let id: Id = phone.device.device_id().parse().unwrap();
    sync(|| {
        let enrollment = alice
            .client
            .enrollments(org)
            .unwrap()
            .into_iter()
            .find(|enrollment| enrollment.device.id == id)
            .unwrap();
        assert_eq!(enrollment.fingerprint, phone.device.fingerprint());
        alice
            .client
            .append_fresh(
                log,
                Body::DeviceAdd {
                    device: enrollment.device,
                },
            )
            .unwrap();
        alice
            .client
            .run_actions(log, &alice.identities(), &[0; 32])
            .unwrap();
    });
}

fn web(domain: &str) -> FillRequest {
    FillRequest {
        domains: vec![domain.into()],
        app: None,
    }
}

fn app(package: &str, certificate: &str) -> FillRequest {
    FillRequest {
        domains: vec![],
        app: Some(AppRequest {
            package: package.into(),
            certificates: vec![certificate.into()],
        }),
    }
}

fn audited(world: &World, action: &str) -> usize {
    let device = world.phone.device.device_id();
    sync(|| world.alice.client.audit_events(world.org, 0))
        .unwrap()
        .iter()
        .filter(|event| {
            event.action == action
                && event.actor_device.map(|id| id.to_string()) == Some(device.clone())
        })
        .count()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_phone_signs_in_enrolls_and_reads_its_stores() {
    let w = world().await;
    let phone = &w.phone.device;
    assert!(phone.signed_in());
    let info = sync(|| phone.refresh()).unwrap();
    assert_eq!(info.name, "Acme");
    assert!(info.active);
    assert!(!info.offline_reads);
    assert_eq!(
        sync(|| phone.enrollment()).unwrap(),
        pasejo_mobile::Enrollment {
            pending: false,
            active: true
        }
    );

    let stores = sync(|| phone.unlock(Arc::clone(&w.phone.agreement))).unwrap();
    assert_eq!(stores.len(), 1);
    assert_eq!(stores[0].id, w.store.to_string());
    assert_eq!(stores[0].label, "Work");
    assert_eq!(
        stores[0].secrets,
        ["example", "hidden", "login", "mail", "webmail"]
    );
    assert_eq!(stores[0].otp, ["example"]);
    assert!(phone.unlocked());
    assert_eq!(phone.stores(), stores);

    let text = sync(|| phone.reveal(w.store.to_string(), "example".into())).unwrap();
    assert_eq!(text, "hunter2\nurl: https://example.com\nuser: alice");
    let code = sync(|| phone.totp(w.store.to_string(), "example".into())).unwrap();
    assert_eq!(code.code.len(), 6);
    assert!((1..=30).contains(&code.remaining_seconds));
    assert_eq!(audited(&w, "object.read"), 2);

    assert_eq!(
        sync(|| phone.reveal(w.store.to_string(), "missing".into())).err(),
        Some(MobileError::NotFound)
    );
    assert_eq!(
        sync(|| phone.reveal(Id::random().to_string(), "example".into())).err(),
        Some(MobileError::NotFound)
    );

    phone.lock();
    assert!(!phone.unlocked());
    assert!(sync(|| phone.reveal(w.store.to_string(), "example".into())).is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn autofill_queries_without_reading_values_and_fills_one_match() {
    let w = world().await;
    let phone = &w.phone.device;
    sync(|| phone.unlock(Arc::clone(&w.phone.agreement))).unwrap();
    let reads = audited(&w, "object.read");

    let result = sync(|| phone.query(web("example.com"), None)).unwrap();
    let names: Vec<&str> = result.matches.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["example", "login"]);
    assert!(result.matches[0].exact && !result.matches[1].exact);
    assert_eq!(result.matches[0].username, "alice");
    assert_eq!(result.matches[0].store_label, "Work");
    assert!(result.lookalikes.is_empty());
    assert_eq!(audited(&w, "object.read"), reads, "a query reads no value");
    assert!(audited(&w, "lookup.read") >= 1);

    let credential =
        sync(|| phone.fill(web("example.com"), w.store.to_string(), "example".into())).unwrap();
    assert_eq!(
        (credential.username.as_str(), credential.password.as_str()),
        ("alice", "hunter2")
    );
    assert_eq!(credential.totp.as_deref().map(str::len), Some(6));

    // A registrable-domain match fills without the TOTP code.
    let login =
        sync(|| phone.fill(web("example.com"), w.store.to_string(), "login".into())).unwrap();
    assert_eq!(login.password, "pw2");
    assert_eq!(login.totp, None);

    // A secret that does not match the request is never released, and
    // neither is one that hides from autofill.
    for (domain, name) in [
        ("attacker.com", "example"),
        ("example.com", "hidden"),
        ("example.com", "mail"),
    ] {
        assert_eq!(
            sync(|| phone.fill(web(domain), w.store.to_string(), name.into())).err(),
            Some(MobileError::NotFound),
            "{domain} {name}"
        );
    }

    let lookalike = sync(|| phone.query(web("examp1e.com"), None)).unwrap();
    assert!(lookalike.matches.is_empty());
    assert_eq!(lookalike.lookalikes, ["example.com"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn apps_fill_only_with_their_pinned_certificate() {
    let w = world().await;
    let phone = &w.phone.device;
    sync(|| phone.unlock(Arc::clone(&w.phone.agreement))).unwrap();
    let result = sync(|| phone.query(app("com.example.mail", CERT), None)).unwrap();
    let names: Vec<&str> = result.matches.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["mail"]);
    let credential = sync(|| {
        phone.fill(
            app("com.example.mail", &CERT.replace(':', "").to_lowercase()),
            w.store.to_string(),
            "mail".into(),
        )
    })
    .unwrap();
    assert_eq!(
        (credential.username.as_str(), credential.password.as_str()),
        ("bob", "mailpw")
    );

    let forged = "AB".repeat(32);
    assert!(
        sync(|| phone.query(app("com.example.mail", &forged), None))
            .unwrap()
            .matches
            .is_empty()
    );
    assert_eq!(
        sync(|| phone.fill(
            app("com.example.mail", &forged),
            w.store.to_string(),
            "mail".into()
        ))
        .err(),
        Some(MobileError::NotFound)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_state_restores_the_signed_in_device() {
    let w = world().await;
    let state = w.phone.device.state();
    assert!(!state.contains("hunter2"));
    let restored = Device::restore(state, Arc::clone(&w.phone.signer), vec![]).unwrap();
    assert!(restored.signed_in());
    assert_eq!(restored.organization(), Some(w.org.to_string()));
    assert!(sync(|| restored.refresh()).unwrap().active);
    sync(|| restored.unlock(Arc::clone(&w.phone.agreement))).unwrap();
    assert_eq!(
        sync(|| restored.reveal(w.store.to_string(), "example".into())).unwrap(),
        "hunter2\nurl: https://example.com\nuser: alice"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn signing_out_ends_the_session_on_the_server() {
    let w = world().await;
    let phone = &w.phone.device;
    sync(|| phone.unlock(Arc::clone(&w.phone.agreement))).unwrap();
    let state = phone.state();
    sync(|| phone.sign_out()).unwrap();
    assert!(!phone.signed_in());
    assert!(!phone.unlocked());
    assert_eq!(
        sync(|| phone.refresh()).err(),
        Some(MobileError::NotSignedIn)
    );
    // The token in a copy of the old state no longer works either.
    let copy = Device::restore(state, Arc::clone(&w.phone.signer), vec![]).unwrap();
    assert_eq!(
        sync(|| copy.refresh()).err(),
        Some(MobileError::NotSignedIn)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn another_identity_cannot_unlock() {
    let w = world().await;
    let stranger = software_agreement(generate_secret()).unwrap();
    let error = sync(|| w.phone.device.unlock(stranger)).unwrap_err();
    assert!(matches!(error, MobileError::Verification { .. }), "{error}");
    assert!(!w.phone.device.unlocked());
}

struct Refusing(Vec<u8>);

impl KeyAgreement for Refusing {
    fn public_key(&self) -> Vec<u8> {
        self.0.clone()
    }

    fn agree(&self, _share: Vec<u8>) -> Result<Vec<u8>, MobileError> {
        Err(MobileError::Key {
            reason: "authentication required".into(),
        })
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_key_store_that_refuses_reports_why() {
    let w = world().await;
    let refusing = Arc::new(Refusing(w.phone.agreement.public_key()));
    let error = sync(|| w.phone.device.unlock(refusing)).unwrap_err();
    assert_eq!(
        error,
        MobileError::Key {
            reason: "authentication required".into()
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_pending_device_cannot_unlock() {
    let harness = Harness::start().await;
    let alice = laptop(&harness, "alice").await;
    let org = Id::random();
    let log = sync(|| alice.client.create_org(org, "Acme", Settings::default())).unwrap();
    let phone = phone(&harness);
    sign_in(&harness, &phone, "alice").await;
    sync(|| {
        phone
            .device
            .join(org.to_string(), log.genesis_hash().to_string())
    })
    .unwrap();
    // Joining again keeps the pending enrollment.
    let again = sync(|| {
        phone
            .device
            .join(org.to_string(), log.genesis_hash().to_string())
    })
    .unwrap();
    assert!(again.pending);
    assert!(!sync(|| phone.device.refresh()).unwrap().active);
    assert!(matches!(
        sync(|| phone.device.unlock(Arc::clone(&phone.agreement))),
        Err(MobileError::Invalid { .. })
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn another_organizations_genesis_is_refused() {
    let harness = Harness::start().await;
    let alice = laptop(&harness, "alice").await;
    let org = Id::random();
    sync(|| alice.client.create_org(org, "Acme", Settings::default())).unwrap();
    let phone = phone(&harness);
    sign_in(&harness, &phone, "alice").await;
    sync(|| phone.device.join(org.to_string(), b64(&[7; 32]))).unwrap();
    let error = sync(|| phone.device.refresh()).unwrap_err();
    assert!(matches!(error, MobileError::Verification { .. }), "{error}");
    // The device stays pinned to what it joined.
    assert!(matches!(
        sync(|| phone.device.join(Id::random().to_string(), b64(&[7; 32]))),
        Err(MobileError::Invalid { .. })
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_changed_checkpoint_key_is_refused() {
    let harness = Harness::start().await;
    let phone = phone(&harness);
    let error = sync(|| phone.device.connect(Some(b64(&[1; 32])))).unwrap_err();
    assert!(matches!(error, MobileError::Verification { .. }), "{error}");
    let info = sync(|| phone.device.connect(None)).unwrap();
    // Once pinned, a different managed key does not replace the pin.
    assert!(sync(|| phone.device.connect(Some(b64(&[1; 32])))).is_err());
    assert_eq!(sync(|| phone.device.connect(None)).unwrap(), info);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_redirect_of_another_sign_in_is_refused() {
    let harness = Harness::start().await;
    let phone = phone(&harness);
    let first = phone.device.start_sign_in(REDIRECT.into());
    harness.idp.set_user("alice", "alice@example.com", "Alice");
    let redirect = harness.follow_to_callback(&first).await.location().unwrap();
    let _second = phone.device.start_sign_in(REDIRECT.into());
    let error = sync(|| phone.device.finish_sign_in(redirect)).unwrap_err();
    assert!(matches!(error, MobileError::Verification { .. }), "{error}");
    assert!(!phone.device.signed_in());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_rolled_back_index_is_refused_after_unlocking() {
    let w = world().await;
    let phone = &w.phone.device;
    sync(|| phone.unlock(Arc::clone(&w.phone.agreement))).unwrap();
    let sql = w
        .harness
        .state
        .db
        .sql("UPDATE stores SET version = 1 WHERE id = $1");
    sqlx::query(sql)
        .bind(w.store.to_string())
        .execute(w.harness.state.db.pool())
        .await
        .unwrap();
    let error = sync(|| phone.reveal(w.store.to_string(), "example".into())).unwrap_err();
    assert!(matches!(error, MobileError::Verification { .. }), "{error}");
    // The pin survives in the state, so a restored device refuses it too.
    let restored = Device::restore(phone.state(), Arc::clone(&w.phone.signer), vec![]).unwrap();
    let error = sync(|| restored.unlock(Arc::clone(&w.phone.agreement))).unwrap_err();
    assert!(matches!(error, MobileError::Verification { .. }), "{error}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_replaced_lookup_object_is_refused() {
    let w = world().await;
    let phone = &w.phone.device;
    sync(|| phone.unlock(Arc::clone(&w.phone.agreement))).unwrap();
    let sql = w
        .harness
        .state
        .db
        .sql("UPDATE stores SET lookup = $1 WHERE id = $2");
    sqlx::query(sql)
        .bind(vec![0_u8; 64])
        .bind(w.store.to_string())
        .execute(w.harness.state.db.pool())
        .await
        .unwrap();
    let error = sync(|| phone.query(web("example.com"), None)).unwrap_err();
    assert!(matches!(error, MobileError::Verification { .. }), "{error}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_swapped_object_is_refused() {
    let w = world().await;
    let phone = &w.phone.device;
    let stores = sync(|| phone.unlock(Arc::clone(&w.phone.agreement))).unwrap();
    assert_eq!(stores.len(), 1);
    let ids: Vec<(String,)> = sqlx::query_as(
        w.harness
            .state
            .db
            .sql("SELECT id FROM objects WHERE store = $1 ORDER BY id"),
    )
    .bind(w.store.to_string())
    .fetch_all(w.harness.state.db.pool())
    .await
    .unwrap();
    // Every object holds every other's ciphertext; whichever one "example"
    // is, it now carries another's.
    let rows: Vec<(Vec<u8>,)> = sqlx::query_as(
        w.harness
            .state
            .db
            .sql("SELECT ciphertext FROM objects WHERE store = $1 ORDER BY id"),
    )
    .bind(w.store.to_string())
    .fetch_all(w.harness.state.db.pool())
    .await
    .unwrap();
    let sql = w
        .harness
        .state
        .db
        .sql("UPDATE objects SET ciphertext = $1 WHERE store = $2 AND id = $3");
    for (index, (id,)) in ids.iter().enumerate() {
        let (other,) = &rows[(index + 1) % rows.len()];
        sqlx::query(sql)
            .bind(other.clone())
            .bind(w.store.to_string())
            .bind(id.clone())
            .execute(w.harness.state.db.pool())
            .await
            .unwrap();
    }
    let error = sync(|| phone.reveal(w.store.to_string(), "example".into())).unwrap_err();
    assert!(matches!(error, MobileError::Verification { .. }), "{error}");
    let error =
        sync(|| phone.fill(web("example.com"), w.store.to_string(), "example".into())).unwrap_err();
    assert!(matches!(error, MobileError::Verification { .. }), "{error}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_revoked_device_loses_its_stores() {
    let mut w = world().await;
    let phone = &w.phone.device;
    sync(|| phone.unlock(Arc::clone(&w.phone.agreement))).unwrap();
    let id: Id = phone.device_id().parse().unwrap();
    sync(|| {
        w.alice
            .client
            .append_fresh(&mut w.log, Body::DeviceRevoke { device: id })
    })
    .unwrap();
    // The server ends the sessions of a revoked device.
    assert_eq!(
        sync(|| phone.refresh()).err(),
        Some(MobileError::NotSignedIn)
    );
    assert!(!phone.signed_in());
    assert!(!phone.unlocked());
}

#[tokio::test(flavor = "multi_thread")]
async fn organizations_that_allow_offline_reads_say_so() {
    let mut w = world().await;
    let mut settings = w.log.state().settings;
    settings.offline_reads = true;
    sync(|| {
        w.alice
            .client
            .append_fresh(&mut w.log, Body::OrgSettings { settings })
    })
    .unwrap();
    assert!(sync(|| w.phone.device.refresh()).unwrap().offline_reads);
}

fn set_offline_reads(w: &mut World, offline_reads: bool) {
    let mut settings = w.log.state().settings;
    settings.offline_reads = offline_reads;
    sync(|| {
        w.alice
            .client
            .append_fresh(&mut w.log, Body::OrgSettings { settings })
    })
    .unwrap();
}

/// A copy of `device` whose server URL points at a closed port, as a phone
/// without network sees its server.
fn without_network(device: &Device, signer: &Arc<dyn DeviceSigner>) -> Arc<Device> {
    with_server(device, signer, "http://127.0.0.1:9")
}

fn with_server(device: &Device, signer: &Arc<dyn DeviceSigner>, server: &str) -> Arc<Device> {
    let mut state: serde_json::Value = serde_json::from_str(&device.state()).unwrap();
    state["server"] = server.into();
    Device::restore(state.to_string(), Arc::clone(signer), vec![]).unwrap()
}

fn journal_len(device: &Device) -> usize {
    let state: serde_json::Value = serde_json::from_str(&device.state()).unwrap();
    state["journal"].as_object().map_or(0, |stores| {
        stores
            .values()
            .map(|reads| reads.as_array().unwrap().len())
            .sum()
    })
}

fn records(device: &Device) -> usize {
    let state: serde_json::Value = serde_json::from_str(&device.state()).unwrap();
    state["offline"].as_array().map_or(0, Vec::len)
}

#[tokio::test(flavor = "multi_thread")]
async fn values_read_while_offline_reads_are_allowed_open_without_network_and_are_reported() {
    let mut w = world().await;
    set_offline_reads(&mut w, true);
    let phone = &w.phone.device;
    let store = w.store.to_string();
    sync(|| phone.unlock(Arc::clone(&w.phone.agreement))).unwrap();
    sync(|| phone.reveal(store.clone(), "example".into())).unwrap();
    sync(|| phone.totp(store.clone(), "example".into())).unwrap();
    sync(|| phone.reveal(store.clone(), "login".into())).unwrap();
    assert_eq!(records(phone), 3);
    assert!(!phone.state().contains("hunter2"));

    let away = without_network(phone, &w.phone.signer);
    let stores = sync(|| away.unlock(Arc::clone(&w.phone.agreement))).unwrap();
    assert!(away.offline());
    assert_eq!(stores.len(), 1);
    assert_eq!(stores[0].label, "Work");
    assert_eq!(stores[0].secrets, ["example", "login"]);
    assert_eq!(stores[0].otp, ["example"]);
    assert_eq!(
        sync(|| away.reveal(store.clone(), "example".into())).unwrap(),
        "hunter2\nurl: https://example.com\nuser: alice"
    );
    assert_eq!(
        sync(|| away.reveal(store.clone(), "mail".into())).err(),
        Some(MobileError::NotFound)
    );
    let found = sync(|| away.query(web("example.com"), None)).unwrap();
    assert_eq!(found.matches[0].name, "example");
    let credential =
        sync(|| away.fill(web("example.com"), store.clone(), "example".into())).unwrap();
    assert_eq!(credential.password, "hunter2");
    assert_eq!(credential.totp.unwrap().len(), 6);
    assert_eq!(journal_len(&away), 3);
    assert_eq!(audited(&w, "object.read"), 3);
    assert_eq!(audited(&w, "object.read.offline"), 0);

    let back = with_server(&away, &w.phone.signer, &w.harness.url);
    sync(|| back.unlock(Arc::clone(&w.phone.agreement))).unwrap();
    assert!(!back.offline());
    assert_eq!(journal_len(&back), 0);
    assert_eq!(audited(&w, "object.read.offline"), 3);
}

#[tokio::test(flavor = "multi_thread")]
async fn nothing_is_kept_while_offline_reads_are_not_allowed() {
    let w = world().await;
    let phone = &w.phone.device;
    sync(|| phone.unlock(Arc::clone(&w.phone.agreement))).unwrap();
    sync(|| phone.reveal(w.store.to_string(), "example".into())).unwrap();
    assert_eq!(records(phone), 0);
    let away = without_network(phone, &w.phone.signer);
    assert!(matches!(
        sync(|| away.unlock(Arc::clone(&w.phone.agreement))),
        Err(MobileError::Network { .. })
    ));
    assert!(!away.unlocked());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_copy_is_deleted_once_the_log_disallows_offline_reads() {
    let mut w = world().await;
    set_offline_reads(&mut w, true);
    let phone = Arc::clone(&w.phone.device);
    sync(|| phone.unlock(Arc::clone(&w.phone.agreement))).unwrap();
    sync(|| phone.reveal(w.store.to_string(), "example".into())).unwrap();
    assert_eq!(records(&phone), 1);
    set_offline_reads(&mut w, false);
    sync(|| phone.refresh()).unwrap();
    assert_eq!(records(&phone), 0);
    let away = without_network(&phone, &w.phone.signer);
    assert!(sync(|| away.unlock(Arc::clone(&w.phone.agreement))).is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn another_identity_cannot_open_the_copy() {
    let mut w = world().await;
    set_offline_reads(&mut w, true);
    let phone = &w.phone.device;
    sync(|| phone.unlock(Arc::clone(&w.phone.agreement))).unwrap();
    sync(|| phone.reveal(w.store.to_string(), "example".into())).unwrap();
    let away = without_network(phone, &w.phone.signer);
    let stranger = software_agreement(generate_secret()).unwrap();
    assert!(sync(|| away.unlock(stranger)).is_err());
    assert!(!away.unlocked());
}

#[tokio::test(flavor = "multi_thread")]
async fn signing_out_reports_and_deletes_the_copy() {
    let mut w = world().await;
    set_offline_reads(&mut w, true);
    let phone = &w.phone.device;
    sync(|| phone.unlock(Arc::clone(&w.phone.agreement))).unwrap();
    sync(|| phone.reveal(w.store.to_string(), "example".into())).unwrap();
    let away = without_network(phone, &w.phone.signer);
    sync(|| away.unlock(Arc::clone(&w.phone.agreement))).unwrap();
    sync(|| away.reveal(w.store.to_string(), "example".into())).unwrap();
    let back = with_server(&away, &w.phone.signer, &w.harness.url);
    sync(|| back.sign_out()).unwrap();
    assert_eq!(records(&back), 0);
    assert_eq!(journal_len(&back), 0);
    assert_eq!(audited(&w, "object.read.offline"), 1);
}

struct Statements(Vec<&'static str>);

impl DomainVerifier for Statements {
    fn verified(&self, domain: String) -> bool {
        self.0.contains(&domain.as_str())
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn an_app_matches_hosts_whose_statement_file_names_it() {
    let w = world().await;
    let phone = &w.phone.device;
    sync(|| phone.unlock(Arc::clone(&w.phone.agreement))).unwrap();
    let request = app("com.example.webmail", &"00".repeat(32));

    let unverified = sync(|| phone.query(request.clone(), None)).unwrap();
    assert!(unverified.matches.is_empty());
    let refused =
        sync(|| phone.query(request.clone(), Some(Arc::new(Statements(vec![]))))).unwrap();
    assert!(refused.matches.is_empty());

    let verified = sync(|| {
        phone.query(
            request.clone(),
            Some(Arc::new(Statements(vec!["login.webmail.net"]))),
        )
    })
    .unwrap();
    assert_eq!(verified.matches.len(), 1);
    assert_eq!(verified.matches[0].name, "webmail");
    assert_eq!(verified.request.domains, ["login.webmail.net"]);
    let credential = sync(|| {
        phone.fill(
            verified.request.clone(),
            w.store.to_string(),
            "webmail".into(),
        )
    })
    .unwrap();
    assert_eq!(credential.password, "webpw");
    assert_eq!(credential.username, "carol");
    assert_eq!(
        sync(|| phone.fill(request, w.store.to_string(), "webmail".into())).err(),
        Some(MobileError::NotFound)
    );

    let other = sync(|| {
        phone.query(
            app("com.example.other", &"00".repeat(32)),
            Some(Arc::new(Statements(vec!["login.webmail.net"]))),
        )
    })
    .unwrap();
    assert!(other.matches.is_empty());
}
