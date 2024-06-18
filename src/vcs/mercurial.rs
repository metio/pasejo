use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use duct::cmd;

use crate::vcs::vcs::VersionControlSystem;

pub(crate) struct Mercurial {}

impl VersionControlSystem for Mercurial {
    fn init(&self, path: &Path) -> Result<()> {
        cmd!("hg", "init", path).run().with_context(|| {
            format!(
                "Failed to initialize Mercurial repository at {}",
                path.display()
            )
        })?;
        Ok(())
    }

    fn commit(&self, store_root_path: &Path, file_to_commit: &Path, message: &str) -> Result<()> {
        cmd!("hg", "--cwd", store_root_path, "add", file_to_commit)
            .run()
            .with_context(|| {
                format!(
                    "Failed to add file '{}' to Mercurial repository at '{}'",
                    file_to_commit.display(),
                    store_root_path.display()
                )
            })?;
        cmd!(
            "hg",
            "--cwd",
            store_root_path,
            "commit",
            "--message",
            message
        )
        .run()
        .with_context(|| {
            format!(
                "Failed to commit file '{}' to Mercurial repository at '{}'",
                file_to_commit.display(),
                store_root_path.display()
            )
        })?;
        Ok(())
    }
}
