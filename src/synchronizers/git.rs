// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::synchronizers::synchronizer::Synchronizer;
use anyhow::Context;
use duct::cmd;
use std::path::PathBuf;

pub struct Git {
    pub store_path: PathBuf,
}

impl Synchronizer for Git {
    fn push(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.store_path.parent() {
            cmd!("git", "add", &self.store_path)
                .stdout_null()
                .stderr_null()
                .dir(parent)
                .run()
                .context("Failed to stage store")?;

            cmd!("git", "commit", "--message", "pasejo commit")
                .stdout_null()
                .stderr_null()
                .dir(parent)
                .run()
                .context("Failed to commit store")?;

            cmd!("git", "push")
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
            cmd!("git", "pull")
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
}
