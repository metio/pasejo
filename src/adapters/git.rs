use std::path::Path;

use anyhow::Result;

pub trait GitAdapter {
    fn init(&self, path: &Path) -> Result<()>;
}
