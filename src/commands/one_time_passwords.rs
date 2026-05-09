// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::cli::{clipboard, i18n, prompts};
use crate::commands::store_op::{StoreMutation, with_store, with_store_then};
use crate::models::cli::{Cli, OtpCommands};
use crate::models::configuration::Configuration;
use crate::models::password_store::{OneTimePassword, OneTimePasswordType, PasswordStore};
use crate::one_time_passwords::parser::parse_otp_args;
use crate::secrets;
use std::time::Duration;
use zeroize::Zeroizing;

pub fn dispatch(
    command: &OtpCommands,
    cli: &Cli,
    configuration: &Configuration,
) -> anyhow::Result<()> {
    match command {
        OtpCommands::Add(args) => add(
            configuration,
            args.store_selection.store.as_ref(),
            &args.password_path,
            &parse_otp_args(
                args.otp_type.as_ref(),
                args.algorithm.as_ref(),
                args.secret.as_ref(),
                args.digits,
                args.period,
                args.counter,
                args.skew,
                args.url.as_ref(),
                args.qrcode.as_ref(),
            )?,
            args.force,
            cli.offline,
        ),
        OtpCommands::Copy(args) => copy(
            configuration,
            args.store_selection.store.as_ref(),
            args.force,
            &args.source_path,
            &args.target_path,
            cli.offline,
        ),
        OtpCommands::List(args) => list(
            configuration,
            args.store_selection.store.as_ref(),
            args.tree,
            cli.offline,
        ),
        OtpCommands::Move(args) => mv(
            configuration,
            args.store_selection.store.as_ref(),
            args.force,
            &args.current_path,
            &args.new_path,
            cli.offline,
        ),
        OtpCommands::Remove(args) => remove(
            configuration,
            args.store_selection.store.as_ref(),
            args.force,
            &args.password_path,
            cli.offline,
        ),
        OtpCommands::Show(args) => show(
            configuration,
            args.store_selection.store.as_ref(),
            &args.password_path,
            args.clip,
            cli.offline,
        ),
    }
}

fn add(
    configuration: &Configuration,
    store_name: Option<&String>,
    password_path: &str,
    password: &OneTimePassword,
    force: bool,
    offline: bool,
) -> anyhow::Result<()> {
    with_store(configuration, store_name, offline, |_, store| {
        let allow_overwrite = !store.otp.contains_key(password_path)
            || force
            || prompts::get_confirmation_from_user("Overwrite existing one-time password?")?;
        let mutation = insert_one_time_password(store, password_path, password, allow_overwrite)?;
        i18n::one_time_password_added(password_path);
        Ok(((), mutation))
    })
}

fn remove(
    configuration: &Configuration,
    store_name: Option<&String>,
    force: bool,
    password_path: &str,
    offline: bool,
) -> anyhow::Result<()> {
    with_store(configuration, store_name, offline, |_, store| {
        let allow_remove = !store.otp.contains_key(password_path)
            || force
            || prompts::get_confirmation_from_user("Remove existing one-time password?")?;
        let mutation = remove_one_time_password(store, password_path, allow_remove)?;
        i18n::one_time_password_removed(password_path);
        Ok(((), mutation))
    })
}

fn list(
    configuration: &Configuration,
    store_name: Option<&String>,
    tree: bool,
    offline: bool,
) -> anyhow::Result<()> {
    with_store(configuration, store_name, offline, |_, store| {
        if tree {
            print!(
                "{}",
                secrets::format_as_tree("", &store.otp_names_as_list())
            );
        } else {
            for secret in store.otp_names_as_list() {
                println!("{secret}");
            }
        }
        Ok(((), StoreMutation::Unchanged))
    })
}

fn show(
    configuration: &Configuration,
    store_name: Option<&String>,
    password_path: &str,
    clip: bool,
    offline: bool,
) -> anyhow::Result<()> {
    with_store_then(
        configuration,
        store_name,
        offline,
        |_, store| {
            let Some(password) = store.otp.get_mut(password_path) else {
                anyhow::bail!("No one-time password found at '{password_path}'")
            };
            let is_hotp = password.otp_type == OneTimePasswordType::Hotp;
            let code = password.generate()?;
            let formatted = format_code(code, password.digits);
            let mutation = if is_hotp {
                StoreMutation::Modified
            } else {
                StoreMutation::Unchanged
            };
            Ok((formatted, mutation))
        },
        |code: &Zeroizing<String>| {
            if clip {
                let duration = Duration::from_secs(configuration.clipboard_timeout.unwrap_or(45));
                let notify = configuration.clipboard_notify.unwrap_or(true);
                i18n::one_time_password_copy_into_clipboard(password_path, &duration);
                clipboard::copy_text_to_clipboard(code.as_str(), duration, notify)?;
            } else {
                i18n::one_time_password_show(password_path);
                println!("{}", code.as_str());
            }
            Ok(())
        },
    )?;
    Ok(())
}

fn mv(
    configuration: &Configuration,
    store_name: Option<&String>,
    force: bool,
    current_path: &str,
    new_path: &str,
    offline: bool,
) -> anyhow::Result<()> {
    with_store(configuration, store_name, offline, |_, store| {
        let allow_overwrite = !store.otp.contains_key(new_path)
            || force
            || prompts::get_confirmation_from_user("Overwrite existing one-time password?")?;
        let mutation = move_one_time_password(store, current_path, new_path, allow_overwrite)?;
        i18n::one_time_password_moved(current_path, new_path);
        Ok(((), mutation))
    })
}

fn copy(
    configuration: &Configuration,
    store_name: Option<&String>,
    force: bool,
    source_path: &str,
    target_path: &str,
    offline: bool,
) -> anyhow::Result<()> {
    with_store(configuration, store_name, offline, |_, store| {
        let allow_overwrite = !store.otp.contains_key(target_path)
            || force
            || prompts::get_confirmation_from_user("Overwrite existing one-time password?")?;
        let mutation = copy_one_time_password(store, source_path, target_path, allow_overwrite)?;
        i18n::one_time_password_copied(source_path, target_path);
        Ok(((), mutation))
    })
}

/// Insert `password` at `password_path`. Bails when a password already
/// exists there and `allow_overwrite` is `false`. The handler computes
/// `allow_overwrite` from `--force` and the interactive prompt; the
/// helper itself is pure so the data-mutation path can be tested without
/// stdin or a real store.
fn insert_one_time_password(
    store: &mut PasswordStore,
    password_path: &str,
    password: &OneTimePassword,
    allow_overwrite: bool,
) -> anyhow::Result<StoreMutation> {
    if store.otp.contains_key(password_path) && !allow_overwrite {
        anyhow::bail!(
            "One-time password already exists at {password_path}. Use --force to overwrite."
        );
    }
    store.otp.insert(password_path.to_owned(), password.clone());
    Ok(StoreMutation::Modified)
}

/// Remove the password at `password_path`. Bails when the entry doesn't
/// exist *or* when it does but `allow_remove` is `false` (the user
/// declined the overwrite prompt). The not-found message is preferred
/// over the not-allowed message when both apply.
fn remove_one_time_password(
    store: &mut PasswordStore,
    password_path: &str,
    allow_remove: bool,
) -> anyhow::Result<StoreMutation> {
    if !store.otp.contains_key(password_path) {
        anyhow::bail!("No one-time password found at '{password_path}'");
    }
    if !allow_remove {
        anyhow::bail!(
            "Not allowed to remove one-time password at {password_path}. Use --force to overwrite."
        );
    }
    store.otp.remove(password_path);
    Ok(StoreMutation::Modified)
}

/// Move `current_path` to `new_path`. Bails when the destination is
/// occupied and `allow_overwrite` is `false`, or when the source is
/// missing. The destination check runs first so a forbidden overwrite
/// never disturbs the source entry.
fn move_one_time_password(
    store: &mut PasswordStore,
    current_path: &str,
    new_path: &str,
    allow_overwrite: bool,
) -> anyhow::Result<StoreMutation> {
    if store.otp.contains_key(new_path) && !allow_overwrite {
        anyhow::bail!(
            "One-time password already exists at {new_path}. Use --force to overwrite."
        );
    }
    let Some(password) = store.otp.remove(current_path) else {
        anyhow::bail!("No one-time password found at '{current_path}'");
    };
    store.otp.insert(new_path.to_owned(), password);
    Ok(StoreMutation::Modified)
}

/// Copy the password at `source_path` to `target_path`, leaving the
/// source intact. Same overwrite/missing semantics as
/// [`move_one_time_password`].
fn copy_one_time_password(
    store: &mut PasswordStore,
    source_path: &str,
    target_path: &str,
    allow_overwrite: bool,
) -> anyhow::Result<StoreMutation> {
    if store.otp.contains_key(target_path) && !allow_overwrite {
        anyhow::bail!(
            "One-time password already exists at {target_path}. Use --force to overwrite."
        );
    }
    let Some(password) = store.otp.get(source_path) else {
        anyhow::bail!("No one-time password found at '{source_path}'");
    };
    let copy = password.clone();
    store.otp.insert(target_path.to_owned(), copy);
    Ok(StoreMutation::Modified)
}

/// Zero-pad an OTP code to the declared digit width.
///
/// `OneTimePassword::generate` returns the raw integer, so a 6-digit code
/// of `54321` would render without its leading zero and be rejected by
/// the verifying server. This pads with `0`s up to `digits`. The caller
/// owns wiping the buffer; we wrap it in `Zeroizing` since the formatted
/// digits are themselves sensitive material.
fn format_code(code: u32, digits: u8) -> Zeroizing<String> {
    Zeroizing::new(format!("{code:0width$}", width = usize::from(digits)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pads_with_leading_zeros_for_six_digits() {
        let formatted = format_code(54321, 6);
        assert_eq!(formatted.as_str(), "054321");
    }

    #[test]
    fn pads_zero_to_full_width() {
        let formatted = format_code(0, 6);
        assert_eq!(formatted.as_str(), "000000");
    }

    #[test]
    fn does_not_pad_when_value_already_fills_width() {
        let formatted = format_code(999_999, 6);
        assert_eq!(formatted.as_str(), "999999");
    }

    #[test]
    fn pads_to_eight_digits_for_eight_digit_otps() {
        let formatted = format_code(12_345, 8);
        assert_eq!(formatted.as_str(), "00012345");
    }

    #[test]
    fn pads_to_four_digits_for_four_digit_otps() {
        let formatted = format_code(7, 4);
        assert_eq!(formatted.as_str(), "0007");
    }

    #[test]
    fn output_length_always_matches_digit_count() {
        // 9 is the largest digit count whose `max_in_range` still fits in
        // a u32; in practice OTPs cap at 8 digits, so this is plenty of
        // headroom. Pushing to 10 would overflow `10u32.pow(10)`.
        for digits in 4..=9u8 {
            let max_in_range = 10u32.pow(u32::from(digits)) - 1;
            for code in [0, 7, max_in_range] {
                let formatted = format_code(code, digits);
                assert_eq!(
                    formatted.len(),
                    usize::from(digits),
                    "code={code}, digits={digits}, formatted={formatted:?}",
                );
            }
        }
    }

    /// Build a distinguishable `OneTimePassword` per period. Tests use
    /// the `period` field as a discriminator so they can assert *which*
    /// password ended up at a given path after a mutation. We build by
    /// mutation rather than `..Default::default()` because
    /// `OneTimePassword` implements `ZeroizeOnDrop`, which forbids moves
    /// out of any of its non-`Copy` fields.
    fn otp_with_period(period: u64) -> OneTimePassword {
        let mut otp = OneTimePassword::default();
        otp.period = period;
        otp
    }

    fn store_with(entries: &[(&str, OneTimePassword)]) -> PasswordStore {
        let mut store = PasswordStore::default();
        for (path, otp) in entries {
            store.otp.insert((*path).to_owned(), otp.clone());
        }
        store
    }

    // ---- insert_one_time_password ---------------------------------------

    #[test]
    fn insert_writes_a_new_entry_into_an_empty_path() {
        let mut store = PasswordStore::default();
        let password = otp_with_period(30);

        let mutation = insert_one_time_password(&mut store, "github", &password, false).unwrap();

        assert!(matches!(mutation, StoreMutation::Modified));
        assert_eq!(store.otp.get("github"), Some(&password));
    }

    #[test]
    fn insert_overwrites_existing_entry_when_allowed() {
        let mut store = store_with(&[("github", otp_with_period(30))]);
        let replacement = otp_with_period(60);

        let mutation = insert_one_time_password(&mut store, "github", &replacement, true).unwrap();

        assert!(matches!(mutation, StoreMutation::Modified));
        assert_eq!(store.otp.get("github"), Some(&replacement));
    }

    #[test]
    fn insert_bails_on_existing_entry_when_overwrite_not_allowed() {
        let original = otp_with_period(30);
        let mut store = store_with(&[("github", original.clone())]);
        let replacement = otp_with_period(60);

        let result = insert_one_time_password(&mut store, "github", &replacement, false);

        assert!(result.is_err());
        // Original entry must be untouched on the bail path.
        assert_eq!(store.otp.get("github"), Some(&original));
    }

    // ---- remove_one_time_password ---------------------------------------

    #[test]
    fn remove_drops_the_entry_when_allowed() {
        let mut store = store_with(&[("github", otp_with_period(30))]);

        let mutation = remove_one_time_password(&mut store, "github", true).unwrap();

        assert!(matches!(mutation, StoreMutation::Modified));
        assert!(!store.otp.contains_key("github"));
    }

    #[test]
    fn remove_bails_when_user_declined_and_keeps_entry() {
        let original = otp_with_period(30);
        let mut store = store_with(&[("github", original.clone())]);

        let result = remove_one_time_password(&mut store, "github", false);

        assert!(result.is_err());
        assert_eq!(store.otp.get("github"), Some(&original));
    }

    #[test]
    fn remove_bails_when_entry_does_not_exist() {
        let mut store = PasswordStore::default();

        let result = remove_one_time_password(&mut store, "missing", true);

        assert!(result.is_err());
        let message = result.unwrap_err().to_string();
        assert!(
            message.contains("No one-time password found"),
            "expected not-found message, got: {message}"
        );
    }

    // ---- move_one_time_password -----------------------------------------

    #[test]
    fn move_relocates_entry_when_destination_is_empty() {
        let original = otp_with_period(30);
        let mut store = store_with(&[("old", original.clone())]);

        let mutation = move_one_time_password(&mut store, "old", "new", false).unwrap();

        assert!(matches!(mutation, StoreMutation::Modified));
        assert!(!store.otp.contains_key("old"));
        assert_eq!(store.otp.get("new"), Some(&original));
    }

    #[test]
    fn move_overwrites_destination_when_allowed() {
        let source = otp_with_period(30);
        let target = otp_with_period(60);
        let mut store = store_with(&[("old", source.clone()), ("new", target)]);

        let mutation = move_one_time_password(&mut store, "old", "new", true).unwrap();

        assert!(matches!(mutation, StoreMutation::Modified));
        assert!(!store.otp.contains_key("old"));
        // Destination now holds the source's value, not the original target.
        assert_eq!(store.otp.get("new"), Some(&source));
    }

    #[test]
    fn move_bails_when_destination_exists_and_overwrite_not_allowed() {
        let source = otp_with_period(30);
        let target = otp_with_period(60);
        let mut store = store_with(&[("old", source.clone()), ("new", target.clone())]);

        let result = move_one_time_password(&mut store, "old", "new", false);

        assert!(result.is_err());
        // Both entries untouched when the overwrite is rejected.
        assert_eq!(store.otp.get("old"), Some(&source));
        assert_eq!(store.otp.get("new"), Some(&target));
    }

    #[test]
    fn move_bails_when_source_is_missing() {
        let mut store = PasswordStore::default();

        let result = move_one_time_password(&mut store, "missing", "new", true);

        assert!(result.is_err());
        let message = result.unwrap_err().to_string();
        assert!(
            message.contains("No one-time password found"),
            "expected not-found message, got: {message}"
        );
    }

    #[test]
    fn move_to_same_path_is_a_noop_when_overwrite_allowed() {
        let original = otp_with_period(30);
        let mut store = store_with(&[("github", original.clone())]);

        let mutation = move_one_time_password(&mut store, "github", "github", true).unwrap();

        assert!(matches!(mutation, StoreMutation::Modified));
        assert_eq!(store.otp.get("github"), Some(&original));
    }

    // ---- copy_one_time_password -----------------------------------------

    #[test]
    fn copy_duplicates_entry_to_new_path() {
        let original = otp_with_period(30);
        let mut store = store_with(&[("source", original.clone())]);

        let mutation = copy_one_time_password(&mut store, "source", "target", false).unwrap();

        assert!(matches!(mutation, StoreMutation::Modified));
        // Both source and target now hold equal values.
        assert_eq!(store.otp.get("source"), Some(&original));
        assert_eq!(store.otp.get("target"), Some(&original));
    }

    #[test]
    fn copy_overwrites_destination_when_allowed() {
        let source = otp_with_period(30);
        let target = otp_with_period(60);
        let mut store = store_with(&[("source", source.clone()), ("target", target)]);

        let mutation = copy_one_time_password(&mut store, "source", "target", true).unwrap();

        assert!(matches!(mutation, StoreMutation::Modified));
        // Source still present, target now mirrors source.
        assert_eq!(store.otp.get("source"), Some(&source));
        assert_eq!(store.otp.get("target"), Some(&source));
    }

    #[test]
    fn copy_bails_when_destination_exists_and_overwrite_not_allowed() {
        let source = otp_with_period(30);
        let target = otp_with_period(60);
        let mut store = store_with(&[("source", source.clone()), ("target", target.clone())]);

        let result = copy_one_time_password(&mut store, "source", "target", false);

        assert!(result.is_err());
        // Both entries untouched on the bail path.
        assert_eq!(store.otp.get("source"), Some(&source));
        assert_eq!(store.otp.get("target"), Some(&target));
    }

    #[test]
    fn copy_bails_when_source_is_missing() {
        let mut store = PasswordStore::default();

        let result = copy_one_time_password(&mut store, "missing", "target", true);

        assert!(result.is_err());
        let message = result.unwrap_err().to_string();
        assert!(
            message.contains("No one-time password found"),
            "expected not-found message, got: {message}"
        );
    }
}
