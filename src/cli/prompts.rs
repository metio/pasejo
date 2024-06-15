// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::io::{BufReader, IsTerminal, Read, stdin};

use anyhow::Context;
use inquire::{Confirm, Editor, Password};

pub fn read_secret_from_user_input(secret_path: &str, multiline: bool) -> anyhow::Result<String> {
    let secret = if stdin().is_terminal() {
        let message = &format!("Enter secret for {secret_path}:");
        let failure = format!("Could not read secret for {secret_path}:");
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

pub fn edit_secret(secret_path: &str, current_value: &str) -> anyhow::Result<String> {
    Editor::new(&format!("Enter secret for {secret_path}:"))
        .with_predefined_text(current_value)
        .prompt()
        .context(format!("Could not read secret for {secret_path}:"))
}

pub fn get_confirmation_from_user(message: &str) -> anyhow::Result<bool> {
    if stdin().is_terminal() {
        let confirmation = Confirm::new(message)
            .with_default(false)
            .prompt()
            .context("Cannot get user confirmation")?;
        Ok(confirmation)
    } else {
        anyhow::bail!(
            "Cannot get user confirmation from non-terminal input. Use --force to skip confirmation."
        )
    }
}
