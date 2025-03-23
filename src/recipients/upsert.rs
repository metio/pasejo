// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::cli::logs;
use crate::recipients::format;

pub fn recipient(
    mut recipients: String,
    public_key: &String,
    name: Option<&String>,
) -> (String, bool) {
    let recipient = format::recipient(public_key, name);
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
        logs::recipient_updated(public_key);
    } else {
        // add new recipient - requires re-encryption of entire store
        re_encryption_required = true;
        if recipients.is_empty() {
            recipients = recipient;
        } else {
            recipients = recipients + "\n" + &recipient;
        }
        logs::recipient_added(public_key);
    }

    (recipients, re_encryption_required)
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
        let (recipients, _) = recipient(recipients, &public_key, name);
        assert_eq!(recipients, expectation);
    }

    #[test]
    fn insert_new_recipient_with_name() {
        let recipients = String::from("");
        let public_key = String::from("12345");
        let name = Some(String::from("test"));
        let expectation = "# test\n\
            12345";
        let (recipients, _) = recipient(recipients, &public_key, name.as_ref());
        assert_eq!(recipients, expectation);
    }

    #[test]
    fn insert_new_recipient_with_existing_recipients() {
        let recipients = String::from("abcde");
        let public_key = String::from("12345");
        let name = None;
        let expectation = "abcde\n\
        12345";
        let (recipients, _) = recipient(recipients, &public_key, name);
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
        let (recipients, _) = recipient(recipients, &public_key, name.as_ref());
        assert_eq!(recipients, expectation);
    }

    #[test]
    fn upsert_recipient_with_new_name() {
        let recipients = String::from("12345");
        let public_key = String::from("12345");
        let name = Some(String::from("test"));
        let expectation = "# test\n\
            12345";
        let (recipients, _) = recipient(recipients, &public_key, name.as_ref());
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
        let (recipients, _) = recipient(recipients, &public_key, name.as_ref());
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
        let (recipients, _) = recipient(recipients, &public_key, name.as_ref());
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
        let (recipients, _) = recipient(recipients, &public_key, name.as_ref());
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
        let (recipients, _) = recipient(recipients, &public_key, name.as_ref());
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
        let (recipients, _) = recipient(recipients, &public_key, name.as_ref());
        assert_eq!(recipients, expectation);
    }
}
