// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::cli::{clipboard, i18n, prompts};
use crate::commands::store_op::{StoreMutation, with_store, with_store_then};
use crate::models::cli::SecretCommands;
use crate::models::configuration::Configuration;
use crate::secrets;
use std::time::Duration;
use zeroize::Zeroizing;

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
        SecretCommands::Show(args) => show(
            configuration,
            args.store_selection.store.as_ref(),
            args.qrcode,
            &args.secret_path,
            LineSelector::from_args(args.line, args.skip_lines),
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
            args.numbers,
            args.lowercase_letters,
            args.uppercase_letters,
            args.symbols,
            args.spaces,
            args.exclude_similar_characters,
            args.strict,
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
            offline,
        ),
    }
}

fn add(
    configuration: &Configuration,
    store_name: Option<&String>,
    secret_path: &str,
    force: bool,
    multiline: bool,
    offline: bool,
) -> anyhow::Result<()> {
    with_store(configuration, store_name, offline, |_, store| {
        if store.secrets.contains_key(secret_path)
            && !force
            && !prompts::get_confirmation_from_user("Overwrite existing secret?")?
        {
            anyhow::bail!("Secret already exists at {secret_path}. Use --force to overwrite.");
        }
        let secret = prompts::read_secret_from_user_input(secret_path, multiline)?;
        store.secrets.insert(secret_path.to_owned(), secret);
        i18n::secret_added(secret_path);
        Ok(((), StoreMutation::Modified))
    })
}

fn audit(
    configuration: &Configuration,
    store_name: Option<&String>,
    secret_path: Option<&String>,
    offline: bool,
) -> anyhow::Result<()> {
    use passwords::{analyzer, scorer};
    with_store(configuration, store_name, offline, |_, store| {
        if let Some(secret_path) = secret_path {
            let value = store
                .secrets
                .get(secret_path)
                .ok_or_else(|| anyhow::anyhow!("No secret found at '{secret_path}'"))?;
            i18n::password_strength(secret_path, scorer::score(&analyzer::analyze(value)));
        } else {
            for (key, value) in &store.secrets {
                i18n::password_strength(key, scorer::score(&analyzer::analyze(value)));
            }
        }
        Ok(((), StoreMutation::Unchanged))
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
        if store.secrets.contains_key(target_path)
            && !force
            && !prompts::get_confirmation_from_user("Overwrite existing secret?")?
        {
            anyhow::bail!("Secret already exists at {target_path}. Use --force to overwrite.");
        }
        let Some(secret) = store.secrets.get(source_path) else {
            anyhow::bail!("No secret found at '{source_path}'")
        };
        store
            .secrets
            .insert(target_path.to_owned(), secret.to_owned());
        i18n::secret_copied(source_path, target_path);
        Ok(((), StoreMutation::Modified))
    })
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
        if store.secrets.contains_key(new_path)
            && !force
            && !prompts::get_confirmation_from_user("Overwrite existing secret?")?
        {
            anyhow::bail!("Secret already exists at {new_path}. Use --force to overwrite.");
        }
        let Some(secret) = store.secrets.remove(current_path) else {
            anyhow::bail!("No secret found at '{current_path}'")
        };
        store.secrets.insert(new_path.to_owned(), secret);
        i18n::secret_moved(current_path, new_path);
        Ok(((), StoreMutation::Modified))
    })
}

fn list(
    configuration: &Configuration,
    store_name: Option<&String>,
    tree: bool,
    offline: bool,
) -> anyhow::Result<()> {
    with_store(configuration, store_name, offline, |registration, store| {
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
    })
}

fn remove(
    configuration: &Configuration,
    store_name: Option<&String>,
    force: bool,
    secret_path: &str,
    offline: bool,
) -> anyhow::Result<()> {
    with_store(configuration, store_name, offline, |_, store| {
        if store.secrets.contains_key(secret_path)
            && !force
            && !prompts::get_confirmation_from_user("Remove existing secret?")?
        {
            anyhow::bail!(
                "Not allowed to remove secret at {secret_path}. Use --force to overwrite."
            );
        }
        let Some(removed) = store.secrets.remove(secret_path) else {
            anyhow::bail!("No secret found at '{secret_path}'")
        };
        drop(Zeroizing::new(removed));
        i18n::secret_removed(secret_path);
        Ok(((), StoreMutation::Modified))
    })
}

#[derive(Debug, Clone, Copy)]
enum LineSelector {
    All,
    /// 1-indexed line number. Positive counts from the start; negative
    /// from the end. Never 0 (clap rejects 0 at parse time).
    Line(isize),
    /// Skip the first N lines. N > 0 (clap rejects 0).
    SkipFirst(u64),
}

impl LineSelector {
    fn from_args(line: Option<isize>, skip_lines: Option<u64>) -> Self {
        match (line, skip_lines) {
            (Some(n), _) => Self::Line(n),
            (None, Some(n)) => Self::SkipFirst(n),
            (None, None) => Self::All,
        }
    }
}

fn extract_line(decrypted_text: &str, selector: LineSelector) -> Zeroizing<String> {
    let extracted = match selector {
        LineSelector::All => decrypted_text.to_owned(),
        LineSelector::Line(n) if n > 0 => {
            let index = usize::try_from(n).unwrap_or(usize::MAX).saturating_sub(1);
            decrypted_text
                .lines()
                .nth(index)
                .unwrap_or("")
                .to_owned()
        }
        LineSelector::Line(n) => {
            // n < 0 here (clap rejects 0); -1 == last line.
            let lines: Vec<&str> = decrypted_text.lines().collect();
            let from_end = n.unsigned_abs();
            if from_end == 0 || from_end > lines.len() {
                String::new()
            } else {
                (*lines.get(lines.len() - from_end).unwrap_or(&"")).to_owned()
            }
        }
        LineSelector::SkipFirst(n) => decrypted_text
            .lines()
            .skip(usize::try_from(n).unwrap_or(usize::MAX))
            .collect::<Vec<&str>>()
            .join("\n"),
    };
    Zeroizing::new(extracted)
}

fn show(
    configuration: &Configuration,
    store_name: Option<&String>,
    qrcode: bool,
    secret_path: &str,
    selector: LineSelector,
    clip: bool,
    offline: bool,
) -> anyhow::Result<()> {
    with_store_then(
        configuration,
        store_name,
        offline,
        |_, store| {
            let Some(decrypted_text) = store.secrets.get(secret_path) else {
                anyhow::bail!("No secret found at '{secret_path}'")
            };
            Ok((extract_line(decrypted_text, selector), StoreMutation::Unchanged))
        },
        |text_to_show: &Zeroizing<String>| {
            if qrcode {
                i18n::secret_show_as_qrcode(secret_path);
                qr2term::print_qr(text_to_show.as_str())?;
            } else if clip {
                let duration = Duration::from_secs(configuration.clipboard_timeout.unwrap_or(45));
                let notify = configuration.clipboard_notify.unwrap_or(true);
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
    with_store(configuration, store_name, offline, |_, store| {
        if store.secrets.contains_key(secret_path)
            && !force
            && !inplace
            && !prompts::get_confirmation_from_user("Overwrite existing secret?")?
        {
            anyhow::bail!(
                "Secret already exists at {secret_path}. Use --force to overwrite entirely or --inplace to modify its first line in-place."
            );
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
        let secret = generator.generate_one().map_err(anyhow::Error::msg)?;

        if inplace {
            if let Some(current_value) = store.secrets.get(secret_path) {
                let secret = Zeroizing::new(secret);
                let mut remainder = current_value.lines().skip(1).collect::<Vec<&str>>();
                remainder.splice(..0, vec![secret.as_str()]);
                store
                    .secrets
                    .insert(secret_path.to_owned(), remainder.join("\n"));
            } else {
                store.secrets.insert(secret_path.to_owned(), secret);
            }
        } else {
            store.secrets.insert(secret_path.to_owned(), secret);
        }

        i18n::secret_generated(secret_path);
        Ok(((), StoreMutation::Modified))
    })
}

fn edit(
    configuration: &Configuration,
    store_name: Option<&String>,
    secret_path: &str,
    offline: bool,
) -> anyhow::Result<()> {
    with_store(configuration, store_name, offline, |_, store| {
        let Some(current_value) = store.secrets.get(secret_path) else {
            anyhow::bail!(
                "Secret does not exist at {secret_path}. Use 'pasejo secret add' to create it."
            );
        };
        let secret = prompts::edit_secret(secret_path, current_value)?;
        store.secrets.insert(secret_path.to_owned(), secret);
        i18n::secret_edited(secret_path);
        Ok(((), StoreMutation::Modified))
    })
}

fn grep(
    configuration: &Configuration,
    store_name: Option<&String>,
    search_string: &String,
    regex: bool,
    offline: bool,
) -> anyhow::Result<()> {
    with_store(configuration, store_name, offline, |_, store| {
        if regex {
            let re = regex::Regex::new(search_string)?;
            for (key, value) in &store.secrets {
                if re.is_match(value) {
                    i18n::secret_search_match(key, value);
                }
            }
        } else {
            for (key, value) in &store.secrets {
                if value.contains(search_string) {
                    i18n::secret_search_match(key, value);
                }
            }
        }
        Ok(((), StoreMutation::Unchanged))
    })
}

#[cfg(test)]
mod tests {
    use super::{LineSelector, extract_line};

    const THREE_LINE_SECRET: &str = "password\nuser: alice\nurl: https://example.com";

    #[test]
    fn from_args_with_neither_arg_is_all() {
        assert!(matches!(
            LineSelector::from_args(None, None),
            LineSelector::All
        ));
    }

    #[test]
    fn from_args_with_only_line_is_line() {
        assert!(matches!(
            LineSelector::from_args(Some(2), None),
            LineSelector::Line(2)
        ));
    }

    #[test]
    fn from_args_with_only_skip_lines_is_skip_first() {
        assert!(matches!(
            LineSelector::from_args(None, Some(3)),
            LineSelector::SkipFirst(3)
        ));
    }

    #[test]
    fn from_args_prefers_line_when_both_present() {
        // clap enforces mutual exclusion, so this combination cannot arrive at
        // runtime — but the resolver picks `line` as a defensive fallback.
        assert!(matches!(
            LineSelector::from_args(Some(1), Some(2)),
            LineSelector::Line(1)
        ));
    }

    #[test]
    fn all_returns_full_secret() {
        let result = extract_line(THREE_LINE_SECRET, LineSelector::All);
        assert_eq!(result.as_str(), THREE_LINE_SECRET);
    }

    #[test]
    fn line_one_returns_first_line() {
        let result = extract_line(THREE_LINE_SECRET, LineSelector::Line(1));
        assert_eq!(result.as_str(), "password");
    }

    #[test]
    fn line_in_range_returns_that_line() {
        let result = extract_line(THREE_LINE_SECRET, LineSelector::Line(2));
        assert_eq!(result.as_str(), "user: alice");
    }

    #[test]
    fn line_at_end_returns_that_line() {
        let result = extract_line(THREE_LINE_SECRET, LineSelector::Line(3));
        assert_eq!(result.as_str(), "url: https://example.com");
    }

    #[test]
    fn line_just_past_end_returns_empty() {
        let result = extract_line(THREE_LINE_SECRET, LineSelector::Line(4));
        assert_eq!(result.as_str(), "");
    }

    #[test]
    fn line_far_past_end_returns_empty_not_full_secret() {
        let result = extract_line(THREE_LINE_SECRET, LineSelector::Line(99));
        assert_eq!(result.as_str(), "");
    }

    #[test]
    fn line_isize_max_returns_empty_not_full_secret() {
        let result = extract_line(THREE_LINE_SECRET, LineSelector::Line(isize::MAX));
        assert_eq!(result.as_str(), "");
    }

    #[test]
    fn line_minus_one_returns_last_line() {
        let result = extract_line(THREE_LINE_SECRET, LineSelector::Line(-1));
        assert_eq!(result.as_str(), "url: https://example.com");
    }

    #[test]
    fn line_minus_two_returns_second_to_last_line() {
        let result = extract_line(THREE_LINE_SECRET, LineSelector::Line(-2));
        assert_eq!(result.as_str(), "user: alice");
    }

    #[test]
    fn line_negative_equal_to_line_count_returns_first_line() {
        let result = extract_line(THREE_LINE_SECRET, LineSelector::Line(-3));
        assert_eq!(result.as_str(), "password");
    }

    #[test]
    fn line_negative_past_start_returns_empty() {
        let result = extract_line(THREE_LINE_SECRET, LineSelector::Line(-4));
        assert_eq!(result.as_str(), "");
    }

    #[test]
    fn line_isize_min_returns_empty_not_full_secret() {
        let result = extract_line(THREE_LINE_SECRET, LineSelector::Line(isize::MIN));
        assert_eq!(result.as_str(), "");
    }

    #[test]
    fn skip_first_one_returns_everything_after_password() {
        let result = extract_line(THREE_LINE_SECRET, LineSelector::SkipFirst(1));
        assert_eq!(result.as_str(), "user: alice\nurl: https://example.com");
    }

    #[test]
    fn skip_first_two_returns_last_line_only() {
        let result = extract_line(THREE_LINE_SECRET, LineSelector::SkipFirst(2));
        assert_eq!(result.as_str(), "url: https://example.com");
    }

    #[test]
    fn skip_first_equal_to_line_count_returns_empty() {
        let result = extract_line(THREE_LINE_SECRET, LineSelector::SkipFirst(3));
        assert_eq!(result.as_str(), "");
    }

    #[test]
    fn skip_first_past_end_returns_empty() {
        let result = extract_line(THREE_LINE_SECRET, LineSelector::SkipFirst(99));
        assert_eq!(result.as_str(), "");
    }

    #[test]
    fn skip_first_u64_max_returns_empty_not_full_secret() {
        let result = extract_line(THREE_LINE_SECRET, LineSelector::SkipFirst(u64::MAX));
        assert_eq!(result.as_str(), "");
    }

    #[test]
    fn empty_secret_with_all_selector_returns_empty() {
        let result = extract_line("", LineSelector::All);
        assert_eq!(result.as_str(), "");
    }

    #[test]
    fn empty_secret_with_any_line_selector_returns_empty() {
        assert_eq!(extract_line("", LineSelector::Line(1)).as_str(), "");
        assert_eq!(extract_line("", LineSelector::Line(5)).as_str(), "");
        assert_eq!(extract_line("", LineSelector::Line(-1)).as_str(), "");
    }

    #[test]
    fn empty_secret_with_skip_first_returns_empty() {
        let result = extract_line("", LineSelector::SkipFirst(1));
        assert_eq!(result.as_str(), "");
    }

    #[test]
    fn single_line_secret_with_line_one_returns_that_line() {
        let result = extract_line("only-line", LineSelector::Line(1));
        assert_eq!(result.as_str(), "only-line");
    }

    #[test]
    fn single_line_secret_with_line_minus_one_returns_that_line() {
        let result = extract_line("only-line", LineSelector::Line(-1));
        assert_eq!(result.as_str(), "only-line");
    }

    #[test]
    fn single_line_secret_with_line_two_returns_empty() {
        let result = extract_line("only-line", LineSelector::Line(2));
        assert_eq!(result.as_str(), "");
    }

    #[test]
    fn trailing_newline_does_not_count_as_extra_line() {
        let result = extract_line("a\nb\n", LineSelector::Line(3));
        assert_eq!(result.as_str(), "");
    }
}
