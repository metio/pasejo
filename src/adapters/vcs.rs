use anyhow::Context;
use anyhow::Result;
use duct::cmd;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub trait VersionControlSystem {
    fn init(&self, path: &Path) -> Result<()>;
    fn commit(&self, store_root_path: &Path, file_to_commit: &Path, message: &str) -> Result<()>;
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, clap::ValueEnum)]
pub enum VersionControlSystems {
    #[default]
    None,
    Git,
    Mercurial,
}

impl VersionControlSystems {
    pub fn select_implementation(&self) -> Box<dyn VersionControlSystem> {
        match self {
            VersionControlSystems::None => Box::new(None {}),
            VersionControlSystems::Git => Box::new(Git {}),
            VersionControlSystems::Mercurial => Box::new(Mercurial {}),
        }
    }
}

pub struct Git {}

impl VersionControlSystem for Git {
    fn init(&self, path: &Path) -> Result<()> {
        cmd!("git", "-C", path, "init")
            .stdout_capture()
            .run()
            .with_context(|| {
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

pub struct Mercurial {}

impl VersionControlSystem for Mercurial {
    fn init(&self, path: &Path) -> Result<()> {
        cmd!("hg", "init", path)
            .stdout_capture()
            .run()
            .with_context(|| {
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

pub struct None {}

impl VersionControlSystem for None {
    fn init(&self, _: &Path) -> Result<()> {
        Ok(())
    }

    fn commit(&self, _: &Path, _: &Path, _: &str) -> Result<()> {
        Ok(())
    }
}
