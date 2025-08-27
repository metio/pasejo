// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::synchronizers::synchronizer::{Synchronizer, Synchronizers};
use anyhow::Context;
use duct::cmd;
use std::path::PathBuf;

pub struct Mercurial {
    pub store_path: PathBuf,
}

impl Synchronizer for Mercurial {
    fn push(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.store_path.parent() {
            cmd!("hg", "add", &self.store_path)
                .stdout_null()
                .stderr_null()
                .dir(parent)
                .run()
                .context("Failed to stage store")?;

            cmd!("hg", "commit", "--message", "pasejo commit")
                .stdout_null()
                .stderr_null()
                .dir(parent)
                .run()
                .context("Failed to commit store")?;

            cmd!("hg", "push")
                .stdout_null()
                .stderr_null()
                .dir(parent)
                .run()
                .context("Failed to push store")?;

            Ok(())
        } else {
            anyhow::bail!("Cannot determine parent of store path");
        }
    }

    fn pull(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.store_path.parent() {
            cmd!("hg", "pull")
                .stdout_null()
                .stderr_null()
                .dir(parent)
                .run()
                .context("Failed to pull store")?;

            Ok(())
        } else {
            anyhow::bail!("Cannot determine parent of store path");
        }
    }

    fn should_pull(
        &self,
        pull_interval_seconds: Option<u64>,
        store_name: &str,
    ) -> anyhow::Result<bool> {
        Synchronizers::should_pull(pull_interval_seconds, store_name)
    }
}
