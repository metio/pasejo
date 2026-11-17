// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use otp_std::{Algorithm, Base, Digits, Period, Secret, Skew, Totp};
use pasejo_protocol::store::OtpParams;
use zeroize::Zeroizing;

use crate::error::MobileError;

/// A TOTP code and the seconds until the next one.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TotpCode {
    pub code: String,
    pub remaining_seconds: u64,
}

/// The TOTP code of `params` at `time` seconds since the Unix epoch.
///
/// HOTP entries are refused: showing a code without storing the bumped
/// counter would hand out the same code twice, which RFC 4226 forbids.
pub fn totp_at(params: &OtpParams, time: u64) -> Result<TotpCode, MobileError> {
    if params.otp_type != "totp" {
        return Err(MobileError::invalid(
            "only TOTP codes can be shown on this device",
        ));
    }
    let algorithm = match params.algorithm.as_str() {
        "sha1" => Algorithm::Sha1,
        "sha256" => Algorithm::Sha256,
        "sha512" => Algorithm::Sha512,
        _ => return Err(MobileError::invalid("unknown one-time password algorithm")),
    };
    let invalid = || MobileError::invalid("invalid one-time password");
    // otp-std's decode error embeds the secret, so it is replaced.
    let secret = Secret::decode(&params.secret).map_err(|_| invalid())?;
    let base = Base::builder()
        .secret(secret)
        .digits(Digits::new(params.digits).map_err(|_| invalid())?)
        .algorithm(algorithm)
        .build();
    let totp = Totp::builder()
        .base(base)
        .period(Period::new(params.period).map_err(|_| invalid())?)
        .skew(Skew::new(params.skew))
        .build();
    let code = Zeroizing::new(totp.generate_string_at(time));
    Ok(TotpCode {
        code: code.to_string(),
        remaining_seconds: totp.time_to_live_at(time),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn params(algorithm: &str, digits: u8) -> OtpParams {
        OtpParams {
            // RFC 6238's SHA-1 test secret, "12345678901234567890".
            secret: "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ".into(),
            otp_type: "totp".into(),
            algorithm: algorithm.into(),
            digits,
            period: 30,
            counter: 0,
            skew: 0,
        }
    }

    #[test]
    fn codes_match_the_rfc_6238_test_vectors() {
        for (time, code) in [
            (59, "94287082"),
            (1_111_111_109, "07081804"),
            (20_000_000_000, "65353130"),
        ] {
            assert_eq!(
                totp_at(&params("sha1", 8), time).unwrap().code,
                code,
                "{time}"
            );
        }
    }

    #[test]
    fn six_digit_codes_keep_leading_zeros() {
        assert_eq!(
            totp_at(&params("sha1", 6), 1_111_111_109).unwrap().code,
            "081804"
        );
    }

    #[test]
    fn the_remaining_seconds_count_down_to_the_next_period() {
        assert_eq!(
            totp_at(&params("sha1", 6), 59).unwrap().remaining_seconds,
            1
        );
        assert_eq!(
            totp_at(&params("sha1", 6), 60).unwrap().remaining_seconds,
            30
        );
    }

    #[test]
    fn hotp_entries_are_refused() {
        let mut hotp = params("sha1", 6);
        hotp.otp_type = "hotp".into();
        assert!(totp_at(&hotp, 59).is_err());
    }

    #[test]
    fn malformed_entries_are_refused_without_echoing_the_secret() {
        let mut broken = params("md5", 6);
        assert!(totp_at(&broken, 59).is_err());
        broken = params("sha1", 6);
        broken.secret = "not base32 !!".into();
        let error = totp_at(&broken, 59).unwrap_err();
        assert!(!error.to_string().contains("base32"), "{error}");
        broken = params("sha1", 6);
        broken.period = 0;
        assert!(totp_at(&broken, 59).is_err());
    }
}
