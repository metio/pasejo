// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::collections::BTreeSet;

use termtree::Tree;

pub fn format_as_tree(prefix: &str, secrets: &[String]) -> Tree<String> {
    let mut root = Tree::new(String::from(prefix));

    let mut seen = BTreeSet::new();
    for secret in secrets {
        let nested_prefix = secret.split('/').next().unwrap_or(secret);
        if seen.insert(nested_prefix) {
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
    fn format_keeps_names_that_extend_a_sibling_name() {
        let tree = format_as_tree("", &[String::from("github"), String::from("github-work")]);
        assert_eq!(tree.to_string(), "\n├── github\n└── github-work\n");

        let tree = format_as_tree(
            "",
            &[String::from("secret-1/a"), String::from("secret-10/b")],
        );
        assert_eq!(
            tree.to_string(),
            "\n├── secret-1\n│   └── a\n└── secret-10\n    └── b\n"
        );
    }

    #[test]
    fn format_lists_a_name_once_when_a_longer_sibling_sorts_between_its_entries() {
        let tree = format_as_tree(
            "",
            &[
                String::from("github"),
                String::from("github-work"),
                String::from("github/token"),
            ],
        );
        assert_eq!(
            tree.to_string(),
            "\n├── github\n│   └── token\n└── github-work\n"
        );
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
