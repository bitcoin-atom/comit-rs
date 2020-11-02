use crate::{
    local_swap_id::LocalSwapId,
    storage::{db::schema::swaps, Text, Timestamp},
};
use anyhow::Result;
use comit::Role;
use diesel::{prelude::*, SqliteConnection};
use libp2p::PeerId;
use time::OffsetDateTime;

#[derive(thiserror::Error, Debug, Clone, Copy)]
#[error("no swap exists in the database for id {0}")]
pub struct NoSwapExists(pub LocalSwapId);

#[derive(Identifiable, Queryable, PartialEq, Debug)]
#[table_name = "swaps"]
pub struct Swap {
    id: i32,
    #[diesel(deserialize_as = "Text<LocalSwapId>")]
    pub local_swap_id: LocalSwapId,
    #[diesel(deserialize_as = "Text<Role>")]
    pub role: Role,
    #[diesel(deserialize_as = "Text<PeerId>")]
    pub counterparty_peer_id: PeerId,
    #[diesel(deserialize_as = "Timestamp")]
    pub start_of_swap: OffsetDateTime,
}

#[derive(Insertable, Debug, Clone)]
#[table_name = "swaps"]
pub struct InsertableSwap {
    pub local_swap_id: Text<LocalSwapId>,
    pub role: Text<Role>,
    pub counterparty_peer_id: Text<PeerId>,
    pub start_of_swap: i64,
}

impl InsertableSwap {
    pub fn insert(self, conn: &SqliteConnection) -> Result<i32> {
        let local_swap_id = self.local_swap_id.0;

        diesel::insert_into(swaps::dsl::swaps)
            .values(self)
            .execute(conn)?;

        let swap_fk = swap_id_fk!(local_swap_id).first(conn)?;

        Ok(swap_fk)
    }
}

impl InsertableSwap {
    pub fn new(
        swap_id: LocalSwapId,
        counterparty: PeerId,
        role: Role,
        start_of_swap: OffsetDateTime,
    ) -> Self {
        InsertableSwap {
            local_swap_id: Text(swap_id),
            role: Text(role),
            counterparty_peer_id: Text(counterparty),
            start_of_swap: start_of_swap.timestamp(),
        }
    }
}
