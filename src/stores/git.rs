use std::fs;
use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use duct::cmd;

use crate::stores::api::Store;

pub struct Git {}

impl Store for Git {
    fn init(path: &Path) -> Result<()> {
        fs::create_dir_all(path)
            .with_context(|| format!("Failed to initialize store at {}", path))?;
        cmd!("git", "-C", path, "init")
            .run()
            .with_context(|| format!("Failed to initialize store at {}", path))?;
        Ok(())
    }
}
