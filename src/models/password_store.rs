// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use otp_std::{Algorithm, Base, Counter, Digits, Hotp, Period, Secret, Skew, Totp};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct PasswordStore {
    /// Recipients used in this store
    pub recipients: Vec<Recipient>,

    /// Secrets available in this store
    pub secrets: BTreeMap<String, String>,

    /// One-time passwords available in this store
    pub otp: BTreeMap<String, OneTimePassword>,
}

impl PasswordStore {
    pub fn secret_names_as_list(&self) -> Vec<String> {
        self.secrets.keys().cloned().collect()
    }

    pub fn otp_names_as_list(&self) -> Vec<String> {
        self.otp.keys().cloned().collect()
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Recipient {
    pub name: String,
    pub public_key: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct OneTimePassword {
    pub secret: String,
    pub otp_type: OneTimePasswordType,
    pub algorithm: OneTimePasswordAlgorithm,
    pub digits: u8,
    pub period: u64,
    pub counter: u64,
    pub skew: u64,
}

impl OneTimePassword {
    pub fn generate(&mut self) -> anyhow::Result<u32> {
        match &self.otp_type {
            OneTimePasswordType::Totp => self.generate_totp(),
            OneTimePasswordType::Hotp => self.generate_hotp(),
        }
    }

    fn generate_totp(&self) -> anyhow::Result<u32> {
        let code = Totp::builder()
            .base(self.base()?)
            .period(Period::new(self.period)?)
            .skew(Skew::new(self.skew))
            .build()
            .generate();

        Ok(code)
    }

    fn generate_hotp(&mut self) -> anyhow::Result<u32> {
        self.counter += 1;

        let code = Hotp::builder()
            .base(self.base()?)
            .counter(Counter::new(self.counter))
            .build()
            .generate();

        Ok(code)
    }

    fn base(&self) -> anyhow::Result<Base> {
        let secret = Secret::decode(&self.secret)?;
        let base = Base::builder()
            .secret(secret)
            .digits(Digits::new(self.digits)?)
            .algorithm(self.algorithm.clone().into())
            .build();
        Ok(base)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Default, Clone, clap::ValueEnum)]
pub enum OneTimePasswordType {
    #[default]
    Totp,
    Hotp,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Default, Clone, clap::ValueEnum)]
pub enum OneTimePasswordAlgorithm {
    #[default]
    Sha1,
    Sha256,
    Sha512,
}

impl From<OneTimePasswordAlgorithm> for Algorithm {
    fn from(value: OneTimePasswordAlgorithm) -> Self {
        match value {
            OneTimePasswordAlgorithm::Sha1 => Self::Sha1,
            OneTimePasswordAlgorithm::Sha256 => Self::Sha256,
            OneTimePasswordAlgorithm::Sha512 => Self::Sha512,
        }
    }
}

impl From<Algorithm> for OneTimePasswordAlgorithm {
    fn from(value: Algorithm) -> Self {
        match value {
            Algorithm::Sha1 => Self::Sha1,
            Algorithm::Sha256 => Self::Sha256,
            Algorithm::Sha512 => Self::Sha512,
        }
    }
}
