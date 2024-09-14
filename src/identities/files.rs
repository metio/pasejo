use age::cli_common::{read_identities, StdinGuard};

pub fn read(identity_files: Vec<String>) -> anyhow::Result<Vec<Box<dyn age::Identity>>> {
    let parsed_identities = read_identities(identity_files, None, &mut StdinGuard::new(true))?;
    Ok(parsed_identities)
}
