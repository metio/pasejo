// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

mod callbacks;
mod files;

pub use self::files::{is_passphrase_protected, read, read_without_terminal};
