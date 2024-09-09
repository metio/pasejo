use std::fs;
use std::path::PathBuf;

use clap::error::ErrorKind;

use crate::adapters::file_system;
use crate::cli::{constants, errors, printer};
use crate::models::configuration::Store;

pub fn add(
    store: &Store,
    public_key: &String,
    name: &Option<String>,
    secret_path: &Option<PathBuf>,
) -> anyhow::Result<()> {
    let path_is_directory = validate_given_path(store, secret_path);

    let (recipients_file_in_store, absolute_path_to_recipients_file) =
        calculate_recipients_file_paths(store, secret_path, path_is_directory);

    if absolute_path_to_recipients_file.is_file() {
        // update existing .recipients file
        let recipients = fs::read_to_string(&absolute_path_to_recipients_file)?;
        let (updated_recipients, _) = upsert_recipient(recipients, public_key, name);
        fs::write(&absolute_path_to_recipients_file, updated_recipients)?;
    } else {
        // create new .recipients file
        let recipient = format_recipient(public_key, name);
        file_system::append_file(&absolute_path_to_recipients_file, &recipient)?;
        printer::recipient_added();
    }

    store.vcs.select_implementation().commit(
        store.path.as_ref(),
        &recipients_file_in_store,
        &format!("Added recipient '{public_key}'"),
    )?;
    Ok(())
}

fn calculate_recipients_file_paths(
    store: &Store,
    secret_path: &Option<PathBuf>,
    path_is_directory: bool,
) -> (PathBuf, PathBuf) {
    let recipients_file_in_store = secret_path.clone().map_or_else(
        || PathBuf::from(constants::RECIPIENTS_DOT_EXTENSION),
        |p| {
            if path_is_directory {
                p.join(constants::RECIPIENTS_DOT_EXTENSION)
            } else {
                file_system::append_file_extension(p, constants::RECIPIENTS_FILE_EXTENSION)
            }
        },
    );
    let absolute_path_to_recipients_file = store.resolve_path(&recipients_file_in_store);
    (recipients_file_in_store, absolute_path_to_recipients_file)
}

fn validate_given_path(store: &Store, path: &Option<PathBuf>) -> bool {
    path.as_ref().map_or(true, |path| {
        let absolute_path_to_secret_file = store.resolve_path(file_system::append_file_extension(
            path.clone(),
            constants::SECRET_FILE_EXTENSION,
        ));
        let absolute_path_to_secret_directory = store.resolve_path(path);
        let file_exists = absolute_path_to_secret_file.is_file();
        let directory_exists = absolute_path_to_secret_directory.is_dir();
        if !file_exists && !directory_exists {
            errors::error_exit(
                "recipient",
                "add",
                ErrorKind::InvalidValue,
                &format!("invalid value '{}' for '--path <PATH>': path does not match any secret or folder in the store", path.display()));
        } else {
            directory_exists
        }
    })
}

fn upsert_recipient(
    mut recipients: String,
    public_key: &String,
    name: &Option<String>,
) -> (String, bool) {
    let recipient = format_recipient(public_key, name);
    let mut re_encryption_required = false;
    if let Some(public_key_index) = recipients.find(public_key) {
        // public key already exists as recipient
        // update name only - no re-encrypt required
        let start_index = recipients
            .get(..public_key_index)
            .and_then(|substring| substring.rfind('#'))
            .filter(|&comment_index| {
                recipients
                    .get(comment_index..public_key_index)
                    .map(|substring| substring.matches('\n').collect())
                    .is_some_and(|matches: Vec<&str>| matches.len() == 1)
            })
            .unwrap_or(public_key_index);
        recipients.replace_range(start_index..public_key_index + public_key.len(), &recipient);
        printer::recipient_updated();
    } else {
        // add new recipient - requires re-encryption of entire store
        re_encryption_required = true;
        if recipients.is_empty() {
            recipients = recipient;
        } else {
            recipients = recipients + "\n" + &recipient;
        }
        printer::recipient_added();
    }

    (recipients, re_encryption_required)
}

pub fn format_recipient(public_key: &String, name: &Option<String>) -> String {
    match name {
        Some(name) if !name.is_empty() => format!("# {name}\n{public_key}"),
        _ => public_key.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::vcs::VersionControlSystems;

    #[test]
    fn file_paths_for_file() {
        let store = Store {
            path: String::from("some/store"),
            name: String::from("test"),
            vcs: VersionControlSystems::None,
            identities: vec![],
        };
        let path = Some(PathBuf::from("secret-name"));
        let (relative, absolute) = calculate_recipients_file_paths(&store, &path, false);
        assert_eq!(relative, PathBuf::from("secret-name.recipients"));
        assert_eq!(absolute, PathBuf::from("some/store/secret-name.recipients"));
    }

    #[test]
    fn file_paths_for_directory() {
        let store = Store {
            path: String::from("some/store"),
            name: String::from("test"),
            vcs: VersionControlSystems::None,
            identities: vec![],
        };
        let path = Some(PathBuf::from("some/folder"));
        let (relative, absolute) = calculate_recipients_file_paths(&store, &path, true);
        assert_eq!(relative, PathBuf::from("some/folder/.recipients"));
        assert_eq!(
            absolute,
            PathBuf::from("some/store/some/folder/.recipients")
        );
    }

    #[test]
    fn insert_new_recipient() {
        let recipients = String::from("");
        let public_key = String::from("12345");
        let name = None;
        let expectation = "12345";
        let (recipients, _) = upsert_recipient(recipients, &public_key, &name);
        assert_eq!(recipients, expectation);
    }

    #[test]
    fn insert_new_recipient_with_name() {
        let recipients = String::from("");
        let public_key = String::from("12345");
        let name = Some(String::from("test"));
        let expectation = "# test\n\
            12345";
        let (recipients, _) = upsert_recipient(recipients, &public_key, &name);
        assert_eq!(recipients, expectation);
    }

    #[test]
    fn insert_new_recipient_with_existing_recipients() {
        let recipients = String::from("abcde");
        let public_key = String::from("12345");
        let name = None;
        let expectation = "abcde\n\
        12345";
        let (recipients, _) = upsert_recipient(recipients, &public_key, &name);
        assert_eq!(recipients, expectation);
    }

    #[test]
    fn insert_new_recipient_with_name_and_existing_recipients() {
        let recipients = String::from("abcde");
        let public_key = String::from("12345");
        let name = Some(String::from("test"));
        let expectation = "abcde\n\
        # test\n\
        12345";
        let (recipients, _) = upsert_recipient(recipients, &public_key, &name);
        assert_eq!(recipients, expectation);
    }

    #[test]
    fn upsert_recipient_with_new_name() {
        let recipients = String::from("12345");
        let public_key = String::from("12345");
        let name = Some(String::from("test"));
        let expectation = "# test\n\
            12345";
        let (recipients, _) = upsert_recipient(recipients, &public_key, &name);
        assert_eq!(recipients, expectation);
    }

    #[test]
    fn upsert_recipient_with_updated_name() {
        let recipients = String::from(
            "# old\n\
            12345",
        );
        let public_key = String::from("12345");
        let name = Some(String::from("new"));
        let expectation = "# new\n\
            12345";
        let (recipients, _) = upsert_recipient(recipients, &public_key, &name);
        assert_eq!(recipients, expectation);
    }

    #[test]
    fn upsert_recipient_below_recipient_with_comment() {
        let recipients = String::from(
            "# other\n\
            abcde\n\
            12345",
        );
        let public_key = String::from("12345");
        let name = Some(String::from("new"));
        let expectation = "# other\n\
            abcde\n\
            # new\n\
            12345";
        let (recipients, _) = upsert_recipient(recipients, &public_key, &name);
        assert_eq!(recipients, expectation);
    }

    #[test]
    fn upsert_recipient_above_recipient_with_comment() {
        let recipients = String::from(
            "abcde\n\
            12345\n\
            # other\n\
            54321",
        );
        let public_key = String::from("12345");
        let name = Some(String::from("new"));
        let expectation = "abcde\n\
            # new\n\
            12345\n\
            # other\n\
            54321";
        let (recipients, _) = upsert_recipient(recipients, &public_key, &name);
        assert_eq!(recipients, expectation);
    }

    #[test]
    fn upsert_recipient_with_empty_name() {
        let recipients = String::from(
            "# old\n\
            12345",
        );
        let public_key = String::from("12345");
        let name = Some(String::from(""));
        let expectation = "12345";
        let (recipients, _) = upsert_recipient(recipients, &public_key, &name);
        assert_eq!(recipients, expectation);
    }

    #[test]
    fn upsert_recipient_with_no_name() {
        let recipients = String::from(
            "# old\n\
            12345",
        );
        let public_key = String::from("12345");
        let name = None;
        let expectation = "12345";
        let (recipients, _) = upsert_recipient(recipients, &public_key, &name);
        assert_eq!(recipients, expectation);
    }
}
