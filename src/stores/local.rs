use std::fs;
use std::path::Path;

use anyhow::Context;
use anyhow::Result;

use crate::stores::api::Store;

struct Local {}

impl Store for Local {
    fn init(path: &Path) -> Result<()> {
        fs::create_dir_all(path).with_context(|| format!("Failed to initialize store at {}", path))
    }
}
