mod file;
mod serde_bitcoin_network;
mod settings;
mod validation;

use crate::ethereum::ChainId;
use libp2p::Multiaddr;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub use self::{
    file::File,
    settings::{AllowedOrigins, Settings},
    validation::validate_connection_to_network,
};

lazy_static::lazy_static! {
    pub static ref LND_URL: Url = Url::parse("https://localhost:8080").expect("static string to be a valid url");
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Data {
    pub dir: PathBuf,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Network {
    pub listen: Vec<Multiaddr>,
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
            network: bitcoin::Network::Bitcoin,
            bitcoind: Bitcoind {
                node_url: Url::parse("http://localhost:8332")
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
    pub geth: Geth,
}

impl From<Ethereum> for file::Ethereum {
    fn from(ethereum: Ethereum) -> Self {
        file::Ethereum {
            chain_id: ethereum.chain_id,
            geth: Some(ethereum.geth),
        }
    }
}

impl Default for Ethereum {
    fn default() -> Self {
        Self {
            chain_id: ChainId::mainnet(),
            geth: Geth {
                node_url: Url::parse("http://localhost:8545")
                    .expect("static string to be a valid url"),
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Geth {
    pub node_url: Url,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Lightning {
    pub network: bitcoin::Network,
    pub lnd: Lnd,
}

impl Default for Lightning {
    fn default() -> Self {
        Self {
            network: bitcoin::Network::Bitcoin,
            lnd: Lnd::default(),
        }
    }
}

impl From<Lightning> for file::Lightning {
    fn from(lightning: Lightning) -> Self {
        file::Lightning {
            lnd: Some(file::Lnd {
                rest_api_url: lightning.lnd.rest_api_url,
                dir: lightning.lnd.dir,
            }),
            network: lightning.network,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Lnd {
    pub rest_api_url: Url,
    pub dir: PathBuf,
    pub cert_path: PathBuf,
    pub readonly_macaroon_path: PathBuf,
}

impl Default for Lnd {
    fn default() -> Self {
        Self::new(bitcoin::Network::Bitcoin)
    }
}

impl Lnd {
    fn new(network: bitcoin::Network) -> Self {
        Self {
            rest_api_url: LND_URL.clone(),
            dir: default_lnd_dir(),
            cert_path: default_lnd_cert_path(default_lnd_dir()),
            readonly_macaroon_path: default_lnd_readonly_macaroon_path(default_lnd_dir(), network),
        }
    }
}

fn default_lnd_dir() -> PathBuf {
    crate::lnd_dir().expect("no home directory")
}

fn default_lnd_cert_path(lnd_dir: PathBuf) -> PathBuf {
    lnd_dir.join("tls.cert")
}

fn default_lnd_readonly_macaroon_path(lnd_dir: PathBuf, network: bitcoin::Network) -> PathBuf {
    let network_dir = match network {
        bitcoin::Network::Bitcoin => "mainnet",
        bitcoin::Network::Testnet => "testnet",
        bitcoin::Network::Regtest => "regtest",
    };
    lnd_dir
        .join("data")
        .join("chain")
        .join("bitcoin")
        .join(network_dir)
        .join("readonly.macaroon")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_deserializes_correctly() {
        let file_contents = vec![
            r#"
            listen = ["/ip4/0.0.0.0/tcp/9939"]
            "#,
            r#"
            listen = ["/ip4/0.0.0.0/tcp/9939", "/ip4/127.0.0.1/tcp/9939"]
            "#,
        ];

        let expected = vec![
            Network {
                listen: vec!["/ip4/0.0.0.0/tcp/9939".parse().unwrap()],
            },
            Network {
                listen: (vec![
                    "/ip4/0.0.0.0/tcp/9939".parse().unwrap(),
                    "/ip4/127.0.0.1/tcp/9939".parse().unwrap(),
                ]),
            },
        ];

        let actual = file_contents
            .into_iter()
            .map(toml::from_str)
            .collect::<Result<Vec<Network>, toml::de::Error>>()
            .unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn lnd_deserializes_correctly() {
        let actual = toml::from_str(
            r#"
            rest_api_url = "https://localhost:8080"
            dir = "~/.local/share/comit/lnd"
            "#,
        );

        let expected = file::Lnd {
            rest_api_url: LND_URL.clone(),
            dir: PathBuf::from("~/.local/share/comit/lnd"),
        };

        assert_eq!(actual, Ok(expected));
    }

    #[test]
    fn lightning_deserializes_correctly() {
        let actual = toml::from_str(
            r#"
            network = "regtest"
            [lnd]
            rest_api_url = "https://localhost:8080"
            dir = "/path/to/lnd"
            "#,
        );

        let expected = file::Lightning {
            network: bitcoin::Network::Regtest,
            lnd: Some(file::Lnd {
                rest_api_url: LND_URL.clone(),
                dir: PathBuf::from("/path/to/lnd"),
            }),
        };

        assert_eq!(actual, Ok(expected));
    }
}
