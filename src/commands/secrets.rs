use crate::adapters::file_system;
use crate::models::configuration::Store;
use age::cli_common::{read_recipients, StdinGuard};
use age::Encryptor;
use anyhow::Context;
use inquire::{Editor, Password};
use std::fs::write;
use std::io::Write;
use std::path::PathBuf;

pub fn insert(
    store: &Store,
    multiline: &bool,
    _force: &bool,
    inherit: &bool,
    secret_path: &String,
) -> anyhow::Result<()> {
    let message = &format!("Enter secret for {}:", secret_path);
    let failure = format!("Could not read secret for {}:", secret_path);
    let secret = if *multiline {
        Editor::new(message).prompt().context(failure)?
    } else {
        Password::new(message).prompt().context(failure)?
    };

    let (absolute_secret_path, _) = store.paths_for(&Some(PathBuf::from(secret_path)), ".age");
    let recipients_path = store.find_nearest_recipients(PathBuf::from(secret_path), *inherit)?;
    let recipients = read_recipients(
        vec![],
        vec![recipients_path.display().to_string()],
        vec![],
        None,
        &mut StdinGuard::new(true),
    )?;
    let encryptor = Encryptor::with_recipients(recipients).expect("No recipients found");
    if let Some(parent) = absolute_secret_path.parent() {
        file_system::mkdir_parents(parent)?;
    }
    let mut encrypted = vec![];
    let mut writer = encryptor.wrap_output(&mut encrypted)?;
    writer.write_all(secret.as_bytes())?;
    writer.finish()?;
    write(absolute_secret_path, encrypted)?;

    Ok(())
}
