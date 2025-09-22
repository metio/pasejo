// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::cli::{clipboard, logs, prompts};
use crate::hooks::executor::HookExecutor;
use crate::models::configuration::Configuration;
use crate::secrets;
use anyhow::Context;
use passwords::{analyzer, scorer};
use std::time::Duration;

pub fn add(
    configuration: &Configuration,
    store_name: Option<&String>,
    secret_path: &str,
    force: bool,
    multiline: bool,
    offline: bool,
) -> anyhow::Result<()> {
    if let Some(registration) = configuration.select_store(store_name) {
        let hooks = HookExecutor {
            configuration,
            registration,
            offline,
            force: false,
        };

        hooks.execute_pull_commands()?;

        let mut store = configuration
            .decrypt_store(registration)
            .context("Cannot decrypt store")?;

        if store.secrets.contains_key(secret_path)
            && !force
            && !prompts::get_confirmation_from_user("Overwrite existing secret?")?
        {
            anyhow::bail!("Secret already exists at {secret_path}. Use --force to overwrite.");
        }

        let secret = &prompts::read_secret_from_user_input(secret_path, multiline)?;

        store.secrets.insert(secret_path.to_owned(), secret.clone());

        Configuration::encrypt_store(registration, &store).context("Cannot encrypt store")?;

        logs::secret_added(secret_path);

        hooks.execute_push_commands()?;

        Ok(())
    } else {
        anyhow::bail!(
            "No store found in configuration. Run 'pasejo store add ...' first to add one"
        )
    }
}

pub fn audit(
    configuration: &Configuration,
    store_name: Option<&String>,
    secret_path: Option<&String>,
    offline: bool,
) -> anyhow::Result<()> {
    if let Some(registration) = configuration.select_store(store_name) {
        let hooks = HookExecutor {
            configuration,
            registration,
            offline,
            force: false,
        };

        hooks.execute_pull_commands()?;

        let store = configuration
            .decrypt_store(registration)
            .context("Cannot decrypt store")?;

        if let Some(secret_path) = secret_path {
            let value = store
                .secrets
                .get(secret_path)
                .ok_or_else(|| anyhow::anyhow!("No secret found at '{secret_path}'"))?;
            let password_strength = scorer::score(&analyzer::analyze(value));
            println!("{secret_path}: {password_strength}/100");
        } else {
            for (key, value) in store.secrets {
                let password_strength = scorer::score(&analyzer::analyze(value));
                println!("{key}: {password_strength}/100");
            }
        }

        Ok(())
    } else {
        anyhow::bail!(
            "No store found in configuration. Run 'pasejo store add ...' first to add one"
        )
    }
}

pub fn copy(
    configuration: &Configuration,
    store_name: Option<&String>,
    force: bool,
    source_path: &str,
    target_path: &str,
    offline: bool,
) -> anyhow::Result<()> {
    if let Some(registration) = configuration.select_store(store_name) {
        let hooks = HookExecutor {
            configuration,
            registration,
            offline,
            force: false,
        };

        hooks.execute_pull_commands()?;

        let mut store = configuration
            .decrypt_store(registration)
            .context("Cannot decrypt store")?;

        if store.secrets.contains_key(target_path)
            && !force
            && !prompts::get_confirmation_from_user("Overwrite existing secret?")?
        {
            anyhow::bail!("Secret already exists at {target_path}. Use --force to overwrite.");
        }

        if let Some(secret) = store.secrets.get(source_path) {
            store
                .secrets
                .insert(target_path.to_owned(), secret.to_owned());
            Configuration::encrypt_store(registration, &store).context("Cannot encrypt store")?;

            logs::secret_copied(source_path, target_path);

            hooks.execute_push_commands()?;

            Ok(())
        } else {
            anyhow::bail!("No secret found at '{source_path}'")
        }
    } else {
        anyhow::bail!(
            "No store found in configuration. Run 'pasejo store add ...' first to add one"
        )
    }
}

pub fn mv(
    configuration: &Configuration,
    store_name: Option<&String>,
    force: bool,
    current_path: &str,
    new_path: &str,
    offline: bool,
) -> anyhow::Result<()> {
    if let Some(registration) = configuration.select_store(store_name) {
        let hooks = HookExecutor {
            configuration,
            registration,
            offline,
            force: false,
        };

        hooks.execute_pull_commands()?;

        let mut store = configuration
            .decrypt_store(registration)
            .context("Cannot decrypt store")?;

        if store.secrets.contains_key(new_path)
            && !force
            && !prompts::get_confirmation_from_user("Overwrite existing secret?")?
        {
            anyhow::bail!("Secret already exists at {new_path}. Use --force to overwrite.");
        }

        if let Some(secret) = store.secrets.remove(current_path) {
            store.secrets.insert(new_path.to_owned(), secret);
            Configuration::encrypt_store(registration, &store).context("Cannot encrypt store")?;

            logs::secret_moved(current_path, new_path);

            hooks.execute_push_commands()?;

            Ok(())
        } else {
            anyhow::bail!("No secret found at '{current_path}'")
        }
    } else {
        anyhow::bail!(
            "No store found in configuration. Run 'pasejo store add ...' first to add one"
        )
    }
}

pub fn list(
    configuration: &Configuration,
    store_name: Option<&String>,
    tree: bool,
    offline: bool,
) -> anyhow::Result<()> {
    if let Some(registration) = configuration.select_store(store_name) {
        let hooks = HookExecutor {
            configuration,
            registration,
            offline,
            force: false,
        };

        hooks.execute_pull_commands()?;

        let store = configuration
            .decrypt_store(registration)
            .context("Cannot decrypt store")?;

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
        Ok(())
    } else {
        anyhow::bail!(
            "No store found in configuration. Run 'pasejo store add ...' first to add one"
        )
    }
}

pub fn remove(
    configuration: &Configuration,
    store_name: Option<&String>,
    force: bool,
    secret_path: &str,
    offline: bool,
) -> anyhow::Result<()> {
    if let Some(registration) = configuration.select_store(store_name) {
        let hooks = HookExecutor {
            configuration,
            registration,
            offline,
            force: false,
        };

        hooks.execute_pull_commands()?;

        let mut store = configuration
            .decrypt_store(registration)
            .context("Cannot decrypt store")?;

        if store.secrets.contains_key(secret_path)
            && !force
            && !prompts::get_confirmation_from_user("Remove existing secret?")?
        {
            anyhow::bail!(
                "Not allowed to remove secret at {secret_path}. Use --force to overwrite."
            );
        }

        if store.secrets.remove(secret_path).is_some() {
            Configuration::encrypt_store(registration, &store).context("Cannot encrypt store")?;

            logs::secret_removed(secret_path);

            hooks.execute_push_commands()?;

            Ok(())
        } else {
            anyhow::bail!("No secret found at '{secret_path}'")
        }
    } else {
        anyhow::bail!(
            "No store found in configuration. Run 'pasejo store add ...' first to add one"
        )
    }
}

pub fn show(
    configuration: &Configuration,
    store_name: Option<&String>,
    qrcode: bool,
    secret_path: &str,
    line: Option<isize>,
    clip: bool,
    offline: bool,
) -> anyhow::Result<()> {
    if let Some(registration) = configuration.select_store(store_name) {
        let hooks = HookExecutor {
            configuration,
            registration,
            offline,
            force: false,
        };

        hooks.execute_pull_commands()?;

        let store = configuration
            .decrypt_store(registration)
            .context("Cannot decrypt store")?;

        if let Some(decrypted_text) = store.secrets.get(secret_path) {
            let text_to_show = line.map_or_else(
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

            if qrcode {
                logs::secret_show_as_qrcode(secret_path);
                qr2term::print_qr(&text_to_show)?;
            } else if clip {
                let duration = Duration::from_secs(configuration.clipboard_timeout.unwrap_or(45));
                logs::secret_copy_into_clipboard(secret_path, &duration);
                clipboard::copy_text_to_clipboard(&text_to_show, duration)?;
            } else {
                logs::secret_show_as_text(secret_path);
                println!("{text_to_show}");
            }
            Ok(())
        } else {
            anyhow::bail!("No secret found at '{secret_path}'")
        }
    } else {
        anyhow::bail!(
            "No store found in configuration. Run 'pasejo store add ...' first to add one"
        )
    }
}

#[allow(clippy::fn_params_excessive_bools)]
#[allow(clippy::too_many_arguments)]
pub fn generate(
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
    if let Some(registration) = configuration.select_store(store_name) {
        let hooks = HookExecutor {
            configuration,
            registration,
            offline,
            force: false,
        };

        hooks.execute_pull_commands()?;

        let mut store = configuration
            .decrypt_store(registration)
            .context("Cannot decrypt store")?;

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

        Configuration::encrypt_store(registration, &store).context("Cannot encrypt store")?;

        logs::secret_generated(secret_path);

        hooks.execute_push_commands()?;

        Ok(())
    } else {
        anyhow::bail!(
            "No store found in configuration. Run 'pasejo store add ...' first to add one"
        )
    }
}

pub fn edit(
    configuration: &Configuration,
    store_name: Option<&String>,
    secret_path: &str,
    offline: bool,
) -> anyhow::Result<()> {
    if let Some(registration) = configuration.select_store(store_name) {
        let hooks = HookExecutor {
            configuration,
            registration,
            offline,
            force: false,
        };

        hooks.execute_pull_commands()?;

        let mut store = configuration
            .decrypt_store(registration)
            .context("Cannot decrypt store")?;

        if let Some(current_value) = store.secrets.get(secret_path) {
            let secret = &prompts::edit_secret(secret_path, current_value)?;

            store.secrets.insert(secret_path.to_owned(), secret.clone());

            Configuration::encrypt_store(registration, &store).context("Cannot encrypt store")?;

            logs::secret_edited(secret_path);

            hooks.execute_push_commands()?;

            Ok(())
        } else {
            anyhow::bail!(
                "Secret does not exist at {secret_path}. Use 'pasejo secret add' to create it."
            );
        }
    } else {
        anyhow::bail!(
            "No store found in configuration. Run 'pasejo store add ...' first to add one"
        )
    }
}

pub fn grep(
    configuration: &Configuration,
    store_name: Option<&String>,
    search_string: &String,
    regex: bool,
    offline: bool,
) -> anyhow::Result<()> {
    if let Some(registration) = configuration.select_store(store_name) {
        let hooks = HookExecutor {
            configuration,
            registration,
            offline,
            force: false,
        };

        hooks.execute_pull_commands()?;

        let store = configuration
            .decrypt_store(registration)
            .context("Cannot decrypt store")?;

        if regex {
            let re = regex::Regex::new(search_string)?;
            for (key, value) in &store.secrets {
                if re.is_match(value) {
                    println!("{key}:\n{value}");
                }
            }
        } else {
            for (key, value) in &store.secrets {
                if value.contains(search_string) {
                    println!("{key}:\n{value}");
                }
            }
        }

        Ok(())
    } else {
        anyhow::bail!(
            "No store found in configuration. Run 'pasejo store add ...' first to add one"
        )
    }
}
