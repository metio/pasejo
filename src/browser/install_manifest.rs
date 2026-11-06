// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Writes the per-user native-messaging manifest that browsers consult
//! before they will spawn `pasejo browser host`. Two layers ship together:
//!
//! 1. A tiny wrapper script per browser under the user's data directory.
//!    Native-messaging manifests can only point at an executable path — they
//!    cannot pass arguments — so the wrapper exists to invoke `pasejo browser
//!    host` on behalf of the browser, with one `--allowed-caller` per extension
//!    the browser's manifest admits. Callers who already maintain their own
//!    wrapper can pass `--binary` to skip wrapper creation; the supplied path
//!    is written into the manifest verbatim.
//! 2. The JSON manifest itself, written atomically into the browser's
//!    documented per-user lookup directory (Linux: under `~/.config` or
//!    `~/.mozilla`; macOS: under `~/Library/Application Support`). On Windows
//!    browsers locate the manifest through the registry instead of a fixed
//!    directory, so the manifest is written under the data dir and an
//!    `HKEY_CURRENT_USER` value is pointed at it (see `register_native_host`),
//!    and the wrapper is a `.cmd` batch file rather than a POSIX shell script.

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

/// The directories and OS the install resolves its paths against.
struct InstallDirs {
    os: &'static str,
    home: Option<PathBuf>,
    data: Option<PathBuf>,
}

impl ManifestJson<'_> {
    /// The extension identities this manifest admits, in the form the browser
    /// passes them to the host: origins for chromium-family browsers, add-on
    /// ids for Firefox.
    fn allowed_callers(&self) -> &[String] {
        self.allowed_extensions
            .as_deref()
            .or(self.allowed_origins.as_deref())
            .unwrap_or_default()
    }
}

impl InstallDirs {
    fn from_env() -> Self {
        Self {
            os: std::env::consts::OS,
            home: UserDirs::new().map(|dirs| dirs.home_dir().to_path_buf()),
            data: ProjectDirs::from("wtf", "metio", "pasejo")
                .map(|dirs| dirs.data_dir().to_path_buf()),
        }
    }

    fn home(&self) -> Result<&Path> {
        self.home
            .as_deref()
            .context(i18n::error_browser_install_manifest_no_home_dir())
    }

    fn data(&self) -> Result<&Path> {
        self.data
            .as_deref()
            .context(i18n::error_browser_install_manifest_no_data_dir())
    }
}

pub fn run(args: &InstallManifestArgs) -> Result<()> {
    install(args, &InstallDirs::from_env())
}

fn install(args: &InstallManifestArgs, dirs: &InstallDirs) -> Result<()> {
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
    for browser in &args.browsers {
        if !is_valid_host_name(*browser, host_name) {
            bail!(i18n::error_browser_install_manifest_invalid_host_name(
                host_name,
                browser.as_str()
            ));
        }
    }
    let binary = resolve_binary(args.binary.as_deref())?;
    let envs = parse_env_pairs(&args.envs)?;

    // Build (and thereby validate, e.g. chromium browsers requiring an
    // extension id) and locate EVERY manifest and wrapper before writing any,
    // so a later browser's validation failure can't leave earlier browsers
    // half-installed.
    let mut prepared = Vec::with_capacity(args.browsers.len());
    for browser in &args.browsers {
        prepared.push(prepare(args, dirs, *browser, host_name, &binary, &envs)?);
    }

    for item in &prepared {
        let Some(target) = &item.target else {
            if let Some((wrapper_path, _)) = &item.wrapper {
                // `--print` skips wrapper installation, but the printed
                // manifest's `path` field still references the wrapper under
                // the data directory. A user piping the JSON into the
                // manifest location by hand would end up pointing the browser
                // at a script that doesn't exist — say so on stderr (the JSON
                // on stdout stays pipeable).
                i18n::browser_install_manifest_print_wrapper_note(wrapper_path);
            }
            println!("{}", item.json);
            continue;
        };
        if let Some((wrapper_path, content)) = &item.wrapper {
            install_wrapper(wrapper_path, content)?;
        }
        atomic_write::write(target, item.json.as_bytes())?;
        // No-op on Unix (the manifest already sits in the lookup directory);
        // on Windows this writes the HKCU registry value pointing at it.
        register_native_host(item.browser, host_name, target)?;
        i18n::browser_install_manifest_written(item.browser.as_str(), target);
    }
    Ok(())
}

/// Everything one browser's installation writes, computed without touching
/// the filesystem.
struct PreparedInstall {
    browser: InstallManifestBrowser,
    json: String,
    /// `None` with `--print`.
    target: Option<PathBuf>,
    /// The wrapper's path and content; `None` with `--binary`.
    wrapper: Option<(PathBuf, String)>,
}

/// Each browser gets its own wrapper, because the wrapper admits exactly the
/// extensions that browser's manifest lists: a shared wrapper would be
/// rewritten by the next `install-manifest` run for another browser and
/// refuse the first browser's extension. With `--binary` the manifest
/// references that path and no wrapper is written.
fn prepare(
    args: &InstallManifestArgs,
    dirs: &InstallDirs,
    browser: InstallManifestBrowser,
    host_name: &str,
    binary: &Path,
    envs: &[(String, String)],
) -> Result<PreparedInstall> {
    let wrapper_path = if args.binary.is_some() {
        None
    } else {
        Some(wrapper_path(dirs, browser, host_name)?)
    };
    let path_field = wrapper_path
        .as_deref()
        .unwrap_or(binary)
        .display()
        .to_string();
    let manifest = build_manifest_json(browser, host_name, &path_field, &args.extension_ids)?;
    let wrapper = match wrapper_path {
        Some(path) => Some((
            path,
            wrapper_content(binary, envs, manifest.allowed_callers())?,
        )),
        None => None,
    };
    let target = if args.print {
        None
    } else {
        Some(manifest_path_for_browser(dirs, browser, host_name)?)
    };
    Ok(PreparedInstall {
        browser,
        json: serde_json::to_string_pretty(&manifest)?,
        target,
        wrapper,
    })
}

/// The host-name grammar the browser enforces: dot-separated, non-empty
/// parts of ASCII letters, digits, and underscores. Firefox accepts
/// uppercase letters; the chromium family accepts lowercase only.
fn is_valid_host_name(browser: InstallManifestBrowser, name: &str) -> bool {
    let allow_uppercase = browser == InstallManifestBrowser::Firefox;
    name.split('.').all(|part| {
        !part.is_empty()
            && part.chars().all(|c| {
                c.is_ascii_lowercase()
                    || c.is_ascii_digit()
                    || c == '_'
                    || (allow_uppercase && c.is_ascii_uppercase())
            })
    })
}

fn resolve_binary(binary_override: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = binary_override {
        return Ok(path.to_path_buf());
    }
    std::env::current_exe().context(i18n::error_browser_install_manifest_no_current_exe())
}

fn wrapper_path(
    dirs: &InstallDirs,
    browser: InstallManifestBrowser,
    host_name: &str,
) -> Result<PathBuf> {
    // `.cmd` so Windows launches it as a batch script (the manifest's `path`
    // points here); no extension on Unix, where the shebang drives execution.
    let filename = if cfg!(windows) {
        format!("{host_name}.cmd")
    } else {
        host_name.to_owned()
    };
    Ok(browser_data_dir(dirs, browser)?.join(filename))
}

fn browser_data_dir(dirs: &InstallDirs, browser: InstallManifestBrowser) -> Result<PathBuf> {
    Ok(dirs.data()?.join("native-messaging").join(browser.as_str()))
}

/// The wrapper script for the current platform. The wrapper exists because
/// native-messaging manifests carry no `args` field, so something has to
/// invoke `pasejo browser host` (with the user's `--env` variables and the
/// manifest's extensions as `--allowed-caller`) on the browser's behalf. Both
/// generators are pure and always compiled, so each is unit-testable
/// regardless of the host OS.
fn wrapper_content(
    binary: &Path,
    envs: &[(String, String)],
    allowed_callers: &[String],
) -> Result<String> {
    if cfg!(windows) {
        windows_wrapper_content(binary, envs, allowed_callers)
    } else {
        Ok(posix_wrapper_content(binary, envs, allowed_callers))
    }
}

/// The `--allowed-caller` values precede `"$@"`, because the host reads the
/// browser's arguments as trailing values once the first of them appears.
fn posix_wrapper_content(
    binary: &Path,
    envs: &[(String, String)],
    allowed_callers: &[String],
) -> String {
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
    body.push_str(" browser host");
    for caller in allowed_callers {
        body.push_str(" --allowed-caller ");
        body.push_str(&shell_quote_str(caller));
    }
    body.push_str(" \"$@\"\n");
    body
}

/// A `.cmd` wrapper for Windows. The browser launches the host via `cmd /c`,
/// so the batch file sets any `--env` variables and then runs the binary with
/// `browser host`, the `--allowed-caller` values, and `%*`, which forwards the
/// arguments the browser appends (the `chrome-extension://…` origin and
/// `--parent-window=…`, or Firefox's manifest path and add-on id). `@echo off`
/// keeps command echoes out of the stdio stream the native-messaging protocol
/// owns. Values are wrapped in `set "KEY=VALUE"` (tolerating spaces and most
/// metacharacters); a value with `"` or a line break cannot be quoted that way
/// and is rejected. Allowed callers are extension ids, which
/// `resolve_extension_ids` limits to characters that need no escaping inside
/// double quotes. Batch expands `%` inside quotes too, so every `%` — in
/// values, callers and the binary path — is doubled to `%%`; a Windows path
/// can never contain `"`, so quoting the binary is safe. cmd decodes each
/// batch line in the console's current code page, so a wrapper with non-ASCII
/// bytes switches to UTF-8 (65001) before reaching them.
fn windows_wrapper_content(
    binary: &Path,
    envs: &[(String, String)],
    allowed_callers: &[String],
) -> Result<String> {
    let header = "@echo off\r\nrem Generated by `pasejo browser install-manifest`.\r\n";
    let mut body = String::new();
    for (key, value) in envs {
        if value.contains(['"', '\r', '\n']) {
            bail!(i18n::error_browser_install_manifest_invalid_env_value_windows(key));
        }
        body.push_str("set \"");
        body.push_str(key);
        body.push('=');
        body.push_str(&value.replace('%', "%%"));
        body.push_str("\"\r\n");
    }
    body.push('"');
    body.push_str(&binary.display().to_string().replace('%', "%%"));
    body.push_str("\" browser host");
    for caller in allowed_callers {
        body.push_str(" --allowed-caller \"");
        body.push_str(&caller.replace('%', "%%"));
        body.push('"');
    }
    body.push_str(" %*\r\n");
    if body.is_ascii() {
        Ok(format!("{header}{body}"))
    } else {
        Ok(format!("{header}chcp 65001 >nul 2>&1\r\n{body}"))
    }
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

/// The wrapper is owner-only and executable on Unix; on Windows a `.cmd` is
/// executable by extension and ACLs are inherited.
fn install_wrapper(path: &Path, content: &str) -> Result<()> {
    atomic_write::write_with_mode(path, content.as_bytes(), 0o700)
}

fn manifest_path_for_browser(
    dirs: &InstallDirs,
    browser: InstallManifestBrowser,
    host_name: &str,
) -> Result<PathBuf> {
    // On Windows the manifest is not placed in a magic per-browser directory;
    // a registry value (written by `register_native_host`) points the browser
    // at it. Keep it per-browser under the data dir so each browser's distinct
    // `allowed_origins` / `allowed_extensions` manifest gets its own file.
    if cfg!(windows) {
        return windows_manifest_path(dirs, browser, host_name);
    }
    let home = dirs.home()?;
    let relative = match (dirs.os, browser) {
        ("linux", InstallManifestBrowser::Firefox) => ".mozilla/native-messaging-hosts",
        ("linux", InstallManifestBrowser::Chrome) => ".config/google-chrome/NativeMessagingHosts",
        ("linux", InstallManifestBrowser::Chromium) => ".config/chromium/NativeMessagingHosts",
        ("linux", InstallManifestBrowser::Brave) => {
            ".config/BraveSoftware/Brave-Browser/NativeMessagingHosts"
        }
        ("linux", InstallManifestBrowser::Edge) => ".config/microsoft-edge/NativeMessagingHosts",
        ("linux", InstallManifestBrowser::Vivaldi) => ".config/vivaldi/NativeMessagingHosts",
        ("linux", InstallManifestBrowser::Opera) => ".config/opera/NativeMessagingHosts",
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
        ("macos", InstallManifestBrowser::Opera) => {
            "Library/Application Support/com.operasoftware.Opera/NativeMessagingHosts"
        }
        (other, _) => bail!(i18n::error_browser_install_manifest_unsupported_os(other)),
    };
    Ok(home.join(relative).join(format!("{host_name}.json")))
}

/// Per-browser manifest path under the user's data directory, used on Windows
/// where the lookup happens through the registry rather than a fixed directory.
/// Always compiled (pure path computation) so it is unit-testable off Windows.
fn windows_manifest_path(
    dirs: &InstallDirs,
    browser: InstallManifestBrowser,
    host_name: &str,
) -> Result<PathBuf> {
    Ok(browser_data_dir(dirs, browser)?.join(format!("{host_name}.json")))
}

/// The `HKEY_CURRENT_USER` subkey under which `browser` looks up
/// native-messaging hosts on Windows; the host name is appended as a child key
/// by `register_native_host`. The Firefox, Chrome, Chromium, and Edge paths are
/// documented by their vendors; the chromium forks (Brave, Vivaldi, Opera) use
/// their own vendor subkeys here and are best-effort until verified on Windows
/// — a wrong fork path is a one-line fix. Pure, so it is unit-testable off
/// Windows (where the only non-test caller, `register_native_host`, is
/// compiled out).
#[cfg_attr(not(windows), allow(dead_code))]
const fn windows_registry_subkey(browser: InstallManifestBrowser) -> &'static str {
    match browser {
        InstallManifestBrowser::Firefox => r"Software\Mozilla\NativeMessagingHosts",
        InstallManifestBrowser::Chrome => r"Software\Google\Chrome\NativeMessagingHosts",
        InstallManifestBrowser::Chromium => r"Software\Chromium\NativeMessagingHosts",
        InstallManifestBrowser::Edge => r"Software\Microsoft\Edge\NativeMessagingHosts",
        InstallManifestBrowser::Brave => {
            r"Software\BraveSoftware\Brave-Browser\NativeMessagingHosts"
        }
        InstallManifestBrowser::Vivaldi => r"Software\Vivaldi\NativeMessagingHosts",
        InstallManifestBrowser::Opera => r"Software\Opera Software\NativeMessagingHosts",
    }
}

/// Point the browser at the manifest file. On Windows this writes the HKCU
/// registry value the browser consults; on Unix the manifest already sits in
/// the browser's lookup directory, so there is nothing to register.
#[cfg(windows)]
fn register_native_host(
    browser: InstallManifestBrowser,
    host_name: &str,
    manifest_path: &Path,
) -> Result<()> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let subkey = format!("{}\\{host_name}", windows_registry_subkey(browser));
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey(&subkey)
        .with_context(|| i18n::error_browser_install_manifest_registry_failed(&subkey))?;
    // The default (unnamed) value holds the absolute path to the manifest JSON.
    key.set_value("", &manifest_path.display().to_string())
        .with_context(|| i18n::error_browser_install_manifest_registry_failed(&subkey))?;
    Ok(())
}

// Platform shim: its `Result` signature mirrors the Windows version (which
// can fail) and is propagated with `?` at the call site, so the always-`Ok`
// body and `Result` return are intentional rather than removable.
#[cfg(not(windows))]
#[allow(clippy::unnecessary_wraps, clippy::missing_const_for_fn)]
fn register_native_host(
    _browser: InstallManifestBrowser,
    _host_name: &str,
    _manifest_path: &Path,
) -> Result<()> {
    Ok(())
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
        if let Some(invalid) = supplied.iter().find(|id| !is_valid_extension_id(id)) {
            bail!(i18n::error_browser_install_manifest_invalid_extension_id(
                invalid
            ));
        }
        return Ok(supplied.to_vec());
    }
    match browser {
        InstallManifestBrowser::Firefox => Ok(vec![FIREFOX_DEFAULT_EXTENSION_ID.to_owned()]),
        other => bail!(i18n::error_browser_install_manifest_needs_extension_id(
            other.as_str()
        )),
    }
}

/// Chromium extension ids are 32 letters `a`–`p`; Firefox add-on ids are
/// either `name@domain` of ASCII letters, digits, `-`, `.` and `_`, or a GUID
/// in curly braces. Admitting only the union of those characters keeps every
/// id safe to quote in both wrapper formats.
fn is_valid_extension_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '@' | '.' | '_' | '-' | '{' | '}'))
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
    fn posix_wrapper_content_includes_exec_browser_host() {
        let content = posix_wrapper_content(Path::new("/usr/local/bin/pasejo"), &[], &[]);
        assert!(content.starts_with("#!/bin/sh\n"));
        assert!(content.contains("exec '/usr/local/bin/pasejo' browser host \"$@\"\n"));
        assert!(!content.contains("export "));
    }

    #[test]
    fn posix_wrapper_content_emits_export_lines_before_exec() {
        let envs = vec![
            ("PASEJO_DEFAULT_STORE".to_owned(), "work".to_owned()),
            ("SSH_AUTH_SOCK".to_owned(), "/run/user/1000/ssh".to_owned()),
        ];
        let content = posix_wrapper_content(Path::new("/usr/local/bin/pasejo"), &envs, &[]);
        let store_export = content
            .find("export PASEJO_DEFAULT_STORE='work'")
            .expect("first export missing");
        let socket_export = content
            .find("export SSH_AUTH_SOCK='/run/user/1000/ssh'")
            .expect("second export missing");
        let exec_idx = content.find("exec ").expect("exec missing");
        assert!(
            store_export < exec_idx && socket_export < exec_idx,
            "exports must precede exec so the host inherits them",
        );
    }

    #[test]
    fn posix_wrapper_content_quotes_env_values_with_specials() {
        let envs = vec![("MY_VAR".to_owned(), "needs 'escape'".to_owned())];
        let content = posix_wrapper_content(Path::new("/bin/pasejo"), &envs, &[]);
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
                parse_env_pairs(std::slice::from_ref(&entry)).is_err(),
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
                parse_env_pairs(std::slice::from_ref(&entry)).is_ok(),
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
            InstallManifestBrowser::Opera,
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
        let path = manifest_path_for_browser(
            &InstallDirs::from_env(),
            InstallManifestBrowser::Firefox,
            HOST_NAME,
        )
        .unwrap();
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

    #[test]
    fn manifest_path_resolves_for_every_browser_on_a_supported_os() {
        use clap::ValueEnum;

        let os = std::env::consts::OS;
        if os != "linux" && os != "macos" {
            // The unsupported-OS bail is covered by its own behavior; this
            // test only guards the per-browser arms on the OSes we support.
            return;
        }
        // Iterating every clap variant means adding a browser without its
        // per-OS directory arm fails here (it would otherwise fall through to
        // the unsupported-OS bail at runtime) rather than shipping broken.
        for browser in InstallManifestBrowser::value_variants() {
            let path = manifest_path_for_browser(&InstallDirs::from_env(), *browser, HOST_NAME)
                .unwrap_or_else(|error| {
                    panic!("no native-messaging directory for {browser:?} on {os}: {error}")
                });
            let rendered = path.display().to_string();
            assert!(
                rendered.to_lowercase().contains("nativemessaginghosts")
                    || rendered.contains("native-messaging-hosts"),
                "{browser:?} path {rendered} does not look like a native-messaging dir"
            );
            assert!(rendered.ends_with(&format!("{HOST_NAME}.json")));
        }
    }

    #[test]
    fn windows_wrapper_content_runs_browser_host_via_cmd() {
        let content =
            windows_wrapper_content(Path::new(r"C:\Program Files\pasejo\pasejo.exe"), &[], &[])
                .unwrap();
        assert!(content.starts_with("@echo off\r\n"));
        assert!(
            content.contains("\"C:\\Program Files\\pasejo\\pasejo.exe\" browser host %*\r\n"),
            "{content}"
        );
        assert!(!content.contains("set \""));
    }

    #[test]
    fn windows_wrapper_content_quotes_env_and_doubles_percent() {
        let envs = vec![(String::from("PASEJO_DEFAULT_STORE"), String::from("wo%rk"))];
        let content = windows_wrapper_content(Path::new(r"C:\pasejo.exe"), &envs, &[]).unwrap();
        // `%` is doubled for batch and the value is quoted; the `set` line must
        // precede the exec so the variable is in scope for the host.
        assert!(
            content.contains("set \"PASEJO_DEFAULT_STORE=wo%%rk\"\r\n"),
            "{content}"
        );
        let set_idx = content.find("set \"PASEJO_DEFAULT_STORE").unwrap();
        let exec_idx = content.find("browser host %*").unwrap();
        assert!(set_idx < exec_idx);
    }

    #[test]
    fn windows_wrapper_content_doubles_percent_in_binary_path() {
        let content =
            windows_wrapper_content(Path::new(r"C:\Users\a%b%\pasejo.exe"), &[], &[]).unwrap();
        assert!(
            content.contains("\"C:\\Users\\a%%b%%\\pasejo.exe\" browser host %*\r\n"),
            "{content}"
        );
    }

    fn assert_utf8_code_page_precedes_non_ascii(content: &str) {
        let switch = content
            .find("chcp 65001")
            .unwrap_or_else(|| panic!("no code-page switch in {content}"));
        let first_non_ascii = content
            .find(|c: char| !c.is_ascii())
            .unwrap_or(content.len());
        assert!(switch < first_non_ascii, "{content}");
    }

    #[test]
    fn windows_wrapper_content_switches_to_utf8_for_a_non_ascii_binary_path() {
        let content = windows_wrapper_content(
            Path::new(r"C:\Users\Jürgen\.cargo\bin\pasejo.exe"),
            &[],
            &[],
        )
        .unwrap();
        assert_utf8_code_page_precedes_non_ascii(&content);
        assert!(
            content.contains("\"C:\\Users\\Jürgen\\.cargo\\bin\\pasejo.exe\" browser host %*\r\n"),
            "{content}"
        );
    }

    #[test]
    fn windows_wrapper_content_switches_to_utf8_for_a_non_ascii_env_value() {
        let envs = vec![(String::from("PASEJO_DEFAULT_STORE"), String::from("büro"))];
        let content = windows_wrapper_content(Path::new(r"C:\pasejo.exe"), &envs, &[]).unwrap();
        assert_utf8_code_page_precedes_non_ascii(&content);
    }

    #[test]
    fn windows_wrapper_content_keeps_the_code_page_for_ascii_content() {
        let content = windows_wrapper_content(Path::new(r"C:\pasejo.exe"), &[], &[]).unwrap();
        assert!(!content.contains("chcp"), "{content}");
    }

    #[test]
    fn windows_wrapper_rejects_env_values_that_break_out_of_set() {
        i18n::init_for_tests();
        for value in ["a\r\necho pwned", "a\necho pwned", "a\"&calc&\""] {
            let envs = vec![(String::from("X"), value.to_owned())];
            let error = windows_wrapper_content(Path::new(r"C:\p.exe"), &envs, &[]).unwrap_err();
            assert!(error.to_string().contains("'X'"), "{value:?}: {error}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn install_wrapper_makes_an_existing_non_executable_wrapper_executable() {
        use std::os::unix::fs::PermissionsExt;

        i18n::init_for_tests();
        let temp = assert_fs::TempDir::new().unwrap();
        let path = temp.path().join(HOST_NAME);
        std::fs::write(&path, "old").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();

        install_wrapper(&path, "#!/bin/sh\n").unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "#!/bin/sh\n");
    }

    #[test]
    fn host_name_grammar_follows_each_browser() {
        use clap::ValueEnum;

        for browser in InstallManifestBrowser::value_variants() {
            assert!(is_valid_host_name(*browser, HOST_NAME), "{browser:?}");
            assert!(is_valid_host_name(*browser, "a_1.b2"), "{browser:?}");
            for bad in ["", ".a", "a.", "a..b", "../x", "/tmp/x", "a-b", "a b", "ä"] {
                assert!(!is_valid_host_name(*browser, bad), "{browser:?} {bad}");
            }
        }
        assert!(is_valid_host_name(
            InstallManifestBrowser::Firefox,
            "Wtf.Metio"
        ));
        assert!(!is_valid_host_name(
            InstallManifestBrowser::Chrome,
            "Wtf.Metio"
        ));
    }

    fn firefox_args() -> InstallManifestArgs {
        InstallManifestArgs {
            browsers: vec![InstallManifestBrowser::Firefox],
            extension_ids: vec![],
            print: false,
            binary: None,
            host_name: None,
            envs: vec![],
        }
    }

    fn temp_dirs(temp: &assert_fs::TempDir, os: &'static str) -> InstallDirs {
        InstallDirs {
            os,
            home: Some(temp.path().join("home")),
            data: Some(temp.path().join("data")),
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn install_on_an_unsupported_os_writes_nothing() {
        i18n::init_for_tests();
        let temp = assert_fs::TempDir::new().unwrap();
        let dirs = temp_dirs(&temp, "freebsd");

        assert!(install(&firefox_args(), &dirs).is_err());

        assert!(!temp.path().join("data").exists());
        assert!(!temp.path().join("home").exists());
    }

    #[cfg(not(windows))]
    #[test]
    fn install_writes_the_wrapper_and_the_manifest() {
        i18n::init_for_tests();
        let temp = assert_fs::TempDir::new().unwrap();
        let dirs = temp_dirs(&temp, "linux");

        install(&firefox_args(), &dirs).unwrap();

        let wrapper = temp
            .path()
            .join("data/native-messaging/firefox")
            .join(HOST_NAME);
        assert!(wrapper.is_file());
        let manifest = temp
            .path()
            .join("home/.mozilla/native-messaging-hosts")
            .join(format!("{HOST_NAME}.json"));
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(manifest).unwrap()).unwrap();
        assert_eq!(json["path"], wrapper.display().to_string());
    }

    #[cfg(not(windows))]
    #[test]
    fn each_browser_keeps_a_wrapper_that_admits_only_its_own_extensions() {
        i18n::init_for_tests();
        let temp = assert_fs::TempDir::new().unwrap();
        let dirs = temp_dirs(&temp, "linux");

        install(&firefox_args(), &dirs).unwrap();
        install(
            &InstallManifestArgs {
                browsers: vec![InstallManifestBrowser::Chrome],
                extension_ids: vec!["abcdefghijklmnop".to_owned()],
                ..firefox_args()
            },
            &dirs,
        )
        .unwrap();

        let firefox = std::fs::read_to_string(
            temp.path()
                .join("data/native-messaging/firefox")
                .join(HOST_NAME),
        )
        .unwrap();
        let chrome = std::fs::read_to_string(
            temp.path()
                .join("data/native-messaging/chrome")
                .join(HOST_NAME),
        )
        .unwrap();
        assert!(
            firefox.contains("browser host --allowed-caller 'pasejo@metio.wtf' \"$@\""),
            "{firefox}"
        );
        assert!(!firefox.contains("chrome-extension"), "{firefox}");
        assert!(
            chrome.contains(
                "browser host --allowed-caller 'chrome-extension://abcdefghijklmnop/' \"$@\""
            ),
            "{chrome}"
        );
        assert!(!chrome.contains("pasejo@metio.wtf"), "{chrome}");
    }

    #[cfg(not(windows))]
    #[test]
    fn install_with_a_binary_writes_no_wrapper() {
        i18n::init_for_tests();
        let temp = assert_fs::TempDir::new().unwrap();
        let dirs = temp_dirs(&temp, "linux");

        install(
            &InstallManifestArgs {
                binary: Some(PathBuf::from("/opt/launcher")),
                ..firefox_args()
            },
            &dirs,
        )
        .unwrap();

        assert!(!temp.path().join("data").exists());
        let manifest = temp
            .path()
            .join("home/.mozilla/native-messaging-hosts")
            .join(format!("{HOST_NAME}.json"));
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(manifest).unwrap()).unwrap();
        assert_eq!(json["path"], "/opt/launcher");
    }

    #[test]
    fn wrapper_path_is_per_browser_under_the_data_dir() {
        let path = wrapper_path(
            &InstallDirs::from_env(),
            InstallManifestBrowser::Edge,
            HOST_NAME,
        )
        .unwrap();
        let parent = path.parent().unwrap();
        assert!(
            parent.ends_with(Path::new("native-messaging").join("edge")),
            "{}",
            path.display()
        );
        let file_name = path.file_name().unwrap().to_string_lossy().into_owned();
        let expected = if cfg!(windows) {
            format!("{HOST_NAME}.cmd")
        } else {
            HOST_NAME.to_owned()
        };
        assert_eq!(file_name, expected);
    }

    #[test]
    fn allowed_callers_are_the_identities_the_manifest_admits() {
        let firefox =
            build_manifest_json(InstallManifestBrowser::Firefox, HOST_NAME, "/x", &[]).unwrap();
        assert_eq!(firefox.allowed_callers(), [FIREFOX_DEFAULT_EXTENSION_ID]);
        let chrome = build_manifest_json(
            InstallManifestBrowser::Brave,
            HOST_NAME,
            "/x",
            &["abcdefghijklmnop".to_owned(), "ponmlkjihgfedcba".to_owned()],
        )
        .unwrap();
        assert_eq!(
            chrome.allowed_callers(),
            [
                "chrome-extension://abcdefghijklmnop/",
                "chrome-extension://ponmlkjihgfedcba/"
            ]
        );
    }

    #[test]
    fn posix_wrapper_passes_each_allowed_caller_before_the_browser_arguments() {
        let content = posix_wrapper_content(
            Path::new("/usr/local/bin/pasejo"),
            &[],
            &[
                "pasejo@metio.wtf".to_owned(),
                "{0a1b2c3d-0000-4000-8000-0123456789ab}".to_owned(),
            ],
        );
        assert!(
            content.ends_with(
                "exec '/usr/local/bin/pasejo' browser host --allowed-caller 'pasejo@metio.wtf' \
                 --allowed-caller '{0a1b2c3d-0000-4000-8000-0123456789ab}' \"$@\"\n"
            ),
            "{content}"
        );
    }

    #[test]
    fn windows_wrapper_passes_each_allowed_caller_before_the_browser_arguments() {
        let content = windows_wrapper_content(
            Path::new(r"C:\pasejo.exe"),
            &[],
            &["chrome-extension://abcdefghijklmnop/".to_owned()],
        )
        .unwrap();
        assert!(
            content.ends_with(
                "\"C:\\pasejo.exe\" browser host --allowed-caller \
                 \"chrome-extension://abcdefghijklmnop/\" %*\r\n"
            ),
            "{content}"
        );
    }

    #[test]
    fn extension_ids_are_limited_to_the_characters_browsers_use() {
        i18n::init_for_tests();
        for good in [
            "abcdefghijklmnopabcdefghijklmnop",
            "pasejo@metio.wtf",
            "my-addon_1@example.org",
            "{0a1b2c3d-0000-4000-8000-0123456789ab}",
        ] {
            assert!(
                build_manifest_json(
                    InstallManifestBrowser::Firefox,
                    HOST_NAME,
                    "/x",
                    &[good.to_owned()]
                )
                .is_ok(),
                "{good}"
            );
        }
        for bad in ["", "a b", "a\"b", "a%b", "a'b", "a\nb", "a/b", "ä@b"] {
            for browser in [
                InstallManifestBrowser::Firefox,
                InstallManifestBrowser::Chrome,
            ] {
                let error =
                    build_manifest_json(browser, HOST_NAME, "/x", &[bad.to_owned()]).unwrap_err();
                assert!(
                    error.to_string().contains("--extension-id"),
                    "{bad:?}: {error}"
                );
            }
        }
    }

    #[test]
    fn windows_registry_subkey_is_defined_for_every_browser() {
        use clap::ValueEnum;

        // A `match` over the enum — iterating every variant proves none was
        // left out (a missing arm would fail to compile, but this also pins
        // the shape: an HKCU `Software\…\NativeMessagingHosts` subkey).
        for browser in InstallManifestBrowser::value_variants() {
            let subkey = windows_registry_subkey(*browser);
            assert!(subkey.starts_with(r"Software\"), "{browser:?}: {subkey}");
            assert!(
                subkey.ends_with(r"\NativeMessagingHosts"),
                "{browser:?}: {subkey}"
            );
        }
        assert_eq!(
            windows_registry_subkey(InstallManifestBrowser::Edge),
            r"Software\Microsoft\Edge\NativeMessagingHosts"
        );
        assert_eq!(
            windows_registry_subkey(InstallManifestBrowser::Firefox),
            r"Software\Mozilla\NativeMessagingHosts"
        );
    }

    #[test]
    fn windows_manifest_path_is_per_browser_under_the_data_dir() {
        let path = windows_manifest_path(
            &InstallDirs::from_env(),
            InstallManifestBrowser::Chrome,
            HOST_NAME,
        )
        .unwrap();
        let rendered = path.display().to_string();
        assert!(rendered.contains("native-messaging"), "{rendered}");
        assert!(rendered.contains("chrome"), "{rendered}");
        assert!(
            rendered.ends_with(&format!("{HOST_NAME}.json")),
            "{rendered}"
        );
    }
}
