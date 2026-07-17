//! The SoroGuard keeper.
//!
//! Watches a guardian contract, and when one of its rules starts to hold, submits the
//! `execute` call that pulls the user out. The keeper holds no authority over any user's
//! funds. It pays a fee, signs the outer call, and lets the guardian and the user's own
//! policy decide whether anything happens.
//!
//! Run one, run ten. They don't coordinate and they don't trust each other. The guardian
//! re-checks every trigger on chain, so a wrong or lying keeper wastes only its own fee.

mod config;
mod signer;
mod submit;
mod tx;
mod watch;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use config::Config;
use signer::Signer;
use stellar_rpc_client::Client;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

/// Environment variable holding the keeper's `S...` secret seed.
const SECRET_ENV: &str = "SOROGUARD_KEEPER_SECRET";

#[derive(Parser)]
#[command(name = "soroguard-keeper", version, about)]
struct Cli {
    /// Path to the keeper's TOML config.
    #[arg(short, long, default_value = "soroguard.toml")]
    config: PathBuf,

    /// Check every watched rule once and exit, without submitting anything.
    #[arg(long)]
    once: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "soroguard_keeper=info".into()),
        )
        .init();

    let cli = Cli::parse();
    let config = Config::load(&cli.config)?;

    let secret = std::env::var(SECRET_ENV)
        .with_context(|| format!("set {SECRET_ENV} to the keeper's S... seed"))?;
    let signer = Signer::from_seed(&secret)?;

    let client = Client::new(&config.rpc_url).context("building RPC client")?;
    let ledger = client
        .get_latest_ledger()
        .await
        .context("cannot reach the RPC endpoint")?;

    info!(
        keeper = %signer.strkey(),
        guardian = %config.guardian_id,
        rules = ?config.rules,
        ledger = ledger.sequence,
        "keeper online",
    );

    if cli.once {
        sweep(&client, &signer, &config, cli.once).await;
        return Ok(());
    }

    loop {
        tokio::select! {
            _ = sweep(&client, &signer, &config, false) => {
                sleep(Duration::from_secs(config.interval_secs)).await;
            }
            _ = tokio::signal::ctrl_c() => {
                info!("shutting down");
                return Ok(());
            }
        }
    }
}

/// One pass over every watched rule.
async fn sweep(client: &Client, signer: &Signer, config: &Config, dry_run: bool) {
    for &id in &config.rules {
        match watch::rule_holds(client, signer, &config.guardian_id, id).await {
            Ok(false) => {}
            Ok(true) if dry_run => info!(rule = id, "would fire (dry run)"),
            Ok(true) => fire(client, signer, config, id).await,
            Err(e) => warn!(rule = id, error = %e, "could not evaluate rule"),
        }
    }
}

async fn fire(client: &Client, signer: &Signer, config: &Config, id: u32) {
    info!(rule = id, "trigger holds, submitting execute");
    match submit::execute(
        client,
        signer,
        &config.network_passphrase,
        &config.guardian_id,
        id,
        config.auth_ttl_ledgers,
    )
    .await
    {
        Ok(hash) => info!(rule = id, tx = %hex::encode(hash.0), "submitted"),
        Err(e) => error!(rule = id, error = %e, "execute failed"),
    }
}
