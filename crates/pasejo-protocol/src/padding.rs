// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Padmé padding above a floor.
//!
//! Every padded plaintext is at least [`MIN_PADDED`] bytes, so the server
//! cannot tell typical secrets apart by size. Above the floor the scheme is
//! Padmé from Nikitin et al., "Reducing Metadata Leakage from Encrypted Files
//! and Communication with PURBs", 2019: a padded length reveals O(log log L)
//! bits of the real length L, and the overhead stays below 12%.

use zeroize::Zeroizing;

use crate::Error;

/// The smallest padded plaintext. A secret with its URL, user name and a few
/// lines of notes fits, so all of those look the same size.
pub const MIN_PADDED: u64 = 1024;

/// The padded length for `length` bytes: [`MIN_PADDED`] or its Padmé length,
/// whichever is larger.
#[must_use]
pub const fn padded_length(length: u64) -> u64 {
    let padded = padme(length);
    if padded < MIN_PADDED {
        MIN_PADDED
    } else {
        padded
    }
}

/// The Padmé length for `length` bytes.
#[must_use]
pub const fn padme(length: u64) -> u64 {
    if length < 2 {
        return length;
    }
    let exponent = 63 - length.leading_zeros() as u64;
    let bits = 64 - exponent.leading_zeros() as u64;
    let mask = (1u64 << (exponent - bits)) - 1;
    (length + mask) & !mask
}

/// Prefixes `plaintext` with its 4-byte big-endian length and appends zero
/// bytes up to the padded length of the result.
///
/// # Errors
///
/// Fails for plaintexts of 4 GiB or more.
pub fn pad(plaintext: &[u8]) -> Result<Zeroizing<Vec<u8>>, Error> {
    let length =
        u32::try_from(plaintext.len()).map_err(|_| Error::Invalid("plaintext too long"))?;
    let total = usize::try_from(padded_length(u64::from(length) + 4))
        .map_err(|_| Error::Invalid("plaintext too long"))?;
    let mut padded = Zeroizing::new(Vec::with_capacity(total));
    padded.extend_from_slice(&length.to_be_bytes());
    padded.extend_from_slice(plaintext);
    padded.resize(total, 0);
    Ok(padded)
}

/// Strips the padding added by [`pad`]. A padded plaintext whose length is
/// not exactly the padded length, or whose padding is not zero, is rejected,
/// so each plaintext has a single valid padded form.
///
/// # Errors
///
/// Fails on a malformed padded plaintext.
pub fn unpad(padded: &[u8]) -> Result<&[u8], Error> {
    let (prefix, rest) = padded
        .split_first_chunk::<4>()
        .ok_or(Error::Encoding("padding"))?;
    let length = u32::from_be_bytes(*prefix) as usize;
    if length > rest.len()
        || padded_length(length as u64 + 4) != padded.len() as u64
        || rest[length..].iter().any(|byte| *byte != 0)
    {
        return Err(Error::Encoding("padding"));
    }
    Ok(&rest[..length])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_lengths_are_unchanged() {
        for length in 0..=8 {
            assert_eq!(padme(length), length);
        }
    }

    #[test]
    fn padme_matches_the_reference_values() {
        // Values worked out by hand from the formula in the paper.
        let cases = [
            (9, 10),
            (10, 10),
            (11, 12),
            (17, 18),
            (33, 36),
            (100, 104),
            (1000, 1024),
            (1025, 1088),
            (5000, 5120),
            (65_537, 67_584),
        ];
        for (length, expected) in cases {
            assert_eq!(padme(length), expected, "padme({length})");
        }
    }

    #[test]
    fn padme_never_shrinks_and_stays_within_twelve_percent() {
        for length in 2..20_000u64 {
            let padded = padme(length);
            assert!(padded >= length);
            assert!(padded * 100 <= length * 112 + 100, "{length} -> {padded}");
        }
    }

    #[test]
    fn padding_round_trips() {
        for size in [0usize, 1, 5, 60, 61, 300, 4096] {
            let plaintext: Vec<u8> = (0..size).map(|index| (index % 251) as u8 + 1).collect();
            let padded = pad(&plaintext).unwrap();
            assert_eq!(padded.len() as u64, padded_length(size as u64 + 4));
            assert_eq!(unpad(&padded).unwrap(), plaintext.as_slice());
        }
    }

    #[test]
    fn malformed_padding_is_rejected() {
        let mut padded = pad(b"hello world, padded").unwrap().to_vec();
        assert!(unpad(&padded[..3]).is_err());
        let last = padded.len() - 1;
        padded[last] = 1;
        assert!(unpad(&padded).is_err());
        let mut grown = pad(b"abc").unwrap().to_vec();
        grown.push(0);
        assert!(unpad(&grown).is_err());
        let mut lying = pad(b"abc").unwrap().to_vec();
        lying[2] = 8;
        assert!(unpad(&lying).is_err());
    }

    #[test]
    fn short_plaintexts_all_pad_to_the_floor() {
        for size in [0usize, 1, 40, 500, 1020] {
            assert_eq!(
                pad(&vec![7; size]).unwrap().len() as u64,
                MIN_PADDED,
                "{size}"
            );
        }
        assert_eq!(padded_length(1025), padme(1025));
        assert!(padded_length(1025) > MIN_PADDED);
    }

    #[test]
    fn a_short_plaintext_padded_without_the_floor_is_rejected() {
        let mut padded = vec![0; usize::try_from(padme(4 + 5)).unwrap()];
        padded[3] = 5;
        assert!(unpad(&padded).is_err());
    }
}
