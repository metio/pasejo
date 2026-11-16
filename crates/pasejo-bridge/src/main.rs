// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use pasejo_bridge::bridge::Bridge;
use pasejo_bridge::config::Settings;
use pasejo_bridge::device::Device;
use pasejo_bridge::http::{AppState, router};

/// Serves secrets of pasejo remote stores to external-secrets-operator.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Writes new device keys to a file that must not exist yet.
    Keygen {
        /// Where to write the keys.
        #[arg(long, default_value = "device.json")]
        output: PathBuf,
    },
    /// Prints a new bearer token for a tenant and the line for the
    /// configuration.
    Credential {
        /// The tenant label the server records with each read, for example
        /// the Kubernetes namespace.
        #[arg(long)]
        tenant: String,
    },
    /// Serves the API.
    Serve {
        /// The configuration file.
        #[arg(
            long,
            env = "PASEJO_BRIDGE_CONFIG",
            default_value = "/etc/pasejo-bridge/config.toml"
        )]
        config: PathBuf,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Keygen { output } => {
            let device = Device::generate();
            device.create(&output)?;
            eprintln!(
                "Wrote device {} to {}.\nRecipient: {}\nFingerprint: {}",
                device.id,
                output.display(),
                device.recipient(),
                device.identity("").fingerprint()
            );
            Ok(())
        }
        Command::Credential { tenant } => {
            if !pasejo_protocol::api::valid_tenant(&tenant) {
                anyhow::bail!(
                    "a tenant label has 1 to {} ASCII letters, digits, '-', '.', '_' or '/'",
                    pasejo_protocol::api::MAX_TENANT_LEN
                );
            }
            let (token, hash) = pasejo_bridge::credential();
            println!(
                "# The bearer token for tenant {tenant}; store it in the tenant's webhook Secret.\n{}\n\n# Add to the bridge configuration:\n[[credentials]]\ntenant = \"{tenant}\"\ntoken_sha256 = \"{hash}\"",
                token.as_str()
            );
            Ok(())
        }
        Command::Serve { config } => serve(&config),
    }
}

#[tokio::main]
async fn serve(path: &std::path::Path) -> Result<()> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    let settings = Settings::load(path)?;
    let device = Device::load(&settings.config.device_file)?;
    let listen = settings.config.listen;
    let credentials = Arc::new(settings.credentials.clone());
    let bridge = Arc::new(Mutex::new(Bridge::new(&settings, device)));
    {
        let bridge = Arc::clone(&bridge);
        tokio::task::spawn_blocking(move || {
            let Ok(mut bridge) = bridge.lock() else {
                return;
            };
            match bridge.ready() {
                Ok(_) => tracing::info!("signed in; the device is active"),
                Err(failure) => tracing::warn!(?failure, "not ready yet"),
            }
        });
    }
    let listener = tokio::net::TcpListener::bind(listen)
        .await
        .with_context(|| format!("cannot listen on {listen}"))?;
    tracing::info!(%listen, "serving");
    axum::serve(
        listener,
        router(AppState {
            bridge,
            credentials,
        }),
    )
    .with_graceful_shutdown(shutdown())
    .await?;
    Ok(())
}

async fn shutdown() {
    let interrupt = tokio::signal::ctrl_c();
    #[cfg(unix)]
    {
        let Ok(mut terminate) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        else {
            let _ = interrupt.await;
            return;
        };
        tokio::select! {
            _ = interrupt => {}
            _ = terminate.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = interrupt.await;
    }
}
