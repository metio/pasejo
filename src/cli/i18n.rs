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
pub(crate) fn init_for_tests() {
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
fl_string!(
    error_recipient_not_found_in_store,
    "error-recipient-not-found-in-store"
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
