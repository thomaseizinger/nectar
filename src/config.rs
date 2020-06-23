pub mod file;
mod serde_bitcoin_network;
pub mod settings;
pub mod validation;

use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::path::{PathBuf, Path};

pub use self::{file::File, settings::Settings};
use anyhow::Context;
use comit::ethereum::ChainId;

// TODO: This might need some adaption
// TODO: Additionally, always creating a default will require some adaption (because of infura configuration)
// Linux: /home/<user>/.config/comit/
// Windows: C:\Users\<user>\AppData\Roaming\comit\config\
// OSX: /Users/<user>/Library/Preferences/comit/
fn config_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "nectar")
        .map(|proj_dirs| proj_dirs.config_dir().to_path_buf())
}

pub fn default_config_path() -> anyhow::Result<PathBuf> {
    config_dir()
        .map(|dir| Path::join(&dir, "nectar.toml"))
        .context("Could not generate default configuration path")
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Spread {
    pub percent: f64,
}

impl Default for Spread {
    fn default() -> Self {
        Spread {
            percent: 3.0
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Bitcoin {
    #[serde(with = "crate::config::serde_bitcoin_network")]
    pub network: bitcoin::Network,
    pub bitcoind: Bitcoind,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Bitcoind {
    pub node_url: Url,
}

impl Default for Bitcoin {
    fn default() -> Self {
        Self {
            network: bitcoin::Network::Regtest,
            bitcoind: Bitcoind {
                node_url: Url::parse("http://localhost:18443")
                    .expect("static string to be a valid url"),
            },
        }
    }
}

impl From<Bitcoin> for file::Bitcoin {
    fn from(bitcoin: Bitcoin) -> Self {
        file::Bitcoin {
            network: bitcoin.network,
            bitcoind: Some(bitcoin.bitcoind),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Ethereum {
    pub chain_id: ChainId,
    pub rpc_endpoint: RpcEndpoint,
}

impl From<Ethereum> for file::Ethereum {
    fn from(ethereum: Ethereum) -> Self {
        file::Ethereum {
            chain_id: ethereum.chain_id,
            rpc_endpoint: Some(ethereum.rpc_endpoint),
        }
    }
}

impl Default for Ethereum {
    fn default() -> Self {
        Self {
            chain_id: ChainId::mainnet(),
            rpc_endpoint: RpcEndpoint {
                url: Url::parse("https://main-rpc.linkpool.io/")
                    .expect("static string to be a valid url"),
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RpcEndpoint {
    pub url: Url,
}
