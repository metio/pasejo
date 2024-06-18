use std::path::Path;

use anyhow::Result;

pub trait FileSystem {
    fn mkdir_parents(&self, path: &Path) -> Result<()>;
}
