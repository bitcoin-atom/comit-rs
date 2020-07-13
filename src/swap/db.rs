use self::{
    hbit::{HbitFunded, HbitRedeemed, HbitRefunded},
    herc20::{Herc20Deployed, Herc20Funded, Herc20Redeemed, Herc20Refunded},
};
use crate::{swap::SwapKind, SwapId};
use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};

mod hbit;
mod herc20;

pub trait Load<T>: Send + Sync + 'static {
    fn load(&self, swap_id: SwapId) -> anyhow::Result<Option<T>>;
}

pub trait Save<T>: Send + Sync + 'static {
    fn save(&self, elem: T, swap_id: SwapId) -> anyhow::Result<()>;
}

#[derive(Debug, Clone, Copy)]
pub struct Created;

#[derive(Debug)]
pub struct Database {
    db: sled::Db,
    #[cfg(test)]
    tmp_dir: tempdir::TempDir,
}

impl Database {
    #[cfg(not(test))]
    pub fn new(path: &std::path::Path) -> anyhow::Result<Self> {
        let path = path
            .to_str()
            .ok_or_else(|| anyhow!("The path is not utf-8 valid: {:?}", path))?;
        let db = sled::open(path).context(format!("Could not open the DB at {}", path))?;
        Ok(Database { db })
    }

    #[cfg(test)]
    pub fn new_test() -> anyhow::Result<Self> {
        let tmp_dir = tempdir::TempDir::new("nectar_test").unwrap();
        let db = sled::open(tmp_dir.path()).context(format!(
            "Could not open the DB at {}",
            tmp_dir.path().display()
        ))?;

        Ok(Database { db, tmp_dir })
    }

    pub fn insert(&self, _swap: SwapKind) -> anyhow::Result<()> {
        todo!()
    }

    pub fn load_all(&self) -> anyhow::Result<Vec<SwapKind>> {
        todo!()
    }

    pub fn delete(&self, swap_id: &SwapId) -> anyhow::Result<()> {
        let key = swap_id.as_bytes();

        self.db
            .remove(key)
            .context(format!("Could not delete swap {}", swap_id))
            .map(|_| ())
    }

    fn _insert(&self, swap_id: &SwapId, swap: &Swap) -> anyhow::Result<()> {
        let key = swap_id.as_bytes();
        // TODO: Consider using https://github.com/3Hren/msgpack-rust instead
        let value = serde_json::to_vec(&swap)
            .context(format!("Could not serialize the swap: {:?}", swap))?;

        self.db
            .insert(&key, value)
            .context(format!("Could not insert swap {}", swap_id))?;

        Ok(())
    }

    fn get(&self, swap_id: &SwapId) -> anyhow::Result<Swap> {
        let swap = self
            .db
            .get(swap_id.as_bytes())?
            .ok_or_else(|| anyhow!("Swap does not exists {}", swap_id))?;

        serde_json::from_slice(&swap).context("Could not deserialize swap")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Swap {
    pub hbit_funded: Option<HbitFunded>,
    pub hbit_redeemed: Option<HbitRedeemed>,
    pub hbit_refunded: Option<HbitRefunded>,
    pub herc20_deployed: Option<Herc20Deployed>,
    pub herc20_funded: Option<Herc20Funded>,
    pub herc20_redeemed: Option<Herc20Redeemed>,
    pub herc20_refunded: Option<Herc20Refunded>,
}

impl Default for Swap {
    fn default() -> Self {
        Swap {
            hbit_funded: None,
            hbit_redeemed: None,
            hbit_refunded: None,
            herc20_deployed: None,
            herc20_funded: None,
            herc20_redeemed: None,
            herc20_refunded: None,
        }
    }
}

// Kind of bending the arm of the trait
impl Save<Created> for Database {
    fn save(&self, _event: Created, swap_id: SwapId) -> anyhow::Result<()> {
        let stored_swap = self.get(&swap_id);

        match stored_swap {
            Ok(_) => Err(anyhow!("Swap is already stored")),
            Err(_) => {
                let swap = Swap::default();
                let new_value =
                    serde_json::to_vec(&swap).context("Could not serialize new swap value")?;

                self.db
                    .compare_and_swap(swap_id.as_bytes(), Option::<Vec<u8>>::None, Some(new_value))
                    .context("Could not write in the DB")?
                    .context("Stored swap somehow changed, aborting saving")
            }
        }
    }
}
