use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

pub(crate) trait VCS {
    fn init(&self, path: &Path) -> Result<()>;
    fn commit(&self, store_root_path: &Path, file_to_commit: &Path, message: &str) -> Result<()>;
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, clap::ValueEnum)]
pub enum VCSTypes {
    #[default]
    None,
    Git,
    Mercurial,
}

impl VCSTypes {
    pub fn as_str(&self) -> &'static str {
        match self {
            VCSTypes::None => "none",
            VCSTypes::Git => "git",
            VCSTypes::Mercurial => "hg",
        }
    }

    pub(crate) fn select_implementation(&self) -> Box<dyn VCS> {
        match self {
            VCSTypes::None => Box::new(crate::vcs::none::None{}),
            VCSTypes::Git => Box::new(crate::vcs::git::Git {}),
            VCSTypes::Mercurial => Box::new(crate::vcs::mercurial::Mercurial {}),
        }
    }
}
