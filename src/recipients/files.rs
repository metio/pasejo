use std::path::{Path, PathBuf};

use age::cli_common::{read_recipients, StdinGuard};
use age::Recipient;

use crate::adapters::file_system;
use crate::cli::constants;

pub fn for_secret_path(secret_path: &Option<PathBuf>, path_is_directory: bool) -> PathBuf {
    secret_path.clone().map_or_else(
        || PathBuf::from(constants::RECIPIENTS_DOT_EXTENSION),
        |p| {
            if path_is_directory {
                p.join(constants::RECIPIENTS_DOT_EXTENSION)
            } else {
                file_system::append_file_extension(p, constants::RECIPIENTS_FILE_EXTENSION)
            }
        },
    )
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
        let path = Some(PathBuf::from("secret-name"));
        let file = for_secret_path(&path, false);
        assert_eq!(file, PathBuf::from("secret-name.age-recipients"));
    }

    #[test]
    fn recipients_file_for_secret_directory() {
        let path = Some(PathBuf::from("some/folder"));
        let file = for_secret_path(&path, true);
        assert_eq!(file, PathBuf::from("some/folder/.age-recipients"));
    }

    #[test]
    fn recipients_file_for_empty_secret() {
        let file = for_secret_path(&None, true);
        assert_eq!(file, PathBuf::from(".age-recipients"));
    }
}
