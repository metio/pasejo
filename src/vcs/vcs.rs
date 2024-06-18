use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

pub(crate) trait VersionControlSystem {
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
    pub fn as_str(&self) -> &'static str {
        match self {
            VersionControlSystems::None => "none",
            VersionControlSystems::Git => "git",
            VersionControlSystems::Mercurial => "hg",
        }
    }

    pub(crate) fn select_implementation(&self) -> Box<dyn VersionControlSystem> {
        match self {
            VersionControlSystems::None => Box::new(crate::vcs::none::None{}),
            VersionControlSystems::Git => Box::new(crate::vcs::git::Git {}),
            VersionControlSystems::Mercurial => Box::new(crate::vcs::mercurial::Mercurial {}),
        }
    }
}
