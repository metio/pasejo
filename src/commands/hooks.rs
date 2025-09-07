// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::hooks::executor::HookExecutor;
use crate::models::cli::HookCommands;
use crate::models::configuration::Configuration;

pub fn dispatch(command: &HookCommands, configuration: Configuration) -> anyhow::Result<()> {
    match command {
        HookCommands::Get(args) => get(
            &configuration,
            args.store_selection.store.as_ref(),
            args.global,
        ),
        HookCommands::Set(args) => set(
            configuration,
            args.store_selection.store.as_ref(),
            args.global,
            &args.pull,
            &args.push,
            args.prepend,
            args.append,
        ),
        HookCommands::Run(args) => {
            if let Some(all) = args.all
                && all
            {
                for store in &configuration.stores {
                    run(&configuration, Some(&store.name), args.pull, args.push)?;
                }
                Ok(())
            } else {
                run(
                    &configuration,
                    args.store_selection.store.as_ref(),
                    args.pull,
                    args.push,
                )
            }
        }
    }
}

fn get(
    configuration: &Configuration,
    store_name: Option<&String>,
    global: bool,
) -> anyhow::Result<()> {
    if global {
        for command in &configuration.pull_commands {
            println!("global pull: {command}");
        }
        for command in &configuration.push_commands {
            println!("global push: {command}");
        }
        Ok(())
    } else if let Some(registration) = configuration.select_store(store_name) {
        for command in &configuration.pull_commands {
            println!("global pull: {command}");
        }
        for command in &registration.pull_commands {
            println!("store pull: {command}");
        }
        for command in &configuration.push_commands {
            println!("global push: {command}");
        }
        for command in &registration.push_commands {
            println!("store push: {command}");
        }

        Ok(())
    } else {
        anyhow::bail!(
            "No store found in configuration and no --global flag specified. Run 'pasejo store add ...' first to add one"
        )
    }
}

fn set(
    mut configuration: Configuration,
    store_name: Option<&String>,
    global: bool,
    pull: &[String],
    push: &[String],
    prepend: bool,
    append: bool,
) -> anyhow::Result<()> {
    if global {
        if prepend {
            configuration
                .pull_commands
                .splice(0..0, pull.iter().cloned());
            configuration
                .push_commands
                .splice(0..0, push.iter().cloned());
        } else if append {
            configuration.pull_commands.append(&mut pull.to_owned());
            configuration.push_commands.append(&mut push.to_owned());
        } else {
            pull.clone_into(&mut configuration.pull_commands);
            push.clone_into(&mut configuration.push_commands);
        }
    } else if let Some(registration) = configuration.select_store_mut(store_name) {
        if prepend {
            registration
                .pull_commands
                .splice(0..0, pull.iter().cloned());
            registration
                .push_commands
                .splice(0..0, push.iter().cloned());
        } else if append {
            registration.pull_commands.append(&mut pull.to_owned());
            registration.push_commands.append(&mut push.to_owned());
        } else {
            pull.clone_into(&mut registration.pull_commands);
            push.clone_into(&mut registration.push_commands);
        }
    } else {
        anyhow::bail!(
            "No store found in configuration and no --global flag specified. Run 'pasejo store add ...' first to add one"
        )
    }

    configuration.save_configuration()
}

pub fn run(
    configuration: &Configuration,
    store_name: Option<&String>,
    pull: Option<bool>,
    push: Option<bool>,
) -> anyhow::Result<()> {
    if let Some(registration) = configuration.select_store(store_name) {
        let hooks = HookExecutor {
            configuration,
            registration,
            offline: false,
            force: true,
        };
        if pull.unwrap_or(false) {
            hooks.execute_pull_commands()?;
        }
        if push.unwrap_or(false) {
            hooks.execute_push_commands()?;
        }
        Ok(())
    } else {
        anyhow::bail!(
            "No store found in configuration. Run 'pasejo store add ...' first to add one"
        )
    }
}
