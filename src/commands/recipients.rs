use std::path;

use crate::adapters::file_system;
use crate::cli::configuration;

pub(crate) fn add(
    file_system: Box<dyn file_system::FileSystem>,
    store: &configuration::Store,
    public_key: &String,
    name: &Option<String>,
    path: &Option<path::PathBuf>,
) -> anyhow::Result<()> {
    let store_root_path: &path::Path = store.path.as_ref();
    let vcs = store.vcs.select_implementation();

    if path.is_none() {
        let recipients_file = &path::PathBuf::from(".recipients");
        let root_recipients_file = &store_root_path.join(recipients_file);
        if root_recipients_file.try_exists()? && root_recipients_file.is_file() {
            let root_recipients_data = file_system.read_file(root_recipients_file)?;
            if !root_recipients_data.contains(public_key) {
                let recipient = match name {
                    Some(name) => format!("# {}\n{}", name, public_key),
                    None => public_key.clone(),
                };
                file_system.append_file(root_recipients_file, recipient)?;
            } else {
                // check comment
            }
        } else {
            let recipient = match name {
                Some(name) => format!("# {}\n{}", name, public_key),
                None => public_key.clone(),
            };
            file_system.append_file(root_recipients_file, recipient)?;
        }
        vcs.commit(
            store_root_path,
            recipients_file,
            &format!("Added recipient {}", public_key),
        )?;
    } else {
        file_system.reverse_walk(store_root_path);
    }

    Ok(())
}
