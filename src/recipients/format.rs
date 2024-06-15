// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

pub fn format_recipient(public_key: &str, name: &str) -> String {
    if name.is_empty() {
        public_key.to_owned()
    } else {
        format!("# {name}\n{public_key}")
    }
}
