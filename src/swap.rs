//! Execute a swap.

mod action;
#[cfg(test)]
mod alice;
pub mod bitcoin;
mod bob;
mod comit;
mod db;
pub mod ethereum;

use crate::{
    network::Taker,
    swap::{action::LedgerTime, bob::WalletBob},
    SwapId,
};
use std::sync::Arc;

pub use self::comit::{hbit, herc20};
pub use db::Database;

// TODO: This is awkward to manipulate
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SwapKind {
    HbitHerc20(SwapParams),
    Herc20Hbit(SwapParams),
}

impl SwapKind {
    pub fn params(&self) -> SwapParams {
        match self {
            SwapKind::HbitHerc20(params) | SwapKind::Herc20Hbit(params) => params.clone(),
        }
    }

    pub async fn execute(
        &self,
        db: Arc<Database>,
        bitcoin_wallet: Arc<crate::bitcoin::Wallet>,
        ethereum_wallet: Arc<crate::ethereum::Wallet>,
        bitcoin_connector: Arc<comit::btsieve::bitcoin::BitcoindConnector>,
        ethereum_connector: Arc<comit::btsieve::ethereum::Web3Connector>,
    ) -> anyhow::Result<()> {
        let bitcoin_wallet = bitcoin::Wallet {
            inner: bitcoin_wallet,
            connector: Arc::clone(&bitcoin_connector),
        };
        let ethereum_wallet = ethereum::Wallet {
            inner: ethereum_wallet,
            connector: Arc::clone(&ethereum_connector),
        };

        match self {
            SwapKind::HbitHerc20(SwapParams {
                hbit_params,
                herc20_params,
                secret_hash,
                start_of_swap,
                swap_id,
                ..
            }) => {
                let bob = WalletBob {
                    alpha_wallet: bitcoin_wallet,
                    beta_wallet: ethereum_wallet,
                    db,
                    swap_id: *swap_id,
                    secret_hash: *secret_hash,
                    start_of_swap: *start_of_swap,
                    beta_expiry: herc20_params.expiry,
                };

                comit::hbit_herc20_bob(
                    bob,
                    bitcoin_connector.as_ref(),
                    ethereum_connector.as_ref(),
                    *hbit_params,
                    herc20_params.clone(),
                    *start_of_swap,
                )
                .await?
            }
            SwapKind::Herc20Hbit(SwapParams {
                hbit_params,
                herc20_params,
                secret_hash,
                start_of_swap,
                swap_id,
                ..
            }) => {
                let bob = WalletBob {
                    alpha_wallet: ethereum_wallet,
                    beta_wallet: bitcoin_wallet,
                    db,
                    swap_id: *swap_id,
                    secret_hash: *secret_hash,
                    start_of_swap: *start_of_swap,
                    beta_expiry: herc20_params.expiry,
                };

                comit::herc20_hbit_bob(
                    bob,
                    ethereum_connector.as_ref(),
                    bitcoin_connector.as_ref(),
                    herc20_params.clone(),
                    *hbit_params,
                    *start_of_swap,
                )
                .await?
            }
        };

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SwapParams {
    pub hbit_params: hbit::Params,
    pub herc20_params: herc20::Params,
    pub secret_hash: comit::SecretHash,
    // TODO: Why naive and not DateTime<Local>?
    pub start_of_swap: chrono::NaiveDateTime,
    pub swap_id: SwapId,
    pub taker: Taker,
}

#[cfg(test)]
impl Default for SwapParams {
    fn default() -> Self {
        use crate::swap::hbit::SecretHash;
        use ::bitcoin::secp256k1;
        use std::str::FromStr;

        let secret_hash =
            SecretHash::new(comit::Secret::from(*b"hello world, you are beautiful!!"));

        SwapParams {
            hbit_params: hbit::Params {
                shared: hbit::SharedParams {
                    network: ::bitcoin::Network::Regtest,
                    asset: comit::asset::Bitcoin::from_sat(12_345_678),
                    redeem_identity: comit::bitcoin::PublicKey::from_str(
                        "039b6347398505f5ec93826dc61c19f47c66c0283ee9be980e29ce325a0f4679ef",
                    )
                    .unwrap(),
                    refund_identity: comit::bitcoin::PublicKey::from_str(
                        "032e58afe51f9ed8ad3cc7897f634d881fdbe49a81564629ded8156bebd2ffd1af",
                    )
                    .unwrap(),
                    expiry: 12345678u32.into(),
                    secret_hash,
                },
                transient_sk: secp256k1::SecretKey::from_str(
                    "01010101010101010001020304050607ffff0000ffff00006363636363636363",
                )
                .unwrap(),
            },
            herc20_params: herc20::Params {
                asset: comit::asset::Erc20 {
                    token_contract: Default::default(),
                    quantity: comit::asset::Erc20Quantity::from_wei_dec_str(
                        "4_000_000_000_000_000_000",
                    )
                    .unwrap(),
                },
                redeem_identity: Default::default(),
                refund_identity: Default::default(),
                expiry: 987654321.into(),
                secret_hash,
                chain_id: 42.into(),
            },
            secret_hash: SecretHash::new(comit::Secret::from(*b"hello world, you are beautiful!!")),
            start_of_swap: chrono::Local::now().naive_local(),
            swap_id: Default::default(),
            taker: Taker::default(),
        }
    }
}

#[cfg(all(test, feature = "test-docker"))]
mod tests {
    use super::*;
    use crate::{
        swap::{
            alice::WalletAlice,
            bitcoin,
            comit::{
                asset::{
                    self,
                    ethereum::{Erc20Quantity, FromWei},
                },
                btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector},
                ethereum::ChainId,
                identity, Secret, SecretHash, Timestamp,
            },
        },
        test_harness, Seed, SwapId,
    };
    use ::bitcoin::secp256k1;
    use chrono::Utc;
    use std::{str::FromStr, sync::Arc};
    use testcontainers::clients;

    fn hbit_params(
        secret_hash: SecretHash,
        network: ::bitcoin::Network,
    ) -> (hbit::SharedParams, bitcoin::SecretKey, bitcoin::SecretKey) {
        let asset = asset::Bitcoin::from_sat(100_000_000);
        let expiry = Timestamp::now().plus(60 * 60);

        let (transient_refund_sk, transient_refund_pk) = {
            let transient_refund_sk = secp256k1::SecretKey::from_str(
                "01010101010101010001020304050607ffff0000ffff00006363636363636363",
            )
            .unwrap();
            let transient_refund_pk =
                identity::Bitcoin::from_secret_key(&crate::SECP, &transient_refund_sk);

            (transient_refund_sk, transient_refund_pk)
        };

        let (transient_redeem_sk, transient_redeem_pk) = {
            let transient_redeem_sk = secp256k1::SecretKey::from_str(
                "01010101010101010001020304050607ffff0000ffff00006363636363636363",
            )
            .unwrap();
            let transient_redeem_pk =
                identity::Bitcoin::from_secret_key(&crate::SECP, &transient_redeem_sk);

            (transient_redeem_sk, transient_redeem_pk)
        };

        let shared_params = hbit::SharedParams {
            network,
            asset,
            redeem_identity: transient_redeem_pk,
            refund_identity: transient_refund_pk,
            expiry,
            secret_hash,
        };

        (shared_params, transient_refund_sk, transient_redeem_sk)
    }

    fn secret() -> Secret {
        let bytes = b"hello world, you are beautiful!!";
        Secret::from(*bytes)
    }

    #[tokio::test]
    async fn execute_alice_hbit_herc20_swap() -> anyhow::Result<()> {
        let client = clients::Cli::default();

        let alice_db = Arc::new(Database::new_test().unwrap());
        let bob_db = Arc::new(Database::new_test().unwrap());

        let bitcoin_network = ::bitcoin::Network::Regtest;
        let (bitcoin_connector, bitcoind_url, bitcoin_blockchain) = {
            let blockchain = test_harness::bitcoin::Blockchain::new(&client)?;
            blockchain.init().await?;

            let node_url = blockchain.node_url.clone();

            (
                Arc::new(BitcoindConnector::new(
                    node_url.clone(),
                    crate::bitcoin::Network::Regtest,
                )?),
                node_url,
                blockchain,
            )
        };
        let ethereum_chain_id = ethereum::ChainId::regtest();
        let (ethereum_connector, ethereum_node_url, ethereum_blockchain, token_contract) = {
            let mut blockchain = test_harness::ethereum::Blockchain::new(&client)?;
            blockchain.init().await?;

            let node_url = blockchain.node_url.clone();
            let token_contract = blockchain.token_contract();

            (
                Arc::new(Web3Connector::new(node_url.clone())),
                node_url,
                blockchain,
                token_contract,
            )
        };

        let (alice_bitcoin_wallet, alice_ethereum_wallet) = {
            let seed = Seed::random().unwrap();
            let bitcoin_wallet = {
                let wallet =
                    crate::bitcoin::Wallet::new(seed, bitcoind_url.clone(), bitcoin_network)
                        .await?;

                bitcoin_blockchain
                    .mint(
                        wallet.new_address().await?,
                        asset::Bitcoin::from_sat(1_000_000_000).into(),
                    )
                    .await?;

                wallet
            };
            let ethereum_wallet = crate::ethereum::Wallet::new(
                seed,
                ethereum_node_url.clone(),
                crate::ethereum::Chain::new(ChainId::regtest(), token_contract),
            )
            .await?;

            // mint ether to pay for gas
            ethereum_blockchain
                .mint_ether(
                    ethereum_wallet.account(),
                    1_000_000_000_000_000_000u64.into(),
                    ethereum_chain_id,
                )
                .await?;

            (
                bitcoin::Wallet {
                    inner: Arc::new(bitcoin_wallet),
                    connector: Arc::clone(&bitcoin_connector),
                },
                ethereum::Wallet {
                    inner: Arc::new(ethereum_wallet),
                    connector: Arc::clone(&ethereum_connector),
                },
            )
        };

        let (bob_bitcoin_wallet, bob_ethereum_wallet) = {
            let seed = Seed::random().unwrap();
            let bitcoin_wallet =
                crate::bitcoin::Wallet::new(seed, bitcoind_url.clone(), bitcoin_network).await?;
            let ethereum_wallet = crate::ethereum::Wallet::new(
                seed,
                ethereum_node_url,
                crate::ethereum::Chain::new(ChainId::regtest(), token_contract),
            )
            .await?;

            ethereum_blockchain
                .mint_erc20_token(
                    ethereum_wallet.account(),
                    asset::Erc20::new(token_contract, Erc20Quantity::from_wei(5_000_000_000u64)),
                    ethereum_chain_id,
                )
                .await?;

            // mint ether to pay for gas
            ethereum_blockchain
                .mint_ether(
                    ethereum_wallet.account(),
                    1_000_000_000_000_000_000u64.into(),
                    ethereum_chain_id,
                )
                .await?;

            (
                bitcoin::Wallet {
                    inner: Arc::new(bitcoin_wallet),
                    connector: Arc::clone(&bitcoin_connector),
                },
                ethereum::Wallet {
                    inner: Arc::new(ethereum_wallet),
                    connector: Arc::clone(&ethereum_connector),
                },
            )
        };

        let secret = secret();
        let secret_hash = SecretHash::new(secret);

        let start_of_swap = Utc::now().naive_local();
        let beta_expiry = Timestamp::now().plus(60 * 60);

        let (hbit_params, hbit_transient_refund_sk, hbit_transient_redeem_sk) =
            hbit_params(secret_hash, bitcoin_network);

        let herc20_params = herc20::params(
            secret_hash,
            ethereum_chain_id,
            alice_ethereum_wallet.inner.account(),
            bob_ethereum_wallet.inner.account(),
            token_contract,
            beta_expiry,
        );

        let alice_swap = {
            let swap_id = SwapId::default();

            let swap = SwapKind::HbitHerc20(SwapParams {
                hbit_params: hbit::Params {
                    shared: hbit_params,
                    transient_sk: hbit_transient_refund_sk,
                },
                herc20_params: herc20_params.clone(),
                secret_hash,
                start_of_swap,
                swap_id,
                taker: Taker::default(),
            });

            alice_db.insert(swap).unwrap();

            let hbit_params = hbit::Params::new(hbit_params, hbit_transient_refund_sk);
            let alice = WalletAlice {
                alpha_wallet: alice_bitcoin_wallet.clone(),
                beta_wallet: alice_ethereum_wallet.clone(),
                db: Arc::clone(&alice_db),
                swap_id,
                secret,
                start_of_swap,
                beta_expiry: herc20_params.expiry,
            };

            comit::hbit_herc20_alice(
                alice,
                bitcoin_connector.as_ref(),
                ethereum_connector.as_ref(),
                hbit_params,
                herc20_params.clone(),
                secret,
                start_of_swap,
            )
        };

        let bob_swap = {
            let swap_id = SwapId::default();

            let swap = SwapKind::HbitHerc20(SwapParams {
                hbit_params: hbit::Params {
                    shared: hbit_params,
                    transient_sk: hbit_transient_redeem_sk,
                },
                herc20_params: herc20_params.clone(),
                secret_hash,
                start_of_swap,
                swap_id,
                taker: Taker::default(),
            });

            bob_db.insert(swap).unwrap();

            let hbit_params = hbit::Params::new(hbit_params, hbit_transient_redeem_sk);
            let bob = WalletBob {
                alpha_wallet: bob_bitcoin_wallet.clone(),
                beta_wallet: bob_ethereum_wallet.clone(),
                db: bob_db,
                swap_id,
                secret_hash,
                start_of_swap,
                beta_expiry: herc20_params.expiry,
            };

            comit::hbit_herc20_bob(
                bob,
                bitcoin_connector.as_ref(),
                ethereum_connector.as_ref(),
                hbit_params,
                herc20_params.clone(),
                start_of_swap,
            )
        };

        let alice_bitcoin_starting_balance = alice_bitcoin_wallet.inner.balance().await?;
        let bob_bitcoin_starting_balance = bob_bitcoin_wallet.inner.balance().await?;

        let alice_erc20_starting_balance = alice_ethereum_wallet
            .inner
            .erc20_balance(token_contract)
            .await?;
        let bob_erc20_starting_balance = bob_ethereum_wallet
            .inner
            .erc20_balance(token_contract)
            .await?;

        futures::future::try_join(alice_swap, bob_swap).await?;

        let alice_bitcoin_final_balance = alice_bitcoin_wallet.inner.balance().await?;
        let bob_bitcoin_final_balance = bob_bitcoin_wallet.inner.balance().await?;
        let bitcoin_max_fee = bitcoin::Amount::from_sat(100000);

        let alice_erc20_final_balance = alice_ethereum_wallet
            .inner
            .erc20_balance(token_contract)
            .await?;
        let bob_erc20_final_balance = bob_ethereum_wallet
            .inner
            .erc20_balance(token_contract)
            .await?;

        assert!(
            alice_bitcoin_final_balance
                >= alice_bitcoin_starting_balance - hbit_params.asset.into() - bitcoin_max_fee
        );
        assert!(
            bob_bitcoin_final_balance
                >= bob_bitcoin_starting_balance + hbit_params.asset.into() - bitcoin_max_fee
        );

        assert_eq!(
            alice_erc20_final_balance.quantity.to_u256(),
            alice_erc20_starting_balance.quantity.to_u256()
                + herc20_params.asset.quantity.to_u256()
        );
        assert_eq!(
            bob_erc20_final_balance.quantity.to_u256(),
            bob_erc20_starting_balance.quantity.to_u256() - herc20_params.asset.quantity.to_u256()
        );

        Ok(())
    }
}
