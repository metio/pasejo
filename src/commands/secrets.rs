// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::cli::{logs, prompts};
use crate::models::configuration::Configuration;
use crate::secrets;
use anyhow::Context;
use notify_rust::{Notification, Timeout};
use std::path::Path;
use std::thread;
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
        let store_path = Path::new(&registration.path);
        let synchronizer = registration.synchronizer.select_implementation(store_path);

        if !offline {
            logs::store_sync_pull(&registration.name);
            synchronizer.pull()?;
        }

        let mut store = configuration
            .decrypt_store(registration)
            .context("Cannot decrypt store")?;

        if store.secrets.contains_key(secret_path)
            && !force
            && prompts::get_confirmation_from_user("Overwrite existing secret?")?
        {
            anyhow::bail!("Secret already exists at {secret_path}. Use --force to overwrite.");
        }

        let secret = &prompts::read_secret_from_user_input(secret_path, multiline)?;

        store.secrets.insert(secret_path.to_owned(), secret.clone());

        Configuration::encrypt_store(registration, &store).context("Cannot encrypt store")?;

        if !offline {
            logs::store_sync_push(&registration.name);
            synchronizer.push()?;
        }

        logs::secret_inserted(secret_path);
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
        let store_path = Path::new(&registration.path);
        let synchronizer = registration.synchronizer.select_implementation(store_path);

        if !offline {
            logs::store_sync_pull(&registration.name);
            synchronizer.pull()?;
        }

        let mut store = configuration
            .decrypt_store(registration)
            .context("Cannot decrypt store")?;

        if store.secrets.contains_key(target_path)
            && !force
            && prompts::get_confirmation_from_user("Overwrite existing secret?")?
        {
            anyhow::bail!("Secret already exists at {target_path}. Use --force to overwrite.");
        }

        if let Some(secret) = store.secrets.get(source_path) {
            store
                .secrets
                .insert(target_path.to_owned(), secret.to_owned());
            Configuration::encrypt_store(registration, &store).context("Cannot encrypt store")?;

            if !offline {
                logs::store_sync_push(&registration.name);
                synchronizer.push()?;
            }

            // logs::secret_moved(current_path, new_path);
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
        let store_path = Path::new(&registration.path);
        let synchronizer = registration.synchronizer.select_implementation(store_path);

        if !offline {
            logs::store_sync_pull(&registration.name);
            synchronizer.pull()?;
        }

        let mut store = configuration
            .decrypt_store(registration)
            .context("Cannot decrypt store")?;

        if store.secrets.contains_key(new_path)
            && !force
            && prompts::get_confirmation_from_user("Overwrite existing secret?")?
        {
            anyhow::bail!("Secret already exists at {new_path}. Use --force to overwrite.");
        }

        if let Some(secret) = store.secrets.remove(current_path) {
            store.secrets.insert(new_path.to_owned(), secret);
            Configuration::encrypt_store(registration, &store).context("Cannot encrypt store")?;

            if !offline {
                logs::store_sync_push(&registration.name);
                synchronizer.push()?;
            }

            // logs::secret_moved(current_path, new_path);
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
        let store_path = Path::new(&registration.path);
        let synchronizer = registration.synchronizer.select_implementation(store_path);

        if !offline {
            logs::store_sync_pull(&registration.name);
            synchronizer.pull()?;
        }

        let store = configuration
            .decrypt_store(registration)
            .context("Cannot decrypt store")?;

        if tree {
            print!(
                "{}",
                secrets::format_as_tree("", &store.secret_names_as_list())
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
        let store_path = Path::new(&registration.path);
        let synchronizer = registration.synchronizer.select_implementation(store_path);

        if !offline {
            logs::store_sync_pull(&registration.name);
            synchronizer.pull()?;
        }

        let mut store = configuration
            .decrypt_store(registration)
            .context("Cannot decrypt store")?;

        if store.secrets.contains_key(secret_path)
            && !force
            && prompts::get_confirmation_from_user("Remove existing secret?")?
        {
            anyhow::bail!(
                "Not allowed to remove secret at {secret_path}. Use --force to overwrite."
            );
        }

        if store.secrets.remove(secret_path).is_some() {
            Configuration::encrypt_store(registration, &store).context("Cannot encrypt store")?;

            if !offline {
                synchronizer.push()?;
            }

            // logs::secret_removed(current_path, new_path);
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
        let store_path = Path::new(&registration.path);
        let synchronizer = registration.synchronizer.select_implementation(store_path);

        if !offline {
            logs::store_sync_pull(&registration.name);
            synchronizer.pull()?;
        }

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
            } else {
                logs::secret_show_as_text(secret_path);
                println!("{text_to_show}");
            }
            if clip {
                // logs::secret_show_as_clipboard(secret_path);
                let mut clipboard = arboard::Clipboard::new()?;
                clipboard.set_text(text_to_show)?;
                thread::sleep(Duration::from_secs(
                    configuration.clipboard_timeout.unwrap_or(45),
                ));
                clipboard.clear()?;
                Notification::new()
                    .summary("pasejo")
                    .body("Clipboard cleared")
                    .timeout(Timeout::Default)
                    .show()?;
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
        let store_path = Path::new(&registration.path);
        let synchronizer = registration.synchronizer.select_implementation(store_path);

        if !offline {
            logs::store_sync_pull(&registration.name);
            synchronizer.pull()?;
        }

        let mut store = configuration
            .decrypt_store(registration)
            .context("Cannot decrypt store")?;

        if store.secrets.contains_key(secret_path)
            && !force
            && !inplace
            && prompts::get_confirmation_from_user("Overwrite existing secret?")?
        {
            anyhow::bail!(
                "Secret already exists at {secret_path}. Use --force to overwrite or --inplace to its first line in-place."
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

        if !offline {
            logs::store_sync_push(&registration.name);
            synchronizer.push()?;
        }

        logs::secret_generated(secret_path);
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
        let store_path = Path::new(&registration.path);
        let synchronizer = registration.synchronizer.select_implementation(store_path);

        if !offline {
            logs::store_sync_pull(&registration.name);
            synchronizer.pull()?;
        }

        let mut store = configuration
            .decrypt_store(registration)
            .context("Cannot decrypt store")?;

        if let Some(current_value) = store.secrets.get(secret_path) {
            let secret = &prompts::edit_secret(secret_path, current_value)?;

            store.secrets.insert(secret_path.to_owned(), secret.clone());

            Configuration::encrypt_store(registration, &store).context("Cannot encrypt store")?;

            if !offline {
                logs::store_sync_push(&registration.name);
                synchronizer.push()?;
            }

            logs::secret_inserted(secret_path);
            Ok(())
        } else {
            anyhow::bail!(
                "Secret does not exist at {secret_path}. Use 'pasejo secret insert' to create it."
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
        let store_path = Path::new(&registration.path);
        let synchronizer = registration.synchronizer.select_implementation(store_path);

        if !offline {
            logs::store_sync_pull(&registration.name);
            synchronizer.pull()?;
        }

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
