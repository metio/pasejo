// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::path::{Path, PathBuf, absolute};

use anyhow::Context;
use anyhow::Result;

use crate::cli::i18n;
use crate::models::configuration::Configuration;
use crate::models::environment::{EnvAssignment, SecretReference, is_valid_variable_name};
use crate::secrets::selection::Selection;

pub fn store_name(input: &str) -> Result<String> {
    let configuration =
        Configuration::cached().context(i18n::error_could_not_load_configuration())?;
    configuration
        .canonical_store_name(input)
        .ok_or_else(|| anyhow::anyhow!(i18n::error_store_does_not_exist(input)))
}

/// The end of the UTC day `input` names as `YYYY-MM-DD`, in milliseconds
/// since the Unix epoch.
pub fn utc_date(input: &str) -> Result<u64> {
    let invalid = || anyhow::anyhow!(i18n::error_invalid_date(input));
    let mut parts = input.split('-');
    let (Some(year), Some(month), Some(day), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(invalid());
    };
    let year: i64 = year.parse().map_err(|_| invalid())?;
    let month: i64 = month.parse().map_err(|_| invalid())?;
    let day: i64 = day.parse().map_err(|_| invalid())?;
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return Err(invalid()),
    };
    if !(1970..=9999).contains(&year) || !(1..=days_in_month).contains(&day) {
        return Err(invalid());
    }
    // Days from civil, after Howard Hinnant's algorithm.
    let shifted = if month <= 2 { year - 1 } else { year };
    let era = shifted.div_euclid(400);
    let year_of_era = shifted - era * 400;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    u64::try_from((days + 1) * 86_400_000).map_err(|_| invalid())
}

pub fn nonzero_isize(input: &str) -> Result<isize> {
    let value: isize = input
        .parse()
        .with_context(|| i18n::error_invalid_line_number(input))?;
    if value == 0 {
        anyhow::bail!(i18n::error_line_number_must_not_be_zero())
    }
    Ok(value)
}

pub fn positive_u64(input: &str) -> Result<u64> {
    let value: u64 = input
        .parse()
        .with_context(|| i18n::error_invalid_count(input))?;
    if value == 0 {
        anyhow::bail!(i18n::error_count_must_not_be_zero())
    }
    Ok(value)
}

pub fn existing_file(input: &str) -> Result<PathBuf> {
    let path = Path::new(input);
    let absolute_path = absolute(path)?;

    if absolute_path.is_file() {
        Ok(path.to_path_buf())
    } else {
        anyhow::bail!(i18n::error_file_does_not_exist(input))
    }
}

/// `VARIABLE=SECRET`: the variable receives the first line of the secret,
/// the conventional place of its password. Everything after the first `=`
/// is the secret's path.
pub fn env_assignment(input: &str) -> Result<EnvAssignment> {
    let (variable, secret) = input
        .split_once('=')
        .filter(|(_, secret)| !secret.is_empty())
        .ok_or_else(|| anyhow::anyhow!(i18n::error_env_assignment_invalid(input)))?;
    assignment(variable, secret, Selection::Line(1))
}

/// `VARIABLE=SECRET=FIELD`: the variable receives the value of the field.
/// The path runs from the first `=` to the last one, so it may contain `=`
/// while the field key may not.
pub fn env_field_assignment(input: &str) -> Result<EnvAssignment> {
    let (variable, secret, field) = input
        .split_once('=')
        .and_then(|(variable, rest)| {
            rest.rsplit_once('=')
                .map(|(secret, field)| (variable, secret, field))
        })
        .filter(|(_, secret, field)| !secret.is_empty() && !field.is_empty())
        .ok_or_else(|| anyhow::anyhow!(i18n::error_env_field_assignment_invalid(input)))?;
    assignment(variable, secret, Selection::Field(field.to_owned()))
}

fn assignment(variable: &str, secret: &str, selection: Selection) -> Result<EnvAssignment> {
    if !is_valid_variable_name(variable) {
        anyhow::bail!(i18n::error_env_variable_name_invalid(variable));
    }
    Ok(EnvAssignment {
        variable: variable.to_owned(),
        reference: SecretReference {
            store: None,
            secret: secret.to_owned(),
            selection,
        },
    })
}

#[cfg(test)]
mod tests {
    use assert_fs::TempDir;
    use assert_fs::prelude::*;

    use super::*;

    #[test]
    fn utc_dates_end_at_the_next_midnight() {
        crate::cli::i18n::init_for_tests();
        assert_eq!(utc_date("1970-01-01").unwrap(), 86_400_000);
        assert_eq!(utc_date("2000-02-29").unwrap(), 951_868_800_000);
        assert_eq!(utc_date("2026-11-16").unwrap(), 1_794_873_600_000);
        for bad in [
            "2026-02-29",
            "2026-13-01",
            "2026-1",
            "x-1-1",
            "1969-12-31",
            "2026-01-01-01",
        ] {
            assert!(utc_date(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn existing_file_accepts_real_file() {
        let temp = TempDir::new().unwrap();
        let file = temp.child("present.txt");
        file.write_str("contents").unwrap();
        let path_str = file.path().to_str().unwrap();

        let result = existing_file(path_str).unwrap();
        assert_eq!(result, std::path::PathBuf::from(path_str));
    }

    #[test]
    fn existing_file_rejects_missing_path() {
        let temp = TempDir::new().unwrap();
        let missing = temp.child("does-not-exist.txt");
        let path_str = missing.path().to_str().unwrap();

        assert!(existing_file(path_str).is_err());
    }

    #[test]
    fn nonzero_isize_accepts_positive_and_negative_values() {
        assert_eq!(nonzero_isize("1").unwrap(), 1);
        assert_eq!(nonzero_isize("42").unwrap(), 42);
        assert_eq!(nonzero_isize("-1").unwrap(), -1);
        assert_eq!(nonzero_isize("-100").unwrap(), -100);
    }

    #[test]
    fn nonzero_isize_rejects_zero() {
        i18n::init_for_tests();
        let err = nonzero_isize("0").unwrap_err().to_string();
        assert!(err.contains("must not be 0"));
    }

    #[test]
    fn nonzero_isize_rejects_non_numeric() {
        assert!(nonzero_isize("abc").is_err());
        assert!(nonzero_isize("").is_err());
        assert!(nonzero_isize("1.5").is_err());
    }

    #[test]
    fn existing_file_rejects_directory() {
        let temp = TempDir::new().unwrap();
        let path_str = temp.path().to_str().unwrap();

        assert!(existing_file(path_str).is_err());
    }

    #[test]
    fn env_assignment_binds_the_first_line_of_the_secret() {
        let assignment = env_assignment("TOKEN=api/github.com/token").unwrap();
        assert_eq!(assignment.variable, "TOKEN");
        assert_eq!(assignment.reference.secret, "api/github.com/token");
        assert_eq!(assignment.reference.selection, Selection::Line(1));
        assert_eq!(assignment.reference.store, None);
    }

    #[test]
    fn env_assignment_keeps_later_equals_signs_in_the_path() {
        let assignment = env_assignment("TOKEN=a=b").unwrap();
        assert_eq!(assignment.reference.secret, "a=b");
    }

    #[test]
    fn env_assignment_refuses_malformed_input() {
        crate::cli::i18n::init_for_tests();
        for input in [
            "TOKEN",
            "TOKEN=",
            "=secret",
            "1TOKEN=secret",
            "TO KEN=secret",
        ] {
            assert!(env_assignment(input).is_err(), "{input}");
        }
    }

    #[test]
    fn env_field_assignment_splits_the_field_off_at_the_last_equals_sign() {
        let assignment = env_field_assignment("USER=db/a=b=username").unwrap();
        assert_eq!(assignment.variable, "USER");
        assert_eq!(assignment.reference.secret, "db/a=b");
        assert_eq!(
            assignment.reference.selection,
            Selection::Field(String::from("username"))
        );
    }

    #[test]
    fn env_field_assignment_refuses_malformed_input() {
        crate::cli::i18n::init_for_tests();
        for input in [
            "USER=db",
            "USER==username",
            "USER=db=",
            "=db=username",
            "1U=db=x",
        ] {
            assert!(env_field_assignment(input).is_err(), "{input}");
        }
    }
}
