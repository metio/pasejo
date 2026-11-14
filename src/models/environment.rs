// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! References from environment variables to values inside secrets, as
//! `secret resolve` reads them from JSON and `exec` from its flags.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::cli::i18n;
use crate::secrets::selection::Selection;

/// One value inside a secret: the secret's path, optionally in a store other
/// than the selected one, and which part of it to read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretReference {
    pub store: Option<String>,
    pub secret: String,
    pub selection: Selection,
}

/// An environment variable bound to a value inside a secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvAssignment {
    pub variable: String,
    pub reference: SecretReference,
}

/// Names a POSIX shell accepts: ASCII letters, digits and underscores, not
/// starting with a digit. Other names exist on some systems, but a child
/// written as a shell script could not read them.
pub fn is_valid_variable_name(name: &str) -> bool {
    let mut characters = name.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

/// One entry of the JSON object `secret resolve` reads: the variable name is
/// the key, this is the value.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReferenceSpec {
    secret: String,
    #[serde(default)]
    store: Option<String>,
    #[serde(default)]
    line: Option<isize>,
    #[serde(default)]
    field: Option<String>,
}

/// Parses the JSON object `secret resolve` reads, mapping each variable
/// name to a reference. A reference without `line` or `field` selects the
/// first line, the conventional place of the password.
pub fn parse_references(json: &str) -> anyhow::Result<BTreeMap<String, SecretReference>> {
    let specs: BTreeMap<String, ReferenceSpec> =
        serde_json::from_str(json).map_err(|error| anyhow::anyhow!(error.to_string()))?;
    specs
        .into_iter()
        .map(|(variable, spec)| {
            if !is_valid_variable_name(&variable) {
                anyhow::bail!(i18n::error_env_variable_name_invalid(&variable));
            }
            let selection = match (spec.line, spec.field) {
                (Some(_), Some(_)) => {
                    anyhow::bail!(i18n::error_env_reference_line_and_field(&variable))
                }
                (Some(0), None) => anyhow::bail!(i18n::error_line_number_must_not_be_zero()),
                (Some(line), None) => Selection::Line(line),
                (None, Some(field)) => Selection::Field(field),
                (None, None) => Selection::Line(1),
            };
            let reference = SecretReference {
                store: spec.store,
                secret: spec.secret,
                selection,
            };
            Ok((variable, reference))
        })
        .collect()
}

/// Collects `exec`'s assignments into one map, refusing a variable that is
/// assigned twice: which of the two values the command saw would otherwise
/// depend on flag order.
pub fn collect_assignments<'a>(
    assignments: impl IntoIterator<Item = &'a EnvAssignment>,
) -> anyhow::Result<BTreeMap<String, SecretReference>> {
    let mut references = BTreeMap::new();
    for assignment in assignments {
        if references
            .insert(assignment.variable.clone(), assignment.reference.clone())
            .is_some()
        {
            anyhow::bail!(i18n::error_env_variable_duplicate(&assignment.variable));
        }
    }
    Ok(references)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(store: Option<&str>, secret: &str, selection: Selection) -> SecretReference {
        SecretReference {
            store: store.map(str::to_owned),
            secret: secret.to_owned(),
            selection,
        }
    }

    #[test]
    fn variable_names_follow_the_posix_shell_rules() {
        for valid in ["A", "_", "GH_TOKEN", "a1", "_9"] {
            assert!(is_valid_variable_name(valid), "{valid}");
        }
        for invalid in ["", "1A", "A-B", "A B", "Ä", "A=B"] {
            assert!(!is_valid_variable_name(invalid), "{invalid}");
        }
    }

    #[test]
    fn parse_references_reads_every_selector() {
        let references = parse_references(
            r#"{
                "TOKEN": {"secret": "api/token"},
                "USER": {"secret": "db", "store": "work", "field": "username"},
                "LAST": {"secret": "notes", "line": -1}
            }"#,
        )
        .unwrap();
        assert_eq!(
            references,
            BTreeMap::from([
                (
                    String::from("LAST"),
                    reference(None, "notes", Selection::Line(-1))
                ),
                (
                    String::from("TOKEN"),
                    reference(None, "api/token", Selection::Line(1))
                ),
                (
                    String::from("USER"),
                    reference(
                        Some("work"),
                        "db",
                        Selection::Field(String::from("username"))
                    )
                ),
            ])
        );
    }

    #[test]
    fn parse_references_refuses_invalid_input() {
        crate::cli::i18n::init_for_tests();
        for json in [
            "not json",
            r#"{"TOKEN": {"secret": "a", "line": 1, "field": "b"}}"#,
            r#"{"TOKEN": {"secret": "a", "line": 0}}"#,
            r#"{"1TOKEN": {"secret": "a"}}"#,
            r#"{"TOKEN": {"secret": "a", "unknown": true}}"#,
            r#"{"TOKEN": {}}"#,
            r#"["TOKEN"]"#,
        ] {
            assert!(parse_references(json).is_err(), "{json}");
        }
    }

    #[test]
    fn collect_assignments_refuses_a_variable_assigned_twice() {
        crate::cli::i18n::init_for_tests();
        let assignment = |secret: &str| EnvAssignment {
            variable: String::from("TOKEN"),
            reference: reference(None, secret, Selection::Line(1)),
        };
        assert!(collect_assignments(&[assignment("a"), assignment("b")]).is_err());
        assert_eq!(collect_assignments(&[assignment("a")]).unwrap().len(), 1);
    }
}
