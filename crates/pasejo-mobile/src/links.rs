// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Digital Asset Links: whether a domain declares that an app, identified
//! by its package name and signing certificate, may receive its sign-in
//! credentials.

use std::io::Read;
use std::time::Duration;

use serde::Deserialize;

use crate::error::MobileError;

const RELATIONS: [&str; 2] = [
    "delegate_permission/common.get_login_creds",
    "delegate_permission/common.handle_all_urls",
];
/// Included statement files followed beyond the first one.
const MAX_INCLUDES: usize = 10;
const MAX_BYTES: u64 = 256 * 1024;
const TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Deserialize)]
struct Statement {
    #[serde(default)]
    relation: Vec<String>,
    target: Option<Target>,
    include: Option<String>,
}

#[derive(Deserialize)]
struct Target {
    namespace: String,
    package_name: Option<String>,
    #[serde(default)]
    sha256_cert_fingerprints: Vec<String>,
}

/// A certificate fingerprint in one spelling: upper-case hex without
/// separators.
pub fn normalize_fingerprint(fingerprint: &str) -> String {
    fingerprint
        .chars()
        .filter(char::is_ascii_hexdigit)
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .http_status_as_error(false)
        .max_redirects(0)
        .build()
        .into()
}

fn fetch(agent: &ureq::Agent, url: &str) -> Result<Vec<Statement>, MobileError> {
    let network = |reason: String| MobileError::Network { reason };
    let response = agent
        .get(url)
        .call()
        .map_err(|error| network(error.to_string()))?;
    if response.status() != 200 {
        return Err(MobileError::NotFound);
    }
    let json = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(';')
                .next()
                .is_some_and(|kind| kind.trim().eq_ignore_ascii_case("application/json"))
        });
    if !json {
        return Err(MobileError::invalid("the statement file is not JSON"));
    }
    let mut body = Vec::new();
    response
        .into_body()
        .into_reader()
        .take(MAX_BYTES + 1)
        .read_to_end(&mut body)
        .map_err(|error| network(error.to_string()))?;
    if body.len() as u64 > MAX_BYTES {
        return Err(MobileError::invalid("the statement file is too large"));
    }
    serde_json::from_slice(&body).map_err(|_| MobileError::invalid("malformed statement file"))
}

/// Whether the statement file at `url` lets an app receive credentials.
///
/// The file, with its includes, must let the app `package` signed by one of
/// `certificates` (SHA-256 fingerprints) receive sign-in credentials. Includes
/// must use the scheme of `url` and at most ten files are read.
///
/// # Errors
///
/// Fails when the file cannot be fetched or parsed; a domain whose file
/// does not name the app is `Ok(false)`.
// UniFFI passes arguments from the foreign side as owned values.
#[allow(clippy::needless_pass_by_value)]
#[uniffi::export]
pub fn verify_statements_at(
    url: String,
    package: String,
    certificates: Vec<String>,
) -> Result<bool, MobileError> {
    let wanted: Vec<String> = certificates
        .iter()
        .map(|certificate| normalize_fingerprint(certificate))
        .filter(|certificate| certificate.len() == 64)
        .collect();
    if wanted.is_empty() {
        return Ok(false);
    }
    let scheme = url
        .split_once("://")
        .map(|(scheme, _)| format!("{scheme}://"))
        .ok_or_else(|| MobileError::invalid("the statement URL has no scheme"))?;
    let agent = agent();
    let mut pending = vec![url];
    let mut fetched = 0;
    while let Some(next) = pending.pop() {
        if fetched > MAX_INCLUDES {
            return Err(MobileError::invalid("too many included statement files"));
        }
        fetched += 1;
        for statement in fetch(&agent, &next)? {
            if let Some(include) = statement.include {
                if !include.starts_with(&scheme) {
                    return Err(MobileError::invalid(
                        "an included statement file uses another scheme",
                    ));
                }
                pending.push(include);
                continue;
            }
            let Some(target) = statement.target else {
                continue;
            };
            let related = statement
                .relation
                .iter()
                .any(|relation| RELATIONS.contains(&relation.as_str()));
            let names_app = target.namespace == "android_app"
                && target.package_name.as_deref() == Some(package.as_str());
            let signed = target
                .sha256_cert_fingerprints
                .iter()
                .any(|fingerprint| wanted.contains(&normalize_fingerprint(fingerprint)));
            if related && names_app && signed {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

/// Whether `https://<domain>/.well-known/assetlinks.json` lets the app
/// receive the domain's credentials; see [`verify_statements_at`]. The
/// statement names this exact host, so subdomains are not covered.
///
/// # Errors
///
/// Fails for a domain that is not a plain host name, and like
/// [`verify_statements_at`].
// UniFFI passes arguments from the foreign side as owned values.
#[allow(clippy::needless_pass_by_value)]
#[uniffi::export]
pub fn verify_app_domain(
    domain: String,
    package: String,
    certificates: Vec<String>,
) -> Result<bool, MobileError> {
    let plain = !domain.is_empty()
        && domain
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
    if !plain {
        return Err(MobileError::invalid("not a host name"));
    }
    verify_statements_at(
        format!(
            "https://{}/.well-known/assetlinks.json",
            domain.to_ascii_lowercase()
        ),
        package,
        certificates,
    )
}

#[cfg(test)]
pub mod tests {
    #![allow(clippy::unwrap_used)]

    use std::collections::BTreeMap;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    use super::*;

    pub const CERT: &str = "14:6D:E9:83:C5:73:06:50:D8:EE:B9:95:2F:34:FC:64:16:A0:83:42:E6:1D:BE:A8:8A:04:96:B2:3F:CF:44:E5";

    /// Status, content type and body by path.
    type Routes = Arc<Mutex<BTreeMap<String, (u16, String, String)>>>;

    /// A loopback HTTP server answering each path with a status, a content
    /// type and a body.
    pub struct Files {
        pub base: String,
        pub routes: Routes,
    }

    impl Files {
        pub fn start() -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let base = format!("http://{}", listener.local_addr().unwrap());
            let routes: Routes = Arc::default();
            let served = Arc::clone(&routes);
            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    let Ok(mut stream) = stream else { continue };
                    let mut line = String::new();
                    let mut reader = BufReader::new(stream.try_clone().unwrap());
                    if reader.read_line(&mut line).is_err() {
                        continue;
                    }
                    loop {
                        let mut header = String::new();
                        if reader.read_line(&mut header).unwrap_or(0) == 0 || header == "\r\n" {
                            break;
                        }
                    }
                    let path = line.split(' ').nth(1).unwrap_or("/").to_owned();
                    let (status, kind, body) = served
                        .lock()
                        .unwrap()
                        .get(&path)
                        .cloned()
                        .unwrap_or_else(|| (404, "text/plain".into(), String::new()));
                    let _ = write!(
                        stream,
                        "HTTP/1.1 {status} X\r\ncontent-type: {kind}\r\ncontent-length: {}\r\nlocation: /elsewhere\r\nconnection: close\r\n\r\n{body}",
                        body.len()
                    );
                }
            });
            Self { base, routes }
        }

        pub fn serve(&self, path: &str, status: u16, kind: &str, body: &str) {
            self.routes
                .lock()
                .unwrap()
                .insert(path.into(), (status, kind.into(), body.into()));
        }

        pub fn url(&self, path: &str) -> String {
            format!("{}{path}", self.base)
        }
    }

    fn statement(relation: &str, package: &str, cert: &str) -> String {
        format!(
            r#"[{{"relation":["{relation}"],"target":{{"namespace":"android_app","package_name":"{package}","sha256_cert_fingerprints":["{cert}"]}}}}]"#
        )
    }

    fn verify(files: &Files, path: &str, package: &str, cert: &str) -> Result<bool, MobileError> {
        verify_statements_at(files.url(path), package.into(), vec![cert.into()])
    }

    #[test]
    fn a_statement_naming_the_app_and_its_certificate_verifies() {
        let files = Files::start();
        files.serve(
            "/a",
            200,
            "application/json",
            &statement(RELATIONS[0], "com.example.mail", CERT),
        );
        files.serve(
            "/b",
            200,
            "application/json; charset=utf-8",
            &statement(RELATIONS[1], "com.example.mail", CERT),
        );
        assert_eq!(verify(&files, "/a", "com.example.mail", CERT), Ok(true));
        assert_eq!(
            verify(
                &files,
                "/b",
                "com.example.mail",
                &CERT.replace(':', "").to_lowercase()
            ),
            Ok(true)
        );
    }

    #[test]
    fn another_package_certificate_or_relation_does_not_verify() {
        let files = Files::start();
        files.serve(
            "/a",
            200,
            "application/json",
            &statement(RELATIONS[0], "com.example.mail", CERT),
        );
        files.serve(
            "/c",
            200,
            "application/json",
            &statement(
                "delegate_permission/common.use_as_origin",
                "com.example.mail",
                CERT,
            ),
        );
        assert_eq!(verify(&files, "/a", "com.attacker.mail", CERT), Ok(false));
        assert_eq!(
            verify(&files, "/a", "com.example.mail", &"AB".repeat(32)),
            Ok(false)
        );
        assert_eq!(verify(&files, "/c", "com.example.mail", CERT), Ok(false));
        assert_eq!(verify(&files, "/a", "com.example.mail", "AB:CD"), Ok(false));
    }

    #[test]
    fn includes_are_followed_up_to_a_loop_or_another_origin() {
        let files = Files::start();
        files.serve(
            "/root",
            200,
            "application/json",
            &format!(r#"[{{"include":"{}"}}]"#, files.url("/leaf")),
        );
        files.serve(
            "/leaf",
            200,
            "application/json",
            &statement(RELATIONS[0], "com.example.mail", CERT),
        );
        assert_eq!(verify(&files, "/root", "com.example.mail", CERT), Ok(true));
        files.serve(
            "/loop",
            200,
            "application/json",
            &format!(r#"[{{"include":"{}"}}]"#, files.url("/loop")),
        );
        assert!(verify(&files, "/loop", "com.example.mail", CERT).is_err());
        files.serve(
            "/https",
            200,
            "application/json",
            r#"[{"include":"https://example.com/x"}]"#,
        );
        assert!(verify(&files, "/https", "com.example.mail", CERT).is_err());
    }

    /// Serves a chain of `includes` included files ending in a statement.
    fn chain(files: &Files, prefix: &str, includes: usize) {
        for step in 0..includes {
            let next = files.url(&format!("/{prefix}{}", step + 1));
            files.serve(
                &format!("/{prefix}{step}"),
                200,
                "application/json",
                &format!(r#"[{{"include":"{next}"}}]"#),
            );
        }
        let leaf = statement(RELATIONS[0], "com.example.mail", CERT);
        files.serve(
            &format!("/{prefix}{includes}"),
            200,
            "application/json",
            &leaf,
        );
    }

    #[test]
    fn ten_includes_are_followed_and_an_eleventh_is_refused() {
        let files = Files::start();
        chain(&files, "ten", 10);
        assert_eq!(verify(&files, "/ten0", "com.example.mail", CERT), Ok(true));
        chain(&files, "eleven", 11);
        assert!(verify(&files, "/eleven0", "com.example.mail", CERT).is_err());
    }

    #[test]
    fn redirects_wrong_types_and_malformed_files_are_refused() {
        let files = Files::start();
        let body = statement(RELATIONS[0], "com.example.mail", CERT);
        files.serve("/redirect", 301, "application/json", &body);
        files.serve("/html", 200, "text/html", &body);
        files.serve("/broken", 200, "application/json", "{not json");
        files.serve("/large", 200, "application/json", &" ".repeat(300 * 1024));
        assert_eq!(
            verify(&files, "/redirect", "com.example.mail", CERT),
            Err(MobileError::NotFound)
        );
        assert!(verify(&files, "/html", "com.example.mail", CERT).is_err());
        assert!(verify(&files, "/broken", "com.example.mail", CERT).is_err());
        assert!(verify(&files, "/large", "com.example.mail", CERT).is_err());
        assert_eq!(
            verify(&files, "/missing", "com.example.mail", CERT),
            Err(MobileError::NotFound)
        );
    }

    #[test]
    fn only_plain_host_names_are_looked_up() {
        for domain in [
            "",
            "example.com/path",
            "user@example.com",
            "example.com:8443",
            "exa mple.com",
        ] {
            assert!(
                verify_app_domain(domain.into(), "com.example".into(), vec![CERT.into()]).is_err(),
                "{domain}"
            );
        }
    }

    #[test]
    fn fingerprints_are_compared_in_one_spelling() {
        assert_eq!(normalize_fingerprint("ab:cd:EF"), "ABCDEF");
    }
}
