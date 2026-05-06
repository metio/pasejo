// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use otp_std::{Algorithm, Base, Counter, Digits, Hotp, Period, Secret, Skew, Totp};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct PasswordStore {
    /// Recipients used in this store
    pub recipients: Vec<Recipient>,

    /// Secrets available in this store
    pub secrets: BTreeMap<String, String>,

    /// One-time passwords available in this store
    pub otp: BTreeMap<String, OneTimePassword>,
}

impl Zeroize for PasswordStore {
    fn zeroize(&mut self) {
        self.recipients.zeroize();

        self.secrets
            .values_mut()
            .for_each(zeroize::Zeroize::zeroize);
        self.secrets.clear();

        self.otp.values_mut().for_each(zeroize::Zeroize::zeroize);
        self.otp.clear();
    }
}

impl ZeroizeOnDrop for PasswordStore {}

impl PasswordStore {
    pub fn secret_names_as_list(&self) -> Vec<String> {
        self.secrets.keys().cloned().collect()
    }

    pub fn otp_names_as_list(&self) -> Vec<String> {
        self.otp.keys().cloned().collect()
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct Recipient {
    pub name: String,
    pub public_key: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct OneTimePassword {
    pub secret: String,
    #[zeroize(skip)]
    pub otp_type: OneTimePasswordType,
    #[zeroize(skip)]
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

    fn base(&self) -> anyhow::Result<Base<'_>> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_otp() -> OneTimePassword {
        OneTimePassword {
            secret: String::from("JBSWY3DPEHPK3PXP"),
            otp_type: OneTimePasswordType::Totp,
            algorithm: OneTimePasswordAlgorithm::Sha1,
            digits: 6,
            period: 30,
            counter: 0,
            skew: 0,
        }
    }

    fn populated_store() -> PasswordStore {
        let mut store = PasswordStore::default();
        store.recipients.push(Recipient {
            name: String::from("alice"),
            public_key: String::from("age1xyz"),
        });
        store
            .secrets
            .insert(String::from("aaa"), String::from("hunter2"));
        store
            .secrets
            .insert(String::from("bbb"), String::from("hunter3"));
        store.otp.insert(String::from("github"), sample_otp());
        store
    }

    #[test]
    fn secret_names_as_list_returns_keys_in_btree_order() {
        let store = populated_store();
        assert_eq!(
            store.secret_names_as_list(),
            vec![String::from("aaa"), String::from("bbb")]
        );
    }

    #[test]
    fn secret_names_as_list_is_empty_for_default_store() {
        assert!(PasswordStore::default().secret_names_as_list().is_empty());
    }

    #[test]
    fn otp_names_as_list_returns_keys() {
        let store = populated_store();
        assert_eq!(store.otp_names_as_list(), vec![String::from("github")]);
    }

    #[test]
    fn otp_names_as_list_is_empty_for_default_store() {
        assert!(PasswordStore::default().otp_names_as_list().is_empty());
    }

    #[test]
    fn zeroize_clears_recipients_secrets_and_otp() {
        let mut store = populated_store();
        store.zeroize();
        assert!(store.recipients.is_empty());
        assert!(store.secrets.is_empty());
        assert!(store.otp.is_empty());
    }

    #[test]
    fn algorithm_round_trips_through_otp_std_algorithm() {
        for original in [
            OneTimePasswordAlgorithm::Sha1,
            OneTimePasswordAlgorithm::Sha256,
            OneTimePasswordAlgorithm::Sha512,
        ] {
            let converted: Algorithm = original.clone().into();
            let back: OneTimePasswordAlgorithm = converted.into();
            assert_eq!(original, back);
        }
    }

    #[test]
    fn generate_hotp_increments_counter_each_call() {
        let mut otp = OneTimePassword {
            secret: String::from("JBSWY3DPEHPK3PXP"),
            otp_type: OneTimePasswordType::Hotp,
            algorithm: OneTimePasswordAlgorithm::Sha1,
            digits: 6,
            period: 30,
            counter: 0,
            skew: 0,
        };
        otp.generate().unwrap();
        assert_eq!(otp.counter, 1);
        otp.generate().unwrap();
        assert_eq!(otp.counter, 2);
    }

    #[test]
    fn generate_totp_does_not_change_counter() {
        let mut otp = sample_otp();
        let initial_counter = otp.counter;
        otp.generate().unwrap();
        assert_eq!(otp.counter, initial_counter);
    }

    #[test]
    fn generate_totp_returns_value_within_digit_range() {
        let mut otp = sample_otp();
        let code = otp.generate().unwrap();
        assert!(
            code < 10u32.pow(u32::from(otp.digits)),
            "code {code} exceeds {} digits",
            otp.digits
        );
    }

    #[test]
    fn generate_hotp_returns_value_within_digit_range() {
        let mut otp = OneTimePassword {
            secret: String::from("JBSWY3DPEHPK3PXP"),
            otp_type: OneTimePasswordType::Hotp,
            algorithm: OneTimePasswordAlgorithm::Sha1,
            digits: 6,
            period: 30,
            counter: 0,
            skew: 0,
        };
        let code = otp.generate().unwrap();
        assert!(code < 10u32.pow(u32::from(otp.digits)));
    }

    #[test]
    fn default_otp_type_is_totp() {
        assert_eq!(OneTimePasswordType::default(), OneTimePasswordType::Totp);
    }

    #[test]
    fn default_algorithm_is_sha1() {
        assert_eq!(
            OneTimePasswordAlgorithm::default(),
            OneTimePasswordAlgorithm::Sha1
        );
    }
}
