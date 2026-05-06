// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::cli::constants::APPLICATION_NAME;
use crate::exporters::parser::{ParsedSecret, parse_secret};
use crate::models::password_store::PasswordStore;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

pub fn json(
    store: &PasswordStore,
    organization_id: Option<&String>,
    collection_id: Option<&String>,
    collection_name: Option<&String>,
    username_keys: &[String],
    uri_keys: &[String],
    pretty: Option<bool>,
) -> anyhow::Result<String> {
    if let Some(organization_id) = organization_id {
        let collection = BitwardenCollection {
            id: collection_id
                .cloned()
                .unwrap_or_else(|| Uuid::now_v7().to_string()),
            organization_id: organization_id.clone(),
            name: collection_name
                .cloned()
                .unwrap_or_else(|| String::from(APPLICATION_NAME)),
            external_id: None,
        };
        let mut items = vec![];

        for (name, secret) in &store.secrets {
            let parsed_secret = parse_secret(secret);

            let (username, uris, fields) = parsed_secret.extract_fields(username_keys, uri_keys);

            items.push(BitwardenItem {
                id: Uuid::now_v7().to_string(),
                folder_id: None,
                organization_id: Some(organization_id.clone()),
                collection_ids: vec![collection.id.clone()],
                name: name.clone(),
                notes: parsed_secret.notes.join("\n"),
                fields,
                login: BitwardenLogin {
                    username: username.clone(),
                    password: parsed_secret.password.unwrap_or_default(),
                    uris,
                },
                item_type: 1,
            });
        }

        let bitwarden = BitwardenOrganization {
            collections: vec![collection],
            items,
        };

        let output = if let Some(pretty) = pretty
            && pretty
        {
            serde_json::to_string_pretty(&bitwarden)?
        } else {
            serde_json::to_string(&bitwarden)?
        };
        Ok(output)
    } else {
        let mut folders = BTreeMap::new();
        let mut items = vec![];

        for (name, secret) in &store.secrets {
            let parsed_secret = parse_secret(secret);

            let folder_id = if let Some((folder, _)) = name.rsplit_once('/') {
                let id = Uuid::now_v7().to_string();
                folders
                    .entry(id.clone())
                    .or_insert_with(|| BitwardenFolder {
                        id: id.clone(),
                        name: folder.to_owned(),
                    });
                Some(id)
            } else {
                None
            };

            let (username, uris, fields) = parsed_secret.extract_fields(username_keys, uri_keys);

            items.push(BitwardenItem {
                id: Uuid::now_v7().to_string(),
                folder_id,
                organization_id: None,
                collection_ids: vec![],
                name: name.clone(),
                notes: parsed_secret.notes.join("\n"),
                fields,
                login: BitwardenLogin {
                    username: username.clone(),
                    password: parsed_secret.password.unwrap_or_default(),
                    uris,
                },
                item_type: 1,
            });
        }

        let bitwarden = BitwardenIndividual {
            folders: folders.values().cloned().collect(),
            items,
        };

        let output = if let Some(pretty) = pretty
            && pretty
        {
            serde_json::to_string_pretty(&bitwarden)?
        } else {
            serde_json::to_string(&bitwarden)?
        };
        Ok(output)
    }
}

impl ParsedSecret {
    fn extract_fields(
        &self,
        username_keys: &[String],
        uri_keys: &[String],
    ) -> (String, Vec<BitwardenUri>, Vec<BitwardenField>) {
        let mut username = String::new();
        let mut uris: Vec<BitwardenUri> = vec![];
        let mut fields: Vec<BitwardenField> = vec![];
        for (key, value) in &self.fields {
            if username_keys.contains(key) {
                username.clone_from(value);
            } else if uri_keys.contains(key) {
                uris.push(BitwardenUri {
                    uri_match: None,
                    uri: value.clone(),
                });
            } else {
                fields.push(BitwardenField {
                    name: key.clone(),
                    value: value.clone(),
                    field_type: 0,
                });
            }
        }

        (username, uris, fields)
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct BitwardenIndividual {
    pub folders: Vec<BitwardenFolder>,
    pub items: Vec<BitwardenItem>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct BitwardenOrganization {
    pub collections: Vec<BitwardenCollection>,
    pub items: Vec<BitwardenItem>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct BitwardenFolder {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct BitwardenCollection {
    pub id: String,
    pub name: String,
    pub organization_id: String,
    pub external_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct BitwardenItem {
    pub id: String,
    pub folder_id: Option<String>,
    pub organization_id: Option<String>,
    pub collection_ids: Vec<String>,
    pub name: String,
    pub notes: String,
    pub fields: Vec<BitwardenField>,
    pub login: BitwardenLogin,
    #[serde(rename = "type")]
    pub item_type: u8,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct BitwardenField {
    pub name: String,
    pub value: String,
    #[serde(rename = "type")]
    pub field_type: u8,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct BitwardenLogin {
    pub username: String,
    pub password: String,
    pub uris: Vec<BitwardenUri>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct BitwardenUri {
    #[serde(rename = "match")]
    pub uri_match: Option<String>,
    pub uri: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn store_with(secrets: &[(&str, &str)]) -> PasswordStore {
        let mut store = PasswordStore::default();
        for (name, value) in secrets {
            store
                .secrets
                .insert((*name).to_string(), (*value).to_string());
        }
        store
    }

    #[test]
    fn individual_export_is_valid_json() {
        let store = store_with(&[("login", "hunter2\nuser: alice")]);

        let output = json(&store, None, None, None, &[], &[], None).unwrap();

        let parsed: Value = serde_json::from_str(&output).unwrap();
        assert!(parsed.get("items").is_some());
        assert!(parsed.get("folders").is_some());
        assert!(parsed.get("collections").is_none());
    }

    #[test]
    fn organization_export_emits_collection() {
        let store = store_with(&[("login", "hunter2")]);
        let org_id = String::from("org-123");
        let collection_id = String::from("coll-456");
        let collection_name = String::from("MyTeam");

        let output = json(
            &store,
            Some(&org_id),
            Some(&collection_id),
            Some(&collection_name),
            &[],
            &[],
            None,
        )
        .unwrap();

        let parsed: Value = serde_json::from_str(&output).unwrap();
        let collections = parsed.get("collections").unwrap().as_array().unwrap();
        assert_eq!(collections.len(), 1);
        assert_eq!(collections[0]["id"], "coll-456");
        assert_eq!(collections[0]["organization_id"], "org-123");
        assert_eq!(collections[0]["name"], "MyTeam");
    }

    #[test]
    fn nested_secret_path_creates_folder_in_individual_export() {
        let store = store_with(&[("work/email", "hunter2")]);

        let output = json(&store, None, None, None, &[], &[], None).unwrap();

        let parsed: Value = serde_json::from_str(&output).unwrap();
        let folders = parsed.get("folders").unwrap().as_array().unwrap();
        assert_eq!(folders.len(), 1);
        assert_eq!(folders[0]["name"], "work");
        let items = parsed.get("items").unwrap().as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["name"], "work/email");
    }

    #[test]
    fn flat_secret_has_no_folder_id() {
        let store = store_with(&[("simple", "hunter2")]);

        let output = json(&store, None, None, None, &[], &[], None).unwrap();

        let parsed: Value = serde_json::from_str(&output).unwrap();
        let items = parsed.get("items").unwrap().as_array().unwrap();
        assert_eq!(items[0]["folder_id"], Value::Null);
    }

    #[test]
    fn username_keys_promote_field_to_login_username() {
        let store = store_with(&[("login", "hunter2\nuser: alice")]);
        let username_keys = vec![String::from("user")];

        let output = json(&store, None, None, None, &username_keys, &[], None).unwrap();

        let parsed: Value = serde_json::from_str(&output).unwrap();
        let item = &parsed["items"][0];
        assert_eq!(item["login"]["username"], "alice");
        // The user field must not also be in the generic fields list.
        let fields = item["fields"].as_array().unwrap();
        assert!(fields.iter().all(|f| f["name"] != "user"));
    }

    #[test]
    fn uri_keys_promote_field_to_login_uri() {
        let store = store_with(&[("login", "hunter2\nurl: https://example.com")]);
        let uri_keys = vec![String::from("url")];

        let output = json(&store, None, None, None, &[], &uri_keys, None).unwrap();

        let parsed: Value = serde_json::from_str(&output).unwrap();
        let uris = parsed["items"][0]["login"]["uris"].as_array().unwrap();
        assert_eq!(uris.len(), 1);
        assert_eq!(uris[0]["uri"], "https://example.com");
    }

    #[test]
    fn other_fields_become_generic_fields() {
        let store = store_with(&[("login", "hunter2\nnote: something\nother: value")]);

        let output = json(&store, None, None, None, &[], &[], None).unwrap();

        let parsed: Value = serde_json::from_str(&output).unwrap();
        let fields = parsed["items"][0]["fields"].as_array().unwrap();
        assert_eq!(fields.len(), 2);
        // Fields come from a BTreeMap so they are sorted by key.
        assert_eq!(fields[0]["name"], "note");
        assert_eq!(fields[0]["value"], "something");
        assert_eq!(fields[1]["name"], "other");
        assert_eq!(fields[1]["value"], "value");
    }

    #[test]
    fn notes_are_joined_with_newlines() {
        let store = store_with(&[("login", "hunter2\nfirst note\nsecond note")]);

        let output = json(&store, None, None, None, &[], &[], None).unwrap();

        let parsed: Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["items"][0]["notes"], "first note\nsecond note");
    }

    #[test]
    fn pretty_flag_produces_indented_output() {
        let store = store_with(&[("a", "b")]);

        let compact = json(&store, None, None, None, &[], &[], None).unwrap();
        let pretty = json(&store, None, None, None, &[], &[], Some(true)).unwrap();

        assert!(!compact.contains('\n'));
        assert!(pretty.contains('\n'));
    }

    #[test]
    fn empty_store_produces_empty_items() {
        let store = PasswordStore::default();

        let output = json(&store, None, None, None, &[], &[], None).unwrap();

        let parsed: Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["items"].as_array().unwrap().len(), 0);
        assert_eq!(parsed["folders"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn organization_export_uses_default_collection_name_when_not_specified() {
        let store = PasswordStore::default();
        let org_id = String::from("org-123");

        let output = json(&store, Some(&org_id), None, None, &[], &[], None).unwrap();

        let parsed: Value = serde_json::from_str(&output).unwrap();
        let collections = parsed["collections"].as_array().unwrap();
        assert_eq!(collections[0]["name"], env!("CARGO_PKG_NAME"));
    }
}
