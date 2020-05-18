use crate::{
    asset,
    db::{CreatedSwap, Save},
    http_api::{problem, routes::into_rejection, DialInformation, Http},
    identity,
    network::ListenAddresses,
    swap_protocols::{
        halight, herc20, ledger, Facade, Herc20HalightBitcoinCreateSwapParams, LocalSwapId,
        Rfc003Facade, Role,
    },
};
use http_api_problem::HttpApiProblem;
use libp2p::{Multiaddr, PeerId};
use serde::{Deserialize, Serialize};
use warp::{http::StatusCode, Rejection, Reply};

#[derive(Serialize, Debug)]
pub struct InfoResource {
    id: Http<PeerId>,
    listen_addresses: Vec<Multiaddr>,
}

pub async fn get_info(id: PeerId, dependencies: Rfc003Facade) -> Result<impl Reply, Rejection> {
    let listen_addresses = dependencies.listen_addresses().await.to_vec();

    Ok(warp::reply::json(&InfoResource {
        id: Http(id),
        listen_addresses,
    }))
}

pub async fn get_info_siren(
    id: PeerId,
    dependencies: Rfc003Facade,
) -> Result<impl Reply, Rejection> {
    let listen_addresses = dependencies.listen_addresses().await.to_vec();

    Ok(warp::reply::json(
        &siren::Entity::default()
            .with_properties(&InfoResource {
                id: Http(id),
                listen_addresses,
            })
            .map_err(|e| {
                tracing::error!("failed to set properties of entity: {:?}", e);
                HttpApiProblem::with_title_and_type_from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })
            .map_err(into_rejection)?
            .with_link(
                siren::NavigationalLink::new(&["collection"], "/swaps").with_class_member("swaps"),
            )
            .with_link(
                siren::NavigationalLink::new(&["collection", "edit"], "/swaps/rfc003")
                    .with_class_member("swaps")
                    .with_class_member("rfc003"),
            ),
    ))
}

pub async fn post_herc20_halight_bitcoin(
    body: serde_json::Value,
    facade: Facade,
) -> Result<impl Reply, Rejection> {
    let body = Body::<Herc20EthereumErc20, HalightLightningBitcoin>::deserialize(&body)
        .map_err(anyhow::Error::new)
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    let swap_params: Herc20HalightBitcoinCreateSwapParams = body.clone().into();

    let swap_id = LocalSwapId::default();
    let reply = warp::reply::reply();

    let swap = CreatedSwap::<herc20::CreatedSwap, halight::CreatedSwap> {
        swap_id,
        alpha: body.alpha.into(),
        beta: body.beta.into(),
        peer: body.peer.into(),
        address_hint: None,
        role: body.role.0,
    };

    facade
        .save(swap)
        .await
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    facade
        .initiate_communication(swap_id, swap_params)
        .await
        .map(|_| {
            warp::reply::with_status(
                warp::reply::with_header(reply, "Location", format!("/swaps/{}", swap_id)),
                StatusCode::CREATED,
            )
        })
        .map_err(problem::from_anyhow)
        .map_err(into_rejection)
}

// `warp::reply::Json` is used as a return type to please the compiler
// until proper logic is implemented
#[allow(clippy::needless_pass_by_value)]
pub async fn post_halight_bitcoin_herc20(
    body: serde_json::Value,
    _facade: Facade,
) -> Result<warp::reply::Json, Rejection> {
    let _body = Body::<HalightLightningBitcoin, Herc20EthereumErc20>::deserialize(&body)
        .map_err(anyhow::Error::new)
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    tracing::error!("Lightning routes are not yet supported");
    Err(warp::reject::custom(
        HttpApiProblem::new("Route not yet supported.")
            .set_status(StatusCode::BAD_REQUEST)
            .set_detail("This route is not yet supported."),
    ))
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct Body<A, B> {
    pub alpha: A,
    pub beta: B,
    pub peer: DialInformation,
    pub role: Http<Role>,
}

impl From<Body<Herc20EthereumErc20, HalightLightningBitcoin>>
    for Herc20HalightBitcoinCreateSwapParams
{
    fn from(body: Body<Herc20EthereumErc20, HalightLightningBitcoin>) -> Self {
        Self {
            role: body.role.0,
            peer: body.peer.into(),
            ethereum_identity: body.alpha.identity,
            ethereum_absolute_expiry: body.alpha.absolute_expiry.into(),
            ethereum_amount: body.alpha.amount,
            lightning_identity: body.beta.identity,
            lightning_cltv_expiry: body.beta.cltv_expiry.into(),
            lightning_amount: body.beta.amount.0,
            token_contract: body.alpha.contract_address,
        }
    }
}

impl From<Herc20EthereumErc20> for herc20::CreatedSwap {
    fn from(p: Herc20EthereumErc20) -> Self {
        herc20::CreatedSwap {
            amount: p.amount,
            identity: p.identity,
            chain_id: p.chain_id,
            token_contract: p.contract_address,
            absolute_expiry: p.absolute_expiry,
        }
    }
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct HalightLightningBitcoin {
    pub amount: Http<asset::Bitcoin>,
    pub identity: identity::Lightning,
    pub network: Http<ledger::Lightning>,
    pub cltv_expiry: u32,
}

impl From<HalightLightningBitcoin> for halight::CreatedSwap {
    fn from(p: HalightLightningBitcoin) -> Self {
        halight::CreatedSwap {
            amount: *p.amount,
            identity: p.identity,
            network: *p.network,
            cltv_expiry: p.cltv_expiry,
        }
    }
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct Herc20EthereumErc20 {
    pub amount: asset::Erc20Quantity,
    pub identity: identity::Ethereum,
    pub chain_id: u32,
    pub contract_address: identity::Ethereum,
    pub absolute_expiry: u32,
}
