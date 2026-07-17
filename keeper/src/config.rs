use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

/// Everything the keeper needs to watch a guardian and act on it.
///
/// The secret key is read from the environment, never the file, so a config can be committed
/// or shared without leaking the key that pays for and signs transactions.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Soroban RPC endpoint, e.g. `https://soroban-testnet.stellar.org`.
    pub rpc_url: String,

    /// Network passphrase the RPC serves. Signatures are bound to it, so a wrong value here
    /// produces transactions the network rejects rather than misapplies.
    pub network_passphrase: String,

    /// The guardian contract this keeper watches, as a `C...` strkey.
    pub guardian_id: String,

    /// Rule ids to watch. A keeper need not watch every rule; several keepers can split the
    /// set between them, and the guardian is the same either way.
    pub rules: Vec<u32>,

    /// Seconds between sweeps.
    #[serde(default = "default_interval")]
    pub interval_secs: u64,

    /// How far ahead a rule's authorization stays valid once submitted, in ledgers.
    #[serde(default = "default_auth_ttl")]
    pub auth_ttl_ledgers: u32,
}

fn default_interval() -> u64 {
    10
}

fn default_auth_ttl() -> u32 {
    60
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading config at {}", path.display()))?;
        let config: Config = toml::from_str(&text).context("parsing config")?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        anyhow::ensure!(!self.rpc_url.is_empty(), "rpc_url must be set");
        anyhow::ensure!(
            !self.network_passphrase.is_empty(),
            "network_passphrase must be set"
        );
        anyhow::ensure!(!self.rules.is_empty(), "no rules to watch");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_config_and_applies_defaults() {
        let toml = r#"
            rpc_url = "https://soroban-testnet.stellar.org"
            network_passphrase = "Test SDF Network ; September 2015"
            guardian_id = "CBQHNAXSI55GX2GN6D67GK7BHVPSLJUGZQEU7WJ5LKR5PNUCGLIMAO4K"
            rules = [0, 1, 2]
        "#;

        let config: Config = toml::from_str(toml).unwrap();
        config.validate().unwrap();

        assert_eq!(config.rules, vec![0, 1, 2]);
        assert_eq!(config.interval_secs, 10);
        assert_eq!(config.auth_ttl_ledgers, 60);
    }

    #[test]
    fn rejects_a_config_with_no_rules() {
        let toml = r#"
            rpc_url = "https://soroban-testnet.stellar.org"
            network_passphrase = "Test SDF Network ; September 2015"
            guardian_id = "CBQHNAXSI55GX2GN6D67GK7BHVPSLJUGZQEU7WJ5LKR5PNUCGLIMAO4K"
            rules = []
        "#;

        let config: Config = toml::from_str(toml).unwrap();
        assert!(config.validate().is_err());
    }
}
