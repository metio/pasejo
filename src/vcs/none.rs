use std::path::Path;

use anyhow::Result;

use crate::vcs::api::VCS;

pub(crate) struct None {}

impl VCS for None {
    fn init(&self, _: &Path) -> Result<()> {
        Ok(())
    }

    fn commit(&self, _: &Path, _: &Path, _: &str) -> Result<()> {
        Ok(())
    }
}
