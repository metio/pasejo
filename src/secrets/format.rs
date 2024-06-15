// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use termtree::Tree;

pub fn format_as_tree(prefix: &str, secrets: &[String]) -> Tree<String> {
    let mut root = Tree::new(String::from(prefix));

    let mut nested_prefix = "";
    for secret in secrets {
        if nested_prefix.is_empty() || !secret.starts_with(nested_prefix) {
            nested_prefix = secret.split('/').next().unwrap_or(secret);
            let secrets_with_prefix = secrets
                .iter()
                .filter_map(|other| other.strip_prefix(&format!("{nested_prefix}/")))
                .map(std::string::ToString::to_string)
                .collect::<Vec<String>>();
            root.push(format_as_tree(nested_prefix, &secrets_with_prefix));
        }
    }

    root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_tree() {
        let secrets = vec![String::from("secret-name")];
        print!("{}", format_as_tree("", &secrets));
    }

    #[test]
    fn format_multiple() {
        let secrets = vec![
            String::from("secret-1"),
            String::from("secret-2"),
            String::from("secret-3"),
        ];
        print!("{}", format_as_tree("", &secrets));
    }

    #[test]
    fn format_nested() {
        let secrets = vec![
            String::from("secret-1/sub-secret-1"),
            String::from("secret-1/sub-secret-2"),
            String::from("secret-2/sub-secret-1"),
            String::from("secret-3/sub1/sub2/sub3"),
            String::from("secret-3/sub1/sub2/sub4"),
            String::from("secret-3/sub1/sub5"),
            String::from("secret-3"),
            String::from("secret-3/sub6"),
        ];
        print!("{}", format_as_tree("", &secrets));
    }
}
