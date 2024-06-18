use std::path::Path;

use anyhow::Result;

use crate::adapters::file_system::FileSystem;
use crate::stores::api::Store;

pub struct LocalStore {
    pub file_system_adapter: Box<dyn FileSystem>,
}

impl Store for LocalStore {
    fn init(&self, path: &Path) -> Result<()> {
        self.file_system_adapter.mkdir_parents(path)
    }
}
