// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::io::{BufReader, IsTerminal, Read, stdin};

use anyhow::Context;
use inquire::{Confirm, Editor, Password};

use crate::cli::i18n;

pub fn read_secret_from_user_input(secret_path: &str, multiline: bool) -> anyhow::Result<String> {
    let secret = if stdin().is_terminal() {
        let message = i18n::prompt_enter_secret(secret_path);
        let failure = i18n::prompt_could_not_read_secret(secret_path);
        if multiline {
            Editor::new(&message).prompt().context(failure)?
        } else {
            Password::new(&message).prompt().context(failure)?
        }
    } else {
        let mut str = String::new();
        BufReader::new(stdin().lock()).read_to_string(&mut str)?;
        str
    };
    Ok(secret)
}

pub fn edit_secret(secret_path: &str, current_value: &str) -> anyhow::Result<String> {
    Editor::new(&i18n::prompt_enter_secret(secret_path))
        .with_predefined_text(current_value)
        .prompt()
        .context(i18n::prompt_could_not_read_secret(secret_path))
}

pub fn get_confirmation_from_user(message: &str) -> anyhow::Result<bool> {
    if stdin().is_terminal() {
        let confirmation = Confirm::new(message)
            .with_default(false)
            .prompt()
            .context(i18n::error_cannot_get_user_confirmation())?;
        Ok(confirmation)
    } else {
        anyhow::bail!(i18n::error_no_confirmation_from_non_terminal())
    }
}
