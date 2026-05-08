// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! User-facing string seam.
//!
//! All translatable text the application emits to the user lives here. The
//! actual messages live as Fluent files in `i18n/<lang>/pasejo.ftl`,
//! embedded into the binary by `rust-embed` and loaded at runtime through
//! `i18n-embed`. English (`en`) is the fallback language; other languages
//! activate when the desktop locale matches.
//!
//! Wrapper functions (one per message) keep call sites free of message ids
//! and Fluent details — they accept native Rust types and emit the result
//! through the appropriate channel (`log` macro for status/error logs,
//! `println!` for stdout, returned `String` for notification bodies).

use std::path::Path;
use std::sync::LazyLock;
use std::time::Duration;

use anyhow::{Context, Result};
use i18n_embed::fluent::{FluentLanguageLoader, fluent_language_loader};
use i18n_embed::{DesktopLanguageRequester, LanguageLoader};
use i18n_embed_fl::fl;
use log::{debug, error, info, warn};
use rust_embed::RustEmbed;
use unic_langid::LanguageIdentifier;

#[derive(RustEmbed)]
#[folder = "i18n/"]
struct Localizations;

static LANGUAGE_LOADER: LazyLock<FluentLanguageLoader> = LazyLock::new(|| {
    let loader: FluentLanguageLoader = fluent_language_loader!();
    loader
        .load_fallback_language(&Localizations)
        .expect("could not load fallback language");
    // Strip the bidi isolation marks Fluent wraps around interpolated
    // values by default. This keeps CLI output pipe-safe and snapshot tests
    // stable. Per i18n-embed docs `set_use_isolating` is a no-op until at
    // least one bundle has been loaded, so it must come *after*
    // `load_fallback_language`.
    loader.set_use_isolating(false);
    loader
});

/// Selects the language based on the desktop locale and loads its messages.
/// Falls back to English when the requested locale has no translation.
pub fn init() -> Result<()> {
    let requested = requested_languages();
    i18n_embed::select(&*LANGUAGE_LOADER, &Localizations, &requested)
        .context("Could not initialize translations")?;
    // `select` may have loaded an additional language bundle whose
    // `is_isolating` flag defaults back to true; reapply the project-wide
    // setting so messages from the newly-selected locale stay free of bidi
    // marks too.
    LANGUAGE_LOADER.set_use_isolating(false);
    Ok(())
}

/// Resolve the user's preferred languages.
///
/// We can't just call `DesktopLanguageRequester::requested_languages()`
/// directly: on macOS that delegates to `CFLocaleCopyPreferredLanguages`,
/// which reads System Preferences and ignores the POSIX `LANG` / `LC_*`
/// environment variables entirely. That makes it impossible to override
/// the locale from the shell or from CI, and it means our translation
/// snapshot tests (`cli_tests_de`, `cli_tests_es`, …) silently get the
/// English fallback on every macOS runner.
///
/// We follow standard CLI conventions instead: POSIX env vars win, the
/// OS-native preference is the fallback. Precedence matches GNU gettext —
/// `LANGUAGE` (colon-separated chain) overrides `LC_ALL`, which overrides
/// `LC_MESSAGES`, which overrides `LANG`. `C` / `POSIX` / unparseable
/// values are treated as "no specific locale", which lets the loader's
/// fallback language (English) be used.
fn requested_languages() -> Vec<LanguageIdentifier> {
    for var in ["LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"] {
        let Ok(value) = std::env::var(var) else {
            continue;
        };
        if value.is_empty() {
            continue;
        }
        let candidates: Vec<&str> = if var == "LANGUAGE" {
            value.split(':').collect()
        } else {
            vec![value.as_str()]
        };
        return candidates
            .into_iter()
            .filter_map(parse_posix_locale)
            .collect();
    }
    DesktopLanguageRequester::requested_languages()
}

/// Strip the codeset (`.UTF-8`) and modifier (`@euro`) suffixes a POSIX
/// locale tag may carry, normalize the underscore POSIX uses to the
/// hyphen BCP 47 expects, and return `None` for empty / `C` / `POSIX`
/// values so the caller can fall through to the next env var or to the
/// loader's fallback language.
fn parse_posix_locale(raw: &str) -> Option<LanguageIdentifier> {
    let trimmed = raw
        .split('.')
        .next()?
        .split('@')
        .next()?
        .trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("C") || trimmed.eq_ignore_ascii_case("POSIX") {
        return None;
    }
    trimmed.replace('_', "-").parse().ok()
}

fn bool_key(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

fn path_string(path: &Path) -> String {
    path.display().to_string()
}

fn duration_string(duration: &Duration) -> String {
    format!("{duration:?}")
}

pub fn recipient_added(public_key: &str) {
    info!(
        "{}",
        fl!(LANGUAGE_LOADER, "recipient-added", public_key = public_key)
    );
}

pub fn recipient_removed(public_key: &str) {
    info!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "recipient-removed",
            public_key = public_key
        )
    );
}

pub fn secret_added(secret_path: &str) {
    info!(
        "{}",
        fl!(LANGUAGE_LOADER, "secret-added", secret_path = secret_path)
    );
}

pub fn secret_edited(secret_path: &str) {
    info!(
        "{}",
        fl!(LANGUAGE_LOADER, "secret-edited", secret_path = secret_path)
    );
}

pub fn one_time_password_added(password_path: &str) {
    info!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "one-time-password-added",
            password_path = password_path
        )
    );
}

pub fn one_time_password_copied(source_path: &str, target_path: &str) {
    info!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "one-time-password-copied",
            source_path = source_path,
            target_path = target_path
        )
    );
}

pub fn one_time_password_moved(source_path: &str, target_path: &str) {
    info!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "one-time-password-moved",
            source_path = source_path,
            target_path = target_path
        )
    );
}

pub fn one_time_password_removed(password_path: &str) {
    info!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "one-time-password-removed",
            password_path = password_path
        )
    );
}

pub fn one_time_password_copy_into_clipboard(password_path: &str, duration: &Duration) {
    let duration = duration_string(duration);
    info!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "one-time-password-copy-into-clipboard",
            password_path = password_path,
            duration = duration.as_str()
        )
    );
}

pub fn secret_generated(secret_path: &str) {
    info!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "secret-generated",
            secret_path = secret_path
        )
    );
}

pub fn secret_show_as_qrcode(secret_path: &str) {
    debug!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "secret-show-as-qrcode",
            secret_path = secret_path
        )
    );
}

pub fn secret_show_as_text(secret_path: &str) {
    debug!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "secret-show-as-text",
            secret_path = secret_path
        )
    );
}

pub fn secret_copy_into_clipboard(secret_path: &str, duration: &Duration) {
    let duration = duration_string(duration);
    info!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "secret-copy-into-clipboard",
            secret_path = secret_path,
            duration = duration.as_str()
        )
    );
}

pub fn one_time_password_show(password_path: &str) {
    debug!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "one-time-password-show",
            password_path = password_path
        )
    );
}

pub fn identity_added(identity_file: &Path) {
    let identity_file = path_string(identity_file);
    info!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "identity-added",
            identity_file = identity_file.as_str()
        )
    );
}

pub fn identity_removed(identity_file: &Path) {
    let identity_file = path_string(identity_file);
    info!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "identity-removed",
            identity_file = identity_file.as_str()
        )
    );
}

pub fn store_add_success(store_name: &str, store_path: &str) {
    info!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "store-add-success",
            store_name = store_name,
            store_path = store_path
        )
    );
}

pub fn store_set_default(store_name: &str) {
    info!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "store-set-default",
            store_name = store_name
        )
    );
}

pub fn store_remove_success(store_name: &str) {
    info!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "store-remove-success",
            store_name = store_name
        )
    );
}

pub fn execute_pull_hooks(store_name: &str) {
    debug!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "execute-pull-hooks",
            store_name = store_name
        )
    );
}

pub fn execute_push_hooks(store_name: &str) {
    debug!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "execute-push-hooks",
            store_name = store_name
        )
    );
}

pub fn recipient_does_not_exist_ignored(public_key: &str) {
    info!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "recipient-does-not-exist-ignored",
            public_key = public_key
        )
    );
}

pub fn no_identities_exist_yet(store_name: &str) {
    warn!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "no-identities-exist-yet",
            store_name = store_name
        )
    );
}

pub fn merge_conflict_recipient_names(public_key: &str, first_name: &str, second_name: &str) {
    error!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "merge-conflict-recipient-names",
            public_key = public_key,
            first_name = first_name,
            second_name = second_name
        )
    );
}

pub fn merge_conflict_recipient_removed_and_renamed(public_key: &str, new_name: &str) {
    error!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "merge-conflict-recipient-removed-and-renamed",
            public_key = public_key,
            new_name = new_name
        )
    );
}

pub fn merge_conflict_values(value_type: &str, secret_path: &str) {
    error!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "merge-conflict-values",
            value_type = value_type,
            secret_path = secret_path
        )
    );
}

pub fn merge_conflict_removed_and_modified(value_type: &str, secret_path: &str) {
    error!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "merge-conflict-removed-and-modified",
            value_type = value_type,
            secret_path = secret_path
        )
    );
}

pub fn secret_copied(source_path: &str, target_path: &str) {
    info!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "secret-copied",
            source_path = source_path,
            target_path = target_path
        )
    );
}

pub fn secret_moved(source_path: &str, target_path: &str) {
    info!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "secret-moved",
            source_path = source_path,
            target_path = target_path
        )
    );
}

pub fn secret_removed(secret_path: &str) {
    info!(
        "{}",
        fl!(LANGUAGE_LOADER, "secret-removed", secret_path = secret_path)
    );
}

pub fn list_global_identity(identity_file: &Path) {
    let identity_file = path_string(identity_file);
    println!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "list-global-identity",
            identity_file = identity_file.as_str()
        )
    );
}

pub fn list_store_identity(identity_file: &Path) {
    let identity_file = path_string(identity_file);
    println!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "list-store-identity",
            identity_file = identity_file.as_str()
        )
    );
}

pub fn list_global_pull_hook(command: &str) {
    println!(
        "{}",
        fl!(LANGUAGE_LOADER, "list-global-pull-hook", command = command)
    );
}

pub fn list_global_push_hook(command: &str) {
    println!(
        "{}",
        fl!(LANGUAGE_LOADER, "list-global-push-hook", command = command)
    );
}

pub fn list_store_pull_hook(command: &str) {
    println!(
        "{}",
        fl!(LANGUAGE_LOADER, "list-store-pull-hook", command = command)
    );
}

pub fn list_store_push_hook(command: &str) {
    println!(
        "{}",
        fl!(LANGUAGE_LOADER, "list-store-push-hook", command = command)
    );
}

pub fn list_store(store_name: &str, store_path: &Path, is_default: bool) {
    let store_path = path_string(store_path);
    println!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "list-store",
            store_name = store_name,
            store_path = store_path.as_str(),
            is_default = bool_key(is_default)
        )
    );
}

pub fn password_strength(secret_path: &str, score: f64) {
    println!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "password-strength",
            secret_path = secret_path,
            score = score
        )
    );
}

pub fn secret_search_match(key: &str, value: &str) {
    println!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "secret-search-match",
            key = key,
            value = value
        )
    );
}

pub fn clipboard_read_for_compare_failed(error: &impl std::fmt::Display) {
    let error = error.to_string();
    debug!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "clipboard-read-for-compare-failed",
            error = error.as_str()
        )
    );
}

pub fn clipboard_ctrlc_handler_install_failed(error: &impl std::fmt::Display) {
    let error = error.to_string();
    warn!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "clipboard-ctrlc-handler-install-failed",
            error = error.as_str()
        )
    );
}

pub fn clipboard_clear_failed(error: &impl std::fmt::Display) {
    let error = error.to_string();
    warn!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "clipboard-clear-failed",
            error = error.as_str()
        )
    );
}

pub fn clipboard_manual_clear_required() {
    error!(
        "{}",
        fl!(LANGUAGE_LOADER, "clipboard-manual-clear-required")
    );
}

pub fn clipboard_notification_dispatch_failed(error: &impl std::fmt::Display) {
    let error = error.to_string();
    debug!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "clipboard-notification-dispatch-failed",
            error = error.as_str()
        )
    );
}

pub fn clipboard_notification_cleared(cancelled: bool) -> String {
    fl!(
        LANGUAGE_LOADER,
        "clipboard-notification-cleared",
        cancelled = bool_key(cancelled)
    )
}

pub fn clipboard_notification_unchanged(cancelled: bool) -> String {
    fl!(
        LANGUAGE_LOADER,
        "clipboard-notification-unchanged",
        cancelled = bool_key(cancelled)
    )
}

pub fn clipboard_notification_forcibly_cleared(cancelled: bool) -> String {
    fl!(
        LANGUAGE_LOADER,
        "clipboard-notification-forcibly-cleared",
        cancelled = bool_key(cancelled)
    )
}

pub fn clipboard_notification_failed(cancelled: bool) -> String {
    fl!(
        LANGUAGE_LOADER,
        "clipboard-notification-failed",
        cancelled = bool_key(cancelled)
    )
}
