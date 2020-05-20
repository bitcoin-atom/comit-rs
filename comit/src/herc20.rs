//! Htlc ERC20 Token atomic swap protocol.

use crate::{
    asset, ethereum::Bytes, htlc_location, identity, timestamp::Timestamp, transaction, Secret,
    SecretHash,
};
use blockchain_contracts::ethereum::rfc003::Erc20Htlc;
use chrono::NaiveDateTime;
use futures::{
    future::{self, Either},
    Stream,
};
use genawaiter::sync::{Co, Gen};

/// Data required to create a swap that involves an ERC20 token.
#[derive(Clone, Debug, PartialEq)]
pub struct CreatedSwap {
    pub asset: asset::Erc20,
    pub identity: identity::Ethereum,
    pub chain_id: u32,
    pub absolute_expiry: u32,
}

/// Resolves when said event has occurred.
#[async_trait::async_trait]
pub trait WaitForDeployed {
    async fn wait_for_deployed(&self, params: Params) -> anyhow::Result<Deployed>;
}

#[async_trait::async_trait]
pub trait WaitForFunded {
    async fn wait_for_funded(&self, params: Params, deployed: Deployed) -> anyhow::Result<Funded>;
}

#[async_trait::async_trait]
pub trait WaitForRedeemed {
    async fn wait_for_redeemed(
        &self,
        params: Params,
        deployed: Deployed,
    ) -> anyhow::Result<Redeemed>;
}

#[async_trait::async_trait]
pub trait WaitForRefunded {
    async fn wait_for_refunded(
        &self,
        params: Params,
        deployed: Deployed,
    ) -> anyhow::Result<Refunded>;
}

/// Represents the events in the herc20 protocol.
#[derive(Debug, Clone, PartialEq, strum_macros::Display)]
pub enum Event {
    /// The protocol was started.
    Started,

    /// The HTLC was deployed and is pending funding.
    Deployed(Deployed),

    /// The HTLC has been funded with ERC20 tokens.
    Funded(Funded),

    /// The HTLC has been destroyed via the redeem path, token have been sent to
    /// the redeemer.
    Redeemed(Redeemed),

    /// The HTLC has been destroyed via the refund path, token has been sent
    /// back to funder.
    Refunded(Refunded),
}

/// Represents the data available at said state.
#[derive(Debug, Clone, PartialEq)]
pub struct Deployed {
    pub transaction: transaction::Ethereum,
    pub location: htlc_location::Ethereum,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Funded {
    Correctly {
        transaction: transaction::Ethereum,
        asset: asset::Erc20,
    },
    Incorrectly {
        transaction: transaction::Ethereum,
        asset: asset::Erc20,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Redeemed {
    pub transaction: transaction::Ethereum,
    pub secret: Secret,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Refunded {
    pub transaction: transaction::Ethereum,
}

/// Creates a new instance of the herc20 protocol.
///
/// Returns a stream of events happening during the execution.
pub fn new<'a, C>(
    connector: &'a C,
    params: Params,
) -> impl Stream<Item = anyhow::Result<Event>> + 'a
where
    C: WaitForDeployed + WaitForFunded + WaitForRedeemed + WaitForRefunded,
{
    Gen::new({
        |co| async move {
            if let Err(error) = watch_ledger(connector, params, &co).await {
                co.yield_(Err(error)).await;
            }
        }
    })
}

async fn watch_ledger<C, R>(
    connector: &C,
    params: Params,
    co: &Co<anyhow::Result<Event>, R>,
) -> anyhow::Result<()>
where
    C: WaitForDeployed + WaitForFunded + WaitForRedeemed + WaitForRefunded,
{
    co.yield_(Ok(Event::Started)).await;

    let deployed = connector.wait_for_deployed(params.clone()).await?;

    co.yield_(Ok(Event::Deployed(deployed.clone()))).await;

    let funded = connector
        .wait_for_funded(params.clone(), deployed.clone())
        .await?;
    co.yield_(Ok(Event::Funded(funded))).await;

    let redeemed = connector.wait_for_redeemed(params.clone(), deployed.clone());
    let refunded = connector.wait_for_refunded(params, deployed);

    match future::try_select(redeemed, refunded).await {
        Ok(Either::Left((redeemed, _))) => {
            co.yield_(Ok(Event::Redeemed(redeemed))).await;
        }
        Ok(Either::Right((refunded, _))) => {
            co.yield_(Ok(Event::Refunded(refunded))).await;
        }
        Err(either) => {
            let (error, _other_future) = either.factor_first();
            return Err(error);
        }
    }

    Ok(())
}

#[derive(Clone, Debug)]
pub struct Params {
    pub asset: asset::Erc20,
    pub redeem_identity: identity::Ethereum,
    pub refund_identity: identity::Ethereum,
    pub expiry: Timestamp,
    pub start_of_swap: NaiveDateTime,
    pub secret_hash: SecretHash,
}

impl Params {
    pub fn bytecode(&self) -> Bytes {
        Erc20Htlc::from(self.clone()).into()
    }
}

impl From<Params> for Erc20Htlc {
    fn from(params: Params) -> Self {
        let refund_address = blockchain_contracts::ethereum::Address(params.refund_identity.into());
        let redeem_address = blockchain_contracts::ethereum::Address(params.redeem_identity.into());
        let token_contract_address =
            blockchain_contracts::ethereum::Address(params.asset.token_contract.into());

        Erc20Htlc::new(
            params.expiry.into(),
            refund_address,
            redeem_address,
            params.secret_hash.into(),
            token_contract_address,
            params.asset.quantity.into(),
        )
    }
}