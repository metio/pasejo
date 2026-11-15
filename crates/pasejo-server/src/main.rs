// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::Context;
use clap::{Parser, Subcommand};
use pasejo_protocol::encoding::{b64, random};
use pasejo_server::audit::{self, Event};
use pasejo_server::auth::break_glass;
use pasejo_server::clock::Clock;
use pasejo_server::config::Config;
use pasejo_server::db::Db;
use pasejo_server::state::{AppState, ServerKeys};
use pasejo_server::{app, time};

/// Syncs pasejo remote stores, manages organizations and audits access.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// The configuration file.
    #[arg(
        long,
        env = "PASEJO_SERVER_CONFIG",
        default_value = "pasejo-server.toml",
        global = true
    )]
    config: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Applies migrations and serves the API.
    Serve,
    /// Applies database migrations and exits.
    Migrate,
    /// Prints a new master key for the master key file.
    Keygen,
    /// Manages the server's break-glass account.
    #[command(subcommand)]
    BreakGlass(BreakGlass),
    /// Manages the audit trail.
    #[command(subcommand)]
    Audit(AuditCommand),
}

#[derive(Subcommand)]
enum BreakGlass {
    /// Creates or replaces an account and prints its credentials once.
    Create {
        #[arg(default_value = "break-glass")]
        username: String,
    },
    /// Disables an account.
    Disable {
        #[arg(default_value = "break-glass")]
        username: String,
    },
}

#[derive(Subcommand)]
enum AuditCommand {
    /// Removes events older than the given number of days, keeping the last
    /// checkpoint before the cut.
    Prune {
        #[arg(long)]
        days: u64,
    },
}

async fn open(config: &Path) -> anyhow::Result<AppState> {
    let config = Config::load(config)?;
    let keys = ServerKeys::load(&config.master_key_file)?;
    let db = Db::connect(&config.database_url).await?;
    db.migrate().await?;
    AppState::new(config, db, keys, Arc::new(Clock::default()))
}

async fn serve(state: AppState) -> anyhow::Result<()> {
    app::spawn_background(&state);
    let listen = state.config.listen;
    let tls = state.config.tls.clone();
    let router = app::router(state.clone()).into_make_service_with_connect_info::<SocketAddr>();
    let handle = axum_server::Handle::new();
    let shutdown = handle.clone();
    tokio::spawn(async move {
        shutdown_signal().await;
        shutdown.graceful_shutdown(Some(std::time::Duration::from_secs(20)));
    });
    audit::record(
        &state,
        None,
        Event::ok("server.start").detail(env!("CARGO_PKG_VERSION")),
    )
    .await
    .map_err(|error| anyhow::anyhow!(error.message))?;
    if let Some(address) = state.config.metrics_listen {
        let metrics = app::metrics_router(state.clone()).into_make_service();
        let metrics_handle = handle.clone();
        tracing::info!(%address, "serving metrics");
        tokio::spawn(async move {
            if let Err(error) = axum_server::bind(address)
                .handle(metrics_handle)
                .serve(metrics)
                .await
            {
                tracing::error!(%error, "the metrics listener stopped");
            }
        });
    }
    tracing::info!(%listen, tls = tls.is_some(), "serving");
    match tls {
        Some(tls) => {
            let rustls =
                axum_server::tls_rustls::RustlsConfig::from_pem_file(&tls.certificate, &tls.key)
                    .await
                    .context("cannot load the TLS certificate and key")?;
            axum_server::bind_rustls(listen, rustls)
                .handle(handle)
                .serve(router)
                .await?;
        }
        None => {
            axum_server::bind(listen)
                .handle(handle)
                .serve(router)
                .await?;
        }
    }
    Ok(())
}

async fn shutdown_signal() {
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

async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Command::Keygen => {
            println!("{}", b64(&random::<32>()));
        }
        Command::Migrate => {
            let config = Config::load(&cli.config)?;
            Db::connect(&config.database_url).await?.migrate().await?;
            println!("migrations applied");
        }
        Command::Serve => serve(open(&cli.config).await?).await?,
        Command::BreakGlass(BreakGlass::Create { username }) => {
            let state = open(&cli.config).await?;
            let created = break_glass::create(&state, &username).await?;
            audit::record(
                &state,
                None,
                Event::ok("break-glass.create").detail(username.clone()),
            )
            .await
            .map_err(|error| anyhow::anyhow!(error.message))?;
            println!("username: {username}");
            println!("password: {}", created.password.as_str());
            println!("totp secret: {}", created.totp_secret.as_str());
            println!("totp uri: {}", created.totp_uri.as_str());
            if !state.config.break_glass.enabled {
                eprintln!(
                    "break-glass login is disabled; set [break_glass] enabled = true to use the account"
                );
            }
        }
        Command::BreakGlass(BreakGlass::Disable { username }) => {
            let state = open(&cli.config).await?;
            break_glass::disable(&state, &username).await?;
            audit::record(
                &state,
                None,
                Event::ok("break-glass.disable").detail(username.clone()),
            )
            .await
            .map_err(|error| anyhow::anyhow!(error.message))?;
            println!("disabled {username}");
        }
        Command::Audit(AuditCommand::Prune { days }) => {
            let state = open(&cli.config).await?;
            let before = state.now().saturating_sub(days * 86_400_000);
            let removed = audit::prune(&state, before)
                .await
                .map_err(|error| anyhow::anyhow!(error.message))?;
            println!("removed {removed} events before {}", time::rfc3339(before));
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("cannot start the runtime: {error}");
            return ExitCode::FAILURE;
        }
    };
    match runtime.block_on(run(cli)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}
