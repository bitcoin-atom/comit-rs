use crate::{
    db::{
        tables::{InsertableHalight, InsertableHerc20, InsertableSwap},
        wrapper_types::{
            custom_sql_types::{Text, U32},
            EthereumAddress,
        },
        CreatedSwap, Error, Load, Save, Sqlite,
    },
    swap_protocols::{halight, han, herc20, HashFunction, Ledger, LocalSwapId, Role},
};
use async_trait::async_trait;

#[async_trait]
impl Save<CreatedSwap<han::CreatedSwap, halight::CreatedSwap>> for Sqlite {
    async fn save(
        &self,
        _: CreatedSwap<han::CreatedSwap, halight::CreatedSwap>,
    ) -> anyhow::Result<()> {
        unimplemented!()
    }
}

#[async_trait]
impl Save<CreatedSwap<herc20::CreatedSwap, halight::CreatedSwap>> for Sqlite {
    async fn save(
        &self,
        created: CreatedSwap<herc20::CreatedSwap, halight::CreatedSwap>,
    ) -> anyhow::Result<()> {
        let local_swap_id = created.swap_id;
        let role = created.role;

        let swap = InsertableSwap::new(created.swap_id, created.peer.clone(), created.role);
        self.save_swap(&swap).await?;

        if let Some(address_hint) = created.address_hint {
            self.save_address_hint(created.peer, &address_hint).await?;
        }

        // Save the herc20 details.
        let redeem_identity = match role {
            Role::Alice => None,
            Role::Bob => Some(Text(EthereumAddress::from(created.alpha.identity))),
        };
        let refund_identity = match role {
            Role::Alice => Some(Text(EthereumAddress::from(created.alpha.identity))),
            Role::Bob => None,
        };
        assert!(redeem_identity.is_some() || refund_identity.is_some());

        let herc = InsertableHerc20 {
            swap_id: 0, // FK, set during save.
            amount: Text(created.alpha.amount.into()),
            chain_id: U32(created.alpha.chain_id),
            expiry: U32(created.alpha.absolute_expiry),
            hash_function: Text(HashFunction::Sha256),
            token_contract: Text(created.alpha.token_contract.into()),
            redeem_identity,
            refund_identity,
            ledger: Text(Ledger::Alpha),
        };

        self.save_herc20(local_swap_id, &herc).await?;

        // Save the halight details.
        let redeem_identity = match role {
            Role::Alice => Some(Text(created.beta.identity)),
            Role::Bob => None,
        };
        let refund_identity = match role {
            Role::Alice => None,
            Role::Bob => Some(Text(created.beta.identity)),
        };
        assert!(redeem_identity.is_some() || refund_identity.is_some());

        let halight = InsertableHalight {
            swap_id: 0, // FK, set during save.
            amount: Text(created.beta.amount.into()),
            network: Text(created.beta.network.into()),
            chain: "bitcoin".to_string(), // We currently only support Lightning on top of Bitcoin.
            cltv_expiry: U32(created.beta.cltv_expiry),
            hash_function: Text(HashFunction::Sha256),
            redeem_identity,
            refund_identity,
            ledger: Text(Ledger::Beta),
        };
        self.save_halight(local_swap_id, &halight).await?;

        Ok(())
    }
}

#[async_trait]
impl Load<CreatedSwap<herc20::CreatedSwap, halight::CreatedSwap>> for Sqlite {
    async fn load(
        &self,
        swap_id: LocalSwapId,
    ) -> anyhow::Result<Option<CreatedSwap<herc20::CreatedSwap, halight::CreatedSwap>>> {
        let swap = self.load_swap(swap_id).await?;
        let herc20 = self.load_herc20(swap_id).await?;
        let halight = self.load_halight(swap_id).await?;

        let role = swap.role.0;
        let peer = swap.counterparty_peer_id.0;

        let address_hint = self.load_address_hint(&peer).await.ok();

        let alpha_identity = match role {
            Role::Alice => herc20
                .refund_identity
                .ok_or_else(|| Error::IdentityNotSet)?,
            Role::Bob => herc20
                .redeem_identity
                .ok_or_else(|| Error::IdentityNotSet)?,
        };
        let beta_identity = match role {
            Role::Alice => halight
                .redeem_identity
                .ok_or_else(|| Error::IdentityNotSet)?,
            Role::Bob => halight
                .refund_identity
                .ok_or_else(|| Error::IdentityNotSet)?,
        };

        let alpha = herc20::CreatedSwap {
            amount: herc20.amount.0.into(),
            identity: alpha_identity.0.into(),
            chain_id: herc20.chain_id.into(),
            token_contract: herc20.token_contract.0.into(),
            absolute_expiry: herc20.expiry.into(),
        };

        let beta = halight::CreatedSwap {
            amount: halight.amount.0.into(),
            identity: beta_identity.0,
            network: halight.network.0.into(),
            cltv_expiry: halight.cltv_expiry.into(),
        };

        let created = CreatedSwap {
            swap_id,
            alpha,
            beta,
            peer,
            address_hint,
            role,
        };

        Ok(Some(created))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        asset,
        db::{
            wrapper_types::{Erc20Amount, EthereumAddress},
            Sqlite,
        },
        identity,
        swap_protocols::ledger::lightning,
        timestamp::Timestamp,
    };
    use libp2p::{Multiaddr, PeerId};
    use std::{path::PathBuf, str::FromStr};

    fn temp_db() -> PathBuf {
        let temp_file = tempfile::Builder::new()
            .suffix(".sqlite")
            .tempfile()
            .unwrap();

        temp_file.into_temp_path().to_path_buf()
    }

    #[tokio::test]
    async fn roundtrip_created_swap() {
        let path = temp_db();
        let db = Sqlite::new(&path).expect("a new db");

        let local_swap_id =
            LocalSwapId::from_str("111152ca-ecf2-4cc6-b35c-b4351ac28a34").expect("valid swap id");
        let role = Role::Alice;
        let peer = PeerId::from_str("QmfUfpC2frwFvcDzpspnfZitHt5wct6n4kpG5jzgRdsxkY")
            .expect("valid peer id");

        let multi_addr = "/ip4/80.123.90.4/tcp/5432";
        let address_hint: Multiaddr = multi_addr.parse().expect("valid multiaddress");

        let alpha_amount = Erc20Amount::from_str("12345").expect("valid ERC20 amount");
        let token_contract = EthereumAddress::from_str("1111e8be41b21f651a71aaB1A85c6813b8bBcCf8")
            .expect("valid etherum identity");
        let alpha_identity = EthereumAddress::from_str("2222e8be41b21f651a71aaB1A85c6813b8bBcCf8")
            .expect("valid redeem identity");
        let alpha_expiry = Timestamp::from(123u32);

        let beta_amount = asset::Bitcoin::from_sat(999);
        let beta_identity = identity::Lightning::random();
        let beta_expiry = Timestamp::from(456u32);

        let created: CreatedSwap<herc20::CreatedSwap, halight::CreatedSwap> = CreatedSwap {
            swap_id: local_swap_id,
            alpha: herc20::CreatedSwap {
                amount: alpha_amount.into(),
                identity: alpha_identity.into(),
                chain_id: 1337,
                token_contract: token_contract.into(),
                absolute_expiry: alpha_expiry.into(),
            },
            beta: halight::CreatedSwap {
                amount: beta_amount,
                identity: beta_identity,
                network: lightning::Regtest,
                cltv_expiry: beta_expiry.into(),
            },
            peer,
            address_hint: Some(address_hint),
            role,
        };

        Save::<CreatedSwap<herc20::CreatedSwap, halight::CreatedSwap>>::save(&db, created.clone())
            .await
            .expect("to be able to save created swap");

        let loaded = Load::<CreatedSwap<herc20::CreatedSwap, halight::CreatedSwap>>::load(
            &db,
            local_swap_id,
        )
        .await
        .expect("to be able to load previously save created swap")
        .expect("some created swap");

        assert_eq!(loaded, created);
    }
}