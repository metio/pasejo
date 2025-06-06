// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::models::password_store::{
    OneTimePassword, OneTimePasswordAlgorithm, OneTimePasswordType,
};
use image;
use otp_std::Auth;
use otp_std::Otp::{Hotp, Totp};
use rqrr;
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
        let (_, content) = grids[0].decode()?;
        parse_from_url(&content)
    } else {
        Ok(OneTimePassword {
            secret: secret.cloned().unwrap_or_default(),
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
    let parsed = Auth::parse_url(url)?;
    match parsed.otp {
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
            otp_type: OneTimePasswordType::Hotp,
            algorithm: totp.base.algorithm.into(),
            digits: totp.base.digits.into(),
            period: 0,
            counter: 0,
            skew: 0,
        }),
    }
}
