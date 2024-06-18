use std::path::PathBuf;

use anyhow::Result;

use crate::stores::api::Store;

pub struct RecipientAdd {
    store: Box<dyn Store>,
}

pub fn recipient_add(store: Box<dyn Store>, path: &Option<PathBuf>) -> Result<()> {
    Ok(())
}
