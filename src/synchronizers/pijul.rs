// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::synchronizers::synchronizer::Synchronizer;
use anyhow::Context;
use duct::cmd;
use std::path::PathBuf;

pub struct Pijul {
    pub store_path: PathBuf,
}

impl Synchronizer for Pijul {
    fn push(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.store_path.parent() {
            cmd!("pijul", "add", &self.store_path)
                .dir(parent)
                .run()
                .context("Failed to stage store")?;

            cmd!("pijul", "record", "--message", "pasejo commit")
                .stdout_null()
                .stderr_null()
                .dir(parent)
                .run()
                .context("Failed to commit store")?;

            cmd!("pijul", "push")
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
            cmd!("pijul", "pull")
                .dir(parent)
                .run()
                .context("Failed to pull store")?;

            Ok(())
        } else {
            anyhow::bail!("Cannot determine parent of store path");
        }
    }
}
