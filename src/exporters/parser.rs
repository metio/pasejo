// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::collections::BTreeMap;

pub struct ParsedSecret {
    pub password: Option<String>,
    pub notes: Vec<String>,
    pub fields: BTreeMap<String, String>,
}

pub fn parse_secret(secret: &str) -> ParsedSecret {
    let mut password: Option<String> = None;
    let mut notes: Vec<String> = vec![];
    let mut fields: BTreeMap<String, String> = BTreeMap::new();

    for line in secret.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.contains(": ") {
            if let Some((key, value)) = line.split_once(": ") {
                fields.insert(String::from(key), String::from(value));
            }
        } else if password.is_none() {
            password = Some(String::from(line));
        } else {
            notes.push(String::from(line));
        }
    }

    ParsedSecret {
        password,
        notes,
        fields,
    }
}
