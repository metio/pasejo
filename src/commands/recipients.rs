use std::path::PathBuf;

use crate::adapters::file_system::FileSystem;
use crate::models::configuration::Store;

pub fn add(
    file_system: Box<dyn FileSystem>,
    store: &Store,
    public_key: &String,
    name: &Option<String>,
    path: &Option<PathBuf>,
) -> anyhow::Result<()> {
    let (recipients_absolute, recipients_relative) = store.paths_for(path, ".recipients");
    let (secret_absolute, _) = store.paths_for(path, ".age");
    let (directory_absolute, _) = store.paths_for(path, "");

    if file_system.file_exists(&secret_absolute)?
        || file_system.directory_exists(&directory_absolute)?
    {
        if file_system.file_exists(&recipients_absolute)? {
            // update existing .recipients file
            let recipients = file_system.read_file(&recipients_absolute)?;
            let (updated_recipients, _) = upsert_recipient(recipients, public_key, name);
            file_system.write_file(&recipients_absolute, updated_recipients)?;
        } else {
            // create new .recipients file
            let recipient = format_recipient(public_key, name);
            file_system.append_file(&recipients_absolute, &recipient)?;
        }

        store.vcs.select_implementation().commit(
            store.path.as_ref(),
            &recipients_relative,
            &format!("Added recipient '{}'", public_key),
        )?;
    }
    println!("Recipient added");
    Ok(())
}

fn upsert_recipient(
    mut recipients: String,
    public_key: &String,
    name: &Option<String>,
) -> (String, bool) {
    let recipient = format_recipient(public_key, name);
    let mut re_encryption_required = false;
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
            // add new recipient - requires re-encryption of entire store
            re_encryption_required = true;
            if recipients.is_empty() {
                recipients = recipient;
            } else {
                recipients = recipients + "\n" + &recipient;
            }
        }
    }
    (recipients, re_encryption_required)
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
