use std::path::{Path, PathBuf};

use anyhow::Context;
use anyhow::Result;
use duct::cmd;
use serde::{Deserialize, Serialize};

pub trait VersionControlSystem {
    fn init(&self) -> Result<()>;
    fn commit(&self, files_to_commit: Vec<&Path>, message: &str) -> Result<()>;
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, clap::ValueEnum)]
pub enum VersionControlSystems {
    #[default]
    None,
    Git,
    Mercurial,
}

impl VersionControlSystems {
    pub fn select_implementation(&self, root: PathBuf) -> Box<dyn VersionControlSystem> {
        match self {
            Self::None => Box::new(None {}),
            Self::Git => Box::new(Git { root }),
            Self::Mercurial => Box::new(Mercurial { root }),
        }
    }
}

pub struct Git {
    pub root: PathBuf,
}

impl VersionControlSystem for Git {
    fn init(&self) -> Result<()> {
        cmd!("git", "-C", &self.root, "init")
            .stdout_capture()
            .run()
            .with_context(|| {
                format!(
                    "Failed to initialize Git repository at {}",
                    &self.root.display()
                )
            })?;
        Ok(())
    }

    fn commit(&self, files_to_commit: Vec<&Path>, message: &str) -> Result<()> {
        for file in &files_to_commit {
            cmd!("git", "-C", &self.root, "add", file)
                .run()
                .with_context(|| {
                    format!(
                        "Failed to add file '{}' in Git repository '{}'",
                        file.display(),
                        &self.root.display()
                    )
                })?;
        }
        cmd!("git", "-C", &self.root, "commit", "--message", message)
            .run()
            .with_context(|| {
                format!(
                    "Failed to commit files '{:?}' in Git repository '{}'",
                    &files_to_commit,
                    &self.root.display()
                )
            })?;
        Ok(())
    }
}

pub struct Mercurial {
    pub root: PathBuf,
}

impl VersionControlSystem for Mercurial {
    fn init(&self) -> Result<()> {
        cmd!("hg", "init", &self.root)
            .stdout_capture()
            .run()
            .with_context(|| {
                format!(
                    "Failed to initialize Mercurial repository at {}",
                    &self.root.display()
                )
            })?;
        Ok(())
    }

    fn commit(&self, files_to_commit: Vec<&Path>, message: &str) -> Result<()> {
        for file in &files_to_commit {
            cmd!("hg", "--cwd", &self.root, "add", file)
                .run()
                .with_context(|| {
                    format!(
                        "Failed to add file '{}' to Mercurial repository at '{}'",
                        file.display(),
                        &self.root.display()
                    )
                })?;
        }
        cmd!("hg", "--cwd", &self.root, "commit", "--message", message)
            .run()
            .with_context(|| {
                format!(
                    "Failed to commit files '{:?}' to Mercurial repository at '{}'",
                    &files_to_commit,
                    &self.root.display()
                )
            })?;
        Ok(())
    }
}

pub struct None {}

impl VersionControlSystem for None {
    fn init(&self) -> Result<()> {
        Ok(())
    }

    fn commit(&self, _: Vec<&Path>, _: &str) -> Result<()> {
        Ok(())
    }
}
