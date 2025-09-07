// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use directories::BaseDirs;
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

pub fn should_execute(
    interval_seconds: Option<u64>,
    paths: Option<(PathBuf, PathBuf)>,
) -> anyhow::Result<bool> {
    if let Some((last_directory, last_file)) = paths {
        if last_file.exists() {
            let last_content = fs::read_to_string(&last_file)?;
            let last_seconds: u64 = last_content.parse()?;
            let epoch_seconds = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_secs();
            let seconds_since_last_execution = epoch_seconds - last_seconds;
            let interval = interval_seconds.unwrap_or(60 * 60 * 24);
            let should_execute = seconds_since_last_execution > interval;

            if should_execute {
                fs::write(last_file, epoch_seconds.to_string())?;
                return Ok(should_execute);
            }
        } else {
            fs::create_dir_all(last_directory)?;
            let epoch_seconds = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_secs();
            fs::write(last_file, epoch_seconds.to_string())?;
        }
    }

    Ok(true)
}

pub fn write_last_execution(paths: Option<(PathBuf, PathBuf)>) -> anyhow::Result<()> {
    if let Some((last_directory, last_file)) = paths {
        if !last_file.exists() {
            fs::create_dir_all(last_directory)?;
        }
        let epoch_seconds = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs();
        Ok(fs::write(last_file, epoch_seconds.to_string())?)
    } else {
        Ok(())
    }
}

pub fn last_pull_paths(store_name: &OsStr) -> Option<(PathBuf, PathBuf)> {
    last_paths(store_name, "last-pull")
}

pub fn last_push_paths(store_name: &OsStr) -> Option<(PathBuf, PathBuf)> {
    last_paths(store_name, "last-push")
}

fn last_paths(store_name: &OsStr, directory: &str) -> Option<(PathBuf, PathBuf)> {
    BaseDirs::new().map(|base_dirs| {
        let data_local_dir = base_dirs.data_local_dir();
        let last_pulls_directory = data_local_dir.join(env!("CARGO_PKG_NAME")).join(directory);
        let last_pull_file = last_pulls_directory.join(store_name);
        (last_pulls_directory, last_pull_file)
    })
}
