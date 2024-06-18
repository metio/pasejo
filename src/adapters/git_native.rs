use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use duct::cmd;

use crate::adapters::git::GitAdapter;

pub struct GitNative {}

impl GitAdapter for GitNative {
    fn init(&self, path: &Path) -> Result<()> {
        cmd!("git", "-C", path, "init").run().with_context(|| {
            format!("Failed to initialize Git repository at {}", path.display())
        })?;
        Ok(())
    }
}
