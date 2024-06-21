use std::path::{Path, PathBuf};

use crate::adapters::file_system::FileSystem;
use crate::cli::configuration::Store;

pub fn add(
    file_system: Box<dyn FileSystem>,
    store: &Store,
    public_key: &String,
    name: &Option<String>,
    path: &Option<PathBuf>,
) -> anyhow::Result<()> {
    let store_root_path: &Path = store.path.as_ref();
    let vcs = store.vcs.select_implementation();

    if path.is_none() {
        // add new recipient to the entire store
        let recipients_file = &PathBuf::from(".recipients");
        let root_recipients_file = &store_root_path.join(recipients_file);

        if root_recipients_file.try_exists()? && root_recipients_file.is_file() {
            // update existing .recipients file
            let root_recipients = file_system.read_file(root_recipients_file)?;
            let updated_recipients = upsert_recipient(root_recipients, public_key, name);
            file_system.write_file(root_recipients_file, updated_recipients)?;
        } else {
            // create new .recipients file
            let recipient = format_recipient(public_key, name);
            file_system.append_file(root_recipients_file, &recipient)?;
        }

        vcs.commit(
            store_root_path,
            recipients_file,
            &format!("Added recipient '{}'", public_key),
        )?;
    } else {
        // add recipient to specific path (folder or single secret)
        file_system.reverse_walk(store_root_path);
    }

    Ok(())
}

fn upsert_recipient(mut recipients: String, public_key: &String, name: &Option<String>) -> String {
    let recipient = format_recipient(public_key, name);
    match recipients.find(public_key) {
        Some(public_key_index) => {
            // public key already exists as recipient
            // update name only - no re-encrypt required
            let start_index = recipients
                .get(..public_key_index)
                .and_then(|substring| substring.rfind("#"))
                .filter(|&comment_index| {
                    recipients
                        .get(comment_index..public_key_index)
                        .map(|substring| substring.matches("\n").collect())
                        .map(|matches: Vec<&str>| matches.len() == 1)
                        .unwrap_or(false)
                })
                .unwrap_or(public_key_index);
            recipients.replace_range(start_index..public_key_index + public_key.len(), &recipient);
        }
        None => {
            // add new recipient
            if recipients.is_empty() {
                recipients = recipient;
            } else {
                recipients = recipients + "\n" + &recipient;
            }
        }
    }
    recipients
}

fn format_recipient(public_key: &String, name: &Option<String>) -> String {
    match name {
        Some(name) if !name.is_empty() => format!("# {}\n{}", name, public_key),
        _ => format!("{}", public_key),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_new_recipient() {
        let recipients = String::from("");
        let public_key = String::from("12345");
        let name = None;
        let expectation = "12345";
        assert_eq!(
            upsert_recipient(recipients, &public_key, &name),
            expectation
        );
    }

    #[test]
    fn insert_new_recipient_with_name() {
        let recipients = String::from("");
        let public_key = String::from("12345");
        let name = Some(String::from("test"));
        let expectation = "# test\n\
            12345";
        assert_eq!(
            upsert_recipient(recipients, &public_key, &name),
            expectation
        );
    }

    #[test]
    fn insert_new_recipient_with_existing_recipients() {
        let recipients = String::from("abcde");
        let public_key = String::from("12345");
        let name = None;
        let expectation = "abcde\n\
        12345";
        assert_eq!(
            upsert_recipient(recipients, &public_key, &name),
            expectation
        );
    }

    #[test]
    fn insert_new_recipient_with_name_and_existing_recipients() {
        let recipients = String::from("abcde");
        let public_key = String::from("12345");
        let name = Some(String::from("test"));
        let expectation = "abcde\n\
        # test\n\
        12345";
        assert_eq!(
            upsert_recipient(recipients, &public_key, &name),
            expectation
        );
    }

    #[test]
    fn upsert_recipient_with_new_name() {
        let recipients = String::from("12345");
        let public_key = String::from("12345");
        let name = Some(String::from("test"));
        let expectation = "# test\n\
            12345";
        assert_eq!(
            upsert_recipient(recipients, &public_key, &name),
            expectation
        );
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
        assert_eq!(
            upsert_recipient(recipients, &public_key, &name),
            expectation
        );
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
        assert_eq!(
            upsert_recipient(recipients, &public_key, &name),
            expectation
        );
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
        assert_eq!(
            upsert_recipient(recipients, &public_key, &name),
            expectation
        );
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
        assert_eq!(
            upsert_recipient(recipients, &public_key, &name),
            expectation
        );
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
        assert_eq!(
            upsert_recipient(recipients, &public_key, &name),
            expectation
        );
    }
}
