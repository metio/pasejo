use crate::models::configuration::{Configuration, Identity};
use std::path::PathBuf;

pub fn add(
    mut configuration: Configuration,
    alias: &Option<String>,
    file: &Option<PathBuf>,
    inline: &Option<String>,
) -> anyhow::Result<()> {
    let identity = Identity {
        file: file.as_ref().map(|path| path.display().to_string()).clone(),
        inline: inline.clone(),
    };
    let result = configuration.add_identity(identity, alias.clone());
    println!("Identity added");
    result
}
