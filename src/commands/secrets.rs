// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::time::Duration;

use zeroize::Zeroizing;

use crate::cli::{clipboard, i18n, prompts};
use crate::commands::environment;
use crate::commands::store_op::{
    StoreMutation, with_store, with_store_fetching, with_store_fetching_then,
};
use crate::models::cli::SecretCommands;
use crate::models::configuration::{
    Configuration, DEFAULT_CLIPBOARD_NOTIFY, DEFAULT_CLIPBOARD_TIMEOUT_SECONDS,
};
use crate::remote::funnel::Fetch;
use crate::secrets;
use crate::secrets::selection::Selection;

pub fn dispatch(
    command: &SecretCommands,
    configuration: &Configuration,
    offline: bool,
) -> anyhow::Result<()> {
    match command {
        SecretCommands::Add(args) => add(
            configuration,
            args.store_selection.store.as_ref(),
            &args.secret_path,
            args.force,
            args.multiline,
            offline,
        ),
        SecretCommands::Audit(args) => audit(
            configuration,
            args.store_selection.store.as_ref(),
            args.secret_path.as_ref(),
            offline,
        ),
        SecretCommands::Copy(args) => copy(
            configuration,
            args.store_selection.store.as_ref(),
            args.force,
            &args.source_path,
            &args.target_path,
            offline,
        ),
        SecretCommands::List(args) => list(
            configuration,
            args.store_selection.store.as_ref(),
            args.tree,
            offline,
        ),
        SecretCommands::Resolve(args) => environment::resolve_from_stdin(
            configuration,
            args.store_selection.store.as_ref(),
            offline,
        ),
        SecretCommands::Show(args) => show(
            configuration,
            args.store_selection.store.as_ref(),
            args.qrcode,
            &args.secret_path,
            &Selection::from_args(args.line, args.skip_lines, args.field.as_deref()),
            args.clip,
            offline,
        ),
        SecretCommands::Move(args) => mv(
            configuration,
            args.store_selection.store.as_ref(),
            args.force,
            &args.current_path,
            &args.new_path,
            offline,
        ),
        SecretCommands::Remove(args) => remove(
            configuration,
            args.store_selection.store.as_ref(),
            args.force,
            &args.secret_path,
            offline,
        ),
        SecretCommands::Generate(args) => generate(
            configuration,
            args.store_selection.store.as_ref(),
            &args.secret_path,
            args.force,
            args.inplace,
            args.length,
            args.include_numbers(),
            args.include_lowercase_letters(),
            args.include_uppercase_letters(),
            args.symbols,
            args.spaces,
            args.excludes_similar_characters(),
            args.is_strict(),
            offline,
        ),
        SecretCommands::Edit(args) => edit(
            configuration,
            args.store_selection.store.as_ref(),
            &args.secret_path,
            offline,
        ),
        SecretCommands::Grep(args) => grep(
            configuration,
            args.store_selection.store.as_ref(),
            &args.search_string,
            args.regex,
            args.show_values,
            offline,
        ),
    }
}

/// Scrubs a plaintext secret displaced by a `BTreeMap::insert`. Every
/// overwrite path (add/copy/mv/edit/generate) hands the displaced
/// value — typically the old password being rotated — through here so
/// it is zeroized rather than dropped as a plain `String`. Call as
/// `scrub_displaced(store.secrets.insert(k, v))`: the `insert` is a
/// method call, so its receiver borrow is two-phase and the value
/// expression (e.g. a `remainder.join` that still reads the store) is
/// allowed during argument evaluation.
fn scrub_displaced(displaced: Option<String>) {
    drop(displaced.map(Zeroizing::new));
}

fn add(
    configuration: &Configuration,
    store_name: Option<&String>,
    secret_path: &str,
    force: bool,
    multiline: bool,
    offline: bool,
) -> anyhow::Result<()> {
    with_store_fetching(
        configuration,
        store_name,
        offline,
        &Fetch::secrets([secret_path]),
        |_, store| {
            if store.secrets.contains_key(secret_path)
                && !force
                && !prompts::get_confirmation_from_user(&i18n::prompt_overwrite_secret())?
            {
                anyhow::bail!(i18n::error_secret_already_exists(secret_path));
            }
            let secret = prompts::read_secret_from_user_input(secret_path, multiline)?;
            scrub_displaced(store.secrets.insert(secret_path.to_owned(), secret));
            i18n::secret_added(secret_path);
            Ok(((), StoreMutation::Modified))
        },
    )
}

fn audit(
    configuration: &Configuration,
    store_name: Option<&String>,
    secret_path: Option<&String>,
    offline: bool,
) -> anyhow::Result<()> {
    with_store(configuration, store_name, offline, |_, store| {
        if let Some(secret_path) = secret_path {
            let value = store
                .secrets
                .get(secret_path)
                .ok_or_else(|| anyhow::anyhow!(i18n::error_no_secret_found(secret_path)))?;
            i18n::password_strength(secret_path, audit_score(value));
        } else {
            for (key, value) in &store.secrets {
                i18n::password_strength(key, audit_score(value));
            }
        }
        Ok(((), StoreMutation::Unchanged))
    })
}

fn audit_score(value: &str) -> f64 {
    use passwords::{analyzer, scorer};
    let password = value
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or_default();
    scorer::score(&analyzer::analyze(password))
}

fn copy(
    configuration: &Configuration,
    store_name: Option<&String>,
    force: bool,
    source_path: &str,
    target_path: &str,
    offline: bool,
) -> anyhow::Result<()> {
    with_store_fetching(
        configuration,
        store_name,
        offline,
        &Fetch::secrets([source_path, target_path]),
        |_, store| {
            if store.secrets.contains_key(target_path)
                && !force
                && !prompts::get_confirmation_from_user(&i18n::prompt_overwrite_secret())?
            {
                anyhow::bail!(i18n::error_secret_already_exists(target_path));
            }
            let Some(secret) = store.secrets.get(source_path) else {
                anyhow::bail!(i18n::error_no_secret_found(source_path))
            };
            scrub_displaced(
                store
                    .secrets
                    .insert(target_path.to_owned(), secret.to_owned()),
            );
            i18n::secret_copied(source_path, target_path);
            Ok(((), StoreMutation::Modified))
        },
    )
}

fn mv(
    configuration: &Configuration,
    store_name: Option<&String>,
    force: bool,
    current_path: &str,
    new_path: &str,
    offline: bool,
) -> anyhow::Result<()> {
    with_store_fetching(
        configuration,
        store_name,
        offline,
        &Fetch::secrets([current_path, new_path]),
        |_, store| {
            if store.secrets.contains_key(new_path)
                && !force
                && !prompts::get_confirmation_from_user(&i18n::prompt_overwrite_secret())?
            {
                anyhow::bail!(i18n::error_secret_already_exists(new_path));
            }
            let Some(secret) = store.secrets.remove(current_path) else {
                anyhow::bail!(i18n::error_no_secret_found(current_path))
            };
            scrub_displaced(store.secrets.insert(new_path.to_owned(), secret));
            i18n::secret_moved(current_path, new_path);
            Ok(((), StoreMutation::Modified))
        },
    )
}

fn list(
    configuration: &Configuration,
    store_name: Option<&String>,
    tree: bool,
    offline: bool,
) -> anyhow::Result<()> {
    with_store_fetching(
        configuration,
        store_name,
        offline,
        &Fetch::Names,
        |registration, store| {
            if tree {
                print!(
                    "{}",
                    secrets::format_as_tree(&registration.name, &store.secret_names_as_list())
                );
            } else {
                for secret in store.secret_names_as_list() {
                    println!("{secret}");
                }
            }
            Ok(((), StoreMutation::Unchanged))
        },
    )
}

fn remove(
    configuration: &Configuration,
    store_name: Option<&String>,
    force: bool,
    secret_path: &str,
    offline: bool,
) -> anyhow::Result<()> {
    with_store_fetching(
        configuration,
        store_name,
        offline,
        &Fetch::Names,
        |_, store| {
            if store.secrets.contains_key(secret_path)
                && !force
                && !prompts::get_confirmation_from_user(&i18n::prompt_remove_secret())?
            {
                anyhow::bail!(i18n::error_not_allowed_to_remove_secret(secret_path));
            }
            let Some(removed) = store.secrets.remove(secret_path) else {
                anyhow::bail!(i18n::error_no_secret_found(secret_path))
            };
            drop(Zeroizing::new(removed));
            i18n::secret_removed(secret_path);
            Ok(((), StoreMutation::Modified))
        },
    )
}

fn show(
    configuration: &Configuration,
    store_name: Option<&String>,
    qrcode: bool,
    secret_path: &str,
    selection: &Selection,
    clip: bool,
    offline: bool,
) -> anyhow::Result<()> {
    with_store_fetching_then(
        configuration,
        store_name,
        offline,
        &Fetch::secrets([secret_path]),
        |_, store| {
            let Some(decrypted_text) = store.secrets.get(secret_path) else {
                anyhow::bail!(i18n::error_no_secret_found(secret_path))
            };
            let text = match (selection.extract(decrypted_text), selection) {
                (Some(text), _) => text,
                (None, Selection::Field(key)) => {
                    anyhow::bail!(i18n::error_no_field_found(secret_path, key))
                }
                // A line outside the secret shows as empty, like an empty line.
                (None, _) => Zeroizing::new(String::new()),
            };
            Ok((text, StoreMutation::Unchanged))
        },
        |text_to_show: &Zeroizing<String>| {
            if qrcode {
                i18n::secret_show_as_qrcode(secret_path);
                qr2term::print_qr(text_to_show.as_str())?;
            } else if clip {
                let duration = Duration::from_secs(
                    configuration
                        .clipboard_timeout
                        .unwrap_or(DEFAULT_CLIPBOARD_TIMEOUT_SECONDS),
                );
                let notify = configuration
                    .clipboard_notify
                    .unwrap_or(DEFAULT_CLIPBOARD_NOTIFY);
                i18n::secret_copy_into_clipboard(secret_path, &duration);
                clipboard::copy_text_to_clipboard(text_to_show.as_str(), duration, notify)?;
            } else {
                i18n::secret_show_as_text(secret_path);
                println!("{}", text_to_show.as_str());
            }
            Ok(())
        },
    )?;
    Ok(())
}

#[allow(clippy::fn_params_excessive_bools)]
#[allow(clippy::too_many_arguments)]
fn generate(
    configuration: &Configuration,
    store_name: Option<&String>,
    secret_path: &str,
    force: bool,
    inplace: bool,
    length: usize,
    numbers: bool,
    lowercase_letters: bool,
    uppercase_letters: bool,
    symbols: bool,
    spaces: bool,
    exclude_similar_characters: bool,
    strict: bool,
    offline: bool,
) -> anyhow::Result<()> {
    with_store_fetching(
        configuration,
        store_name,
        offline,
        &Fetch::secrets([secret_path]),
        |_, store| {
            if store.secrets.contains_key(secret_path)
                && !force
                && !inplace
                && !prompts::get_confirmation_from_user(&i18n::prompt_overwrite_secret())?
            {
                anyhow::bail!(i18n::error_secret_already_exists_overwrite_or_inplace(
                    secret_path
                ));
            }

            let generator = passwords::PasswordGenerator {
                length,
                numbers,
                lowercase_letters,
                uppercase_letters,
                symbols,
                spaces,
                exclude_similar_characters,
                strict,
            };
            let secret = generator
                .generate_one()
                .map_err(|reason| anyhow::anyhow!(localize_generator_error(reason)))?;

            if inplace {
                if let Some(current_value) = store.secrets.get(secret_path) {
                    let secret = Zeroizing::new(secret);
                    let mut remainder = current_value.lines().skip(1).collect::<Vec<&str>>();
                    remainder.splice(..0, vec![secret.as_str()]);
                    scrub_displaced(
                        store
                            .secrets
                            .insert(secret_path.to_owned(), remainder.join("\n")),
                    );
                } else {
                    scrub_displaced(store.secrets.insert(secret_path.to_owned(), secret));
                }
            } else {
                scrub_displaced(store.secrets.insert(secret_path.to_owned(), secret));
            }

            i18n::secret_generated(secret_path);
            Ok(((), StoreMutation::Modified))
        },
    )
}

/// Translates the fixed English error strings of the `passwords` generator.
fn localize_generator_error(reason: &str) -> String {
    match reason {
        "The length of passwords cannot be 0." => i18n::error_password_length_zero(),
        "The length of passwords is too short." => i18n::error_password_length_too_short(),
        "You need to enable at least one kind of characters." => {
            i18n::error_password_no_character_kinds()
        }
        _ => i18n::error_password_generation_failed(reason),
    }
}

fn edit(
    configuration: &Configuration,
    store_name: Option<&String>,
    secret_path: &str,
    offline: bool,
) -> anyhow::Result<()> {
    with_store_fetching(
        configuration,
        store_name,
        offline,
        &Fetch::secrets([secret_path]),
        |_, store| {
            let Some(current_value) = store.secrets.get(secret_path) else {
                anyhow::bail!(i18n::error_secret_does_not_exist_for_edit(secret_path));
            };
            let secret = prompts::edit_secret(secret_path, current_value)?;
            scrub_displaced(store.secrets.insert(secret_path.to_owned(), secret));
            i18n::secret_edited(secret_path);
            Ok(((), StoreMutation::Modified))
        },
    )
}

enum Pattern<'a> {
    Substring(&'a str),
    Regex(regex::Regex),
}

impl<'a> Pattern<'a> {
    fn new(needle: &'a str, use_regex: bool) -> anyhow::Result<Self> {
        if use_regex {
            Ok(Self::Regex(regex::Regex::new(needle)?))
        } else {
            Ok(Self::Substring(needle))
        }
    }

    fn matches(&self, value: &str) -> bool {
        match self {
            Self::Substring(needle) => value.contains(needle),
            Self::Regex(re) => re.is_match(value),
        }
    }
}

fn grep(
    configuration: &Configuration,
    store_name: Option<&String>,
    search_string: &str,
    regex: bool,
    show_values: bool,
    offline: bool,
) -> anyhow::Result<()> {
    with_store(configuration, store_name, offline, |_, store| {
        let pattern = Pattern::new(search_string, regex)?;
        for (key, value) in &store.secrets {
            if pattern.matches(value) {
                if show_values {
                    i18n::secret_search_match(key, value);
                } else {
                    println!("{key}");
                }
            }
        }
        Ok(((), StoreMutation::Unchanged))
    })
}

#[cfg(test)]
mod tests {
    use super::{Pattern, audit_score, localize_generator_error};

    #[test]
    fn localize_generator_error_covers_every_generator_error() {
        crate::cli::i18n::init_for_tests();
        for options in [
            passwords::PasswordGenerator::new().length(0),
            passwords::PasswordGenerator::new()
                .length(2)
                .numbers(true)
                .lowercase_letters(true)
                .uppercase_letters(true)
                .strict(true),
            passwords::PasswordGenerator::new()
                .numbers(false)
                .lowercase_letters(false)
                .uppercase_letters(false)
                .symbols(false)
                .spaces(false),
        ] {
            let reason = options.generate_one().unwrap_err();
            assert!(
                !localize_generator_error(reason).contains(reason),
                "untranslated generator error: {reason}"
            );
        }
    }

    #[test]
    fn audit_score_ignores_metadata_lines() {
        assert!(
            (audit_score("hunter2\nlogin: alice\nurl: https://example.com")
                - audit_score("hunter2"))
            .abs()
                < f64::EPSILON
        );
    }

    #[test]
    fn audit_score_uses_the_first_non_blank_line() {
        assert!((audit_score("\n  \nhunter2\nnote") - audit_score("hunter2")).abs() < f64::EPSILON);
    }

    #[test]
    fn pattern_substring_matches_when_value_contains_needle() {
        let pattern = Pattern::new("foo", false).unwrap();
        assert!(pattern.matches("hello foo world"));
        assert!(pattern.matches("foo"));
        assert!(pattern.matches("foobar"));
    }

    #[test]
    fn pattern_substring_does_not_match_when_value_lacks_needle() {
        let pattern = Pattern::new("foo", false).unwrap();
        assert!(!pattern.matches("bar"));
        assert!(!pattern.matches(""));
    }

    #[test]
    fn pattern_substring_is_case_sensitive() {
        let pattern = Pattern::new("Foo", false).unwrap();
        assert!(pattern.matches("Foobar"));
        assert!(!pattern.matches("foobar"));
    }

    #[test]
    fn pattern_substring_treats_regex_metacharacters_as_literal() {
        let pattern = Pattern::new("[abc].*", false).unwrap();
        assert!(pattern.matches("prefix [abc].* suffix"));
        assert!(!pattern.matches("a"));
    }

    #[test]
    fn pattern_substring_with_empty_needle_matches_everything() {
        let pattern = Pattern::new("", false).unwrap();
        assert!(pattern.matches("anything"));
        assert!(pattern.matches(""));
    }

    #[test]
    fn pattern_regex_matches_when_value_matches() {
        let pattern = Pattern::new(r"^foo\d+$", true).unwrap();
        assert!(pattern.matches("foo123"));
        assert!(pattern.matches("foo0"));
    }

    #[test]
    fn pattern_regex_does_not_match_when_value_does_not_match() {
        let pattern = Pattern::new(r"^foo\d+$", true).unwrap();
        assert!(!pattern.matches("foo"));
        assert!(!pattern.matches("foobar"));
        assert!(!pattern.matches("123foo"));
    }

    #[test]
    fn pattern_regex_is_case_sensitive_by_default() {
        let pattern = Pattern::new("foo", true).unwrap();
        assert!(pattern.matches("foobar"));
        assert!(!pattern.matches("FOOBAR"));
    }

    #[test]
    fn pattern_regex_dot_matches_any_char() {
        let pattern = Pattern::new("f.o", true).unwrap();
        assert!(pattern.matches("foo"));
        assert!(pattern.matches("fXo"));
        assert!(!pattern.matches("fo"));
    }

    #[test]
    fn pattern_regex_invalid_returns_error() {
        assert!(Pattern::new("[unclosed", true).is_err());
        assert!(Pattern::new("(", true).is_err());
    }

    #[test]
    fn pattern_substring_with_invalid_regex_chars_does_not_error() {
        // In substring mode regex syntax is irrelevant — every input is a
        // literal needle.
        assert!(Pattern::new("[unclosed", false).is_ok());
        assert!(Pattern::new("(", false).is_ok());
    }
}
