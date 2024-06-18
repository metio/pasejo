use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use duct::cmd;

use crate::vcs::api::VCS;

pub(crate) struct Git {}

impl VCS for Git {
    fn init(&self, path: &Path) -> Result<()> {
        cmd!("git", "-C", path, "init").run().with_context(|| {
            format!("Failed to initialize Git repository at {}", path.display())
        })?;
        Ok(())
    }

    fn commit(&self, store_root_path: &Path, file_to_commit: &Path, message: &str) -> Result<()> {
        cmd!("git", "-C", store_root_path, "add", file_to_commit)
            .run()
            .with_context(|| {
                format!(
                    "Failed to add file '{}' in Git repository '{}'",
                    file_to_commit.display(),
                    store_root_path.display()
                )
            })?;
        cmd!("git", "-C", store_root_path, "commit", "--message", message)
            .run()
            .with_context(|| {
                format!(
                    "Failed to commit file '{}' in Git repository '{}'",
                    file_to_commit.display(),
                    store_root_path.display()
                )
            })?;
        Ok(())
    }
}
