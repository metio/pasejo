// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::cli::constants::APPLICATION_NAME;
use crate::exporters::parser::{parse_secret, ParsedSecret};
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
