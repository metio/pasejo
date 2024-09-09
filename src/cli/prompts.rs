use anyhow::Context;
use inquire::{Editor, Password};

pub fn read_secret_from_user_input(
    secret_path: &String,
    multiline: bool,
) -> anyhow::Result<String> {
    let message = &format!("Enter secret for {secret_path}:");
    let failure = format!("Could not read secret for {secret_path}:");
    let secret = if multiline {
        Editor::new(message).prompt().context(failure)?
    } else {
        Password::new(message).prompt().context(failure)?
    };
    Ok(secret)
}
