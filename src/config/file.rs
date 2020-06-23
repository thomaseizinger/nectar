use crate::{
    config::{Bitcoind, RpcEndpoint},
};
use config as config_rs;
use log::LevelFilter;
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    path::{Path},
};
use comit::ethereum::ChainId;
use crate::config::Spread;

/// This struct aims to represent the configuration file as it appears on disk.
///
/// Most importantly, optional elements of the configuration file are
/// represented as `Option`s` here. This allows us to create a dedicated step
/// for filling in default values for absent configuration options.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct File {
    pub logging: Option<Logging>,
    pub bitcoin: Option<Bitcoin>,
    pub ethereum: Option<Ethereum>,
    pub spread: Option<Spread>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Bitcoin {
    #[serde(with = "crate::config::serde_bitcoin_network")]
    pub network: bitcoin::Network,
    pub bitcoind: Option<Bitcoind>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Ethereum {
    pub chain_id: ChainId,
    pub rpc_endpoint: Option<RpcEndpoint>,
}

impl File {
    pub fn default() -> Self {
        File {
            logging: Option::None,
            bitcoin: Option::None,
            ethereum: Option::None,
            spread: Option::None,
        }
    }

    pub fn read<D>(config_file: D) -> Result<Self, config_rs::ConfigError>
    where
        D: AsRef<OsStr>,
    {
        let config_file = Path::new(&config_file);

        let mut config = config_rs::Config::new();
        config.merge(config_rs::File::from(config_file))?;
        config.try_into()
    }
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Logging {
    pub level: Option<Level>,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum Level {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl From<LevelFilter> for Level {
    fn from(level: LevelFilter) -> Self {
        match level {
            LevelFilter::Off => Level::Error, // We don't support suppressing all logs.
            LevelFilter::Error => Level::Error,
            LevelFilter::Warn => Level::Warn,
            LevelFilter::Info => Level::Info,
            LevelFilter::Debug => Level::Debug,
            LevelFilter::Trace => Level::Trace,
        }
    }
}

impl From<Level> for LevelFilter {
    fn from(level: Level) -> Self {
        match level {
            Level::Error => LevelFilter::Error,
            Level::Warn => LevelFilter::Warn,
            Level::Info => LevelFilter::Info,
            Level::Debug => LevelFilter::Debug,
            Level::Trace => LevelFilter::Trace,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Bitcoind, RpcEndpoint, Settings};
    use reqwest::Url;
    use spectral::prelude::*;

    #[derive(serde::Deserialize, PartialEq, Debug)]
    struct LoggingOnlyConfig {
        logging: Logging,
    }

    #[test]
    fn full_config_deserializes_correctly() {
        let contents = r#"
[logging]
level = "Debug"

[bitcoin]
network = "regtest"

[bitcoin.bitcoind]
node_url = "http://localhost:18443/"

[ethereum]
chain_id = 1337

[ethereum.rpc_endpoint]
url = "http://localhost:8545/"

[spread]
percent = 3.5
"#;
        let file = File {
            logging: Some(Logging {
                level: Some(Level::Debug),
            }),
            bitcoin: Some(Bitcoin {
                network: bitcoin::Network::Regtest,
                bitcoind: Some(Bitcoind {
                    node_url: "http://localhost:18443".parse().unwrap(),
                }),
            }),
            ethereum: Some(Ethereum {
                chain_id: ChainId::regtest(),
                rpc_endpoint: Some(RpcEndpoint {
                    url: "http://localhost:8545".parse().unwrap(),
                }),
            }),
            spread: Some(Spread {
                percent: 3.5
            })
        };

        let config = toml::from_str::<File>(contents);
        assert_that(&config).is_ok().is_equal_to(file);
    }

    #[test]
    fn config_with_defaults_roundtrip() {
        // we start with the default config file
        let default_file = File::default();

        // convert to settings, this populates all empty fields with defaults
        let effective_settings = Settings::from_config_file_and_defaults(default_file).unwrap();

        // write settings back to file
        let file_with_effective_settings = File::from(effective_settings);

        let serialized = toml::to_string(&file_with_effective_settings).unwrap();
        let file = toml::from_str::<File>(&serialized).unwrap();

        assert_eq!(file, file_with_effective_settings)
    }

    #[test]
    fn bitcoin_deserializes_correctly() {
        let file_contents = vec![
            r#"
            network = "mainnet"
            [bitcoind]
            node_url = "http://example.com:8332"
            "#,
            r#"
            network = "testnet"
            [bitcoind]
            node_url = "http://example.com:18332"
            "#,
            r#"
            network = "regtest"
            [bitcoind]
            node_url = "http://example.com:18443"
            "#,
        ];

        let expected = vec![
            Bitcoin {
                network: bitcoin::Network::Bitcoin,
                bitcoind: Some(Bitcoind {
                    node_url: Url::parse("http://example.com:8332").unwrap(),
                }),
            },
            Bitcoin {
                network: bitcoin::Network::Testnet,
                bitcoind: Some(Bitcoind {
                    node_url: Url::parse("http://example.com:18332").unwrap(),
                }),
            },
            Bitcoin {
                network: bitcoin::Network::Regtest,
                bitcoind: Some(Bitcoind {
                    node_url: Url::parse("http://example.com:18443").unwrap(),
                }),
            },
        ];

        let actual = file_contents
            .into_iter()
            .map(toml::from_str)
            .collect::<Result<Vec<Bitcoin>, toml::de::Error>>()
            .unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn ethereum_deserializes_correctly() {
        let file_contents = vec![
            r#"
            chain_id = 1337
            [rpc_endpoint]
            url = "http://example.com:8545"
            "#,
            r#"
            chain_id = 3
            [rpc_endpoint]
            url = "http://example.com:8545"
            "#,
            r#"
            chain_id = 1
            [rpc_endpoint]
            url = "http://example.com:8545"
            "#,
        ];

        let expected = vec![
            Ethereum {
                chain_id: ChainId::regtest(),
                rpc_endpoint: Some(RpcEndpoint {
                    url: Url::parse("http://example.com:8545").unwrap(),
                }),
            },
            Ethereum {
                chain_id: ChainId::ropsten(),
                rpc_endpoint: Some(RpcEndpoint {
                    url: Url::parse("http://example.com:8545").unwrap(),
                }),
            },
            Ethereum {
                chain_id: ChainId::mainnet(),
                rpc_endpoint: Some(RpcEndpoint {
                    url: Url::parse("http://example.com:8545").unwrap(),
                }),
            },
        ];

        let actual = file_contents
            .into_iter()
            .map(toml::from_str)
            .collect::<Result<Vec<Ethereum>, toml::de::Error>>()
            .unwrap();

        assert_eq!(actual, expected);
    }
}
