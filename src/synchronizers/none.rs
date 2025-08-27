// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::synchronizers::synchronizer::Synchronizer;

pub struct None {}

impl Synchronizer for None {
    fn push(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn pull(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn should_pull(&self, _: Option<u64>, _: &str) -> anyhow::Result<bool> {
        Ok(false)
    }
}
