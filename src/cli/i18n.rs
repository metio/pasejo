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

static LANGUAGE_LOADER: LazyLock<FluentLanguageLoader> =
    LazyLock::new(|| fluent_language_loader!());

/// Loads the fallback language, selects the user's preferred language
/// based on the desktop locale, and applies project-wide Fluent settings.
/// Falls back to English when the requested locale has no translation.
pub fn init() -> Result<()> {
    LANGUAGE_LOADER
        .load_fallback_language(&Localizations)
        .context("Could not load fallback language")?;
    let requested = requested_languages();
    i18n_embed::select(&*LANGUAGE_LOADER, &Localizations, &requested)
        .context("Could not initialize translations")?;
    // Strip the bidi isolation marks Fluent wraps around interpolated
    // values by default. This keeps CLI output pipe-safe and snapshot tests
    // stable. Per i18n-embed docs `set_use_isolating` is a no-op until at
    // least one bundle has been loaded, so it must come *after* the loads
    // above. `select` may also have loaded an additional language bundle
    // whose `is_isolating` flag defaults back to true; this single call
    // applies the project-wide setting to every loaded bundle.
    LANGUAGE_LOADER.set_use_isolating(false);
    Ok(())
}

/// Test-only initializer for the language loader.
///
/// Unit tests don't go through `main`, so `init` never runs and any
/// `fl!` call returns the `"No localization for id: …"` placeholder
/// instead of a real string. This helper loads the English fallback
/// bundle deterministically — skipping the locale-driven `select` step
/// so a host `LANG=de_DE.UTF-8` (or similar) can't change which bundle
/// the assertions see — and turns off bidi isolation so the strings
/// match plain `assert_eq!` comparisons.
///
/// Idempotent: subsequent calls are a no-op, so it's cheap to invoke at
/// the top of every test that depends on resolved messages.
#[cfg(test)]
pub fn init_for_tests() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        LANGUAGE_LOADER
            .load_fallback_language(&Localizations)
            .expect("could not load English fallback bundle for tests");
        LANGUAGE_LOADER.set_use_isolating(false);
    });
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
    resolve_languages(|var| std::env::var(var).ok())
        .unwrap_or_else(DesktopLanguageRequester::requested_languages)
}

/// The pure core of [`requested_languages`], parameterized over the
/// environment `lookup` so the precedence chain can be unit-tested without
/// mutating process-global env vars. Returns `Some(langs)` as soon as one of
/// the gettext-precedence variables (`LANGUAGE` > `LC_ALL` > `LC_MESSAGES` >
/// `LANG`) is set and non-empty — even when every locale in it is `C`/`POSIX`
/// and the resulting list is empty, since the variable being *set* is the
/// signal to stop looking. Returns `None` only when none are set, so the
/// caller falls back to the OS-native requester. `LANGUAGE` is a
/// colon-separated chain; the others carry a single locale.
fn resolve_languages(lookup: impl Fn(&str) -> Option<String>) -> Option<Vec<LanguageIdentifier>> {
    for var in ["LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"] {
        let Some(value) = lookup(var) else {
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
        return Some(
            candidates
                .into_iter()
                .filter_map(parse_posix_locale)
                .collect(),
        );
    }
    None
}

/// Strip the codeset (`.UTF-8`) and modifier (`@euro`) suffixes a POSIX
/// locale tag may carry, normalize the underscore POSIX uses to the
/// hyphen BCP 47 expects, and return `None` for empty / `C` / `POSIX`
/// values so the caller can fall through to the next env var or to the
/// loader's fallback language.
fn parse_posix_locale(raw: &str) -> Option<LanguageIdentifier> {
    let trimmed = raw.split('.').next()?.split('@').next()?.trim();
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("C")
        || trimmed.eq_ignore_ascii_case("POSIX")
    {
        return None;
    }
    trimmed.replace('_', "-").parse().ok()
}

const fn bool_key(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

fn path_string(path: &Path) -> String {
    path.display().to_string()
}

fn duration_string(duration: &Duration) -> String {
    format!("{duration:?}")
}

/// Generates a wrapper function that resolves a Fluent message with the
/// given positional `&str` arguments and emits it through one of the `log`
/// macros (`info!`, `debug!`, `warn!`, `error!`). Each argument name is
/// used both as the function parameter and as the Fluent variable name.
macro_rules! fl_log {
    ($level:ident, $fn:ident, $key:literal $(, $arg:ident)* $(,)?) => {
        pub fn $fn($($arg: &str),*) {
            $level!("{}", fl!(LANGUAGE_LOADER, $key $(, $arg = $arg)*));
        }
    };
}

/// Generates a wrapper function that resolves a Fluent message with the
/// given positional `&str` arguments and writes it to stdout. Used for
/// content the user pipes or visually inspects (list output, search hits)
/// rather than for status logs.
macro_rules! fl_println {
    ($fn:ident, $key:literal $(, $arg:ident)* $(,)?) => {
        pub fn $fn($($arg: &str),*) {
            println!("{}", fl!(LANGUAGE_LOADER, $key $(, $arg = $arg)*));
        }
    };
}

/// Generates a clipboard-notification helper: takes a `cancelled: bool`,
/// returns the resolved Fluent string for the notification body (which
/// `notify-rust` then renders as a desktop popup).
macro_rules! fl_notification {
    ($fn:ident, $key:literal) => {
        pub fn $fn(cancelled: bool) -> String {
            fl!(LANGUAGE_LOADER, $key, cancelled = bool_key(cancelled))
        }
    };
}

/// Generates a wrapper that takes a `&Path`, renders it via `path_string`,
/// and emits a Fluent message through one of the `log` macros.
macro_rules! fl_log_path {
    ($level:ident, $fn:ident, $key:literal, $arg:ident) => {
        pub fn $fn($arg: &Path) {
            let $arg = path_string($arg);
            $level!("{}", fl!(LANGUAGE_LOADER, $key, $arg = $arg.as_str()));
        }
    };
}

/// Like [`fl_log_path`] but writes to stdout. Used by list commands that
/// display path entries.
macro_rules! fl_println_path {
    ($fn:ident, $key:literal, $arg:ident) => {
        pub fn $fn($arg: &Path) {
            let $arg = path_string($arg);
            println!("{}", fl!(LANGUAGE_LOADER, $key, $arg = $arg.as_str()));
        }
    };
}

/// Generates a wrapper that takes any `Display` error, converts it to a
/// `String`, and emits a Fluent message through one of the `log` macros.
/// The Fluent variable is always named `error`.
macro_rules! fl_log_error {
    ($level:ident, $fn:ident, $key:literal) => {
        pub fn $fn(error: &impl std::fmt::Display) {
            let error = error.to_string();
            $level!("{}", fl!(LANGUAGE_LOADER, $key, error = error.as_str()));
        }
    };
}

/// Generates a wrapper that resolves a Fluent message with the given
/// positional `&str` arguments and returns the rendered `String`. Used
/// for content the caller embeds in a `Result` / `anyhow` context, or
/// passes as a prompt string to `inquire`.
macro_rules! fl_string {
    ($fn:ident, $key:literal $(, $arg:ident)* $(,)?) => {
        pub fn $fn($($arg: &str),*) -> String {
            fl!(LANGUAGE_LOADER, $key $(, $arg = $arg)*)
        }
    };
}

// Status logs — info level
fl_log!(info, recipient_added, "recipient-added", public_key);
fl_log!(info, recipient_removed, "recipient-removed", public_key);
fl_log!(info, secret_added, "secret-added", secret_path);
fl_log!(info, secret_edited, "secret-edited", secret_path);
fl_log!(info, secret_generated, "secret-generated", secret_path);
fl_log!(info, secret_removed, "secret-removed", secret_path);
fl_log!(
    info,
    secret_copied,
    "secret-copied",
    source_path,
    target_path
);
fl_log!(info, secret_moved, "secret-moved", source_path, target_path);
fl_log!(
    info,
    one_time_password_added,
    "one-time-password-added",
    password_path
);
fl_log!(
    info,
    one_time_password_removed,
    "one-time-password-removed",
    password_path
);
fl_log!(
    info,
    one_time_password_copied,
    "one-time-password-copied",
    source_path,
    target_path
);
fl_log!(
    info,
    one_time_password_moved,
    "one-time-password-moved",
    source_path,
    target_path
);
fl_log!(
    info,
    store_add_success,
    "store-add-success",
    store_name,
    store_path
);
fl_log!(info, store_set_default, "store-set-default", store_name);
fl_log!(
    info,
    store_remove_success,
    "store-remove-success",
    store_name
);
fl_log!(
    info,
    store_remove_success_with_data,
    "store-remove-success-with-data",
    store_name
);
fl_log!(
    info,
    recipient_does_not_exist_ignored,
    "recipient-does-not-exist-ignored",
    public_key
);

// Status logs — debug level
fl_log!(
    debug,
    secret_show_as_qrcode,
    "secret-show-as-qrcode",
    secret_path
);
fl_log!(
    debug,
    secret_show_as_text,
    "secret-show-as-text",
    secret_path
);
fl_log!(
    debug,
    one_time_password_show,
    "one-time-password-show",
    password_path
);
fl_log!(debug, execute_pull_hooks, "execute-pull-hooks", store_name);
fl_log!(debug, execute_push_hooks, "execute-push-hooks", store_name);
fl_log!(warn, hook_run_nothing_selected, "hook-run-nothing-selected");
fl_log!(warn, export_plaintext_warning, "export-plaintext-warning");
fl_log_path!(info, export_written_to_file, "export-written-to-file", path);

// Status logs — warn / error levels
fl_log!(
    warn,
    no_identities_exist_yet,
    "no-identities-exist-yet",
    store_name
);
fl_log!(
    error,
    merge_conflict_recipient_names,
    "merge-conflict-recipient-names",
    public_key,
    first_name,
    second_name
);
fl_log!(
    error,
    merge_conflict_recipient_removed_and_renamed,
    "merge-conflict-recipient-removed-and-renamed",
    public_key,
    new_name
);
fl_log!(
    error,
    merge_conflict_values,
    "merge-conflict-values",
    value_type,
    secret_path
);
fl_log!(
    error,
    merge_conflict_removed_and_modified,
    "merge-conflict-removed-and-modified",
    value_type,
    secret_path
);

// stdout output — used for content the user pipes or reads.
fl_println!(list_global_pull_hook, "list-global-pull-hook", command);
fl_println!(list_global_push_hook, "list-global-push-hook", command);
fl_println!(list_store_pull_hook, "list-store-pull-hook", command);
fl_println!(list_store_push_hook, "list-store-push-hook", command);
fl_println!(secret_search_match, "secret-search-match", key, value);

// Clipboard desktop notifications — return rendered String for notify-rust.
fl_notification!(
    clipboard_notification_cleared,
    "clipboard-notification-cleared"
);
fl_notification!(
    clipboard_notification_unchanged,
    "clipboard-notification-unchanged"
);
fl_notification!(
    clipboard_notification_forcibly_cleared,
    "clipboard-notification-forcibly-cleared"
);
fl_notification!(
    clipboard_notification_failed,
    "clipboard-notification-failed"
);

// Path wrappers
fl_log_path!(info, identity_added, "identity-added", identity_file);
fl_log_path!(info, identity_removed, "identity-removed", identity_file);
fl_println_path!(list_global_identity, "list-global-identity", identity_file);
fl_println_path!(list_store_identity, "list-store-identity", identity_file);
fl_println_path!(
    list_store_identity_also_global,
    "list-store-identity-also-global",
    identity_file
);

// Error-display wrappers
fl_log_error!(
    debug,
    clipboard_read_for_compare_failed,
    "clipboard-read-for-compare-failed"
);
fl_log_error!(
    warn,
    clipboard_ctrlc_handler_install_failed,
    "clipboard-ctrlc-handler-install-failed"
);
fl_log_error!(warn, clipboard_clear_failed, "clipboard-clear-failed");
fl_log_error!(
    debug,
    clipboard_notification_dispatch_failed,
    "clipboard-notification-dispatch-failed"
);
fl_log_error!(
    debug,
    clipboard_drop_clear_failed,
    "clipboard-drop-clear-failed"
);

// Remaining hand-written wrappers: mixed argument types that don't fit a
// single-shape macro.

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

pub fn clipboard_manual_clear_required() {
    error!(
        "{}",
        fl!(LANGUAGE_LOADER, "clipboard-manual-clear-required")
    );
}

// String-returning helpers — used for anyhow context messages and prompt
// labels. Each renders a Fluent message and returns the result so the
// caller can embed it in a `Result` chain or pass it to `inquire`.

// Prompts
fl_string!(prompt_enter_secret, "prompt-enter-secret", secret_path);
fl_string!(
    prompt_could_not_read_secret,
    "prompt-could-not-read-secret",
    secret_path
);
fl_string!(prompt_overwrite_secret, "prompt-overwrite-secret");
fl_string!(prompt_remove_secret, "prompt-remove-secret");
fl_string!(
    prompt_overwrite_one_time_password,
    "prompt-overwrite-one-time-password"
);
fl_string!(
    prompt_remove_one_time_password,
    "prompt-remove-one-time-password"
);

// Error messages
fl_string!(
    error_cannot_get_user_confirmation,
    "error-cannot-get-user-confirmation"
);
fl_string!(
    error_no_confirmation_from_non_terminal,
    "error-no-confirmation-from-non-terminal"
);
fl_string!(
    error_could_not_load_configuration,
    "error-could-not-load-configuration"
);
fl_string!(
    error_store_does_not_exist,
    "error-store-does-not-exist",
    store_name
);
fl_string!(
    error_invalid_line_number,
    "error-invalid-line-number",
    input
);
fl_string!(
    error_line_number_must_not_be_zero,
    "error-line-number-must-not-be-zero"
);
fl_string!(error_invalid_count, "error-invalid-count", input);
fl_string!(error_count_must_not_be_zero, "error-count-must-not-be-zero");
fl_string!(
    error_file_does_not_exist,
    "error-file-does-not-exist",
    input
);
fl_string!(
    error_secret_already_exists,
    "error-secret-already-exists",
    secret_path
);
fl_string!(
    error_secret_already_exists_overwrite_or_inplace,
    "error-secret-already-exists-overwrite-or-inplace",
    secret_path
);
fl_string!(error_no_secret_found, "error-no-secret-found", secret_path);
fl_string!(
    error_not_allowed_to_remove_secret,
    "error-not-allowed-to-remove-secret",
    secret_path
);
fl_string!(
    error_secret_does_not_exist_for_edit,
    "error-secret-does-not-exist-for-edit",
    secret_path
);
fl_string!(
    error_one_time_password_already_exists,
    "error-one-time-password-already-exists",
    password_path
);
fl_string!(
    error_not_allowed_to_remove_one_time_password,
    "error-not-allowed-to-remove-one-time-password",
    password_path
);
fl_string!(
    error_no_one_time_password_found,
    "error-no-one-time-password-found",
    password_path
);
fl_string!(
    error_no_store_in_configuration,
    "error-no-store-in-configuration"
);
fl_string!(error_unknown_store, "error-unknown-store", store_name);
fl_string!(
    error_decrypt_requires_yes_i_know,
    "error-decrypt-requires-yes-i-know"
);
fl_string!(error_no_store_or_global, "error-no-store-or-global");
fl_string!(
    error_store_name_already_exists,
    "error-store-name-already-exists"
);
fl_string!(
    error_store_path_is_directory,
    "error-store-path-is-directory"
);
fl_string!(
    error_cannot_create_store_path,
    "error-cannot-create-store-path"
);
fl_string!(
    error_cannot_get_store_parent,
    "error-cannot-get-store-parent"
);
fl_string!(error_cannot_identify_store, "error-cannot-identify-store");
fl_string!(
    error_identity_file_already_added,
    "error-identity-file-already-added",
    file
);
fl_string!(
    error_identity_file_not_known,
    "error-identity-file-not-known",
    file
);
fl_string!(
    error_identity_did_you_mean,
    "error-identity-did-you-mean",
    stored
);
fl_string!(
    error_no_identity_files_to_decrypt,
    "error-no-identity-files-to-decrypt"
);
fl_string!(
    error_invalid_ssh_public_key_format,
    "error-invalid-ssh-public-key-format"
);
fl_string!(error_username_empty, "error-username-empty");
fl_string!(
    error_username_contains_dotdot,
    "error-username-contains-dotdot",
    username
);
fl_string!(
    error_username_invalid_character,
    "error-username-invalid-character",
    username
);
fl_string!(
    error_no_public_key_found_in_file,
    "error-no-public-key-found-in-file",
    filename
);
fl_string!(
    error_no_public_keys_for_user,
    "error-no-public-keys-for-user",
    provider,
    username
);
fl_string!(error_no_public_key_source, "error-no-public-key-source");
fl_string!(error_no_qrcode_found, "error-no-qrcode-found", qrcode);
fl_string!(
    error_failed_to_decode_qrcode,
    "error-failed-to-decode-qrcode",
    qrcode
);
fl_string!(error_qr_sandbox_failed, "error-qr-sandbox-failed");
fl_string!(
    error_qr_sandbox_child_signal,
    "error-qr-sandbox-child-signal",
    signal
);
fl_string!(
    error_qr_sandbox_not_enforced,
    "error-qr-sandbox-not-enforced"
);
fl_string!(
    error_cannot_determine_store_name,
    "error-cannot-determine-store-name"
);
fl_string!(
    error_cannot_parse_hook_command,
    "error-cannot-parse-hook-command",
    command
);
fl_string!(
    error_empty_hook_command,
    "error-empty-hook-command",
    command
);
fl_string!(
    error_failed_to_run_hook,
    "error-failed-to-run-hook",
    command
);
fl_string!(
    error_hook_failed_no_detail,
    "error-hook-failed-no-detail",
    command,
    exit
);
fl_string!(
    error_hook_failed_with_detail,
    "error-hook-failed-with-detail",
    command,
    exit,
    detail
);
fl_string!(
    error_cannot_determine_store_parent_path,
    "error-cannot-determine-store-parent-path",
    path
);
fl_string!(
    error_store_path_not_utf8,
    "error-store-path-not-utf8",
    token,
    path
);
fl_string!(
    error_merge_conflict_recipients,
    "error-merge-conflict-recipients"
);
fl_string!(error_merge_conflict, "error-merge-conflict", value_type);
fl_string!(
    error_recipient_not_found_in_store,
    "error-recipient-not-found-in-store"
);
fl_string!(
    error_cannot_remove_last_recipient,
    "error-cannot-remove-last-recipient"
);
fl_string!(
    error_cannot_decrypt_common_ancestor_store,
    "error-cannot-decrypt-common-ancestor-store"
);
fl_string!(
    error_cannot_decrypt_current_version_store,
    "error-cannot-decrypt-current-version-store"
);
fl_string!(
    error_cannot_decrypt_other_version_store,
    "error-cannot-decrypt-other-version-store"
);
fl_string!(error_cannot_read_file, "error-cannot-read-file", path);
fl_string!(
    error_downloading_public_key_failed,
    "error-downloading-public-key-failed",
    provider
);
fl_string!(
    error_download_user_not_found,
    "error-download-user-not-found",
    provider,
    username
);
fl_string!(
    error_download_rate_limited,
    "error-download-rate-limited",
    provider
);
fl_string!(
    error_download_server_error,
    "error-download-server-error",
    provider,
    status
);
fl_string!(error_download_timeout, "error-download-timeout", provider);
fl_string!(
    error_cannot_determine_parent_directory,
    "error-cannot-determine-parent-directory",
    path
);
fl_string!(
    error_failed_to_create_directory,
    "error-failed-to-create-directory",
    path
);
fl_string!(
    error_failed_to_write_file,
    "error-failed-to-write-file",
    path
);
fl_string!(
    error_failed_to_fsync_file,
    "error-failed-to-fsync-file",
    path
);
fl_string!(
    error_failed_to_rename_file,
    "error-failed-to-rename-file",
    from,
    to
);
fl_string!(
    error_failed_to_create_file,
    "error-failed-to-create-file",
    path
);
fl_string!(
    error_could_not_resolve_config_path,
    "error-could-not-resolve-config-path"
);
fl_string!(
    error_could_not_determine_config_path,
    "error-could-not-determine-config-path"
);
fl_string!(
    error_could_not_resolve_store_path,
    "error-could-not-resolve-store-path"
);
fl_string!(
    error_could_not_create_config_dir,
    "error-could-not-create-config-dir"
);
fl_string!(
    error_could_not_migrate_legacy_config,
    "error-could-not-migrate-legacy-config"
);
fl_string!(error_could_not_move_file, "error-could-not-move-file");
fl_string!(error_could_not_copy_file, "error-could-not-copy-file");
fl_string!(
    error_could_not_remove_source_after_copy,
    "error-could-not-remove-source-after-copy"
);
fl_string!(
    error_could_not_read_configuration,
    "error-could-not-read-configuration"
);
fl_string!(
    error_could_not_serialize_migrated_config,
    "error-could-not-serialize-migrated-config"
);
fl_string!(
    error_could_not_store_configuration,
    "error-could-not-store-configuration"
);
fl_string!(
    error_could_not_load_migrated_config,
    "error-could-not-load-migrated-config"
);
fl_string!(
    error_could_not_serialize_configuration,
    "error-could-not-serialize-configuration"
);
fl_string!(
    error_config_not_valid_toml,
    "error-config-not-valid-toml",
    path
);
fl_string!(
    error_config_invalid_integer,
    "error-config-invalid-integer",
    option,
    value
);
fl_string!(
    error_config_invalid_boolean,
    "error-config-invalid-boolean",
    option,
    value
);
fl_string!(
    error_could_not_open_store_for_lock,
    "error-could-not-open-store-for-lock",
    path
);
fl_string!(
    error_could_not_acquire_store_lock,
    "error-could-not-acquire-store-lock",
    path
);
fl_string!(error_cannot_encrypt_store, "error-cannot-encrypt-store");
fl_string!(
    error_failed_to_run_command,
    "error-failed-to-run-command",
    binary
);
fl_string!(
    error_command_exited_with,
    "error-command-exited-with",
    binary,
    exit
);
fl_string!(
    error_browser_host_requires_pipe,
    "error-browser-host-requires-pipe"
);
fl_string!(
    error_browser_install_manifest_no_browser,
    "error-browser-install-manifest-no-browser"
);
fl_string!(
    error_browser_install_manifest_needs_extension_id,
    "error-browser-install-manifest-needs-extension-id",
    browser
);
fl_string!(
    error_browser_install_manifest_extension_id_multiple_browsers,
    "error-browser-install-manifest-extension-id-multiple-browsers"
);
fl_string!(
    error_browser_install_manifest_unsupported_os,
    "error-browser-install-manifest-unsupported-os",
    os
);
fl_string!(
    error_browser_install_manifest_no_home_dir,
    "error-browser-install-manifest-no-home-dir"
);
fl_string!(
    error_browser_install_manifest_no_data_dir,
    "error-browser-install-manifest-no-data-dir"
);
fl_string!(
    error_browser_install_manifest_no_current_exe,
    "error-browser-install-manifest-no-current-exe"
);
fl_string!(
    error_browser_install_manifest_invalid_env,
    "error-browser-install-manifest-invalid-env",
    entry
);
fl_string!(
    error_browser_install_manifest_invalid_env_name,
    "error-browser-install-manifest-invalid-env-name",
    name
);

fl_log_path!(
    warn,
    browser_install_manifest_print_wrapper_note,
    "browser-install-manifest-print-wrapper-note",
    path
);

pub fn browser_install_manifest_written(browser: &str, path: &Path) {
    let path = path_string(path);
    info!(
        "{}",
        fl!(
            LANGUAGE_LOADER,
            "browser-install-manifest-written",
            browser = browser,
            path = path.as_str()
        )
    );
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashMap};

    use super::{parse_posix_locale, resolve_languages};

    /// Build an environment lookup from `(var, value)` pairs.
    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + use<> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        move |var: &str| map.get(var).cloned()
    }

    fn tags(langs: &[unic_langid::LanguageIdentifier]) -> Vec<String> {
        langs.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn parse_posix_locale_strips_codeset_and_modifier() {
        assert_eq!(
            parse_posix_locale("de_DE.UTF-8@euro").unwrap().to_string(),
            "de-DE"
        );
        assert_eq!(
            parse_posix_locale("en_US.UTF-8").unwrap().to_string(),
            "en-US"
        );
        assert_eq!(parse_posix_locale("ja").unwrap().to_string(), "ja");
    }

    #[test]
    fn parse_posix_locale_treats_c_posix_and_empty_as_no_locale() {
        assert!(parse_posix_locale("C").is_none());
        assert!(parse_posix_locale("posix").is_none());
        assert!(parse_posix_locale("").is_none());
        assert!(parse_posix_locale("   ").is_none());
    }

    #[test]
    fn resolve_languages_language_var_wins_and_splits_on_colon() {
        let langs = resolve_languages(env(&[("LANGUAGE", "de:fr"), ("LC_ALL", "es")])).unwrap();
        assert_eq!(tags(&langs), ["de", "fr"]);
    }

    #[test]
    fn resolve_languages_lc_all_beats_lc_messages_and_lang() {
        let langs = resolve_languages(env(&[
            ("LC_ALL", "es"),
            ("LC_MESSAGES", "fr"),
            ("LANG", "de"),
        ]))
        .unwrap();
        assert_eq!(tags(&langs), ["es"]);
    }

    #[test]
    fn resolve_languages_falls_through_to_lang_when_higher_vars_unset() {
        let langs = resolve_languages(env(&[("LANG", "ko_KR.UTF-8")])).unwrap();
        assert_eq!(tags(&langs), ["ko-KR"]);
    }

    #[test]
    fn resolve_languages_returns_none_when_nothing_is_set() {
        // Signals the caller to fall back to the OS-native requester.
        assert!(resolve_languages(env(&[])).is_none());
    }

    #[test]
    fn resolve_languages_returns_some_empty_for_a_set_but_c_locale() {
        // `LANG=C` is *set*, so resolution stops here and yields an empty
        // list (English fallback) rather than consulting the OS requester.
        let langs = resolve_languages(env(&[("LANG", "C")])).unwrap();
        assert!(langs.is_empty());
    }

    /// Every shipped language bundle, embedded at compile time so the
    /// coverage check needs no filesystem access at test time.
    const BUNDLES: &[(&str, &str)] = &[
        ("en", include_str!("../../i18n/en/pasejo.ftl")),
        ("de", include_str!("../../i18n/de/pasejo.ftl")),
        ("es", include_str!("../../i18n/es/pasejo.ftl")),
        ("fr", include_str!("../../i18n/fr/pasejo.ftl")),
        ("it", include_str!("../../i18n/it/pasejo.ftl")),
        ("ja", include_str!("../../i18n/ja/pasejo.ftl")),
        ("ko", include_str!("../../i18n/ko/pasejo.ftl")),
        ("zh", include_str!("../../i18n/zh/pasejo.ftl")),
    ];

    /// This source file, so the test can confirm every message has a wrapper.
    const I18N_SRC: &str = include_str!("i18n.rs");

    /// Extract the Fluent message ids from a `.ftl` bundle. Message ids start
    /// in column 0 with an ASCII letter and are followed by ` =`; comments
    /// (`#`), terms (`-…`), and indented attributes/continuations are not
    /// message ids and are skipped.
    fn message_ids(bundle: &str) -> BTreeSet<String> {
        bundle
            .lines()
            .filter(|line| line.starts_with(|c: char| c.is_ascii_alphabetic()))
            .filter_map(|line| line.split_once(" ="))
            .map(|(id, _)| id.trim().to_string())
            .filter(|id| id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
            .collect()
    }

    fn english_ids() -> BTreeSet<String> {
        message_ids(BUNDLES[0].1)
    }

    #[test]
    fn every_language_defines_exactly_the_english_key_set() {
        // English is the fallback, so a key it has but another bundle lacks
        // silently falls back to English at runtime — the localization hole
        // this guards against. A key a translation has but English lacks is
        // an orphan (usually a typo'd id). Both are failures here, turning a
        // silent runtime fallback into a build failure.
        let english = english_ids();
        assert!(!english.is_empty(), "English bundle has no message ids");
        for (lang, bundle) in &BUNDLES[1..] {
            let theirs = message_ids(bundle);
            let missing: Vec<_> = english.difference(&theirs).collect();
            let orphan: Vec<_> = theirs.difference(&english).collect();
            assert!(
                missing.is_empty() && orphan.is_empty(),
                "`{lang}` key set diverges from English — missing: {missing:?}, orphan: {orphan:?}"
            );
        }
    }

    #[test]
    fn every_english_message_has_a_wrapper_in_this_module() {
        // Each user-facing message is reached only through a `fl_*!` wrapper
        // (or a hand-written `fl!` call) that names its id as a string
        // literal. A key with no such reference is dead weight; flag it so
        // the catalogue and the code stay in step.
        for id in english_ids() {
            let quoted = format!("\"{id}\"");
            assert!(
                I18N_SRC.contains(&quoted),
                "message id `{id}` has no wrapper referencing it in cli/i18n.rs"
            );
        }
    }
}
