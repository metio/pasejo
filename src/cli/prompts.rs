use std::io::{stdin, BufReader, IsTerminal, Read};

use anyhow::Context;
use inquire::{Editor, Password};

pub fn read_secret_from_user_input(
    secret_path: &str,
    multiline: bool,
    secret_type: &str,
) -> anyhow::Result<String> {
    let secret = if stdin().is_terminal() {
        let message = &format!("Enter {secret_type} for {secret_path}:");
        let failure = format!("Could not read {secret_type} for {secret_path}:");
        if multiline {
            Editor::new(message).prompt().context(failure)?
        } else {
            Password::new(message).prompt().context(failure)?
        }
    } else {
        let mut str = String::new();
        BufReader::new(stdin().lock()).read_to_string(&mut str)?;
        str
    };
    Ok(secret)
}
