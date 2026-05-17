// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

pub const APPLICATION_NAME: &str = env!("CARGO_PKG_NAME");

pub const DEFAULT_USERNAME_KEYS: &[&str] = &["login", "email", "username"];

pub const DEFAULT_URI_KEYS: &[&str] = &["uri", "url", "link", "site"];

pub fn default_username_keys() -> Vec<String> {
    DEFAULT_USERNAME_KEYS
        .iter()
        .map(|s| (*s).to_owned())
        .collect()
}

pub fn default_uri_keys() -> Vec<String> {
    DEFAULT_URI_KEYS.iter().map(|s| (*s).to_owned()).collect()
}
