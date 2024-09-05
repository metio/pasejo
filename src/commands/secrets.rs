use crate::adapters::file_system;
use crate::commands::recipients;
use crate::models::configuration::Store;
use age::cli_common::{read_recipients, StdinGuard};
use age::{Encryptor, Recipient};
use anyhow::Context;
use inquire::{Confirm, Editor, Password};
use std::fs::write;
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn insert(
    store: &Store,
    multiline: &bool,
    force: &bool,
    inherit: &bool,
    secret_path: &String,
    recipients: &Vec<String>,
) -> anyhow::Result<()> {
    let secret = read_secret_from_user_input(secret_path, *multiline)?;
    let (absolute_recipients_path, absolute_secret_path) =
        calculate_paths(store, *inherit && recipients.is_empty(), secret_path)?;
    if !recipients.is_empty() {
        replace_recipients(&absolute_recipients_path, recipients, *force)?;
    }
    let recipients_from_file = read_recipient_file(&absolute_recipients_path)?;
    encrypt_secret(secret, &absolute_secret_path, recipients_from_file)?;

    Ok(())
}

fn replace_recipients(
    absolute_recipients_path: &Path,
    recipients: &Vec<String>,
    force: bool,
) -> anyhow::Result<()> {
    if file_system::file_exists(absolute_recipients_path)? {
        if force {
            file_system::remove_file(absolute_recipients_path)?;
            write_recipients(absolute_recipients_path, recipients)?;
            println!("Replaced .recipients file");
        } else {
            let replace_recipients = Confirm::new("Replace existing recipients?")
                .with_default(true)
                .with_help_message("Recipients will be taken from --recipient if confirmed")
                .prompt()
                .context("Could not get user answer")?;
            if replace_recipients {
                file_system::remove_file(absolute_recipients_path)?;
                write_recipients(absolute_recipients_path, recipients)?;
                println!("Replaced .recipients file");
            } else {
                println!("Using existing .recipients file");
            }
        }
    } else {
        write_recipients(absolute_recipients_path, recipients)?;
        println!("Created .recipients file");
    }
    Ok(())
}

fn write_recipients(
    absolute_recipients_path: &Path,
    recipients: &Vec<String>,
) -> anyhow::Result<()> {
    for recipient in recipients {
        let formatted_recipient = match recipient.split_once(",") {
            None => recipients::format_recipient(recipient, &None),
            Some((name, key)) => {
                recipients::format_recipient(&key.to_string(), &Some(name.to_string()))
            }
        };
        file_system::append_file(absolute_recipients_path, &formatted_recipient)?;
    }
    Ok(())
}

fn read_recipient_file(recipients_path: &Path) -> anyhow::Result<Vec<Box<dyn Recipient + Send>>> {
    let recipients = read_recipients(
        vec![],
        vec![recipients_path.display().to_string()],
        vec![],
        None,
        &mut StdinGuard::new(true),
    )?;
    Ok(recipients)
}

fn calculate_paths(
    store: &Store,
    inherit: bool,
    secret_path: &String,
) -> anyhow::Result<(PathBuf, PathBuf)> {
    let relative_path = Path::new(secret_path);
    let absolute_recipients_path = store.find_nearest_recipients(relative_path, inherit)?;
    let absolute_secret_path = store.resolve_path(file_system::append_to_path(
        relative_path.to_path_buf(),
        ".age",
    ));
    if let Some(parent) = absolute_secret_path.parent() {
        file_system::mkdir_parents(parent)?;
    }
    Ok((absolute_recipients_path, absolute_secret_path))
}

fn encrypt_secret(
    secret: String,
    absolute_path: &Path,
    recipients: Vec<Box<dyn Recipient + Send>>,
) -> anyhow::Result<()> {
    let encryptor = Encryptor::with_recipients(recipients).expect("No recipients found");
    let mut encrypted = vec![];
    let mut writer = encryptor.wrap_output(&mut encrypted)?;
    writer.write_all(secret.as_bytes())?;
    writer.finish()?;
    write(absolute_path, encrypted)?;
    Ok(())
}

fn read_secret_from_user_input(secret_path: &String, multiline: bool) -> anyhow::Result<String> {
    let message = &format!("Enter secret for {}:", secret_path);
    let failure = format!("Could not read secret for {}:", secret_path);
    let secret = if multiline {
        Editor::new(message).prompt().context(failure)?
    } else {
        Password::new(message).prompt().context(failure)?
    };
    Ok(secret)
}
