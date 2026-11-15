// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

//! RFC 6238 time-based one-time passwords for the break-glass account.

use hmac::{Hmac, Mac};
use pasejo_protocol::Millis;
use sha1::Sha1;

const STEP_MILLIS: u64 = 30_000;
const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/// The six-digit code for time step `step`.
#[must_use]
pub fn code(secret: &[u8], step: u64) -> String {
    let mut mac = Hmac::<Sha1>::new_from_slice(secret)
        .unwrap_or_else(|_| unreachable!("HMAC takes any key length"));
    mac.update(&step.to_be_bytes());
    let digest = mac.finalize().into_bytes();
    let offset = usize::from(digest[19] & 0x0f);
    let binary = (u32::from(digest[offset]) & 0x7f) << 24
        | u32::from(digest[offset + 1]) << 16
        | u32::from(digest[offset + 2]) << 8
        | u32::from(digest[offset + 3]);
    format!("{:06}", binary % 1_000_000)
}

/// The time step a code typed at `now` most likely belongs to.
#[must_use]
pub const fn step(now: Millis) -> u64 {
    now / STEP_MILLIS
}

/// Finds the step within one step of `now` whose code is `candidate`, and
/// refuses steps at or before `last_used` so a code works once.
#[must_use]
pub fn verify(secret: &[u8], candidate: &str, now: Millis, last_used: u64) -> Option<u64> {
    let current = step(now);
    [current.saturating_sub(1), current, current + 1]
        .into_iter()
        .filter(|step| *step > last_used)
        .find(|step| constant_time_eq(code(secret, *step).as_bytes(), candidate.trim().as_bytes()))
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len() && left.iter().zip(right).fold(0, |acc, (a, b)| acc | (a ^ b)) == 0
}

/// RFC 4648 base32 without padding, as authenticator apps expect.
#[must_use]
pub fn base32(bytes: &[u8]) -> String {
    let mut text = String::new();
    let mut buffer = 0u16;
    let mut bits = 0;
    for byte in bytes {
        buffer = (buffer << 8) | u16::from(*byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            text.push(char::from(ALPHABET[usize::from((buffer >> bits) & 31)]));
        }
    }
    if bits > 0 {
        text.push(char::from(
            ALPHABET[usize::from((buffer << (5 - bits)) & 31)],
        ));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 6238 appendix B, SHA-1 secret "12345678901234567890", truncated to
    // the six digits pasejo uses.
    const SECRET: &[u8] = b"12345678901234567890";

    #[test]
    fn codes_match_the_rfc_vectors() {
        assert_eq!(code(SECRET, 59 / 30), "287082");
        assert_eq!(code(SECRET, 1_111_111_109 / 30), "081804");
        assert_eq!(code(SECRET, 1_234_567_890 / 30), "005924");
        assert_eq!(code(SECRET, 2_000_000_000 / 30), "279037");
    }

    #[test]
    fn codes_verify_within_one_step_and_only_once() {
        let now = 1_234_567_890_000;
        let current = code(SECRET, step(now));
        assert_eq!(verify(SECRET, &current, now, 0), Some(step(now)));
        assert_eq!(verify(SECRET, &current, now + 30_000, 0), Some(step(now)));
        assert_eq!(verify(SECRET, &current, now + 90_000, 0), None);
        assert_eq!(verify(SECRET, &current, now, step(now)), None);
        assert_eq!(
            verify(SECRET, "000000", now, 0).is_some(),
            code(SECRET, step(now)) == "000000"
        );
        assert_eq!(verify(SECRET, "12345", now, 0), None);
    }

    #[test]
    fn base32_matches_rfc_4648() {
        assert_eq!(base32(b""), "");
        assert_eq!(base32(b"f"), "MY");
        assert_eq!(base32(b"fo"), "MZXQ");
        assert_eq!(base32(b"foo"), "MZXW6");
        assert_eq!(base32(b"foobar"), "MZXW6YTBOI");
    }
}
