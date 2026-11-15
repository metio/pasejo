// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The pasejo server: organizations, remote stores and their audit trail.
//! `design/server.md` in the repository describes the design.

// Handler errors are the documented HTTP error responses, and query maps
// come from axum with the default hasher.
#![allow(clippy::missing_errors_doc, clippy::implicit_hasher)]

pub mod actions;
pub mod api;
pub mod app;
pub mod audit;
pub mod auth;
pub mod clock;
pub mod config;
pub mod console;
pub mod db;
pub mod error;
pub mod html;
pub mod jwt;
pub mod net;
pub mod oidc;
pub mod scim;
pub mod state;
pub mod time;
pub mod totp;
pub mod users;
