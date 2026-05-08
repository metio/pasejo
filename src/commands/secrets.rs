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
            args.line,
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

fn show(
    configuration: &Configuration,
    store_name: Option<&String>,
    qrcode: bool,
    secret_path: &str,
    line: Option<isize>,
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
            let extracted = line.map_or_else(
                || decrypted_text.to_owned(),
                |line| {
                    if line >= 0 {
                        decrypted_text
                            .lines()
                            .nth(line.unsigned_abs())
                            .unwrap_or(decrypted_text)
                            .to_owned()
                    } else {
                        decrypted_text
                            .lines()
                            .skip(line.unsigned_abs())
                            .collect::<Vec<&str>>()
                            .join("\n")
                    }
                },
            );
            Ok((Zeroizing::new(extracted), StoreMutation::Unchanged))
        },
        |text_to_show: &Zeroizing<String>| {
            if qrcode {
                i18n::secret_show_as_qrcode(secret_path);
                qr2term::print_qr(text_to_show.as_str())?;
            } else if clip {
                let duration = Duration::from_secs(configuration.clipboard_timeout.unwrap_or(45));
                i18n::secret_copy_into_clipboard(secret_path, &duration);
                clipboard::copy_text_to_clipboard(text_to_show.as_str(), duration)?;
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
