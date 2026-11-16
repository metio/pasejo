// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! `secret resolve` and `exec`: values from secrets, bound to environment
//! variable names.

use std::collections::BTreeMap;
use std::io::{self, Read, Write};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;

use anyhow::{Context, Result};
use zeroize::Zeroizing;

use crate::cli::i18n;
use crate::cli::redact::{MIN_REDACTED_LEN, Redactor};
use crate::commands::store_op::{StoreMutation, require_store, with_store_fetching};
use crate::models::cli::ExecArgs;
use crate::models::configuration::Configuration;
use crate::models::environment::{SecretReference, collect_assignments, parse_references};
use crate::models::password_store::PasswordStore;
use crate::remote::funnel::Fetch;
use crate::secrets::selection::Selection;

/// Values keyed by the caller's names (variable names for `exec`). Each
/// value is wiped when the map drops.
pub type ResolvedValues = BTreeMap<String, Zeroizing<String>>;

/// Resolves every reference, decrypting each store once and running its
/// pull hooks as any read does. References without a store use
/// `default_store`, else the store `--store` would select by default. Stores
/// are opened one after another, each under its own lock, so the result is
/// all values or an error naming the first reference that failed.
pub fn resolve(
    configuration: &Configuration,
    references: &BTreeMap<String, SecretReference>,
    default_store: Option<&String>,
    offline: bool,
) -> Result<ResolvedValues> {
    let mut by_store: BTreeMap<String, Vec<(&String, &SecretReference)>> = BTreeMap::new();
    for (variable, reference) in references {
        let store_name = reference.store.as_ref().or(default_store);
        let registration = require_store(configuration, store_name)
            .with_context(|| i18n::error_value_unresolved(variable))?;
        by_store
            .entry(registration.name.clone())
            .or_default()
            .push((variable, reference));
    }

    let mut values = ResolvedValues::new();
    for (store_name, entries) in &by_store {
        let fetch = Fetch::secrets(
            entries
                .iter()
                .map(|(_, reference)| reference.secret.as_str()),
        );
        let resolved = with_store_fetching(
            configuration,
            Some(store_name),
            offline,
            &fetch,
            |_, store| {
                let resolved = entries
                    .iter()
                    .map(|(variable, reference)| {
                        lookup(store, reference)
                            .with_context(|| i18n::error_value_unresolved(variable))
                            .map(|value| ((*variable).clone(), value))
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok((resolved, StoreMutation::Unchanged))
            },
        )?;
        values.extend(resolved);
    }
    Ok(values)
}

/// A missing line or field is an error here, unlike in `secret show`: an
/// empty variable would pass for a real value in the command that reads it.
fn lookup(store: &PasswordStore, reference: &SecretReference) -> Result<Zeroizing<String>> {
    let Some(secret) = store.secrets.get(&reference.secret) else {
        anyhow::bail!(i18n::error_no_secret_found(&reference.secret))
    };
    match (reference.selection.extract(secret), &reference.selection) {
        (Some(value), _) => Ok(value),
        (None, Selection::Field(key)) => {
            anyhow::bail!(i18n::error_no_field_found(&reference.secret, key))
        }
        (None, Selection::Line(line)) => {
            anyhow::bail!(i18n::error_no_line_found(
                &reference.secret,
                &line.to_string()
            ))
        }
        // Selecting all of a secret, or its remainder, always yields text.
        (None, Selection::All | Selection::SkipFirst(_)) => Ok(Zeroizing::new(String::new())),
    }
}

/// `secret resolve`: reads a JSON object of references from stdin and
/// prints a JSON object of the same names with their values.
pub fn resolve_from_stdin(
    configuration: &Configuration,
    default_store: Option<&String>,
    offline: bool,
) -> Result<()> {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .context(i18n::error_resolve_input_invalid())?;
    let references = parse_references(&input).context(i18n::error_resolve_input_invalid())?;
    let values = resolve(configuration, &references, default_store, offline)?;
    let printable: BTreeMap<&str, &str> = values
        .iter()
        .map(|(variable, value)| (variable.as_str(), value.as_str()))
        .collect();
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, &printable)?;
    writeln!(stdout)?;
    stdout.flush()?;
    Ok(())
}

/// `exec`: resolves the assignments, then runs the command with the values
/// added to the inherited environment. Without `--redact`, Unix replaces
/// this process with the command, so the exit status, signals and the
/// terminal belong to the command alone. With `--redact`, or on Windows,
/// the command runs as a child and pasejo exits with its status.
pub fn exec(configuration: &Configuration, args: &ExecArgs, offline: bool) -> Result<()> {
    let references = collect_assignments(args.env.iter().chain(&args.env_field))?;
    let values = resolve(
        configuration,
        &references,
        args.store_selection.store.as_ref(),
        offline,
    )?;
    let Some((program, arguments)) = args.command.split_first() else {
        return Ok(());
    };
    let mut command = Command::new(program);
    command.args(arguments);
    for (variable, value) in &values {
        command.env(variable, value.as_str());
    }

    let status = if args.redact {
        run_redacted(command, &values)
            .with_context(|| i18n::error_failed_to_run_command(program))?
    } else {
        drop(values);
        run(command).with_context(|| i18n::error_failed_to_run_command(program))?
    };
    exit_like(status)
}

#[cfg(unix)]
fn run(mut command: Command) -> io::Result<ExitStatus> {
    use std::os::unix::process::CommandExt;
    // Only returns when the command could not be started.
    Err(command.exec())
}

#[cfg(not(unix))]
fn run(mut command: Command) -> io::Result<ExitStatus> {
    command.status()
}

/// Runs the command with its stdout and stderr piped through a
/// [`Redactor`], one thread per stream. Stdin stays the terminal's.
fn run_redacted(mut command: Command, values: &ResolvedValues) -> io::Result<ExitStatus> {
    for (variable, value) in values {
        if value.len() < MIN_REDACTED_LEN {
            i18n::exec_value_too_short_to_redact(variable, &MIN_REDACTED_LEN.to_string());
        }
    }
    let redactor = Redactor::new(
        values
            .iter()
            .map(|(variable, value)| (value.as_str(), format!("${{{variable}}}"))),
    );
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    drop(command);
    let child_stdout = child.stdout.take();
    let child_stderr = child.stderr.take();
    thread::scope(|scope| {
        let redactor = &redactor;
        let stdout =
            child_stdout.map(|pipe| scope.spawn(move || redactor.copy(pipe, io::stdout())));
        let stderr =
            child_stderr.map(|pipe| scope.spawn(move || redactor.copy(pipe, io::stderr())));
        let status = child.wait();
        for copier in [stdout, stderr].into_iter().flatten() {
            // A closed stdout of our own ends the copy early; the command's
            // exit status still decides how pasejo exits.
            let _ = copier.join();
        }
        status
    })
}

/// Exits the way the command did: with its exit code, or on Unix with
/// 128 plus the signal that ended it, as shells report it.
fn exit_like(status: ExitStatus) -> Result<()> {
    if status.success() {
        return Ok(());
    }
    let code = status.code().unwrap_or_else(|| signal_exit_code(status));
    io::stdout().flush().ok();
    io::stderr().flush().ok();
    std::process::exit(code)
}

#[cfg(unix)]
fn signal_exit_code(status: ExitStatus) -> i32 {
    use std::os::unix::process::ExitStatusExt;
    status.signal().map_or(1, |signal| 128 + signal)
}

#[cfg(not(unix))]
const fn signal_exit_code(_status: ExitStatus) -> i32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_with(secrets: &[(&str, &str)]) -> PasswordStore {
        let mut store = PasswordStore::default();
        for (name, value) in secrets {
            store
                .secrets
                .insert((*name).to_owned(), (*value).to_owned());
        }
        store
    }

    fn reference(secret: &str, selection: Selection) -> SecretReference {
        SecretReference {
            store: None,
            secret: secret.to_owned(),
            selection,
        }
    }

    #[test]
    fn lookup_reads_the_selected_value() {
        let store = store_with(&[("db", "hunter2\nusername: alice")]);
        assert_eq!(
            lookup(&store, &reference("db", Selection::Line(1)))
                .unwrap()
                .as_str(),
            "hunter2"
        );
        assert_eq!(
            lookup(
                &store,
                &reference("db", Selection::Field(String::from("username")))
            )
            .unwrap()
            .as_str(),
            "alice"
        );
    }

    #[test]
    fn lookup_refuses_a_missing_secret_line_or_field() {
        crate::cli::i18n::init_for_tests();
        let store = store_with(&[("db", "hunter2")]);
        assert!(lookup(&store, &reference("other", Selection::Line(1))).is_err());
        assert!(lookup(&store, &reference("db", Selection::Line(2))).is_err());
        assert!(
            lookup(
                &store,
                &reference("db", Selection::Field(String::from("user")))
            )
            .is_err()
        );
    }
}
