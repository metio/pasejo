// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! The Rust core of the pasejo apps.
//!
//! Every check the CLI makes on what a server returns runs here as the same
//! code, through `pasejo-client`. The apps hold the device keys in their
//! platform's key store and implement [`DeviceSigner`] and [`KeyAgreement`]
//! over them; they keep the [`Device`] state encrypted at rest and contain no
//! protocol logic.

mod device;
mod error;
mod fill;
mod keys;
mod links;
mod offline;
mod otp;

pub use device::{Device, Enrollment, OrgInfo, ServerInfo, StoreView};
pub use error::MobileError;
pub use fill::{
    AppRequest, DomainVerifier, FillCredential, FillMatch, FillRequest, QueryResult,
    package_domains,
};
pub use keys::{
    AgreementIdentity, DeviceSigner, KeyAgreement, SignerAdapter, generate_secret, recipient,
    software_agreement, software_signer,
};
pub use links::{verify_app_domain, verify_statements_at};
pub use otp::TotpCode;

uniffi::setup_scaffolding!();
