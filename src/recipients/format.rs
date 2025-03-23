// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::recipients::format;

pub fn recipient(public_key: &String, name: Option<&String>) -> String {
    match name {
        Some(name) if !name.is_empty() => format!("# {name}\n{public_key}"),
        _ => public_key.clone(),
    }
}

pub fn recipients(recipients: &Vec<String>) -> String {
    let mut result = String::new();
    for recipient in recipients {
        let formatted_recipient = match recipient.split_once(',') {
            None => format::recipient(recipient, None),
            Some((name, key)) => format::recipient(&key.to_string(), Some(&name.to_string())),
        };
        result.push_str(&formatted_recipient);
        result.push('\n');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipient_with_name() {
        let public_key = String::from("12345");
        let name = Some(String::from("test"));
        let expectation = "# test\n\
            12345";
        assert_eq!(recipient(&public_key, name.as_ref()), expectation);
    }

    #[test]
    fn recipient_without_name() {
        let public_key = String::from("12345");
        let name = None;
        let expectation = "12345";
        assert_eq!(recipient(&public_key, name.as_ref()), expectation);
    }

    #[test]
    fn recipients_with_name() {
        let values = vec![String::from("test,12345"), String::from("abc,67890")];
        let expectation = "# test\n\
            12345\n\
            # abc\n\
            67890\n";
        assert_eq!(recipients(&values), expectation);
    }

    #[test]
    fn recipients_without_name() {
        let values = vec![String::from("12345"), String::from("67890")];
        let expectation = "12345\n67890\n";
        assert_eq!(recipients(&values), expectation);
    }
}
