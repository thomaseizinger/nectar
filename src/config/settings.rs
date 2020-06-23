use crate::config::{file, Bitcoin, Bitcoind, Ethereum, File, RpcEndpoint, Spread};
use log::LevelFilter;

/// This structs represents the settings as they are used through out the code.
///
/// An optional setting (represented in this struct as an `Option`) has semantic
/// meaning in cnd. Contrary to that, many configuration values are optional in
/// the config file but may be replaced by default values when the `Settings`
/// are created from a given `Config`.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub logging: Logging,
    pub bitcoin: Bitcoin,
    pub ethereum: Ethereum,
    pub spread: Spread,
}

fn derive_url_bitcoin(bitcoin: Option<file::Bitcoin>) -> Bitcoin {
    match bitcoin {
        None => Bitcoin::default(),
        Some(bitcoin) => {
            let node_url = match bitcoin.bitcoind {
                Some(bitcoind) => bitcoind.node_url,
                None => match bitcoin.network {
                    bitcoin::Network::Bitcoin => "http://localhost:8332"
                        .parse()
                        .expect("to be valid static string"),
                    bitcoin::Network::Testnet => "http://localhost:18332"
                        .parse()
                        .expect("to be valid static string"),
                    bitcoin::Network::Regtest => "http://localhost:18443"
                        .parse()
                        .expect("to be valid static string"),
                },
            };
            Bitcoin {
                network: bitcoin.network,
                bitcoind: Bitcoind { node_url },
            }
        }
    }
}

fn derive_url_ethereum(ethereum: Option<file::Ethereum>) -> Ethereum {
    match ethereum {
        None => Ethereum::default(),
        Some(ethereum) => {
            let node_url = match ethereum.rpc_endpoint {
                None => {
                    // default is linkpool mainnet (publicly accessible)
                    "https://main-rpc.linkpool.io/"
                        .parse()
                        .expect("to be valid static string")
                }
                Some(geth) => geth.url,
            };
            Ethereum {
                chain_id: ethereum.chain_id,
                rpc_endpoint: RpcEndpoint { url: node_url },
            }
        }
    }
}

impl From<Settings> for File {
    fn from(settings: Settings) -> Self {
        let Settings {
            logging: Logging { level },
            bitcoin,
            ethereum,
            spread,
        } = settings;

        File {
            logging: Some(file::Logging {
                level: Some(level.into()),
            }),
            bitcoin: Some(bitcoin.into()),
            ethereum: Some(ethereum.into()),
            spread: Some(spread)
        }
    }
}


#[derive(Clone, Copy, Debug, PartialEq, derivative::Derivative)]
#[derivative(Default)]
pub struct Logging {
    #[derivative(Default(value = "LevelFilter::Info"))]
    pub level: LevelFilter,
}

impl Settings {
    pub fn from_config_file_and_defaults(config_file: File) -> anyhow::Result<Self> {
        let File {
            logging,
            bitcoin,
            ethereum,
            spread,
        } = config_file;

        Ok(Self {
            logging: {
                match logging {
                    None => Logging::default(),
                    Some(inner) => match inner {
                        file::Logging { level: None } => Logging::default(),
                        file::Logging { level: Some(level) } => Logging {
                            level: level.into(),
                        },
                    },
                }
            },
            bitcoin: derive_url_bitcoin(bitcoin),
            ethereum: derive_url_ethereum(ethereum),
            spread: {
                match spread {
                    None => Spread::default(),
                    Some (spread) => spread
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::file};
    use comit::ethereum::ChainId;
    use spectral::prelude::*;

    #[test]
    fn logging_section_defaults_to_info() {
        let config_file = File {
            logging: None,
            ..File::default()
        };

        let settings = Settings::from_config_file_and_defaults(config_file);

        assert_that(&settings)
            .is_ok()
            .map(|settings| &settings.logging)
            .is_equal_to(Logging {
                level: LevelFilter::Info,
            })
    }

    #[test]
    fn bitcoin_defaults() {
        let config_file = File { ..File::default() };

        let settings = Settings::from_config_file_and_defaults(config_file);

        assert_that(&settings)
            .is_ok()
            .map(|settings| &settings.bitcoin)
            .is_equal_to(Bitcoin {
                network: bitcoin::Network::Regtest,
                bitcoind: Bitcoind {
                    node_url: "http://localhost:18443".parse().unwrap(),
                },
            })
    }

    #[test]
    fn bitcoin_defaults_network_only() {
        let defaults = vec![
            (bitcoin::Network::Bitcoin, "http://localhost:8332"),
            (bitcoin::Network::Testnet, "http://localhost:18332"),
            (bitcoin::Network::Regtest, "http://localhost:18443"),
        ];

        for (network, url) in defaults {
            let config_file = File {
                bitcoin: Some(file::Bitcoin {
                    network,
                    bitcoind: None,
                }),
                ..File::default()
            };

            let settings = Settings::from_config_file_and_defaults(config_file);

            assert_that(&settings)
                .is_ok()
                .map(|settings| &settings.bitcoin)
                .is_equal_to(Bitcoin {
                    network,
                    bitcoind: Bitcoind {
                        node_url: url.parse().unwrap(),
                    },
                })
        }
    }

    #[test]
    fn ethereum_defaults() {
        let config_file = File { ..File::default() };

        let settings = Settings::from_config_file_and_defaults(config_file);

        assert_that(&settings)
            .is_ok()
            .map(|settings| &settings.ethereum)
            .is_equal_to(Ethereum {
                chain_id: ChainId::mainnet(),
                rpc_endpoint: RpcEndpoint {
                    url: "https://main-rpc.linkpool.io/".parse().unwrap(),
                },
            })
    }

    #[test]
    fn ethereum_defaults_chain_id_only() {
        let defaults = vec![
            (ChainId::mainnet(), "https://main-rpc.linkpool.io/"),
            (ChainId::ropsten(), "https://main-rpc.linkpool.io/"),
            (ChainId::regtest(), "https://main-rpc.linkpool.io/"),
        ];

        for (chain_id, url) in defaults {
            let ethereum = Some(file::Ethereum {
                chain_id,
                rpc_endpoint: None,
            });
            let config_file = File {
                ethereum,
                ..File::default()
            };

            let settings = Settings::from_config_file_and_defaults(config_file);

            assert_that(&settings)
                .is_ok()
                .map(|settings| &settings.ethereum)
                .is_equal_to(Ethereum {
                    chain_id,
                    rpc_endpoint: RpcEndpoint {
                        url: url.parse().unwrap(),
                    },
                })
        }
    }
}
