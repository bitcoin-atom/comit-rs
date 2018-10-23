use event_store::Event;
use std::{marker::PhantomData, net::SocketAddr};
use swap_protocols::rfc003::{Ledger, Secret};
use swaps::common::TradeId;

#[derive(Clone, Debug)]
pub struct StartSwap<SL: Ledger, TL: Ledger, SA, TA> {
    pub source_ledger: SL,
    pub target_ledger: TL,
    pub target_asset: TA,
    pub source_asset: SA,
    pub secret: Secret,
    pub target_ledger_success_identity: TL::Identity,
    pub source_ledger_refund_identity: SL::Identity,
    pub source_ledger_lock_duration: SL::LockDuration,
    pub remote: SocketAddr,
}

impl<
        SL: Ledger,
        TL: Ledger,
        SA: Clone + Send + Sync + 'static,
        TA: Clone + Send + Sync + 'static,
    > Event for StartSwap<SL, TL, SA, TA>
{
    type Prev = ();
}

#[derive(Clone, Debug)]
pub struct SwapRequestAccepted<SL: Ledger, TL: Ledger, SA, TA> {
    pub target_ledger_refund_identity: TL::Identity,
    pub source_ledger_success_identity: SL::Identity,
    pub target_ledger_lock_duration: TL::LockDuration,
    phantom: PhantomData<(SA, TA)>,
}

impl<SL: Ledger, TL: Ledger, SA, TA> SwapRequestAccepted<SL, TL, SA, TA> {
    pub fn new(
        target_ledger_refund_identity: TL::Identity,
        source_ledger_success_identity: SL::Identity,
        target_ledger_lock_duration: TL::LockDuration,
    ) -> Self {
        SwapRequestAccepted {
            target_ledger_refund_identity,
            source_ledger_success_identity,
            target_ledger_lock_duration,
            phantom: PhantomData,
        }
    }
}

impl<
        SL: Ledger,
        TL: Ledger,
        SA: Clone + Send + Sync + 'static,
        TA: Clone + Send + Sync + 'static,
    > Event for SwapRequestAccepted<SL, TL, SA, TA>
{
    type Prev = StartSwap<SL, TL, SA, TA>;
}
#[derive(Clone, Debug)]
pub struct SwapRequestRejected<SL: Ledger, TL: Ledger, SA, TA> {
    phantom: PhantomData<(SL, TL, SA, TA)>,
}

impl<
        SL: Ledger,
        TL: Ledger,
        SA: Clone + Send + Sync + 'static,
        TA: Clone + Send + Sync + 'static,
    > Event for SwapRequestRejected<SL, TL, SA, TA>
{
    type Prev = StartSwap<SL, TL, SA, TA>;
}

#[allow(clippy::new_without_default_derive)]
impl<SL: Ledger, TL: Ledger, SA, TA> SwapRequestRejected<SL, TL, SA, TA> {
    pub fn new() -> Self {
        SwapRequestRejected {
            phantom: PhantomData,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SourceFunded<SL: Ledger, TL: Ledger, SA, TA> {
    pub uid: TradeId,
    phantom: PhantomData<(SL, TL, SA, TA)>,
}

impl<SL: Ledger, TL: Ledger, SA, TA> SourceFunded<SL, TL, SA, TA> {
    pub fn new(uid: TradeId) -> SourceFunded<SL, TL, SA, TA> {
        SourceFunded {
            uid,
            phantom: PhantomData,
        }
    }
}

impl<
        SL: Ledger,
        TL: Ledger,
        SA: Clone + Send + Sync + 'static,
        TA: Clone + Send + Sync + 'static,
    > Event for SourceFunded<SL, TL, SA, TA>
{
    type Prev = SwapRequestAccepted<SL, TL, SA, TA>;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TargetFunded<SL: Ledger, TL: Ledger, SA, TA> {
    pub address: TL::Address,
    phantom: PhantomData<(SL, SA, TA)>,
}

impl<SL: Ledger, TL: Ledger, SA, TA> TargetFunded<SL, TL, SA, TA> {
    pub fn new(address: TL::Address) -> TargetFunded<SL, TL, SA, TA> {
        TargetFunded {
            address,
            phantom: PhantomData,
        }
    }
}

impl<
        SL: Ledger,
        TL: Ledger,
        SA: Clone + Send + Sync + 'static,
        TA: Clone + Send + Sync + 'static,
    > Event for TargetFunded<SL, TL, SA, TA>
{
    type Prev = SourceFunded<SL, TL, SA, TA>;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TargetRedeemed<SL: Ledger, TL: Ledger, SA, TA> {
    phantom: PhantomData<(SL, TL, SA, TA)>,
}

impl<SL: Ledger, TL: Ledger, SA, TA> TargetRedeemed<SL, TL, SA, TA> {
    pub fn new() -> TargetRedeemed<SL, TL, SA, TA> {
        TargetRedeemed {
            phantom: PhantomData,
        }
    }
}

impl<
        SL: Ledger,
        TL: Ledger,
        SA: Clone + Send + Sync + 'static,
        TA: Clone + Send + Sync + 'static,
    > Event for TargetRedeemed<SL, TL, SA, TA>
{
    type Prev = TargetFunded<SL, TL, SA, TA>;
}
