// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::env;
use std::ffi::OsString;
use std::path::PathBuf;

use directories::BaseDirs;

use crate::cli::environment_variables;

/// The directory holding pasejo's local, never-synchronised state (hook
/// throttle markers, the browser release log): `pasejo` inside
/// `$PASEJO_DATA_DIR` when that is set, inside the OS data-local directory
/// otherwise.
pub fn local_data_dir() -> Option<PathBuf> {
    resolve(env::var_os(environment_variables::PASEJO_DATA_DIR), || {
        BaseDirs::new().map(|base_dirs| base_dirs.data_local_dir().to_path_buf())
    })
}

fn resolve(
    override_dir: Option<OsString>,
    os_data_local_dir: impl FnOnce() -> Option<PathBuf>,
) -> Option<PathBuf> {
    override_dir
        .map(PathBuf::from)
        .or_else(os_data_local_dir)
        .map(|dir| dir.join(env!("CARGO_PKG_NAME")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_override_wins_over_the_os_directory() {
        let dir = resolve(Some(OsString::from("/override")), || {
            Some(PathBuf::from("/os"))
        });
        assert_eq!(dir, Some(PathBuf::from("/override/pasejo")));
    }

    #[test]
    fn the_os_directory_is_used_without_an_override() {
        let dir = resolve(None, || Some(PathBuf::from("/os")));
        assert_eq!(dir, Some(PathBuf::from("/os/pasejo")));
    }

    #[test]
    fn no_directory_resolves_without_either() {
        assert_eq!(resolve(None, || None), None);
    }
}
