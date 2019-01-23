use crate::http_api::asset::{FromHttpAsset, ToHttpAsset};
use bitcoin_support::BitcoinQuantity;
use ethereum_support::{Erc20Token, EtherQuantity};
use std::{
    fmt::{Debug, Display},
    hash::Hash,
};
pub trait Asset:
    Clone
    + Debug
    + Display
    + Send
    + Sync
    + 'static
    + PartialEq
    + Eq
    + Hash
    + FromHttpAsset
    + ToHttpAsset
    + Into<Assets>
{
}

impl Asset for BitcoinQuantity {}
impl Asset for EtherQuantity {}
impl Asset for Erc20Token {}

// FIXME: This might be the same as metadata_store::AssetKind
#[derive(Debug, Clone)]
pub enum Assets {
    Bitcoin(BitcoinQuantity),
    Ether(EtherQuantity),
    Erc20(Erc20Token),
    Unknown { name: String },
}

impl From<BitcoinQuantity> for Assets {
    fn from(quantity: BitcoinQuantity) -> Self {
        Assets::Bitcoin(quantity)
    }
}

impl From<EtherQuantity> for Assets {
    fn from(quantity: EtherQuantity) -> Self {
        Assets::Ether(quantity)
    }
}

impl From<Erc20Token> for Assets {
    fn from(quantity: Erc20Token) -> Self {
        Assets::Erc20(quantity)
    }
}
