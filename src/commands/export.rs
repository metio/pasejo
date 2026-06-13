// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use zeroize::Zeroizing;

use crate::cli::{atomic_write, i18n};
use crate::exporters;
use crate::hooks::executor::HookExecutor;
use crate::models::cli::{BitwardenArgs, ExportCommands};
use crate::models::configuration::Configuration;

pub fn dispatch(
    command: &ExportCommands,
    configuration: &Configuration,
    offline: bool,
) -> anyhow::Result<()> {
    match command {
        ExportCommands::Bitwarden(args) => export_as_json(configuration, args, offline),
    }
}

fn export_as_json(
    configuration: &Configuration,
    args: &BitwardenArgs,
    offline: bool,
) -> anyhow::Result<()> {
    if let Some(registration) = configuration.select_store(args.store_selection.store.as_ref()) {
        let hooks = HookExecutor {
            configuration,
            registration,
            offline,
            force: false,
        };

        hooks.execute_pull_commands()?;

        let store = configuration.decrypt_store(registration)?;

        // The serialized export is the entire vault in cleartext — the
        // largest plaintext expansion in the codebase. Wrap it so the
        // buffer is scrubbed after it is written, rather than left in a
        // freed heap page like a plain `String`.
        let json = Zeroizing::new(exporters::bitwarden::json(
            &store,
            args.organization_id.as_ref(),
            args.collection_id.as_ref(),
            args.collection_name.as_ref(),
            args.username_keys.as_ref(),
            args.uri_keys.as_ref(),
            args.pretty.unwrap_or(false),
        )?);

        // Warn (on stderr) that this is cleartext regardless of sink — a
        // user piping to a file or capturing stdout should know the whole
        // vault just left the encrypted store.
        i18n::export_plaintext_warning();
        if let Some(out_path) = args.out.as_ref() {
            // `atomic_write` creates the file mode 0600 on Unix (sibling
            // tempfile + fsync + rename), avoiding the umask-default
            // permissions a shell `> file` redirection would leave.
            atomic_write::write(out_path, json.as_bytes())?;
            i18n::export_written_to_file(out_path);
        } else {
            println!("{}", &*json);
        }

        Ok(())
    } else {
        anyhow::bail!(i18n::error_no_store_in_configuration())
    }
}
