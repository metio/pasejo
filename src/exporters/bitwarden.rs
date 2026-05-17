// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::cli::constants::APPLICATION_NAME;
use crate::exporters::parser::{ParsedSecret, parse_secret};
use crate::models::password_store::PasswordStore;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

const LOGIN_ITEM_TYPE: u8 = 1;
const TEXT_FIELD_TYPE: u8 = 0;

pub fn json(
    store: &PasswordStore,
    organization_id: Option<&String>,
    collection_id: Option<&String>,
    collection_name: Option<&String>,
    username_keys: &[String],
    uri_keys: &[String],
    pretty: bool,
) -> anyhow::Result<String> {
    organization_id.map_or_else(
        || {
            let payload = build_individual(store, username_keys, uri_keys);
            serialize(&payload, pretty)
        },
        |organization_id| {
            let payload = build_organization(
                store,
                organization_id,
                collection_id,
                collection_name,
                username_keys,
                uri_keys,
            );
            serialize(&payload, pretty)
        },
    )
}

fn build_organization(
    store: &PasswordStore,
    organization_id: &str,
    collection_id: Option<&String>,
    collection_name: Option<&String>,
    username_keys: &[String],
    uri_keys: &[String],
) -> BitwardenOrganization {
    let collection = BitwardenCollection {
        id: collection_id
            .cloned()
            .unwrap_or_else(|| Uuid::now_v7().to_string()),
        organization_id: organization_id.to_owned(),
        name: collection_name
            .cloned()
            .unwrap_or_else(|| String::from(APPLICATION_NAME)),
        external_id: None,
    };
    let items = store
        .secrets
        .iter()
        .map(|(name, secret)| {
            let mut item = BitwardenItem::from_secret(name, secret, username_keys, uri_keys);
            item.organization_id = Some(organization_id.to_owned());
            item.collection_ids = vec![collection.id.clone()];
            item
        })
        .collect();
    BitwardenOrganization {
        collections: vec![collection],
        items,
    }
}

fn build_individual(
    store: &PasswordStore,
    username_keys: &[String],
    uri_keys: &[String],
) -> BitwardenIndividual {
    let mut folders: BTreeMap<String, BitwardenFolder> = BTreeMap::new();
    let items = store
        .secrets
        .iter()
        .map(|(name, secret)| {
            let mut item = BitwardenItem::from_secret(name, secret, username_keys, uri_keys);
            item.folder_id = folder_id_for(name, &mut folders);
            item
        })
        .collect();
    BitwardenIndividual {
        folders: folders.into_values().collect(),
        items,
    }
}

fn folder_id_for(
    secret_name: &str,
    folders: &mut BTreeMap<String, BitwardenFolder>,
) -> Option<String> {
    let (folder_name, _) = secret_name.rsplit_once('/')?;
    let entry = folders
        .entry(folder_name.to_owned())
        .or_insert_with(|| BitwardenFolder {
            id: Uuid::now_v7().to_string(),
            name: folder_name.to_owned(),
        });
    Some(entry.id.clone())
}

fn serialize<T: Serialize>(value: &T, pretty: bool) -> anyhow::Result<String> {
    let output = if pretty {
        serde_json::to_string_pretty(value)?
    } else {
        serde_json::to_string(value)?
    };
    Ok(output)
}

impl BitwardenItem {
    fn from_secret(
        name: &str,
        secret: &str,
        username_keys: &[String],
        uri_keys: &[String],
    ) -> Self {
        let parsed = parse_secret(secret);
        let (username, uris, fields) = parsed.extract_fields(username_keys, uri_keys);
        Self {
            id: Uuid::now_v7().to_string(),
            folder_id: None,
            organization_id: None,
            collection_ids: vec![],
            name: name.to_owned(),
            notes: parsed.notes.join("\n"),
            fields,
            login: BitwardenLogin {
                username,
                password: parsed.password.unwrap_or_default(),
                uris,
            },
            item_type: LOGIN_ITEM_TYPE,
        }
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
                    field_type: TEXT_FIELD_TYPE,
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

        let output = json(&store, None, None, None, &[], &[], false).unwrap();

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
            false,
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

        let output = json(&store, None, None, None, &[], &[], false).unwrap();

        let parsed: Value = serde_json::from_str(&output).unwrap();
        let folders = parsed.get("folders").unwrap().as_array().unwrap();
        assert_eq!(folders.len(), 1);
        assert_eq!(folders[0]["name"], "work");
        let items = parsed.get("items").unwrap().as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["name"], "work/email");
    }

    #[test]
    fn multiple_secrets_in_same_folder_produce_one_folder() {
        let store = store_with(&[
            ("work/email", "hunter2"),
            ("work/calendar", "hunter3"),
            ("work/ssh", "hunter4"),
        ]);

        let output = json(&store, None, None, None, &[], &[], false).unwrap();

        let parsed: Value = serde_json::from_str(&output).unwrap();
        let folders = parsed.get("folders").unwrap().as_array().unwrap();
        assert_eq!(folders.len(), 1);
        assert_eq!(folders[0]["name"], "work");
    }

    #[test]
    fn multiple_secrets_in_same_folder_share_folder_id() {
        let store = store_with(&[("work/email", "hunter2"), ("work/calendar", "hunter3")]);

        let output = json(&store, None, None, None, &[], &[], false).unwrap();

        let parsed: Value = serde_json::from_str(&output).unwrap();
        let folder_id = parsed["folders"][0]["id"].clone();
        let items = parsed.get("items").unwrap().as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["folder_id"], folder_id);
        assert_eq!(items[1]["folder_id"], folder_id);
    }

    #[test]
    fn secrets_in_different_folders_produce_distinct_folders() {
        let store = store_with(&[
            ("work/email", "hunter2"),
            ("personal/email", "hunter3"),
            ("work/calendar", "hunter4"),
            ("personal/banking", "hunter5"),
        ]);

        let output = json(&store, None, None, None, &[], &[], false).unwrap();

        let parsed: Value = serde_json::from_str(&output).unwrap();
        let folders = parsed.get("folders").unwrap().as_array().unwrap();
        assert_eq!(folders.len(), 2);
        let folder_names: Vec<&str> = folders
            .iter()
            .map(|f| f["name"].as_str().unwrap())
            .collect();
        assert!(folder_names.contains(&"work"));
        assert!(folder_names.contains(&"personal"));
        let work_id = folders.iter().find(|f| f["name"] == "work").unwrap()["id"].clone();
        let personal_id = folders.iter().find(|f| f["name"] == "personal").unwrap()["id"].clone();
        assert_ne!(work_id, personal_id);
    }

    #[test]
    fn nested_path_uses_immediate_parent_as_folder() {
        let store = store_with(&[("work/dev/api-key", "hunter2"), ("work/dev/db", "hunter3")]);

        let output = json(&store, None, None, None, &[], &[], false).unwrap();

        let parsed: Value = serde_json::from_str(&output).unwrap();
        let folders = parsed.get("folders").unwrap().as_array().unwrap();
        assert_eq!(folders.len(), 1);
        assert_eq!(folders[0]["name"], "work/dev");
    }

    #[test]
    fn mix_of_flat_and_nested_secrets() {
        let store = store_with(&[
            ("root-secret", "hunter1"),
            ("work/email", "hunter2"),
            ("work/calendar", "hunter3"),
            ("another-root", "hunter4"),
        ]);

        let output = json(&store, None, None, None, &[], &[], false).unwrap();

        let parsed: Value = serde_json::from_str(&output).unwrap();
        let folders = parsed.get("folders").unwrap().as_array().unwrap();
        assert_eq!(folders.len(), 1);
        assert_eq!(folders[0]["name"], "work");

        let items = parsed.get("items").unwrap().as_array().unwrap();
        let work_folder_id = folders[0]["id"].clone();
        let mut flat_count = 0;
        let mut nested_count = 0;
        for item in items {
            if item["folder_id"].is_null() {
                flat_count += 1;
            } else {
                assert_eq!(item["folder_id"], work_folder_id);
                nested_count += 1;
            }
        }
        assert_eq!(flat_count, 2);
        assert_eq!(nested_count, 2);
    }

    #[test]
    fn flat_secret_has_no_folder_id() {
        let store = store_with(&[("simple", "hunter2")]);

        let output = json(&store, None, None, None, &[], &[], false).unwrap();

        let parsed: Value = serde_json::from_str(&output).unwrap();
        let items = parsed.get("items").unwrap().as_array().unwrap();
        assert_eq!(items[0]["folder_id"], Value::Null);
    }

    #[test]
    fn username_keys_promote_field_to_login_username() {
        let store = store_with(&[("login", "hunter2\nuser: alice")]);
        let username_keys = vec![String::from("user")];

        let output = json(&store, None, None, None, &username_keys, &[], false).unwrap();

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

        let output = json(&store, None, None, None, &[], &uri_keys, false).unwrap();

        let parsed: Value = serde_json::from_str(&output).unwrap();
        let uris = parsed["items"][0]["login"]["uris"].as_array().unwrap();
        assert_eq!(uris.len(), 1);
        assert_eq!(uris[0]["uri"], "https://example.com");
    }

    #[test]
    fn other_fields_become_generic_fields() {
        let store = store_with(&[("login", "hunter2\nnote: something\nother: value")]);

        let output = json(&store, None, None, None, &[], &[], false).unwrap();

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

        let output = json(&store, None, None, None, &[], &[], false).unwrap();

        let parsed: Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["items"][0]["notes"], "first note\nsecond note");
    }

    #[test]
    fn pretty_flag_produces_indented_output() {
        let store = store_with(&[("a", "b")]);

        let compact = json(&store, None, None, None, &[], &[], false).unwrap();
        let pretty = json(&store, None, None, None, &[], &[], true).unwrap();

        assert!(!compact.contains('\n'));
        assert!(pretty.contains('\n'));
    }

    #[test]
    fn empty_store_produces_empty_items() {
        let store = PasswordStore::default();

        let output = json(&store, None, None, None, &[], &[], false).unwrap();

        let parsed: Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["items"].as_array().unwrap().len(), 0);
        assert_eq!(parsed["folders"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn organization_export_uses_default_collection_name_when_not_specified() {
        let store = PasswordStore::default();
        let org_id = String::from("org-123");

        let output = json(&store, Some(&org_id), None, None, &[], &[], false).unwrap();

        let parsed: Value = serde_json::from_str(&output).unwrap();
        let collections = parsed["collections"].as_array().unwrap();
        assert_eq!(collections[0]["name"], env!("CARGO_PKG_NAME"));
    }

    #[test]
    fn organization_export_auto_generates_collection_id_when_not_specified() {
        let store = PasswordStore::default();
        let org_id = String::from("org-123");

        let payload = build_organization(&store, &org_id, None, None, &[], &[]);

        // A v7 UUID is 36 chars in canonical form.
        assert_eq!(payload.collections[0].id.len(), 36);
        Uuid::parse_str(&payload.collections[0].id).unwrap();
    }

    #[test]
    fn organization_export_items_have_no_folder_id_even_with_nested_names() {
        let store = store_with(&[("work/email", "hunter2"), ("flat", "hunter3")]);
        let org_id = String::from("org-123");

        let payload = build_organization(&store, &org_id, None, None, &[], &[]);

        for item in &payload.items {
            assert!(item.folder_id.is_none());
            assert_eq!(item.organization_id.as_deref(), Some("org-123"));
            assert_eq!(item.collection_ids, vec![payload.collections[0].id.clone()]);
        }
        // Nested names are preserved (no folder splitting in org mode).
        let names: Vec<&str> = payload.items.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"work/email"));
        assert!(names.contains(&"flat"));
    }

    #[test]
    fn pretty_false_produces_compact_output() {
        let store = store_with(&[("a", "b")]);

        let output = json(&store, None, None, None, &[], &[], false).unwrap();

        assert!(!output.contains('\n'));
    }

    #[test]
    fn empty_secret_yields_empty_password_and_no_fields() {
        let store = store_with(&[("login", "")]);

        let payload = build_individual(&store, &[], &[]);

        assert_eq!(payload.items.len(), 1);
        let item = &payload.items[0];
        assert_eq!(item.login.password, "");
        assert!(item.login.username.is_empty());
        assert!(item.login.uris.is_empty());
        assert!(item.fields.is_empty());
        assert_eq!(item.notes, "");
    }

    #[test]
    fn key_matching_both_username_and_uri_lists_becomes_username() {
        // The else-if order means a key in both lists is treated as username.
        let store = store_with(&[("login", "hunter2\nuser: alice")]);
        let username_keys = vec![String::from("user")];
        let uri_keys = vec![String::from("user")];

        let payload = build_individual(&store, &username_keys, &uri_keys);

        let item = &payload.items[0];
        assert_eq!(item.login.username, "alice");
        assert!(item.login.uris.is_empty());
    }

    #[test]
    fn multiple_uri_keys_produce_multiple_uris() {
        let store = store_with(&[(
            "login",
            "hunter2\nurl: https://example.com\nbackup: https://backup.example.com",
        )]);
        let uri_keys = vec![String::from("url"), String::from("backup")];

        let payload = build_individual(&store, &[], &uri_keys);

        let uris = &payload.items[0].login.uris;
        assert_eq!(uris.len(), 2);
        let uri_values: Vec<&str> = uris.iter().map(|u| u.uri.as_str()).collect();
        assert!(uri_values.contains(&"https://example.com"));
        assert!(uri_values.contains(&"https://backup.example.com"));
    }

    #[test]
    fn item_uses_login_type_constant() {
        let store = store_with(&[("a", "b")]);

        let payload = build_individual(&store, &[], &[]);

        assert_eq!(payload.items[0].item_type, LOGIN_ITEM_TYPE);
    }

    #[test]
    fn generic_field_uses_text_field_type_constant() {
        let store = store_with(&[("login", "hunter2\nnote: something")]);

        let payload = build_individual(&store, &[], &[]);

        assert_eq!(payload.items[0].fields[0].field_type, TEXT_FIELD_TYPE);
    }

    #[test]
    fn item_ids_are_unique_across_export() {
        let store = store_with(&[("a", "h1"), ("b", "h2"), ("work/c", "h3"), ("work/d", "h4")]);

        let payload = build_individual(&store, &[], &[]);

        let ids: std::collections::HashSet<&str> =
            payload.items.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(ids.len(), payload.items.len());
    }

    #[test]
    fn folder_id_for_returns_none_for_flat_name() {
        let mut folders = BTreeMap::new();
        let result = folder_id_for("flat-secret", &mut folders);
        assert!(result.is_none());
        assert!(folders.is_empty());
    }

    #[test]
    fn folder_id_for_inserts_on_first_call() {
        let mut folders = BTreeMap::new();
        let id = folder_id_for("work/email", &mut folders).unwrap();
        assert_eq!(folders.len(), 1);
        assert_eq!(folders["work"].id, id);
        assert_eq!(folders["work"].name, "work");
    }

    #[test]
    fn folder_id_for_reuses_id_on_second_call_for_same_folder() {
        let mut folders = BTreeMap::new();
        let first = folder_id_for("work/email", &mut folders).unwrap();
        let second = folder_id_for("work/calendar", &mut folders).unwrap();
        assert_eq!(first, second);
        assert_eq!(folders.len(), 1);
    }

    #[test]
    fn folder_id_for_creates_distinct_ids_for_distinct_folders() {
        let mut folders = BTreeMap::new();
        let work = folder_id_for("work/email", &mut folders).unwrap();
        let personal = folder_id_for("personal/email", &mut folders).unwrap();
        assert_ne!(work, personal);
        assert_eq!(folders.len(), 2);
    }

    #[test]
    fn folder_id_for_uses_full_parent_path_as_folder_name() {
        let mut folders = BTreeMap::new();
        folder_id_for("work/dev/api-key", &mut folders);
        assert_eq!(folders.len(), 1);
        assert!(folders.contains_key("work/dev"));
    }

    #[test]
    fn from_secret_sets_defaults_for_org_specific_fields() {
        let item = BitwardenItem::from_secret("name", "password", &[], &[]);

        assert!(item.folder_id.is_none());
        assert!(item.organization_id.is_none());
        assert!(item.collection_ids.is_empty());
        assert_eq!(item.name, "name");
        assert_eq!(item.login.password, "password");
        assert_eq!(item.item_type, LOGIN_ITEM_TYPE);
    }

    #[test]
    fn serialize_pretty_inserts_indentation() {
        let folder = BitwardenFolder {
            id: String::from("id"),
            name: String::from("n"),
        };
        let compact = serialize(&folder, false).unwrap();
        let pretty = serialize(&folder, true).unwrap();
        assert!(!compact.contains('\n'));
        assert!(pretty.contains('\n'));
        assert!(pretty.contains("  "));
    }
}
