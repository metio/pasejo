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
    force: bool,
    inherit: bool,
    multiline: bool,
    secret_path: &String,
    recipients: &Vec<String>,
) -> anyhow::Result<()> {
    let secret = &prompts::read_secret_from_user_input(secret_path, multiline, "secret")?;
    let mut absolute_recipients_path =
        store.find_nearest_existing_recipients(secret_path, inherit)?;
    let absolute_secret_path = store.resolve_secret_path(secret_path);
    if let Some(parent) = absolute_secret_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut files_to_commit: Vec<&Path> = vec![&absolute_secret_path];
    if !recipients.is_empty() {
        absolute_recipients_path = store.resolve_recipients_path(secret_path);
        recipients::replace(&absolute_recipients_path, recipients, force)?;
        files_to_commit.push(&absolute_recipients_path);
    }
    let recipients_from_file = recipients::files::read(&absolute_recipients_path)?;
    secrets::encrypt(secret, &absolute_secret_path, &recipients_from_file)?;

    store
        .vcs()
        .commit(files_to_commit, &format!("Added secret '{secret_path}'"))?;

    Ok(())
}

pub fn show(
    store: &Store,
    identity_files: Vec<String>,
    qrcode: bool,
    secret_path: &String,
) -> anyhow::Result<()> {
    let absolute_secret_path = store.resolve_secret_path(secret_path);
    let encrypted = fs::read(&absolute_secret_path)?;
    let decryptor = Decryptor::new_buffered(&encrypted[..])?;
    let mut decrypted = vec![];
    let parsed_identities = identities::read(identity_files)?;
    let mut reader = decryptor.decrypt(parsed_identities.iter().map(std::ops::Deref::deref))?;
    reader.read_to_end(&mut decrypted)?;
    let decrypted_text = String::from_utf8(decrypted)?;
    if qrcode {
        qr2term::print_qr(decrypted_text)?;
    } else {
        println!("{decrypted_text}");
    }
    Ok(())
}

pub fn list(store: &Store, tree: bool) -> anyhow::Result<()> {
    if tree {
        let output = file_system::file_tree(store.name.clone(), PathBuf::from(&store.path), "age")?;
        print!("{output}");
    } else {
        let secrets = file_system::file_list(&store.name, PathBuf::from(&store.path), "age")?;
        for secret in &secrets {
            println!("{secret}");
        }
    }
    Ok(())
}

pub fn mv(store: &Store, current_path: &String, new_path: &String) -> anyhow::Result<()> {
    if store.secret_at_path_exists(Path::new(current_path)) {
        let current_absolute_secret_path = store.resolve_secret_path(current_path);
        let new_absolute_secret_path = store.resolve_secret_path(new_path);

        if let Some(parent) = new_absolute_secret_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::rename(&current_absolute_secret_path, &new_absolute_secret_path)?;

        let mut files_to_commit: Vec<&Path> =
            vec![&current_absolute_secret_path, &new_absolute_secret_path];

        let current_absolute_recipients_path = store.resolve_recipients_path(current_path);
        let new_absolute_recipients_path = store.resolve_recipients_path(new_path);

        if current_absolute_recipients_path.is_file() {
            files_to_commit.push(current_absolute_recipients_path.as_path());
            files_to_commit.push(new_absolute_recipients_path.as_path());
            fs::rename(
                &current_absolute_recipients_path,
                &new_absolute_recipients_path,
            )?;
        }

        store.vcs().commit(
            files_to_commit,
            &format!("Moved secret from '{current_path}' to '{new_path}'"),
        )?;
        Ok(())
    } else {
        anyhow::bail!("No secret exists at '{current_path}'")
    }
}
