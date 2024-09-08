use std::ffi::OsStr;
use std::fs::{write, DirEntry};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::{fs, io};

use age::cli_common::{read_identities, read_recipients, StdinGuard};
use age::{Decryptor, Encryptor, Recipient};
use anyhow::Context;
use inquire::{Confirm, Editor, Password};
use termtree::Tree;

use crate::adapters::file_system;
use crate::commands::recipients;
use crate::models::configuration::{Identity, Store};

pub fn insert(
    store: &Store,
    multiline: bool,
    force: bool,
    inherit: bool,
    secret_path: &String,
    recipients: &Vec<String>,
) -> anyhow::Result<()> {
    let secret = read_secret_from_user_input(secret_path, multiline)?;
    let (absolute_recipients_path, absolute_secret_path) =
        calculate_paths(store, inherit && recipients.is_empty(), secret_path)?;
    if !recipients.is_empty() {
        replace_recipients(&absolute_recipients_path, recipients, force)?;
    }
    let recipients_from_file = read_recipient_file(&absolute_recipients_path)?;
    encrypt_secret(&secret, &absolute_secret_path, recipients_from_file)?;

    Ok(())
}

fn replace_recipients(
    absolute_recipients_path: &Path,
    recipients: &Vec<String>,
    force: bool,
) -> anyhow::Result<()> {
    if absolute_recipients_path.is_file() {
        if force {
            fs::remove_file(absolute_recipients_path)?;
            write_recipients(absolute_recipients_path, recipients)?;
            println!("Replaced .recipients file");
        } else {
            let replace_recipients = Confirm::new("Replace existing recipients?")
                .with_default(true)
                .with_help_message("Recipients will be taken from --recipient if confirmed")
                .prompt()
                .context("Could not get user answer")?;
            if replace_recipients {
                fs::remove_file(absolute_recipients_path)?;
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
        let formatted_recipient = match recipient.split_once(',') {
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
        fs::create_dir_all(parent)?;
    }
    Ok((absolute_recipients_path, absolute_secret_path))
}

fn encrypt_secret(
    secret: &str,
    absolute_path: &Path,
    recipients: Vec<Box<dyn Recipient + Send>>,
) -> anyhow::Result<()> {
    let Some(encryptor) = Encryptor::with_recipients(recipients) else {
        unreachable!()
    };
    let mut encrypted = vec![];
    let mut writer = encryptor.wrap_output(&mut encrypted)?;
    writer.write_all(secret.as_bytes())?;
    writer.finish()?;
    write(absolute_path, encrypted)?;
    Ok(())
}

fn read_secret_from_user_input(secret_path: &String, multiline: bool) -> anyhow::Result<String> {
    let message = &format!("Enter secret for {secret_path}:");
    let failure = format!("Could not read secret for {secret_path}:");
    let secret = if multiline {
        Editor::new(message).prompt().context(failure)?
    } else {
        Password::new(message).prompt().context(failure)?
    };
    Ok(secret)
}

pub fn show(store: &Store, identities: &[Identity], secret_path: &String) -> anyhow::Result<()> {
    let (_, absolute_secret_path) = calculate_paths(store, false, secret_path)?;
    let encrypted = fs::read(&absolute_secret_path)?;
    let Decryptor::Recipients(decryptor) = Decryptor::new_buffered(&encrypted[..])? else {
        unreachable!()
    };
    let mut decrypted = vec![];
    let parsed_identities = read_all_identities(identities)?;
    let mut reader = decryptor.decrypt(parsed_identities.iter().map(std::ops::Deref::deref))?;
    reader.read_to_end(&mut decrypted)?;
    let decrypted_text = String::from_utf8(decrypted)?;
    println!("{decrypted_text}");
    Ok(())
}

fn read_all_identities(identities: &[Identity]) -> anyhow::Result<Vec<Box<dyn age::Identity>>> {
    let filenames: Vec<String> = identities.iter().map(|i| i.file.clone()).collect();
    let parsed_identities = read_identities(filenames, None, &mut StdinGuard::new(true))?;
    Ok(parsed_identities)
}

pub fn list(store: &Store) -> anyhow::Result<()> {
    let output = tree(Some(store.name.clone()), PathBuf::from(&store.path))?;
    print!("{output}");
    Ok(())
}

fn tree<P: AsRef<Path>>(store_name: Option<String>, path: P) -> io::Result<Tree<String>> {
    let mut entries: Vec<DirEntry> = fs::read_dir(&path)?.filter_map(Result::ok).collect();
    entries.sort_by_key(DirEntry::path);
    let result = entries.iter().fold(
        Tree::new(store_name.unwrap_or_else(|| {
            path.as_ref()
                .file_name()
                .and_then(OsStr::to_str)
                .map_or_else(String::new, String::from)
        })),
        |mut root, entry| {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_dir() {
                    if let Ok(subtree) = tree(None, entry.path()) {
                        root.push(subtree);
                    }
                } else if let Some(filename) = entry.path().file_name().and_then(OsStr::to_str) {
                    if Path::new(filename)
                        .extension()
                        .map_or(false, |ext| ext.eq_ignore_ascii_case("age"))
                    {
                        if let Some((secret_name, _)) = filename.rsplit_once('.') {
                            root.push(Tree::new(String::from(secret_name)));
                        }
                    }
                }
            }
            root
        },
    );
    Ok(result)
}
