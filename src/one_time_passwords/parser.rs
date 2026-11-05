// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::borrow::Cow;

use otp_std::Otp::{Hotp, Totp};
use otp_std::auth::query::Query;
use otp_std::base::SECRET;
use otp_std::{Otp, Secret, Type, auth};

use crate::cli::{i18n, sandbox};
use crate::models::cli::OtpAddArgs;
use crate::models::password_store::{OneTimePassword, OneTimePasswordType};

impl OtpAddArgs {
    /// Resolve the user-supplied OTP arguments into a concrete
    /// [`OneTimePassword`]. Precedence: `--url` over `--qrcode` over the
    /// individual `--type`/`--algorithm`/`--secret`/… flags; clap already
    /// enforces that `--url` and `--qrcode` are mutually exclusive and
    /// that neither coexists with the manual flags.
    ///
    /// QR-code input is decoded through
    /// [`sandbox::decode_qrcode_to_otpauth_url`], which on Linux runs the
    /// `image` + `rqrr` parsers in a forked child process restricted by
    /// Landlock so a memory-corruption bug in either crate cannot
    /// exfiltrate identity files or the store.
    pub fn parse_password(&self) -> anyhow::Result<OneTimePassword> {
        if let Some(url) = &self.url {
            return parse_from_url(url);
        }
        if let Some(qrcode) = &self.qrcode {
            let content = sandbox::decode_qrcode_to_otpauth_url(qrcode)?;
            return parse_from_url(&content);
        }
        let secret = self.secret.as_deref().unwrap_or_default().to_uppercase();
        if !Secret::decode(&secret).is_ok_and(|decoded| !decoded.as_bytes().is_empty()) {
            anyhow::bail!(i18n::error_invalid_one_time_password_secret());
        }
        Ok(OneTimePassword {
            secret,
            otp_type: self.otp_type.clone().unwrap_or_default(),
            algorithm: self.algorithm.clone().unwrap_or_default(),
            digits: self.digits.unwrap_or(6),
            period: self.period.unwrap_or(30),
            counter: self.counter.unwrap_or(0),
            skew: self.skew.unwrap_or(0),
        })
    }
}

/// otp-std's errors are English and the secret-decode error embeds the
/// secret, so every failure is replaced by one localized error.
fn parse_from_url(url: &str) -> anyhow::Result<OneTimePassword> {
    extract_from_url(url).map_err(|_| anyhow::anyhow!(i18n::error_invalid_otpauth_url()))
}

fn extract_from_url(url: &str) -> anyhow::Result<OneTimePassword> {
    let url = auth::url::parse(url)?;
    auth::scheme::check_url(&url)?;
    let mut query: Query<'_> = url.query_pairs().collect();
    if let Some(secret) = query.get(SECRET) {
        query.insert(Cow::from(SECRET), Cow::from(secret.to_uppercase()));
    }
    let type_of = Type::extract_from(&url)?;
    let otp = Otp::extract_from(&mut query, type_of)?;

    match otp {
        Hotp(hotp) => Ok(OneTimePassword {
            secret: hotp.base.secret.to_string(),
            otp_type: OneTimePasswordType::Hotp,
            algorithm: hotp.base.algorithm.into(),
            digits: hotp.base.digits.into(),
            period: 0,
            counter: hotp.counter.into(),
            skew: 0,
        }),
        Totp(totp) => Ok(OneTimePassword {
            secret: totp.base.secret.to_string(),
            otp_type: OneTimePasswordType::Totp,
            algorithm: totp.base.algorithm.into(),
            digits: totp.base.digits.into(),
            period: totp.period.into(),
            counter: 0,
            skew: totp.skew.into(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::cli::StoreSelectionArgs;
    use crate::models::password_store::OneTimePasswordAlgorithm;

    fn empty_args() -> OtpAddArgs {
        OtpAddArgs {
            store_selection: StoreSelectionArgs { store: None },
            force: false,
            url: None,
            qrcode: None,
            secret: None,
            otp_type: None,
            algorithm: None,
            digits: None,
            period: None,
            skew: None,
            counter: None,
            password_path: String::new(),
        }
    }

    #[test]
    fn otpauth_url_round_trips_through_the_url_parser() {
        let entries = [
            OneTimePassword {
                secret: String::from("JBSWY3DPEHPK3PXP"),
                otp_type: OneTimePasswordType::Totp,
                algorithm: OneTimePasswordAlgorithm::Sha256,
                digits: 8,
                period: 60,
                counter: 0,
                skew: 1,
            },
            OneTimePassword {
                secret: String::from("JBSWY3DPEHPK3PXP"),
                otp_type: OneTimePasswordType::Hotp,
                algorithm: OneTimePasswordAlgorithm::Sha512,
                digits: 7,
                period: 0,
                counter: 42,
                skew: 0,
            },
        ];
        for entry in entries {
            let args = OtpAddArgs {
                url: Some(entry.otpauth_url("work/Example: alice")),
                ..empty_args()
            };
            assert_eq!(args.parse_password().unwrap(), entry);
        }
    }

    #[test]
    fn defaults_are_filled_in_for_missing_arguments() {
        let args = OtpAddArgs {
            secret: Some(String::from("JBSWY3DPEHPK3PXP")),
            ..empty_args()
        };
        let result = args.parse_password().unwrap();
        assert_eq!(result.secret, "JBSWY3DPEHPK3PXP");
        assert_eq!(result.otp_type, OneTimePasswordType::Totp);
        assert_eq!(result.algorithm, OneTimePasswordAlgorithm::Sha1);
        assert_eq!(result.digits, 6);
        assert_eq!(result.period, 30);
        assert_eq!(result.counter, 0);
        assert_eq!(result.skew, 0);
    }

    #[test]
    fn missing_secret_is_rejected() {
        crate::cli::i18n::init_for_tests();
        assert!(empty_args().parse_password().is_err());
    }

    #[test]
    fn empty_secret_is_rejected() {
        crate::cli::i18n::init_for_tests();
        let args = OtpAddArgs {
            secret: Some(String::new()),
            ..empty_args()
        };
        assert!(args.parse_password().is_err());
    }

    #[test]
    fn secret_that_is_not_base32_is_rejected_without_echoing_it() {
        crate::cli::i18n::init_for_tests();
        let args = OtpAddArgs {
            secret: Some(String::from("JBSWY3DPEHPK3PX0")),
            ..empty_args()
        };
        let err = args.parse_password().unwrap_err();
        assert!(!format!("{err:?}").contains("JBSWY3DPEHPK3PX0"), "{err:?}");
    }

    #[test]
    fn secret_is_uppercased() {
        let args = OtpAddArgs {
            secret: Some(String::from("jbswy3dpehpk3pxp")),
            ..empty_args()
        };
        assert_eq!(args.parse_password().unwrap().secret, "JBSWY3DPEHPK3PXP");
    }

    #[test]
    fn explicit_arguments_override_defaults() {
        let args = OtpAddArgs {
            secret: Some(String::from("ABCD")),
            otp_type: Some(OneTimePasswordType::Hotp),
            algorithm: Some(OneTimePasswordAlgorithm::Sha256),
            digits: Some(8),
            period: Some(60),
            counter: Some(42),
            skew: Some(2),
            ..empty_args()
        };
        let result = args.parse_password().unwrap();
        assert_eq!(result.secret, "ABCD");
        assert_eq!(result.otp_type, OneTimePasswordType::Hotp);
        assert_eq!(result.algorithm, OneTimePasswordAlgorithm::Sha256);
        assert_eq!(result.digits, 8);
        assert_eq!(result.period, 60);
        assert_eq!(result.counter, 42);
        assert_eq!(result.skew, 2);
    }

    #[test]
    fn invalid_url_is_rejected() {
        let args = OtpAddArgs {
            url: Some(String::from("not-a-url")),
            ..empty_args()
        };
        assert!(args.parse_password().is_err());
    }

    #[test]
    fn url_with_undecodable_secret_is_rejected_without_echoing_it() {
        crate::cli::i18n::init_for_tests();
        let args = OtpAddArgs {
            url: Some(String::from(
                "otpauth://totp/Example:alice?secret=JBSWY3DPEHPK3PX0&algorithm=SHA1&digits=6&period=30",
            )),
            ..empty_args()
        };
        let err = args.parse_password().unwrap_err();
        assert!(!format!("{err:?}").contains("JBSWY3DPEHPK3PX0"), "{err:?}");
        assert!(
            err.to_string().contains("otpauth"),
            "expected the localized URL error, got: {err}"
        );
    }

    #[test]
    fn totp_url_yields_totp_one_time_password() {
        let args = OtpAddArgs {
            url: Some(String::from(
                "otpauth://totp/Example:alice?secret=JBSWY3DPEHPK3PXP&issuer=Example&algorithm=SHA1&digits=6&period=30",
            )),
            ..empty_args()
        };
        let result = args.parse_password().unwrap();
        assert_eq!(result.otp_type, OneTimePasswordType::Totp);
        assert_eq!(result.algorithm, OneTimePasswordAlgorithm::Sha1);
        assert_eq!(result.digits, 6);
        assert_eq!(result.period, 30);
        assert_eq!(result.secret, "JBSWY3DPEHPK3PXP");
    }

    #[test]
    fn hotp_url_yields_hotp_with_counter_from_url() {
        let args = OtpAddArgs {
            url: Some(String::from(
                "otpauth://hotp/Example:alice?secret=JBSWY3DPEHPK3PXP&issuer=Example&algorithm=SHA1&digits=6&counter=5",
            )),
            ..empty_args()
        };
        let result = args.parse_password().unwrap();
        assert_eq!(result.otp_type, OneTimePasswordType::Hotp);
        assert_eq!(result.counter, 5);
    }

    #[test]
    fn hotp_url_counter_emits_first_code_at_that_counter_value() {
        // RFC 4226 Appendix D: HOTP(ASCII "12345678901234567890", 5) = 254676.
        // A URL importing counter=5 must produce that exact code on the first
        // call, not the code at counter=6 (which is what a pre-increment
        // implementation would emit).
        let args = OtpAddArgs {
            url: Some(String::from(
                "otpauth://hotp/Example:alice?secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ&algorithm=SHA1&digits=6&counter=5",
            )),
            ..empty_args()
        };
        let mut otp = args.parse_password().unwrap();
        assert_eq!(otp.generate().unwrap(), 254_676);
        assert_eq!(otp.counter, 6);
    }

    #[test]
    fn url_takes_precedence_over_other_arguments() {
        // The url path is taken even when manual arguments are also set;
        // clap normally rejects this combination, but the resolver is
        // defensive: url wins.
        let args = OtpAddArgs {
            url: Some(String::from(
                "otpauth://totp/Example:alice?secret=JBSWY3DPEHPK3PXP&algorithm=SHA1&digits=6&period=30",
            )),
            secret: Some(String::from("ZZZZ")),
            otp_type: Some(OneTimePasswordType::Hotp),
            algorithm: Some(OneTimePasswordAlgorithm::Sha512),
            digits: Some(8),
            period: Some(60),
            ..empty_args()
        };
        let result = args.parse_password().unwrap();
        assert_eq!(result.otp_type, OneTimePasswordType::Totp);
        assert_eq!(result.secret, "JBSWY3DPEHPK3PXP");
    }

    #[test]
    fn url_with_lowercase_secret_is_uppercased() {
        let args = OtpAddArgs {
            url: Some(String::from(
                "otpauth://totp/Example:alice?secret=jbswy3dpehpk3pxp&algorithm=SHA1&digits=6&period=30",
            )),
            ..empty_args()
        };
        assert_eq!(args.parse_password().unwrap().secret, "JBSWY3DPEHPK3PXP");
    }
}
