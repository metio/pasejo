use std::path::Path;

use anyhow::Result;

pub trait Store {
    fn init(path: &Path) -> Result<()>;
}
