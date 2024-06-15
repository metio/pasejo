// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct PasswordStore {
    /// Recipients used in this store
    pub recipients: Vec<Recipient>,

    /// Secrets available in this store
    pub secrets: BTreeMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Recipient {
    pub name: String,
    pub public_key: String,
}

impl PasswordStore {
    pub fn secret_names_as_list(&self) -> Vec<String> {
        self.secrets.keys().cloned().collect()
    }
}
