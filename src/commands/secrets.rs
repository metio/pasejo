use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use age::Decryptor;

use crate::adapters::file_system;
use crate::cli::prompts;
use crate::models::configuration::Store;
use crate::{identities, recipients, secrets};

pub fn insert(
    store: &Store,
    multiline: bool,
    force: bool,
    inherit: bool,
    secret_path: &String,
    recipients: &Vec<String>,
) -> anyhow::Result<()> {
    let secret = prompts::read_secret_from_user_input(secret_path, multiline)?;
    let absolute_recipients_path = store.find_nearest_existing_recipients(secret_path, inherit)?;
    let absolute_secret_path = store.resolve_secret_path(secret_path);
    if let Some(parent) = absolute_secret_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut files_to_commit: Vec<&Path> = vec![&absolute_secret_path];
    if !recipients.is_empty() {
        recipients::replace(&absolute_recipients_path, recipients, force)?;
        files_to_commit.push(&absolute_recipients_path);
    }
    let recipients_from_file = recipients::files::read(&absolute_recipients_path)?;
    secrets::encrypt(&secret, &absolute_secret_path, recipients_from_file)?;

    store
        .vcs
        .select_implementation(PathBuf::from(&store.path))
        .commit(files_to_commit, &format!("Added secret '{secret_path}'"))?;

    Ok(())
}

pub fn show(
    store: &Store,
    identity_files: Vec<String>,
    secret_path: &String,
) -> anyhow::Result<()> {
    let absolute_secret_path = store.resolve_secret_path(secret_path);
    let encrypted = fs::read(&absolute_secret_path)?;
    let Decryptor::Recipients(decryptor) = Decryptor::new_buffered(&encrypted[..])? else {
        unreachable!()
    };
    let mut decrypted = vec![];
    let parsed_identities = identities::read(identity_files)?;
    let mut reader = decryptor.decrypt(parsed_identities.iter().map(std::ops::Deref::deref))?;
    reader.read_to_end(&mut decrypted)?;
    let decrypted_text = String::from_utf8(decrypted)?;
    println!("{decrypted_text}");
    Ok(())
}

pub fn list(store: &Store) -> anyhow::Result<()> {
    let output = file_system::file_tree(store.name.clone(), PathBuf::from(&store.path), "age")?;
    print!("{output}");
    Ok(())
}
