// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use anyhow::Context;

use crate::browser::host;
use crate::browser::install_manifest;
use crate::browser::protocol::QueryResponse;
use crate::browser::release_log::{Release, ReleaseLog};
use crate::cli::i18n;
use crate::commands::store_op::{StoreMutation, with_store};
use crate::models::cli::{BrowserCommands, QueryArgs};
use crate::models::configuration::Configuration;

pub fn dispatch(
    command: &BrowserCommands,
    configuration: &Configuration,
    offline: bool,
) -> anyhow::Result<()> {
    match command {
        BrowserCommands::Query(args) => query(args, configuration, offline),
        BrowserCommands::Host(args) => host::run(configuration, args, offline),
        BrowserCommands::InstallManifest(args) => install_manifest::run(args),
        BrowserCommands::Log => log(),
    }
}

fn query(args: &QueryArgs, configuration: &Configuration, offline: bool) -> anyhow::Result<()> {
    let response = with_store(
        configuration,
        args.store_selection.store.as_ref(),
        offline,
        |registration, store| {
            let response = QueryResponse::search(
                store,
                &registration.name,
                &args.domain,
                &args.uri_keys,
                &args.username_keys,
            );
            Ok((response, StoreMutation::Unchanged))
        },
    )?;
    let json = if args.pretty.unwrap_or(false) {
        serde_json::to_string_pretty(&response)?
    } else {
        serde_json::to_string(&response)?
    };
    println!("{json}");
    Ok(())
}

fn log() -> anyhow::Result<()> {
    let release_log =
        ReleaseLog::in_data_dir().context(i18n::error_browser_release_log_no_data_dir())?;
    let entries = release_log.read()?;
    if entries.unreadable_lines > 0 {
        i18n::browser_log_unreadable_lines(&entries.unreadable_lines.to_string());
    }
    if entries.releases.is_empty() {
        i18n::browser_log_empty();
        return Ok(());
    }
    print!("{}", render_releases(&entries.releases));
    Ok(())
}

/// One tab-separated line per release under a localized header, so the
/// output pipes into `cut` or `column -t`. Control characters in the
/// values are escaped, so a stored name cannot forge extra rows or columns.
fn render_releases(releases: &[Release]) -> String {
    let header = [
        i18n::browser_log_header_time(),
        i18n::browser_log_header_domain(),
        i18n::browser_log_header_store(),
        i18n::browser_log_header_secret(),
        i18n::browser_log_header_caller(),
    ];
    let mut out = header.join("\t");
    out.push('\n');
    for release in releases {
        let row = [
            release.time.as_str(),
            release.domain.as_str(),
            release.store.as_str(),
            release.name.as_str(),
            release.caller.as_deref().unwrap_or("-"),
        ];
        let row: Vec<String> = row.iter().map(|value| escape_controls(value)).collect();
        out.push_str(&row.join("\t"));
        out.push('\n');
    }
    out
}

fn escape_controls(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_control() {
                c.escape_default().to_string()
            } else {
                c.to_string()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(name: &str, caller: Option<&str>) -> Release {
        Release {
            time: "2025-11-06T12:00:00Z".into(),
            domain: "example.com".into(),
            store: "personal".into(),
            name: name.into(),
            caller: caller.map(str::to_owned),
        }
    }

    #[test]
    fn releases_render_as_tab_separated_rows_under_a_header() {
        i18n::init_for_tests();
        let rendered = render_releases(&[
            release("alice", Some("chrome-extension://abcdefgh/")),
            release("bob", None),
        ]);
        assert_eq!(
            rendered,
            "Time\tDomain\tStore\tSecret\tCaller\n\
             2025-11-06T12:00:00Z\texample.com\tpersonal\talice\tchrome-extension://abcdefgh/\n\
             2025-11-06T12:00:00Z\texample.com\tpersonal\tbob\t-\n"
        );
    }

    #[test]
    fn control_characters_cannot_forge_rows_or_columns() {
        i18n::init_for_tests();
        let rendered = render_releases(&[release("a\tb\nc", None)]);
        assert_eq!(rendered.lines().count(), 2, "{rendered}");
        assert!(rendered.contains("a\\tb\\nc"), "{rendered}");
    }
}
