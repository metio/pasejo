// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Integration-test scaffolding: a server on a loopback port backed by a
//! fresh database, a mock OpenID Connect provider, a workload token issuer
//! and clients that sign their requests the way the CLI does.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::missing_panics_doc
)]

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::Router;
use axum::extract::{Form, Query, State};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use ed25519_dalek::{Signer, SigningKey};
use pasejo_protocol::api::{
    Append, Appended, CreateOrg, DeviceAuthorization, DeviceAuthorizationRequest,
    EnrollmentRequest, LogPage, OrgSummary, TokenRequest, TokenResponse,
};
use pasejo_protocol::device::{DeviceKind, DeviceRecord};
use pasejo_protocol::encoding::Fixed;
use pasejo_protocol::encoding::{b64, sha256};
use pasejo_protocol::envelope::{Context, Envelope, TokenClaim};
use pasejo_protocol::log::{Body, OrgLog, RoleChange, Settings, genesis, next_entry, seal_entry};
use pasejo_protocol::request::{NONCE_HEADER, PROOF_HEADER, TIMESTAMP_HEADER, prove};
use pasejo_protocol::{Id, Millis};
use pasejo_server::clock::Clock;
use pasejo_server::config::Config;
use pasejo_server::db::Db;
use pasejo_server::state::{AppState, ServerKeys};
use rand_core::OsRng;
use reqwest::{Method, StatusCode};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use tempfile::TempDir;

pub const MASTER_KEY: [u8; 32] = [42; 32];
pub const WORKLOAD_ISSUER: &str = "https://workload.test";

fn system_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Signs a JWT with Ed25519.
pub fn sign_jwt(key: &SigningKey, kid: &str, claims: &Value) -> String {
    let header = b64(json!({"alg": "EdDSA", "typ": "JWT", "kid": kid})
        .to_string()
        .as_bytes());
    let payload = b64(claims.to_string().as_bytes());
    let input = format!("{header}.{payload}");
    let signature = key.sign(input.as_bytes());
    format!("{input}.{}", b64(&signature.to_bytes()))
}

/// The JWK of an Ed25519 key.
pub fn jwk(key: &SigningKey, kid: &str) -> Value {
    json!({
        "kty": "OKP",
        "crv": "Ed25519",
        "x": b64(key.verifying_key().as_bytes()),
        "kid": kid,
        "alg": "EdDSA",
        "use": "sig",
    })
}

/// Who the mock provider signs in next.
#[derive(Debug, Clone)]
pub struct IdpUser {
    pub subject: String,
    pub email: String,
    pub name: String,
    pub verified: bool,
}

struct Code {
    nonce: String,
    challenge: String,
    redirect: String,
    user: IdpUser,
}

/// A mock OpenID Connect provider that signs in whoever `set_user` names.
pub struct Idp {
    pub issuer: String,
    key: SigningKey,
    user: Mutex<IdpUser>,
    codes: Mutex<HashMap<String, Code>>,
    /// Signs ID tokens with a key the provider does not publish.
    pub forge: AtomicBool,
    /// Echoes a wrong nonce.
    pub wrong_nonce: AtomicBool,
    /// Answers the authorization request with an error.
    pub deny: AtomicBool,
}

impl Idp {
    pub fn set_user(&self, subject: &str, email: &str, name: &str) {
        *self.user.lock().unwrap() = IdpUser {
            subject: subject.into(),
            email: email.into(),
            name: name.into(),
            verified: true,
        };
    }

    pub fn set_unverified(&self) {
        self.user.lock().unwrap().verified = false;
    }
}

async fn discovery(State(idp): State<Arc<Idp>>) -> axum::Json<Value> {
    axum::Json(json!({
        "issuer": idp.issuer,
        "authorization_endpoint": format!("{}/authorize", idp.issuer),
        "token_endpoint": format!("{}/token", idp.issuer),
        "jwks_uri": format!("{}/jwks", idp.issuer),
    }))
}

async fn jwks(State(idp): State<Arc<Idp>>) -> axum::Json<Value> {
    axum::Json(json!({"keys": [jwk(&idp.key, "idp-1")]}))
}

async fn authorize(
    State(idp): State<Arc<Idp>>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    assert_eq!(query["response_type"], "code");
    assert_eq!(query["client_id"], "pasejo");
    assert_eq!(query["code_challenge_method"], "S256");
    assert!(query["scope"].split(' ').any(|scope| scope == "openid"));
    let redirect = query["redirect_uri"].clone();
    let mut url = url::Url::parse(&redirect).unwrap();
    if idp.deny.load(Ordering::Relaxed) {
        url.query_pairs_mut()
            .append_pair("error", "access_denied")
            .append_pair("state", &query["state"]);
        return Redirect::to(url.as_str()).into_response();
    }
    let code = b64(&pasejo_protocol::encoding::random::<16>());
    idp.codes.lock().unwrap().insert(
        code.clone(),
        Code {
            nonce: query["nonce"].clone(),
            challenge: query["code_challenge"].clone(),
            redirect,
            user: idp.user.lock().unwrap().clone(),
        },
    );
    url.query_pairs_mut()
        .append_pair("code", &code)
        .append_pair("state", &query["state"]);
    Redirect::to(url.as_str()).into_response()
}

async fn token(State(idp): State<Arc<Idp>>, Form(form): Form<HashMap<String, String>>) -> Response {
    let Some(code) = idp.codes.lock().unwrap().remove(&form["code"]) else {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(json!({"error": "invalid_grant"})),
        )
            .into_response();
    };
    if b64(&sha256(form["code_verifier"].as_bytes())) != code.challenge
        || form["redirect_uri"] != code.redirect
    {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(json!({"error": "invalid_grant"})),
        )
            .into_response();
    }
    let now = system_seconds();
    let nonce = if idp.wrong_nonce.load(Ordering::Relaxed) {
        "other".to_owned()
    } else {
        code.nonce
    };
    let claims = json!({
        "iss": idp.issuer,
        "sub": code.user.subject,
        "aud": "pasejo",
        "iat": now,
        "exp": now + 86_400,
        "nonce": nonce,
        "email": code.user.email,
        "email_verified": code.user.verified,
        "name": code.user.name,
    });
    let key = if idp.forge.load(Ordering::Relaxed) {
        SigningKey::generate(&mut OsRng)
    } else {
        idp.key.clone()
    };
    axum::Json(json!({"access_token": "x", "token_type": "Bearer", "id_token": sign_jwt(&key, "idp-1", &claims)})).into_response()
}

async fn start_idp() -> Arc<Idp> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let idp = Arc::new(Idp {
        issuer: format!("http://{address}"),
        key: SigningKey::generate(&mut OsRng),
        user: Mutex::new(IdpUser {
            subject: "nobody".into(),
            email: "nobody@example.com".into(),
            name: "Nobody".into(),
            verified: true,
        }),
        codes: Mutex::new(HashMap::new()),
        forge: AtomicBool::new(false),
        wrong_nonce: AtomicBool::new(false),
        deny: AtomicBool::new(false),
    });
    let router = Router::new()
        .route("/.well-known/openid-configuration", get(discovery))
        .route("/jwks", get(jwks))
        .route("/authorize", get(authorize))
        .route("/token", post(token))
        .with_state(Arc::clone(&idp));
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    idp
}

/// A test server's settings.
pub struct Options {
    pub oidc: bool,
    pub break_glass: bool,
    /// TOML appended to the generated configuration.
    pub extra: String,
    /// Top-level TOML keys, placed before any table.
    pub top: String,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            oidc: true,
            break_glass: true,
            extra: String::new(),
            top: String::new(),
        }
    }
}

/// A database for one test: a SQLite file, or a fresh PostgreSQL database
/// when `PASEJO_TEST_POSTGRES` names a server to create it on.
async fn database(dir: &TempDir) -> String {
    match std::env::var("PASEJO_TEST_POSTGRES") {
        Ok(admin) if !admin.is_empty() => {
            sqlx::any::install_default_drivers();
            let name = format!("pasejo_test_{}", Id::random());
            let pool = sqlx::AnyPool::connect(&admin).await.unwrap();
            sqlx::query(&format!("CREATE DATABASE {name}"))
                .execute(&pool)
                .await
                .unwrap();
            let mut url = url::Url::parse(&admin).unwrap();
            url.set_path(&format!("/{name}"));
            url.to_string()
        }
        _ => format!("sqlite://{}/server.db?mode=rwc", dir.path().display()),
    }
}

/// A running server.
pub struct Harness {
    pub state: AppState,
    pub url: String,
    pub http: reqwest::Client,
    pub idp: Arc<Idp>,
    pub workload_key: SigningKey,
    pub dir: TempDir,
}

impl Harness {
    pub async fn start() -> Self {
        Self::start_with(Options::default()).await
    }

    pub async fn start_with(options: Options) -> Self {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "warn".into()),
            )
            .with_test_writer()
            .try_init();
        let dir = tempfile::tempdir().unwrap();
        let idp = start_idp().await;
        let workload_key = SigningKey::generate(&mut OsRng);
        std::fs::write(
            dir.path().join("workload.json"),
            json!({"keys": [jwk(&workload_key, "w-1")]}).to_string(),
        )
        .unwrap();
        std::fs::write(dir.path().join("master.key"), b64(&MASTER_KEY)).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let database_url = database(&dir).await;
        let mut text = format!(
            "listen = \"{address}\"\npublic_url = \"http://{address}\"\ndatabase_url = \"{database_url}\"\nmaster_key_file = \"{}\"\n",
            dir.path().join("master.key").display()
        );
        text.push_str(&options.top);
        if options.oidc {
            text.push_str(&format!(
                "[oidc]\nissuer = \"{}\"\nclient_id = \"pasejo\"\n",
                idp.issuer
            ));
        }
        text.push_str(&format!(
            "[[workload]]\nissuer = \"{WORKLOAD_ISSUER}\"\njwks_file = \"{}\"\n[[workload.rules]]\nsubject = \"system:serviceaccount:ci:*\"\nmachine = \"ci\"\n[[workload.rules]]\nsubject = \"repo:metio/pasejo:ref:refs/heads/main\"\nmachine = \"release\"\n",
            dir.path().join("workload.json").display()
        ));
        if !options.extra.contains("[break_glass]") {
            text.push_str(&format!(
                "[break_glass]\nenabled = {}\n",
                options.break_glass
            ));
        }
        text.push_str(&options.extra);
        let config = Config::parse(&text).unwrap();
        let db = Db::connect(&config.database_url).await.unwrap();
        db.migrate().await.unwrap();
        let state = AppState::new(
            config,
            db,
            ServerKeys::derive(&MASTER_KEY),
            Arc::new(Clock::default()),
        )
        .unwrap();
        let router = pasejo_server::app::router(state.clone())
            .into_make_service_with_connect_info::<SocketAddr>();
        tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        Self {
            state,
            url: format!("http://{address}"),
            http: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap(),
            idp,
            workload_key,
            dir,
        }
    }

    pub fn now(&self) -> Millis {
        self.state.now()
    }

    pub fn advance(&self, millis: i64) {
        self.state.clock.advance(millis);
    }

    /// An unauthenticated request.
    pub async fn raw(&self, method: Method, path: &str, body: Option<Value>) -> Reply {
        let mut request = self.http.request(method, format!("{}{path}", self.url));
        if let Some(body) = body {
            request = request
                .header("content-type", "application/json")
                .body(body.to_string());
        }
        Reply::from(request.send().await.unwrap()).await
    }

    /// A signed `token/v1` claim.
    pub fn claim(&self, key: &SigningKey, purpose: &str, value: &str) -> Envelope {
        Envelope::seal(
            Context::Token,
            &TokenClaim {
                purpose: purpose.into(),
                value: value.into(),
                signing_key: Fixed(key.verifying_key().to_bytes()),
                time: self.now(),
            },
            Id::random(),
            key,
        )
        .unwrap()
    }

    /// Starts a device grant and follows the browser side up to the
    /// approval page; returns the device code and the approval token.
    pub async fn device_grant(
        &self,
        key: &SigningKey,
        subject: &str,
    ) -> (DeviceAuthorization, String) {
        let authorization: DeviceAuthorization = self
            .raw(
                Method::POST,
                "/v1/auth/device",
                Some(
                    serde_json::to_value(DeviceAuthorizationRequest {
                        signing_key: Fixed(key.verifying_key().to_bytes()),
                        label: "laptop".into(),
                    })
                    .unwrap(),
                ),
            )
            .await
            .ok();
        self.idp.set_user(
            subject,
            &format!("{subject}@example.com"),
            &capitalize(subject),
        );
        let page = self
            .follow_to_callback(&authorization.verification_uri_complete)
            .await;
        assert_eq!(page.status, StatusCode::OK, "{}", page.text());
        let approval =
            between(&page.text(), "name=\"approval\" value=\"", "\"").expect("approval token");
        (authorization, approval)
    }

    /// Follows redirects from `url` through the provider to the callback
    /// and returns the callback's reply.
    pub async fn follow_to_callback(&self, url: &str) -> Reply {
        let first = Reply::from(self.http.get(url).send().await.unwrap()).await;
        let Some(provider) = first.location() else {
            return first;
        };
        let second = Reply::from(self.http.get(&provider).send().await.unwrap()).await;
        let callback = second.location().expect("provider redirect");
        Reply::from(self.http.get(&callback).send().await.unwrap()).await
    }

    pub async fn approve(&self, approval: &str, decision: &str) -> Reply {
        Reply::from(
            self.http
                .post(format!("{}/auth/verify/approve", self.url))
                .form(&[("approval", approval), ("decision", decision)])
                .send()
                .await
                .unwrap(),
        )
        .await
    }

    pub async fn poll(&self, key: &SigningKey, device_code: &str) -> Reply {
        let claim = self.claim(key, "device", device_code);
        self.raw(
            Method::POST,
            "/v1/auth/token",
            Some(serde_json::to_value(TokenRequest { claim, jwt: None }).unwrap()),
        )
        .await
    }

    /// Signs `subject` in through the whole device grant.
    pub async fn sign_in(&self, subject: &str) -> Client {
        let key = SigningKey::generate(&mut OsRng);
        let (authorization, approval) = self.device_grant(&key, subject).await;
        assert_eq!(
            self.approve(&approval, "approve").await.status,
            StatusCode::OK
        );
        let response: TokenResponse = self.poll(&key, &authorization.device_code).await.ok();
        self.client(key, response)
    }

    pub fn client(&self, key: SigningKey, response: TokenResponse) -> Client {
        Client {
            url: self.url.clone(),
            http: self.http.clone(),
            clock: Arc::clone(&self.state.clock),
            token: response.token,
            key,
            user: response.user,
        }
    }

    /// A workload token from the configured issuer.
    pub fn workload_jwt(&self, subject: &str, audience: &str, lifetime: i64) -> String {
        let now = system_seconds();
        sign_jwt(
            &self.workload_key,
            "w-1",
            &json!({
                "iss": WORKLOAD_ISSUER,
                "sub": subject,
                "aud": audience,
                "iat": now,
                "exp": now.saturating_add_signed(lifetime),
            }),
        )
    }

    pub async fn workload(&self, key: &SigningKey, jwt: &str) -> Reply {
        let claim = self.claim(key, "workload", &b64(&sha256(jwt.as_bytes())));
        self.raw(
            Method::POST,
            "/v1/auth/workload",
            Some(
                serde_json::to_value(TokenRequest {
                    claim,
                    jwt: Some(jwt.into()),
                })
                .unwrap(),
            ),
        )
        .await
    }

    /// A member with an age device, signed in through the device grant.
    pub async fn member(&self, subject: &str) -> Member {
        let client = self.sign_in(subject).await;
        Member::new(client)
    }

    /// Creates an organization founded by `founder`.
    pub async fn org(&self, founder: &Member, settings: Settings) -> Id {
        let org = Id::random();
        let envelope = genesis(
            org,
            "Acme",
            founder.record.clone(),
            settings,
            self.now(),
            &founder.client.key,
        )
        .unwrap();
        let reply = founder
            .client
            .post("/v1/orgs", &CreateOrg { genesis: envelope })
            .await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
        let summary: OrgSummary = reply.json();
        assert_eq!(summary.id, org);
        org
    }

    /// Enrolls `member` and has `admin` add them with `role`.
    pub async fn join(&self, admin: &Member, member: &Member, org: Id, role: RoleChange) {
        member.enroll(org).await.ok::<Value>();
        admin
            .append(
                org,
                Body::MemberRole {
                    user: member.user(),
                    role,
                },
            )
            .await
            .ok::<Appended>();
        admin
            .append(
                org,
                Body::DeviceAdd {
                    device: member.record.clone(),
                },
            )
            .await
            .ok::<Appended>();
    }
}

pub fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

pub fn between(text: &str, start: &str, end: &str) -> Option<String> {
    let from = text.find(start)? + start.len();
    let to = text[from..].find(end)? + from;
    Some(text[from..to].to_owned())
}

/// A received response.
#[derive(Debug)]
pub struct Reply {
    pub status: StatusCode,
    pub headers: reqwest::header::HeaderMap,
    pub body: Vec<u8>,
}

impl Reply {
    pub async fn from(response: reqwest::Response) -> Self {
        Self {
            status: response.status(),
            headers: response.headers().clone(),
            body: response.bytes().await.unwrap().to_vec(),
        }
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    pub fn json<T: DeserializeOwned>(&self) -> T {
        serde_json::from_slice(&self.body)
            .unwrap_or_else(|error| panic!("{error}: {} {}", self.status, self.text()))
    }

    /// The body, after asserting a success status.
    #[track_caller]
    pub fn ok<T: DeserializeOwned>(&self) -> T {
        assert!(
            self.status.is_success(),
            "expected success, got {} {}",
            self.status,
            self.text()
        );
        self.json()
    }

    /// The error code of an error body.
    pub fn code(&self) -> String {
        serde_json::from_slice::<Value>(&self.body)
            .ok()
            .and_then(|body| {
                body.get("code")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            })
            .unwrap_or_default()
    }

    #[track_caller]
    pub fn expect(&self, status: StatusCode, code: &str) {
        assert_eq!(
            (self.status, self.code().as_str()),
            (status, code),
            "{}",
            self.text()
        );
    }

    pub fn location(&self) -> Option<String> {
        self.headers
            .get("location")
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned)
    }

    pub fn header(&self, name: &str) -> Option<String> {
        self.headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned)
    }
}

/// A client holding a session and the key it is bound to.
#[derive(Clone)]
pub struct Client {
    pub url: String,
    pub http: reqwest::Client,
    pub clock: Arc<Clock>,
    pub token: String,
    pub key: SigningKey,
    pub user: Option<Id>,
}

impl Client {
    /// Sends a request with a valid proof.
    pub async fn send(&self, method: Method, path: &str, body: Vec<u8>) -> Reply {
        let proof = prove(
            method.as_str(),
            path,
            self.clock.now(),
            &body,
            &self.token,
            &self.key,
        );
        self.send_with(
            method,
            path,
            body,
            &[
                ("authorization", proof.authorization),
                (TIMESTAMP_HEADER, proof.timestamp),
                (NONCE_HEADER, proof.nonce),
                (PROOF_HEADER, proof.proof),
            ],
        )
        .await
    }

    /// Sends a request with exactly the given headers.
    pub async fn send_with(
        &self,
        method: Method,
        path: &str,
        body: Vec<u8>,
        headers: &[(&str, String)],
    ) -> Reply {
        let mut request = self
            .http
            .request(method, format!("{}{path}", self.url))
            .body(body);
        for (name, value) in headers {
            request = request.header(*name, value);
        }
        Reply::from(request.send().await.unwrap()).await
    }

    pub async fn get(&self, path: &str) -> Reply {
        self.send(Method::GET, path, Vec::new()).await
    }

    pub async fn post<T: Serialize>(&self, path: &str, body: &T) -> Reply {
        self.send(Method::POST, path, serde_json::to_vec(body).unwrap())
            .await
    }

    pub async fn put<T: Serialize>(&self, path: &str, body: &T) -> Reply {
        self.send(Method::PUT, path, serde_json::to_vec(body).unwrap())
            .await
    }

    pub async fn delete(&self, path: &str) -> Reply {
        self.send(Method::DELETE, path, Vec::new()).await
    }

    pub async fn post_empty(&self, path: &str) -> Reply {
        self.send(Method::POST, path, Vec::new()).await
    }
}

/// An organization member: a session plus the device it is bound to.
pub struct Member {
    pub client: Client,
    pub record: DeviceRecord,
    pub identity: age::x25519::Identity,
}

impl Member {
    pub fn new(client: Client) -> Self {
        let identity = age::x25519::Identity::generate();
        let record = DeviceRecord {
            id: Id::random(),
            user: client.user.expect("a person session"),
            kind: DeviceKind::Age,
            recipient: identity.to_public().to_string(),
            signing_key: Fixed(client.key.verifying_key().to_bytes()),
            label: "laptop".into(),
        };
        Self {
            client,
            record,
            identity,
        }
    }

    pub fn user(&self) -> Id {
        self.record.user
    }

    pub fn identities(&self) -> Vec<Box<dyn age::Identity>> {
        vec![Box::new(self.identity.clone())]
    }

    pub async fn log(&self, org: Id) -> OrgLog {
        let page: LogPage = self.client.get(&format!("/v1/orgs/{org}/log")).await.ok();
        OrgLog::replay(page.entries).unwrap()
    }

    pub async fn sealed(&self, org: Id, body: Body) -> Envelope {
        let log = self.log(org).await;
        let entry = next_entry(&log, self.record.id, self.client.clock.now(), body);
        seal_entry(&entry, &self.client.key).unwrap()
    }

    pub async fn append(&self, org: Id, body: Body) -> Reply {
        let entry = self.sealed(org, body).await;
        self.client
            .post(&format!("/v1/orgs/{org}/log"), &Append { entry })
            .await
    }

    pub async fn enroll(&self, org: Id) -> Reply {
        self.client
            .post(
                &format!("/v1/orgs/{org}/enrollments"),
                &EnrollmentRequest {
                    id: self.record.id,
                    kind: self.record.kind,
                    recipient: self.record.recipient.clone(),
                    label: self.record.label.clone(),
                },
            )
            .await
    }
}

/// Decodes RFC 4648 base32 without padding.
pub fn base32_decode(text: &str) -> Vec<u8> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut bits = 0u32;
    let mut count = 0;
    let mut out = Vec::new();
    for byte in text.bytes().filter(|byte| *byte != b'=') {
        let value = ALPHABET.iter().position(|letter| *letter == byte).unwrap() as u32;
        bits = (bits << 5) | value;
        count += 5;
        if count >= 8 {
            count -= 8;
            out.push((bits >> count) as u8);
            bits &= (1 << count) - 1;
        }
    }
    out
}

/// The TOTP code for `secret` (base32) at the server's current time,
/// shifted by `steps`.
pub fn totp_now(harness: &Harness, secret: &str, steps: i64) -> String {
    let step = (harness.now() / 30_000).saturating_add_signed(steps);
    pasejo_server::totp::code(&base32_decode(secret), step)
}

pub mod stores {
    //! Store fixtures: epochs, wrappings and commits built with the protocol
    //! crate, as a client builds them.

    use pasejo_protocol::Id;
    use pasejo_protocol::api::{ChainLink, Commit, EpochUpload, WrappingUpload};
    use pasejo_protocol::crypto::{EpochKeys, data_commitment, names_commitment, seal_chain};
    use pasejo_protocol::encoding::{Bytes, Hash};
    use pasejo_protocol::log::Body;
    use pasejo_protocol::store::{
        IndexEntry, IndexPayload, IndexPlain, LookupPlain, ObjectPlain, SealedObject, seal_index,
        seal_lookup, seal_object, sign_index,
    };
    use pasejo_protocol::wrapping::{wrap_age, wrap_hpke};

    use super::{Member, Reply};

    /// One secret version a commit writes.
    pub struct Write<'a> {
        pub name: &'a str,
        pub id: Id,
        pub version: u64,
        pub value: &'a str,
    }

    /// A device a wrapping goes to.
    pub enum Target<'a> {
        Age(&'a Member),
        Hpke(Id, [u8; 32]),
    }

    impl Member {
        pub async fn create_store(&self, org: Id) -> Id {
            let store = Id::random();
            self.append(org, Body::StoreCreate { store })
                .await
                .ok::<serde_json::Value>();
            store
        }

        /// Appends a `store.epoch` entry for the devices entitled now and
        /// uploads wrappings to `targets`.
        pub async fn start_epoch(
            &self,
            org: Id,
            store: Id,
            epoch: u64,
            keys: &EpochKeys,
            previous: Option<&EpochKeys>,
            targets: &[Target<'_>],
        ) -> Reply {
            let log = self.log(org).await;
            let entitled = log.state().entitled(store, self.client.clock.now());
            let body = Body::StoreEpoch {
                store,
                epoch,
                names_commit: names_commitment(store, epoch, &keys.names),
                data_commit: data_commitment(store, epoch, keys.data().unwrap()),
                full: entitled.full.into_iter().collect(),
                names_only: entitled.names_only.into_iter().collect(),
            };
            self.append(org, body).await.ok::<serde_json::Value>();
            self.upload_epoch(org, store, epoch, keys, previous, targets)
                .await
        }

        pub async fn upload_epoch(
            &self,
            org: Id,
            store: Id,
            epoch: u64,
            keys: &EpochKeys,
            previous: Option<&EpochKeys>,
            targets: &[Target<'_>],
        ) -> Reply {
            let chain = previous.map(|previous| {
                let chain = seal_chain(store, epoch, keys, previous).unwrap();
                ChainLink {
                    epoch,
                    prev_names: Bytes(chain.prev_names),
                    prev_data: Bytes(chain.prev_data),
                }
            });
            let upload = EpochUpload {
                chain,
                wrappings: wrappings(store, epoch, keys, targets),
            };
            self.client
                .put(
                    &format!("/v1/orgs/{org}/stores/{store}/epochs/{epoch}"),
                    &upload,
                )
                .await
        }
    }

    pub fn wrappings(
        store: Id,
        epoch: u64,
        keys: &EpochKeys,
        targets: &[Target<'_>],
    ) -> Vec<WrappingUpload> {
        targets
            .iter()
            .map(|target| match target {
                Target::Age(member) => {
                    let recipient: age::x25519::Recipient =
                        member.record.recipient.parse().unwrap();
                    WrappingUpload {
                        device: member.record.id,
                        full: true,
                        ciphertext: Bytes(wrap_age(store, epoch, keys, &recipient).unwrap()),
                    }
                }
                Target::Hpke(device, public) => WrappingUpload {
                    device: *device,
                    full: false,
                    ciphertext: Bytes(
                        wrap_hpke(store, epoch, *device, public, &keys.names_only()).unwrap(),
                    ),
                },
            })
            .collect()
    }

    /// Everything a commit needs, kept so tests can alter one part.
    pub struct Draft {
        pub payload: IndexPayload,
        pub plain: IndexPlain,
        pub lookup: Option<Vec<u8>>,
        pub objects: Vec<SealedObject>,
    }

    impl Draft {
        /// The commit, signed by `author`.
        pub fn sign(&self, author: &Member) -> Commit {
            Commit {
                index: sign_index(&self.payload, author.record.id, &author.client.key).unwrap(),
                lookup: self.lookup.clone().map(Bytes),
                objects: self.objects.clone(),
            }
        }
    }

    /// Builds index version `version` on top of `base`, writing `writes`
    /// and removing `deleted`.
    #[allow(clippy::too_many_arguments)]
    pub async fn draft(
        author: &Member,
        org: Id,
        store: Id,
        epoch: u64,
        keys: &EpochKeys,
        version: u64,
        prev: Option<Hash>,
        base: &IndexPlain,
        writes: &[Write<'_>],
        deleted: &[&str],
    ) -> Draft {
        let log = author.log(org).await;
        let mut plain = base.clone();
        let mut objects = Vec::new();
        let mut lookup = LookupPlain::default();
        for write in writes {
            let ciphertext = seal_object(
                store,
                epoch,
                write.id,
                write.version,
                keys.data().unwrap(),
                &ObjectPlain::Secret {
                    secret: write.value.into(),
                },
            )
            .unwrap();
            plain.secrets.insert(
                write.name.into(),
                IndexEntry {
                    id: write.id,
                    version: write.version,
                    hash: Hash::of(&ciphertext),
                },
            );
            objects.push(SealedObject {
                id: write.id,
                version: write.version,
                epoch,
                ciphertext: Bytes(ciphertext),
            });
            lookup.secrets.insert(write.name.into(), String::new());
        }
        let mut removed = Vec::new();
        for name in deleted {
            if let Some(entry) = plain.secrets.remove(*name) {
                removed.push(entry.id);
            }
        }
        let sealed_lookup =
            seal_lookup(store, epoch, version, keys.data().unwrap(), &lookup).unwrap();
        let payload = IndexPayload {
            store,
            version,
            prev,
            epoch,
            log_head: log.head(),
            time: author.client.clock.now(),
            index: Bytes(seal_index(store, epoch, version, &keys.names, &plain).unwrap()),
            lookup_hash: Some(Hash::of(&sealed_lookup)),
            changed: writes.iter().map(|write| write.id).collect(),
            deleted: removed,
        };
        Draft {
            payload,
            plain,
            lookup: Some(sealed_lookup),
            objects,
        }
    }

    pub async fn commit(author: &Member, org: Id, store: Id, commit: &Commit) -> Reply {
        author
            .client
            .post(&format!("/v1/orgs/{org}/stores/{store}/commits"), commit)
            .await
    }
}
