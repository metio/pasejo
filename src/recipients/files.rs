// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::path::{Path, PathBuf};

use age::Recipient;
use age::cli_common::{StdinGuard, read_recipients};

use crate::cli::constants;

pub fn for_secret_path(secret_path: Option<&PathBuf>, path_is_directory: bool) -> PathBuf {
    secret_path.map_or_else(
        || PathBuf::from(constants::RECIPIENTS_DOT_EXTENSION),
        |p| {
            if path_is_directory {
                p.join(constants::RECIPIENTS_DOT_EXTENSION)
            } else {
                for_secret(p)
            }
        },
    )
}

pub fn for_secret(secret_path: &Path) -> PathBuf {
    secret_path.with_extension(constants::RECIPIENTS_FILE_EXTENSION)
}

pub fn secret_of(recipients_file: &Path) -> PathBuf {
    recipients_file.with_extension(constants::SECRET_FILE_EXTENSION)
}

pub fn read(recipients_file: &Path) -> anyhow::Result<Vec<Box<dyn Recipient + Send>>> {
    let recipients = read_recipients(
        vec![],
        vec![recipients_file.display().to_string()],
        vec![],
        None,
        &mut StdinGuard::new(true),
    )?;
    Ok(recipients)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipients_file_for_secret_file() {
        let path = Some(PathBuf::from("secret-name.age"));
        let file = for_secret_path(path.as_ref(), false);
        assert_eq!(file, PathBuf::from("secret-name.age-recipients"));
    }

    #[test]
    fn recipients_file_for_secret_directory() {
        let path = Some(PathBuf::from("some/folder"));
        let file = for_secret_path(path.as_ref(), true);
        assert_eq!(file, PathBuf::from("some/folder/.age-recipients"));
    }

    #[test]
    fn recipients_file_for_empty_secret() {
        let file = for_secret_path(None, true);
        assert_eq!(file, PathBuf::from(".age-recipients"));
    }

    #[test]
    fn recipients_file_for_secret() {
        let path = PathBuf::from("some/name.age");
        let file = for_secret(path.as_path());
        assert_eq!(file, PathBuf::from("some/name.age-recipients"));
    }
}
