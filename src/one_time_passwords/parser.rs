// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::models::password_store::{
    OneTimePassword, OneTimePasswordAlgorithm, OneTimePasswordType,
};
use anyhow::Context;
use otp_std::Otp::{Hotp, Totp};
use otp_std::auth::query::Query;
use otp_std::base::SECRET;
use otp_std::{Otp, Type, auth};
use std::borrow::Cow;
use std::path::PathBuf;

#[allow(clippy::too_many_arguments)]
pub fn parse_otp_args(
    otp_type: Option<&OneTimePasswordType>,
    algorithm: Option<&OneTimePasswordAlgorithm>,
    secret: Option<&String>,
    digits: Option<u8>,
    period: Option<u64>,
    counter: Option<u64>,
    skew: Option<u64>,
    url: Option<&String>,
    qrcode: Option<&PathBuf>,
) -> anyhow::Result<OneTimePassword> {
    if let Some(url) = url {
        // parse otpauth URL
        parse_from_url(url)
    } else if let Some(qrcode) = qrcode {
        // parse otpauth URL from QR code
        let img = image::open(qrcode)?.to_luma8();
        let mut img = rqrr::PreparedImage::prepare(img);
        let grids = img.detect_grids();
        let grid = grids
            .first()
            .ok_or_else(|| anyhow::anyhow!("No QR code found in '{}'", qrcode.display()))?;
        let (_, content) = grid
            .decode()
            .with_context(|| format!("Failed to decode QR code in '{}'", qrcode.display()))?;
        parse_from_url(&content)
    } else {
        Ok(OneTimePassword {
            secret: secret.cloned().unwrap_or_default().to_uppercase(),
            otp_type: otp_type.cloned().unwrap_or_default(),
            algorithm: algorithm.cloned().unwrap_or_default(),
            digits: digits.unwrap_or(6),
            period: period.unwrap_or(30),
            counter: counter.unwrap_or(1),
            skew: skew.unwrap_or(0),
        })
    }
}

fn parse_from_url(url: &str) -> anyhow::Result<OneTimePassword> {
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

    #[test]
    fn defaults_are_filled_in_for_missing_arguments() {
        let result = parse_otp_args(None, None, None, None, None, None, None, None, None).unwrap();
        assert_eq!(result.secret, "");
        assert_eq!(result.otp_type, OneTimePasswordType::Totp);
        assert_eq!(result.algorithm, OneTimePasswordAlgorithm::Sha1);
        assert_eq!(result.digits, 6);
        assert_eq!(result.period, 30);
        assert_eq!(result.counter, 1);
        assert_eq!(result.skew, 0);
    }

    #[test]
    fn secret_is_uppercased() {
        let secret = String::from("jbswy3dpehpk3pxp");
        let result =
            parse_otp_args(None, None, Some(&secret), None, None, None, None, None, None).unwrap();
        assert_eq!(result.secret, "JBSWY3DPEHPK3PXP");
    }

    #[test]
    fn explicit_arguments_override_defaults() {
        let secret = String::from("ABCD");
        let result = parse_otp_args(
            Some(&OneTimePasswordType::Hotp),
            Some(&OneTimePasswordAlgorithm::Sha256),
            Some(&secret),
            Some(8),
            Some(60),
            Some(42),
            Some(2),
            None,
            None,
        )
        .unwrap();
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
        let url = String::from("not-a-url");
        let result =
            parse_otp_args(None, None, None, None, None, None, None, Some(&url), None);
        assert!(result.is_err());
    }

    #[test]
    fn totp_url_yields_totp_one_time_password() {
        let url = String::from(
            "otpauth://totp/Example:alice?secret=JBSWY3DPEHPK3PXP&issuer=Example&algorithm=SHA1&digits=6&period=30",
        );
        let result =
            parse_otp_args(None, None, None, None, None, None, None, Some(&url), None).unwrap();
        assert_eq!(result.otp_type, OneTimePasswordType::Totp);
        assert_eq!(result.algorithm, OneTimePasswordAlgorithm::Sha1);
        assert_eq!(result.digits, 6);
        assert_eq!(result.period, 30);
        assert_eq!(result.secret, "JBSWY3DPEHPK3PXP");
    }

    #[test]
    fn hotp_url_yields_hotp_with_counter_from_url() {
        let url = String::from(
            "otpauth://hotp/Example:alice?secret=JBSWY3DPEHPK3PXP&issuer=Example&algorithm=SHA1&digits=6&counter=5",
        );
        let result =
            parse_otp_args(None, None, None, None, None, None, None, Some(&url), None).unwrap();
        assert_eq!(result.otp_type, OneTimePasswordType::Hotp);
        assert_eq!(result.counter, 5);
    }

    #[test]
    fn url_takes_precedence_over_other_arguments() {
        // Even when manual arguments are provided, the url path should be taken
        // and the manual arguments ignored.
        let manual_secret = String::from("ZZZZ");
        let url = String::from(
            "otpauth://totp/Example:alice?secret=JBSWY3DPEHPK3PXP&algorithm=SHA1&digits=6&period=30",
        );
        let result = parse_otp_args(
            Some(&OneTimePasswordType::Hotp),
            Some(&OneTimePasswordAlgorithm::Sha512),
            Some(&manual_secret),
            Some(8),
            Some(60),
            None,
            None,
            Some(&url),
            None,
        )
        .unwrap();
        assert_eq!(result.otp_type, OneTimePasswordType::Totp);
        assert_eq!(result.secret, "JBSWY3DPEHPK3PXP");
    }

    #[test]
    fn url_with_lowercase_secret_is_uppercased() {
        let url = String::from(
            "otpauth://totp/Example:alice?secret=jbswy3dpehpk3pxp&algorithm=SHA1&digits=6&period=30",
        );
        let result =
            parse_otp_args(None, None, None, None, None, None, None, Some(&url), None).unwrap();
        assert_eq!(result.secret, "JBSWY3DPEHPK3PXP");
    }
}
