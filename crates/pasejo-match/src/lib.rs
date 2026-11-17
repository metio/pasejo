// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Matches stored credentials against the domain a browser or an app asks
//! for. A query returns metadata only; [`matcher::reveal`] releases a
//! credential only for a secret the query returns for the same domain.

pub mod lookalike;
pub mod matcher;
pub mod parser;
pub mod policy;
