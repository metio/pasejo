// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::synchronizers::git::Git;
use crate::synchronizers::mercurial::Mercurial;
use crate::synchronizers::none::None;
use crate::synchronizers::pijul::Pijul;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub trait Synchronizer {
    fn push(&self) -> anyhow::Result<()>;
    fn pull(&self) -> anyhow::Result<()>;
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, clap::ValueEnum)]
pub enum Synchronizers {
    #[default]
    None,
    Git,
    Mercurial,
    Pijul,
    // s3?
}

impl Synchronizers {
    pub fn select_implementation(&self, store_path: &Path) -> Box<dyn Synchronizer> {
        match self {
            Self::None => Box::new(None {}),
            Self::Git => Box::new(Git {
                store_path: store_path.to_path_buf(),
            }),
            Self::Mercurial => Box::new(Mercurial {
                store_path: store_path.to_path_buf(),
            }),
            Self::Pijul => Box::new(Pijul {
                store_path: store_path.to_path_buf(),
            }),
        }
    }
}
