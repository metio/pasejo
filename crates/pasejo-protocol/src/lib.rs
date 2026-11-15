// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! The protocol between pasejo clients and `pasejo-server`: wire types, the
//! encrypted object formats of remote stores, and verification of the
//! membership log. Both sides link this crate, so it does no I/O. The design
//! is described in `design/server.md` at the root of the repository.
