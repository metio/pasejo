// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! A blocking client for `pasejo-server`.
//!
//! The client signs every request with the device key, verifies everything
//! the server returns against the organization's membership log, and builds
//! commits from changes to a store's decrypted content. It holds no state
//! on disk: callers keep the session, the pinned log and the pinned index
//! versions between runs.

pub mod actions;
pub mod audit;
pub mod error;
#[cfg(test)]
mod fixtures;
pub mod keys;
pub mod org;
pub mod recipient;
pub mod reviews;
pub mod session;
pub mod store;
pub mod transport;

pub use crate::error::{Error, Result};
pub use crate::transport::{Client, Clock, DeviceIdentity, Session};
