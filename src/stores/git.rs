use std::path::Path;

use anyhow::Result;

use crate::adapters::file_system::FileSystem;
use crate::adapters::git::GitAdapter;
use crate::stores::api::Store;

pub struct GitStore {
    pub(crate) file_system_adapter: Box<dyn FileSystem>,
    pub(crate) git_adapter: Box<dyn GitAdapter>,
}

impl Store for GitStore {
    fn init(&self, path: &Path) -> Result<()> {
        self.file_system_adapter.mkdir_parents(path)?;
        self.git_adapter.init(path)
    }
}
