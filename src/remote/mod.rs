// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Remote stores on a pasejo server: this installation's device on each
//! server, its session, the organization logs it verified, and the funnel
//! that turns a command's changes to a `PasswordStore` into commits.

pub mod connect;
pub mod device;
pub mod errors;
pub mod funnel;
