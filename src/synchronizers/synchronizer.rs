// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::synchronizers::git::Git;
use crate::synchronizers::mercurial::Mercurial;
use crate::synchronizers::none::None;
use crate::synchronizers::pijul::Pijul;
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::SystemTime;

pub trait Synchronizer {
    fn push(&self) -> anyhow::Result<()>;
    fn pull(&self) -> anyhow::Result<()>;
    fn should_pull(
        &self,
        pull_interval_seconds: Option<u64>,
        store_name: &str,
    ) -> anyhow::Result<bool>;
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, clap::ValueEnum)]
pub enum Synchronizers {
    #[default]
    None,
    Git,
    Mercurial,
    Pijul,
}

impl Synchronizers {
    pub fn select_implementation(&self, store_path: &Path) -> Box<dyn Synchronizer> {
        match self {
            Self::None => Box::new(None {}),
            Self::Git => Box::new(Git {
                store_path: store_path.to_path_buf(),
            }),
            Self::Mercurial => Box::new(Mercurial {
                store_path: store_path.to_path_buf(),
            }),
            Self::Pijul => Box::new(Pijul {
                store_path: store_path.to_path_buf(),
            }),
        }
    }

    pub fn should_pull(
        pull_interval_seconds: Option<u64>,
        store_name: &str,
    ) -> anyhow::Result<bool> {
        if let Some(base_dirs) = BaseDirs::new() {
            let data_local_dir = base_dirs.data_local_dir();
            let last_pull_file = data_local_dir
                .join(env!("CARGO_PKG_NAME"))
                .join("last-pulls")
                .join(store_name);
            if last_pull_file.exists() {
                let last_pull_content = fs::read_to_string(&last_pull_file)?;
                let last_pull_seconds: u64 = last_pull_content.parse()?;
                let now_in_seconds = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)?
                    .as_secs();
                let seconds_since_last_pull = now_in_seconds - last_pull_seconds;
                let interval = pull_interval_seconds.unwrap_or(60 * 60 * 24);
                let should_pull = seconds_since_last_pull > interval;

                if should_pull {
                    fs::write(last_pull_file, now_in_seconds.to_string())?;
                    return Ok(should_pull);
                }
            }
        }

        Ok(true)
    }
}
