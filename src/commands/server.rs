// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! `pasejo server`: registering this installation as a device of a server,
//! signing in and out.

use std::fs;
use std::path::{Path, PathBuf, absolute};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use pasejo_client::session::{Poll, session_valid};
use pasejo_protocol::Id;
use pasejo_protocol::device::org_fingerprint;
use pasejo_protocol::encoding::b64;

use crate::cli::i18n;
use crate::models::cli::{ServerCommands, ServerLoginArgs, ServerLogoutArgs};
use crate::models::configuration::{Configuration, ServerRegistration};
use crate::remote::connect::{self, Remote, normalize_url, parse_hash};
use crate::remote::device::DeviceFiles;
use crate::remote::errors::Localize;

pub fn dispatch(command: &ServerCommands, configuration: &Configuration) -> Result<()> {
    match command {
        ServerCommands::Login(args) => login(configuration, args),
        ServerCommands::Logout(args) => logout(configuration, args),
        ServerCommands::Status => status(configuration),
    }
}

/// The label a device gets when none is given: the host name.
fn default_label() -> String {
    ["HOSTNAME", "COMPUTERNAME"]
        .iter()
        .find_map(|name| std::env::var(name).ok().filter(|value| !value.is_empty()))
        .or_else(|| {
            fs::read_to_string("/etc/hostname")
                .ok()
                .map(|text| text.trim().to_owned())
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_else(|| "pasejo".to_owned())
}

/// The recipient of the first global identity whose public half pasejo can
/// derive: an X25519 identity, or an SSH key with a `.pub` file beside it.
fn derive_recipient(configuration: &Configuration) -> Result<String> {
    for identity in &configuration.identities {
        if let Some(recipient) = recipient_of(&identity.file) {
            return Ok(recipient);
        }
    }
    anyhow::bail!(i18n::error_remote_recipient_required())
}

fn recipient_of(path: &Path) -> Option<String> {
    let text = zeroize::Zeroizing::new(fs::read_to_string(path).ok()?);
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("AGE-SECRET-KEY-")
            && let Ok(identity) = line.parse::<age::x25519::Identity>()
        {
            return Some(identity.to_public().to_string());
        }
    }
    let mut public = path.as_os_str().to_owned();
    public.push(".pub");
    let line = fs::read_to_string(PathBuf::from(public)).ok()?;
    let recipient = line
        .split_whitespace()
        .take(2)
        .collect::<Vec<_>>()
        .join(" ");
    recipient
        .parse::<age::ssh::Recipient>()
        .ok()
        .map(|_| recipient)
}

fn login(configuration: &Configuration, args: &ServerLoginArgs) -> Result<()> {
    if args.emergency {
        return emergency_login(configuration, &args.url);
    }
    let url = normalize_url(&args.url);
    let mut configuration = configuration.clone();
    let workload_token = args
        .workload_token
        .as_ref()
        .map(|path| absolute(path).context(i18n::error_could_not_resolve_store_path()))
        .transpose()?;
    let index = if let Some(index) = configuration
        .servers
        .iter()
        .position(|server| server.url == url)
    {
        if workload_token.is_some() {
            configuration.servers[index]
                .workload_token
                .clone_from(&workload_token);
        }
        index
    } else {
        let recipient = match &args.recipient {
            Some(recipient) => recipient.clone(),
            None => derive_recipient(&configuration)?,
        };
        crate::remote::funnel::validate_recipient(&recipient)?;
        let device = Id::random();
        DeviceFiles::new(device)?.create_signing_key()?;
        configuration.servers.push(ServerRegistration {
            url: url.clone(),
            device: device.to_string(),
            label: args.label.clone().unwrap_or_else(default_label),
            recipient,
            checkpoint_key: String::new(),
            workload_token,
            orgs: Vec::new(),
        });
        configuration.servers.len() - 1
    };
    let mut remote = Remote::offline(&configuration.servers[index])?;
    let info = remote.client.server_info().localized()?;
    let key = b64(&info.checkpoint_key.0);
    let registration = &mut configuration.servers[index];
    if registration.checkpoint_key.is_empty() {
        registration.checkpoint_key = key;
    } else if registration.checkpoint_key != key {
        anyhow::bail!(i18n::error_remote_checkpoint_key_changed(&url));
    }
    if let Some(path) = &registration.workload_token {
        remote.workload_login(path)?;
    } else {
        device_login(&mut remote)?;
    }
    let me = remote.client.me().localized()?;
    configuration.save_configuration()?;
    i18n::remote_logged_in(&url, &me.email);
    Ok(())
}

/// Signs an already registered device in while the server's emergency
/// window is open. The device proves itself with its signing key, so no
/// identity provider is involved and no new device can be created.
fn emergency_login(configuration: &Configuration, url: &str) -> Result<()> {
    let url = normalize_url(url);
    let server = configuration
        .servers
        .iter()
        .find(|server| server.url == url)
        .ok_or_else(|| anyhow::anyhow!(i18n::error_remote_emergency_unregistered(&url)))?;
    let mut remote = Remote::offline(server)?;
    let info = remote.client.server_info().localized()?;
    if server.checkpoint_key != b64(&info.checkpoint_key.0) {
        anyhow::bail!(i18n::error_remote_checkpoint_key_changed(&url));
    }
    let session = remote.client.emergency_login().localized()?;
    remote.files.save_session(&session)?;
    let me = remote.client.me().localized()?;
    i18n::remote_emergency_logged_in(
        &url,
        &me.email,
        &crate::commands::audit::rfc3339(session.expires),
    );
    Ok(())
}

/// Signs in through the device authorization grant: the user approves this
/// device in a browser while the client polls.
fn device_login(remote: &mut Remote) -> Result<()> {
    let authorization = remote.client.start_login().localized()?;
    i18n::remote_login_visit(
        &authorization.verification_uri_complete,
        &authorization.user_code,
    );
    i18n::remote_device_fingerprint(&remote.client.device.fingerprint());
    let mut interval = authorization.interval.max(1);
    let deadline = remote.client.now() + authorization.expires_in * 1000;
    loop {
        thread::sleep(Duration::from_secs(interval));
        match remote.client.poll_login(&authorization).localized()? {
            Poll::Pending => {}
            Poll::SlowDown => interval += 5,
            Poll::Done(session) => return remote.files.save_session(&session),
        }
        if remote.client.now() > deadline {
            anyhow::bail!(i18n::error_remote_login_expired());
        }
    }
}

fn logout(configuration: &Configuration, args: &ServerLogoutArgs) -> Result<()> {
    let server = connect::server(configuration, args.url.as_deref())?.clone();
    let mut remote = Remote::offline(&server)?;
    if let Some(session) = remote.files.session()?
        && session_valid(&session, remote.client.now())
    {
        remote.client.set_session(Some(session));
        // The session ends locally whether or not the server answers.
        let _ = remote.client.logout();
    }
    remote.files.clear_session()?;
    if args.forget {
        let mut configuration = configuration.clone();
        configuration
            .servers
            .retain(|registered| registered.url != server.url);
        configuration.save_configuration()?;
        remote.files.remove()?;
    }
    i18n::remote_logged_out(&server.url);
    Ok(())
}

fn status(configuration: &Configuration) -> Result<()> {
    if configuration.servers.is_empty() {
        i18n::remote_no_servers();
    }
    for server in &configuration.servers {
        let remote = Remote::offline(server)?;
        let session = remote
            .files
            .session()?
            .filter(|session| session_valid(session, remote.client.now()));
        i18n::remote_status_server(
            &server.url,
            &server.label,
            &server.device,
            &remote.client.device.fingerprint(),
            if session.is_some() { "yes" } else { "no" },
        );
        for org in &server.orgs {
            i18n::remote_status_org(
                &org.name,
                &org.id,
                &org_fingerprint(&parse_hash(&org.genesis)?),
            );
        }
    }
    Ok(())
}
