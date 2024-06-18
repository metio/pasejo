use std::path::Path;

use anyhow::Result;

use crate::vcs::vcs::VersionControlSystem;

pub(crate) struct None {}

impl VersionControlSystem for None {
    fn init(&self, _: &Path) -> Result<()> {
        Ok(())
    }

    fn commit(&self, _: &Path, _: &Path, _: &str) -> Result<()> {
        Ok(())
    }
}
