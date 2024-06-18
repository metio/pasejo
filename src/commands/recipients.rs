use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::adapters::file_system::FileSystem;
use crate::cli::configuration::Store;

pub(crate) fn recipient_add(
    store: &Store,
    public_key: &String,
    name: &Option<String>,
    path: &Option<PathBuf>,
    file_system: Box<dyn FileSystem>,
) -> Result<()> {
    let store_root_path: &Path = store.path.as_ref();
    let vcs = store.vcs.select_implementation();

    if path.is_none() {
        let recipients_file = &PathBuf::from(".recipients");
        let root_recipients_file = &store_root_path.join(recipients_file);
        if root_recipients_file.try_exists()? && root_recipients_file.is_file() {
            let root_recipients_data = file_system.read_file(root_recipients_file)?;
            if !root_recipients_data.contains(public_key) {

            }
            let mut root_recipients = Recipients::from_str(&root_recipients_data)?;
            root_recipients.recipients.push(new_recipient);
            file_system.write_file(root_recipients_file, root_recipients.to_str()?)?;
        } else {
            let root_recipients = Recipients {
                recipients: vec![new_recipient],
            };
            file_system.write_file(root_recipients_file, root_recipients.to_str()?)?;
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
