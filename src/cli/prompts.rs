// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::io::{BufReader, IsTerminal, Read, stdin};

use anyhow::Context;
use inquire::{Confirm, Password};

use crate::cli::{editor, i18n};

pub fn read_secret_from_user_input(secret_path: &str, multiline: bool) -> anyhow::Result<String> {
    let secret = if stdin().is_terminal() {
        let failure = i18n::prompt_could_not_read_secret(secret_path);
        if multiline {
            edit_in_editor("").context(failure)?
        } else {
            Password::new(&i18n::prompt_enter_secret(secret_path))
                .prompt()
                .context(failure)?
        }
    } else {
        let mut str = String::new();
        BufReader::new(stdin().lock()).read_to_string(&mut str)?;
        str
    };
    Ok(secret)
}

pub fn edit_secret(secret_path: &str, current_value: &str) -> anyhow::Result<String> {
    edit_in_editor(current_value).context(i18n::prompt_could_not_read_secret(secret_path))
}

/// Moves the edited text out of its `Zeroizing` wrapper without copying, so
/// the only plaintext copy is the one handed to the store.
fn edit_in_editor(initial: &str) -> anyhow::Result<String> {
    let mut edited = editor::edit(initial)?;
    Ok(std::mem::take(&mut *edited))
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
