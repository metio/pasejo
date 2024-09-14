use std::fs;
use std::path::Path;

use anyhow::Context;
use inquire::Confirm;

use crate::adapters::file_system;
use crate::cli::logs;
use crate::recipients::format;

pub fn recipients(
    absolute_recipients_path: &Path,
    recipients: &Vec<String>,
    force: bool,
) -> anyhow::Result<()> {
    if absolute_recipients_path.is_file() {
        if force {
            fs::remove_file(absolute_recipients_path)?;
            file_system::append_file(absolute_recipients_path, &format::recipients(recipients))?;
            logs::recipients_file_replaced(absolute_recipients_path);
        } else {
            let replace_recipients = Confirm::new("Replace existing recipients?")
                .with_default(false)
                .with_help_message("Recipients will be taken from --recipient if confirmed")
                .prompt()
                .context("Could not get user answer")?;
            if replace_recipients {
                fs::remove_file(absolute_recipients_path)?;
                file_system::append_file(
                    absolute_recipients_path,
                    &format::recipients(recipients),
                )?;
                logs::recipients_file_replaced(absolute_recipients_path);
            } else {
                logs::recipients_file_use_existing(absolute_recipients_path);
            }
        }
    } else {
        file_system::append_file(absolute_recipients_path, &format::recipients(recipients))?;
        logs::recipients_file_created(absolute_recipients_path);
    }
    Ok(())
}
