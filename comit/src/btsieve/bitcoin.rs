mod bitcoind_connector;
mod cache;

pub use self::{
    bitcoind_connector::{BitcoindConnector, ChainInfo},
    cache::Cache,
};
use crate::{
    btsieve::{
        find_relevant_blocks, BlockByHash, BlockHash, LatestBlock, Predates, PreviousBlockHash,
    },
    identity,
};
use bitcoin::{self, BitcoinHash, OutPoint};
use chrono::NaiveDateTime;
use genawaiter::{sync::Gen, GeneratorState};

type Hash = bitcoin::BlockHash;
type Block = bitcoin::Block;

impl BlockHash for Block {
    type BlockHash = Hash;

    fn block_hash(&self) -> Hash {
        self.bitcoin_hash()
    }
}

impl PreviousBlockHash for Block {
    type BlockHash = Hash;

    fn previous_block_hash(&self) -> Hash {
        self.header.prev_blockhash
    }
}

pub async fn watch_for_spent_outpoint<C>(
    blockchain_connector: &C,
    start_of_swap: NaiveDateTime,
    from_outpoint: OutPoint,
    identity: identity::Bitcoin,
) -> anyhow::Result<(bitcoin::Transaction, bitcoin::TxIn)>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = Hash>,
{
    let (transaction, txin) = watch(blockchain_connector, start_of_swap, |transaction| {
        transaction
            .input
            .iter()
            .filter(|txin| txin.previous_output == from_outpoint)
            .find(|txin| txin.witness.contains(&identity.to_bytes()))
            .cloned()
    })
    .await?;

    Ok((transaction, txin))
}

pub async fn watch_for_created_outpoint<C>(
    blockchain_connector: &C,
    start_of_swap: NaiveDateTime,
    compute_address: bitcoin::Address,
) -> anyhow::Result<(bitcoin::Transaction, bitcoin::OutPoint)>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = Hash>,
{
    let (transaction, out_point) = watch(blockchain_connector, start_of_swap, |transaction| {
        let txid = transaction.txid();
        transaction
            .output
            .iter()
            .enumerate()
            .map(|(index, txout)| {
                // Casting a usize to u32 can lead to truncation on 64bit platforms
                // However, bitcoin limits the number of inputs to u32 anyway, so this
                // is not a problem for us.
                #[allow(clippy::cast_possible_truncation)]
                (index as u32, txout)
            })
            .find(|(_, txout)| txout.script_pubkey == compute_address.script_pubkey())
            .map(|(vout, _txout)| OutPoint { txid, vout })
    })
    .await?;

    Ok((transaction, out_point))
}

async fn watch<C, S, M>(
    connector: &C,
    start_of_swap: NaiveDateTime,
    sieve: S,
) -> anyhow::Result<(bitcoin::Transaction, M)>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = Hash>,
    S: Fn(&bitcoin::Transaction) -> Option<M>,
{
    let mut block_generator =
        Gen::new({ |co| async { find_relevant_blocks(connector, co, start_of_swap).await } });

    loop {
        match block_generator.async_resume().await {
            GeneratorState::Yielded(block) => {
                for transaction in block.txdata.into_iter() {
                    if let Some(result) = sieve(&transaction) {
                        tracing::trace!("transaction matched {:x}", transaction.txid());
                        return Ok((transaction, result));
                    }
                }
            }
            GeneratorState::Complete(Err(e)) => return Err(e),
            // By matching against the never type explicitly, we assert that the `Ok` value of the
            // result is actually the never type and has not been changed since this line was
            // written. The never type can never be constructed, so we can never reach this line.
            GeneratorState::Complete(Ok(never)) => match never {},
        }
    }
}

impl Predates for Block {
    fn predates(&self, timestamp: NaiveDateTime) -> bool {
        let unix_timestamp = timestamp.timestamp();
        let block_time = self.header.time as i64;

        block_time < unix_timestamp
    }
}