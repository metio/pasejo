// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::cli;
use crate::models::password_store::Recipient;
use anyhow::Result;

pub fn merge_recipients(
    common_ancestor_recipients: &[Recipient],
    current_version_recipients: &[Recipient],
    other_version_recipients: &[Recipient],
) -> Result<Vec<Recipient>> {
    let mut resulting_recipients = vec![];
    let mut merge_conflict = false;

    for recipient in common_ancestor_recipients {
        let current_has_exact_match = current_version_recipients.contains(recipient);
        let other_has_exact_match = other_version_recipients.contains(recipient);
        let current_recipient = current_version_recipients
            .iter()
            .find(|item| item.public_key == recipient.public_key);
        let other_recipient = other_version_recipients
            .iter()
            .find(|item| item.public_key == recipient.public_key);

        if current_has_exact_match && other_has_exact_match {
            // If both current and other versions have the recipient, keep it
            resulting_recipients.push(recipient.clone());
        } else if current_has_exact_match {
            // current has recipient, but other might have changed it
            if let Some(other_recipient) = other_version_recipients
                .iter()
                .find(|&item| item.public_key == recipient.public_key)
            {
                // other has recipient with the same public key but different name
                resulting_recipients.push(other_recipient.clone());
            }
        } else if other_has_exact_match {
            // other has recipient, but current might have changed it
            if let Some(current_recipient) = current_version_recipients
                .iter()
                .find(|&item| item.public_key == recipient.public_key)
            {
                // current has recipient with the same public key but different name
                resulting_recipients.push(current_recipient.clone());
            }
        } else {
            // both current and other might have changed or removed the recipient

            if let (Some(current_recipient), Some(other_recipient)) =
                (current_recipient, other_recipient)
            {
                // both current and other have a recipient with the same public key
                if current_recipient.name == other_recipient.name {
                    // If names are the same, we can keep either one
                    resulting_recipients.push(current_recipient.clone());
                } else {
                    // If names are different, we got a merge conflict
                    merge_conflict = true;
                    cli::logs::merge_conflict_recipient_names(
                        &recipient.public_key,
                        &current_recipient.name,
                        &other_recipient.name,
                    );
                }
            } else if let (Some(current_recipient), None) = (current_recipient, other_recipient) {
                // current has recipient, but other removed it
                merge_conflict = true;
                cli::logs::merge_conflict_recipient_removed_and_renamed(
                    &recipient.public_key,
                    &current_recipient.name,
                );
            } else if let (None, Some(other_recipient)) = (current_recipient, other_recipient) {
                // other has recipient, but current removed it
                merge_conflict = true;
                cli::logs::merge_conflict_recipient_removed_and_renamed(
                    &recipient.public_key,
                    &other_recipient.name,
                );
            }
        }
    }

    for recipient in current_version_recipients {
        if !common_ancestor_recipients
            .iter()
            .any(|item| item.public_key == recipient.public_key)
        {
            // If the recipient is not in the common ancestor, it was added in the current version
            resulting_recipients.push(recipient.clone());
        }
    }

    for recipient in other_version_recipients {
        if !common_ancestor_recipients
            .iter()
            .any(|item| item.public_key == recipient.public_key)
        {
            // If the recipient is not in the common ancestor, it was added in the other version
            // Check if we already have this recipient in the resulting recipients
            let already_added_recipient = resulting_recipients
                .iter()
                .find(|item| item.public_key == recipient.public_key);

            if let Some(already_added_recipient) = already_added_recipient {
                if already_added_recipient.name != recipient.name {
                    // If the recipient is already added but with a different name, we have a conflict
                    merge_conflict = true;
                    cli::logs::merge_conflict_recipient_names(
                        &recipient.public_key,
                        &already_added_recipient.name,
                        &recipient.name,
                    );
                }
            } else {
                // we have not added this recipient yet
                resulting_recipients.push(recipient.clone());
            }
        }
    }

    if merge_conflict {
        anyhow::bail!("Merge conflict detected in recipients. Please resolve manually.")
    }
    Ok(resulting_recipients)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_recipients_with_empty_vectors() {
        let common_ancestor = vec![];
        let current_version = vec![];
        let other_version = vec![];
        let result = merge_recipients(&common_ancestor, &current_version, &other_version);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![]);
    }

    #[test]
    fn merge_recipients_without_change() {
        let recipient = Recipient {
            name: "Alice".to_string(),
            public_key: "123".to_string(),
        };
        let common_ancestor = vec![recipient.clone()];
        let current_version = vec![recipient.clone()];
        let other_version = vec![recipient.clone()];
        let result = merge_recipients(&common_ancestor, &current_version, &other_version);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![recipient.clone()]);
    }

    #[test]
    fn merge_recipients_removed_in_current() {
        let recipient = Recipient {
            name: "Alice".to_string(),
            public_key: "123".to_string(),
        };
        let common_ancestor = vec![recipient.clone()];
        let current_version = vec![];
        let other_version = vec![recipient.clone()];
        let result = merge_recipients(&common_ancestor, &current_version, &other_version);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![]);
    }

    #[test]
    fn merge_recipients_removed_in_other() {
        let recipient = Recipient {
            name: "Alice".to_string(),
            public_key: "123".to_string(),
        };
        let common_ancestor = vec![recipient.clone()];
        let current_version = vec![recipient.clone()];
        let other_version = vec![];
        let result = merge_recipients(&common_ancestor, &current_version, &other_version);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![]);
    }

    #[test]
    fn merge_recipients_removed_in_both() {
        let recipient = Recipient {
            name: "Alice".to_string(),
            public_key: "123".to_string(),
        };
        let common_ancestor = vec![recipient.clone()];
        let current_version = vec![];
        let other_version = vec![];
        let result = merge_recipients(&common_ancestor, &current_version, &other_version);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![]);
    }

    #[test]
    fn merge_recipients_renamed_in_current() {
        let recipient = Recipient {
            name: "Alice".to_string(),
            public_key: "123".to_string(),
        };
        let updated_recipient = Recipient {
            name: "Bob".to_string(),
            public_key: "123".to_string(),
        };
        let common_ancestor = vec![recipient.clone()];
        let current_version = vec![updated_recipient.clone()];
        let other_version = vec![recipient.clone()];
        let result = merge_recipients(&common_ancestor, &current_version, &other_version);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![updated_recipient.clone()]);
    }

    #[test]
    fn merge_recipients_renamed_in_other() {
        let recipient = Recipient {
            name: "Alice".to_string(),
            public_key: "123".to_string(),
        };
        let updated_recipient = Recipient {
            name: "Bob".to_string(),
            public_key: "123".to_string(),
        };
        let common_ancestor = vec![recipient.clone()];
        let current_version = vec![recipient.clone()];
        let other_version = vec![updated_recipient.clone()];
        let result = merge_recipients(&common_ancestor, &current_version, &other_version);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![updated_recipient.clone()]);
    }

    #[test]
    fn merge_recipients_renamed_in_both() {
        let recipient = Recipient {
            name: "Alice".to_string(),
            public_key: "123".to_string(),
        };
        let updated_recipient = Recipient {
            name: "Bob".to_string(),
            public_key: "123".to_string(),
        };
        let common_ancestor = vec![recipient.clone()];
        let current_version = vec![updated_recipient.clone()];
        let other_version = vec![updated_recipient.clone()];
        let result = merge_recipients(&common_ancestor, &current_version, &other_version);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![updated_recipient.clone()]);
    }

    #[test]
    fn merge_recipients_renamed_with_conflict() {
        let recipient = Recipient {
            name: "Alice".to_string(),
            public_key: "123".to_string(),
        };
        let current_recipient = Recipient {
            name: "Bob".to_string(),
            public_key: "123".to_string(),
        };
        let other_recipient = Recipient {
            name: "Eve".to_string(),
            public_key: "123".to_string(),
        };
        let common_ancestor = vec![recipient.clone()];
        let current_version = vec![current_recipient.clone()];
        let other_version = vec![other_recipient.clone()];
        let result = merge_recipients(&common_ancestor, &current_version, &other_version);
        assert!(result.is_err());
    }

    #[test]
    fn merge_recipients_renamed_in_current_removed_in_other() {
        let recipient = Recipient {
            name: "Alice".to_string(),
            public_key: "123".to_string(),
        };
        let current_recipient = Recipient {
            name: "Bob".to_string(),
            public_key: "123".to_string(),
        };
        let common_ancestor = vec![recipient.clone()];
        let current_version = vec![current_recipient.clone()];
        let other_version = vec![];
        let result = merge_recipients(&common_ancestor, &current_version, &other_version);
        assert!(result.is_err());
    }

    #[test]
    fn merge_recipients_renamed_in_other_removed_in_current() {
        let recipient = Recipient {
            name: "Alice".to_string(),
            public_key: "123".to_string(),
        };
        let other_recipient = Recipient {
            name: "Bob".to_string(),
            public_key: "123".to_string(),
        };
        let common_ancestor = vec![recipient.clone()];
        let current_version = vec![];
        let other_version = vec![other_recipient.clone()];
        let result = merge_recipients(&common_ancestor, &current_version, &other_version);
        assert!(result.is_err());
    }

    #[test]
    fn merge_recipients_added_in_current() {
        let recipient = Recipient {
            name: "Alice".to_string(),
            public_key: "123".to_string(),
        };
        let added_recipient = Recipient {
            name: "Bob".to_string(),
            public_key: "456".to_string(),
        };
        let common_ancestor = vec![recipient.clone()];
        let current_version = vec![recipient.clone(), added_recipient.clone()];
        let other_version = vec![recipient.clone()];
        let result = merge_recipients(&common_ancestor, &current_version, &other_version);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            vec![recipient.clone(), added_recipient.clone()]
        );
    }

    #[test]
    fn merge_recipients_added_in_other() {
        let recipient = Recipient {
            name: "Alice".to_string(),
            public_key: "123".to_string(),
        };
        let added_recipient = Recipient {
            name: "Bob".to_string(),
            public_key: "456".to_string(),
        };
        let common_ancestor = vec![recipient.clone()];
        let current_version = vec![recipient.clone()];
        let other_version = vec![recipient.clone(), added_recipient.clone()];
        let result = merge_recipients(&common_ancestor, &current_version, &other_version);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            vec![recipient.clone(), added_recipient.clone()]
        );
    }

    #[test]
    fn merge_recipients_added_in_both() {
        let recipient = Recipient {
            name: "Alice".to_string(),
            public_key: "123".to_string(),
        };
        let added_recipient = Recipient {
            name: "Bob".to_string(),
            public_key: "456".to_string(),
        };
        let common_ancestor = vec![recipient.clone()];
        let current_version = vec![recipient.clone(), added_recipient.clone()];
        let other_version = vec![recipient.clone(), added_recipient.clone()];
        let result = merge_recipients(&common_ancestor, &current_version, &other_version);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            vec![recipient.clone(), added_recipient.clone()]
        );
    }

    #[test]
    fn merge_recipients_added_in_both_with_different_names() {
        let recipient = Recipient {
            name: "Alice".to_string(),
            public_key: "123".to_string(),
        };
        let current_recipient = Recipient {
            name: "Bob".to_string(),
            public_key: "456".to_string(),
        };
        let other_recipient = Recipient {
            name: "Eve".to_string(),
            public_key: "456".to_string(),
        };
        let common_ancestor = vec![recipient.clone()];
        let current_version = vec![recipient.clone(), current_recipient.clone()];
        let other_version = vec![recipient.clone(), other_recipient.clone()];
        let result = merge_recipients(&common_ancestor, &current_version, &other_version);
        assert!(result.is_err());
    }
}
