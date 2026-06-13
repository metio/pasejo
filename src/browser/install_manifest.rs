// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Writes the per-user native-messaging manifest that browsers consult
//! before they will spawn `pasejo browser host`. Two layers ship together:
//!
//! 1. A tiny wrapper script under the user's data directory. Native-messaging
//!    manifests can only point at an executable path — they cannot pass
//!    arguments — so the wrapper exists to invoke `pasejo browser host` on
//!    behalf of the browser. Callers who already maintain their own wrapper
//!    can pass `--binary` to skip wrapper creation; the supplied path is
//!    written into the manifest verbatim.
//! 2. The JSON manifest itself, written atomically into the browser's
//!    documented per-user lookup directory (Linux: under `~/.config` or
//!    `~/.mozilla`; macOS: under `~/Library/Application Support`).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use directories::{ProjectDirs, UserDirs};
use serde::Serialize;

use crate::cli::atomic_write;
use crate::cli::i18n;
use crate::models::cli::{InstallManifestArgs, InstallManifestBrowser};

/// Native-messaging host name. Must match the `NATIVE_HOST` constant
/// shipped in the web extension's `protocol.js`.
pub const HOST_NAME: &str = "wtf.metio.pasejo";

/// Stable Firefox add-on id baked into the web extension via
/// `browser_specific_settings.gecko.id`. Used when `--extension-id`
/// is not supplied to `install-manifest --browser firefox`.
pub const FIREFOX_DEFAULT_EXTENSION_ID: &str = "pasejo@metio.wtf";

const HOST_DESCRIPTION: &str = "pasejo native messaging host";
const HOST_TYPE: &str = "stdio";

#[derive(Debug, Serialize)]
struct ManifestJson<'a> {
    name: &'a str,
    description: &'a str,
    path: String,
    #[serde(rename = "type")]
    host_type: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    allowed_extensions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    allowed_origins: Option<Vec<String>>,
}

pub fn run(args: &InstallManifestArgs) -> Result<()> {
    if args.browsers.is_empty() {
        bail!(i18n::error_browser_install_manifest_no_browser());
    }
    // A single `--extension-id` list cannot be disambiguated across
    // multiple browsers — applying a chromium id to Firefox's gecko-id
    // slot (or vice versa) writes a manifest that never authorizes the
    // real extension, silently. Require one browser per invocation when
    // ids are supplied.
    if args.browsers.len() > 1 && !args.extension_ids.is_empty() {
        bail!(i18n::error_browser_install_manifest_extension_id_multiple_browsers());
    }
    let host_name = args.host_name.as_deref().unwrap_or(HOST_NAME);
    let binary = resolve_binary(args.binary.as_deref())?;
    let envs = parse_env_pairs(&args.envs)?;

    let (manifest_path_field, wrapper) =
        compute_wrapper(args.binary.as_deref(), &binary, host_name, &envs)?;

    if args.print
        && let Some((wrapper_path, _)) = wrapper.as_ref()
    {
        // `--print` skips wrapper installation, but the printed
        // manifest's `path` field still references the wrapper under
        // the data directory. A user piping the JSON into the manifest
        // location by hand would end up pointing the browser at a
        // script that doesn't exist — say so on stderr (the JSON on
        // stdout stays pipeable).
        i18n::browser_install_manifest_print_wrapper_note(wrapper_path);
    }

    // Build (and thereby validate, e.g. chromium browsers requiring an
    // extension id) EVERY manifest before writing any, so a later
    // browser's validation failure can't leave earlier browsers'
    // wrappers and manifests half-installed.
    let mut prepared = Vec::with_capacity(args.browsers.len());
    for browser in &args.browsers {
        let manifest = build_manifest_json(
            *browser,
            host_name,
            &manifest_path_field,
            &args.extension_ids,
        )?;
        let json = serde_json::to_string_pretty(&manifest)?;
        prepared.push((*browser, json));
    }

    for (browser, json) in &prepared {
        if args.print {
            println!("{json}");
            continue;
        }
        if let Some((wrapper_path, content)) = wrapper.as_ref() {
            install_wrapper(wrapper_path, content)?;
        }
        let target = manifest_path_for_browser(*browser, host_name)?;
        atomic_write::write(&target, json.as_bytes())?;
        i18n::browser_install_manifest_written(browser.as_str(), &target);
    }
    Ok(())
}

fn resolve_binary(binary_override: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = binary_override {
        return Ok(path.to_path_buf());
    }
    std::env::current_exe().context(i18n::error_browser_install_manifest_no_current_exe())
}

/// Returns `(path-to-write-into-manifest, optional-wrapper-to-install)`.
/// When the caller supplies their own `--binary`, no wrapper is created
/// and the manifest references that path directly. Otherwise we point the
/// manifest at the wrapper under the data directory and ship its content
/// for the writer to install.
fn compute_wrapper(
    binary_override: Option<&Path>,
    binary: &Path,
    host_name: &str,
    envs: &[(String, String)],
) -> Result<(String, Option<(PathBuf, String)>)> {
    if binary_override.is_some() {
        return Ok((binary.display().to_string(), None));
    }
    let wrapper_path = wrapper_path(host_name)?;
    let content = wrapper_content(binary, envs);
    Ok((
        wrapper_path.display().to_string(),
        Some((wrapper_path, content)),
    ))
}

fn wrapper_path(host_name: &str) -> Result<PathBuf> {
    let dirs = ProjectDirs::from("wtf", "metio", "pasejo")
        .context(i18n::error_browser_install_manifest_no_data_dir())?;
    Ok(dirs.data_dir().join(host_name))
}

fn wrapper_content(binary: &Path, envs: &[(String, String)]) -> String {
    let mut body = String::from("#!/bin/sh\n# Generated by `pasejo browser install-manifest`.\n");
    for (key, value) in envs {
        body.push_str("export ");
        body.push_str(key);
        body.push('=');
        body.push_str(&shell_quote_str(value));
        body.push('\n');
    }
    body.push_str("exec ");
    body.push_str(&shell_quote_str(&binary.display().to_string()));
    body.push_str(" browser host\n");
    body
}

/// Single-quote-wrap the value so spaces, `$`, and other shell metas are
/// passed through verbatim. Embedded single quotes are escaped via the
/// `'\''` idiom POSIX shells understand.
fn shell_quote_str(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Parse each `KEY=VALUE` entry. Splits on the first `=` only so values
/// containing `=` (paths, base64, etc.) round-trip unchanged. Validates
/// `KEY` against POSIX's portable env-var name rule (`[A-Za-z_][A-Za-z0-9_]*`)
/// so the generated wrapper script is guaranteed parseable by `/bin/sh`.
fn parse_env_pairs(raw: &[String]) -> Result<Vec<(String, String)>> {
    raw.iter()
        .map(|entry| {
            let (key, value) = entry.split_once('=').ok_or_else(|| {
                anyhow::anyhow!(i18n::error_browser_install_manifest_invalid_env(entry))
            })?;
            if !is_posix_env_name(key) {
                bail!(i18n::error_browser_install_manifest_invalid_env_name(key));
            }
            Ok((key.to_owned(), value.to_owned()))
        })
        .collect()
}

fn is_posix_env_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(unix)]
fn install_wrapper(path: &Path, content: &str) -> Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    if let Some(parent) = path.parent() {
        let parent_display = parent.display().to_string();
        std::fs::create_dir_all(parent)
            .with_context(|| i18n::error_failed_to_create_directory(&parent_display))?;
    }
    let path_display = path.display().to_string();
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o700)
        .open(path)
        .with_context(|| i18n::error_failed_to_create_file(&path_display))?;
    file.write_all(content.as_bytes())
        .with_context(|| i18n::error_failed_to_write_file(&path_display))?;
    file.sync_all()
        .with_context(|| i18n::error_failed_to_fsync_file(&path_display))?;
    Ok(())
}

#[cfg(not(unix))]
fn install_wrapper(_path: &Path, _content: &str) -> Result<()> {
    bail!(i18n::error_browser_install_manifest_unsupported_os(
        std::env::consts::OS
    ));
}

fn manifest_path_for_browser(browser: InstallManifestBrowser, host_name: &str) -> Result<PathBuf> {
    let dirs = UserDirs::new().context(i18n::error_browser_install_manifest_no_home_dir())?;
    let home = dirs.home_dir();
    let relative = match (std::env::consts::OS, browser) {
        ("linux", InstallManifestBrowser::Firefox) => ".mozilla/native-messaging-hosts",
        ("linux", InstallManifestBrowser::Chrome) => ".config/google-chrome/NativeMessagingHosts",
        ("linux", InstallManifestBrowser::Chromium) => ".config/chromium/NativeMessagingHosts",
        ("linux", InstallManifestBrowser::Brave) => {
            ".config/BraveSoftware/Brave-Browser/NativeMessagingHosts"
        }
        ("linux", InstallManifestBrowser::Edge) => ".config/microsoft-edge/NativeMessagingHosts",
        ("linux", InstallManifestBrowser::Vivaldi) => ".config/vivaldi/NativeMessagingHosts",
        ("macos", InstallManifestBrowser::Firefox) => {
            "Library/Application Support/Mozilla/NativeMessagingHosts"
        }
        ("macos", InstallManifestBrowser::Chrome) => {
            "Library/Application Support/Google/Chrome/NativeMessagingHosts"
        }
        ("macos", InstallManifestBrowser::Chromium) => {
            "Library/Application Support/Chromium/NativeMessagingHosts"
        }
        ("macos", InstallManifestBrowser::Brave) => {
            "Library/Application Support/BraveSoftware/Brave-Browser/NativeMessagingHosts"
        }
        ("macos", InstallManifestBrowser::Edge) => {
            "Library/Application Support/Microsoft Edge/NativeMessagingHosts"
        }
        ("macos", InstallManifestBrowser::Vivaldi) => {
            "Library/Application Support/Vivaldi/NativeMessagingHosts"
        }
        (other, _) => bail!(i18n::error_browser_install_manifest_unsupported_os(other)),
    };
    Ok(home.join(relative).join(format!("{host_name}.json")))
}

fn build_manifest_json<'a>(
    browser: InstallManifestBrowser,
    host_name: &'a str,
    path: &str,
    extension_ids: &[String],
) -> Result<ManifestJson<'a>> {
    let resolved = resolve_extension_ids(browser, extension_ids)?;
    let (allowed_extensions, allowed_origins) = if browser == InstallManifestBrowser::Firefox {
        (Some(resolved), None)
    } else {
        let origins: Vec<String> = resolved
            .into_iter()
            .map(|id| format!("chrome-extension://{id}/"))
            .collect();
        (None, Some(origins))
    };
    Ok(ManifestJson {
        name: host_name,
        description: HOST_DESCRIPTION,
        path: path.to_owned(),
        host_type: HOST_TYPE,
        allowed_extensions,
        allowed_origins,
    })
}

fn resolve_extension_ids(
    browser: InstallManifestBrowser,
    supplied: &[String],
) -> Result<Vec<String>> {
    if !supplied.is_empty() {
        return Ok(supplied.to_vec());
    }
    match browser {
        InstallManifestBrowser::Firefox => Ok(vec![FIREFOX_DEFAULT_EXTENSION_ID.to_owned()]),
        other => bail!(i18n::error_browser_install_manifest_needs_extension_id(
            other.as_str()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::i18n;

    #[test]
    fn shell_quote_wraps_a_simple_value() {
        assert_eq!(
            shell_quote_str("/usr/local/bin/pasejo"),
            "'/usr/local/bin/pasejo'"
        );
    }

    #[test]
    fn shell_quote_escapes_embedded_single_quotes() {
        assert_eq!(
            shell_quote_str("/home/o'connor/bin/pasejo"),
            "'/home/o'\\''connor/bin/pasejo'",
        );
    }

    #[test]
    fn wrapper_content_includes_exec_browser_host() {
        let content = wrapper_content(Path::new("/usr/local/bin/pasejo"), &[]);
        assert!(content.starts_with("#!/bin/sh\n"));
        assert!(content.contains("exec '/usr/local/bin/pasejo' browser host\n"));
        assert!(!content.contains("export "));
    }

    #[test]
    fn wrapper_content_emits_export_lines_before_exec() {
        let envs = vec![
            ("PASEJO_DEFAULT_STORE".to_owned(), "work".to_owned()),
            ("SSH_AUTH_SOCK".to_owned(), "/run/user/1000/ssh".to_owned()),
        ];
        let content = wrapper_content(Path::new("/usr/local/bin/pasejo"), &envs);
        let export_idx = content
            .find("export PASEJO_DEFAULT_STORE='work'")
            .expect("first export missing");
        let export2_idx = content
            .find("export SSH_AUTH_SOCK='/run/user/1000/ssh'")
            .expect("second export missing");
        let exec_idx = content.find("exec ").expect("exec missing");
        assert!(
            export_idx < exec_idx && export2_idx < exec_idx,
            "exports must precede exec so the host inherits them",
        );
    }

    #[test]
    fn wrapper_content_quotes_env_values_with_specials() {
        let envs = vec![("MY_VAR".to_owned(), "needs 'escape'".to_owned())];
        let content = wrapper_content(Path::new("/bin/pasejo"), &envs);
        assert!(content.contains("export MY_VAR='needs '\\''escape'\\'''"));
    }

    #[test]
    fn parse_env_pairs_splits_on_first_equals() {
        i18n::init_for_tests();
        let pairs = parse_env_pairs(&["FOO=a=b=c".into()]).unwrap();
        assert_eq!(pairs, vec![("FOO".to_owned(), "a=b=c".to_owned())]);
    }

    #[test]
    fn parse_env_pairs_rejects_missing_equals() {
        i18n::init_for_tests();
        let err = parse_env_pairs(&["FOO".into()]).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("foo"));
    }

    #[test]
    fn parse_env_pairs_rejects_invalid_posix_names() {
        i18n::init_for_tests();
        for bad in ["", "1FOO", "FOO-BAR", "foo bar", "$X"] {
            let entry = format!("{bad}=value");
            assert!(
                parse_env_pairs(&[entry.clone()]).is_err(),
                "expected `{entry}` to be rejected",
            );
        }
    }

    #[test]
    fn parse_env_pairs_accepts_posix_names() {
        i18n::init_for_tests();
        for good in ["FOO", "_FOO", "FOO_BAR", "F0O", "_"] {
            let entry = format!("{good}=value");
            assert!(
                parse_env_pairs(&[entry.clone()]).is_ok(),
                "expected `{entry}` to be accepted",
            );
        }
    }

    #[test]
    fn firefox_manifest_uses_allowed_extensions_with_default_id() {
        let manifest =
            build_manifest_json(InstallManifestBrowser::Firefox, HOST_NAME, "/x", &[]).unwrap();
        assert_eq!(manifest.name, HOST_NAME);
        assert_eq!(manifest.host_type, "stdio");
        assert!(manifest.allowed_origins.is_none());
        let extensions = manifest.allowed_extensions.unwrap();
        assert_eq!(extensions, vec![FIREFOX_DEFAULT_EXTENSION_ID.to_owned()]);
    }

    #[test]
    fn firefox_manifest_respects_supplied_extension_ids() {
        let ids = vec!["a@example.org".to_owned(), "b@example.org".to_owned()];
        let manifest =
            build_manifest_json(InstallManifestBrowser::Firefox, HOST_NAME, "/x", &ids).unwrap();
        assert_eq!(manifest.allowed_extensions.unwrap(), ids);
    }

    #[test]
    fn chromium_manifest_uses_chrome_extension_origins() {
        let ids = vec!["abcdefghijklmnop".to_owned()];
        let manifest =
            build_manifest_json(InstallManifestBrowser::Chrome, HOST_NAME, "/x", &ids).unwrap();
        assert!(manifest.allowed_extensions.is_none());
        assert_eq!(
            manifest.allowed_origins.unwrap(),
            vec!["chrome-extension://abcdefghijklmnop/".to_owned()]
        );
    }

    #[test]
    fn chromium_manifest_without_extension_id_errors() {
        i18n::init_for_tests();
        for browser in [
            InstallManifestBrowser::Chrome,
            InstallManifestBrowser::Chromium,
            InstallManifestBrowser::Brave,
            InstallManifestBrowser::Edge,
            InstallManifestBrowser::Vivaldi,
        ] {
            let err = build_manifest_json(browser, HOST_NAME, "/x", &[]).unwrap_err();
            assert!(
                err.to_string().to_lowercase().contains("extension"),
                "expected extension-id error for {browser:?}, got: {err}"
            );
        }
    }

    #[test]
    fn manifest_path_for_browser_uses_per_browser_directory() {
        let path = manifest_path_for_browser(InstallManifestBrowser::Firefox, HOST_NAME).unwrap();
        let s = path.display().to_string();
        let expected_fragment = match std::env::consts::OS {
            "linux" => ".mozilla/native-messaging-hosts",
            "macos" => "Library/Application Support/Mozilla/NativeMessagingHosts",
            _ => return,
        };
        assert!(
            s.contains(expected_fragment),
            "path {s} does not contain {expected_fragment}"
        );
        assert!(s.ends_with(&format!("{HOST_NAME}.json")));
    }
}
