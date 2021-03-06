#![allow(clippy::trivially_copy_pass_by_ref)]

use chrono::NaiveDateTime;
use comit::{
    btsieve::{ethereum::ReceiptByHash, BlockByHash, LatestBlock},
    ethereum, Secret, SecretHash, Timestamp,
};
use futures::future::{self, Either};
use std::{sync::Arc, time::Duration};

pub mod hbit {
    use bitcoin::{secp256k1::SecretKey, *};
    use chrono::NaiveDateTime;
    use comit::asset;

    pub use comit::{
        actions::bitcoin::{BroadcastSignedTransaction, SendToAddress},
        btsieve::{BlockByHash, LatestBlock},
        hbit::*,
        htlc_location, transaction, Secret,
    };

    #[derive(Clone, Debug)]
    pub struct PrivateDetailsFunder {
        pub transient_refund_sk: SecretKey,
        pub final_refund_identity: Address,
    }

    #[derive(Clone, Debug)]
    pub struct PrivateDetailsRedeemer {
        pub transient_redeem_sk: SecretKey,
        pub final_redeem_identity: Address,
    }

    #[async_trait::async_trait]
    pub trait Fund {
        async fn fund(&self, params: &Params) -> anyhow::Result<CorrectlyFunded>;
    }

    #[async_trait::async_trait]
    pub trait RedeemAsAlice {
        async fn redeem<SC>(
            &self,
            params: &Params,
            fund_event: CorrectlyFunded,
            // NOTE: Should we move SECP into WalletActor structs?
            secp: &bitcoin::secp256k1::Secp256k1<SC>,
        ) -> anyhow::Result<Redeemed>
        where
            SC: bitcoin::secp256k1::Signing;
    }

    #[async_trait::async_trait]
    pub trait RedeemAsBob {
        async fn redeem<SC>(
            &self,
            params: &Params,
            fund_event: CorrectlyFunded,
            secret: Secret,
            secp: &bitcoin::secp256k1::Secp256k1<SC>,
        ) -> anyhow::Result<Redeemed>
        where
            SC: bitcoin::secp256k1::Signing;
    }

    #[async_trait::async_trait]
    pub trait Refund {
        async fn refund<SC>(
            &self,
            params: &Params,
            fund_event: CorrectlyFunded,
            secp: &bitcoin::secp256k1::Secp256k1<SC>,
        ) -> anyhow::Result<()>
        where
            SC: bitcoin::secp256k1::Signing;
    }

    #[derive(Debug, Clone, Copy)]
    pub struct CorrectlyFunded {
        pub asset: asset::Bitcoin,
        pub location: htlc_location::Bitcoin,
    }

    pub async fn watch_for_funded<C>(
        connector: &C,
        params: &Params,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<CorrectlyFunded>
    where
        C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = BlockHash>,
    {
        match comit::hbit::watch_for_funded(connector, params, start_of_swap).await? {
            comit::hbit::Funded::Correctly {
                asset, location, ..
            } => Ok(CorrectlyFunded { asset, location }),
            comit::hbit::Funded::Incorrectly { .. } => {
                anyhow::bail!("Bitcoin HTLC incorrectly funded")
            }
        }
    }
}

pub mod herc20 {
    use chrono::NaiveDateTime;
    pub use comit::{
        actions::ethereum::*,
        asset,
        btsieve::{ethereum::ReceiptByHash, BlockByHash, LatestBlock},
        ethereum::{Block, ChainId, Hash},
        herc20::*,
        identity, transaction, Secret, SecretHash, Timestamp,
    };

    #[async_trait::async_trait]
    pub trait Deploy {
        async fn deploy(&self, params: &Params) -> anyhow::Result<Deployed>;
    }

    #[async_trait::async_trait]
    pub trait Fund {
        async fn fund(
            &self,
            params: &Params,
            deploy_event: Deployed,
        ) -> anyhow::Result<CorrectlyFunded>;
    }

    #[async_trait::async_trait]
    pub trait RedeemAsAlice {
        async fn redeem(&self, params: &Params, deploy_event: Deployed)
            -> anyhow::Result<Redeemed>;
    }

    #[async_trait::async_trait]
    pub trait RedeemAsBob {
        async fn redeem(
            &self,
            params: &Params,
            deploy_event: Deployed,
            secret: Secret,
        ) -> anyhow::Result<Redeemed>;
    }

    #[async_trait::async_trait]
    pub trait Refund {
        async fn refund(&self, params: &Params, deploy_event: Deployed) -> anyhow::Result<()>;
    }

    #[derive(Debug, Clone)]
    pub struct CorrectlyFunded {
        pub transaction: transaction::Ethereum,
        pub asset: asset::Erc20,
    }

    pub async fn watch_for_funded<C>(
        connector: &C,
        params: Params,
        start_of_swap: NaiveDateTime,
        deployed: Deployed,
    ) -> anyhow::Result<CorrectlyFunded>
    where
        C: LatestBlock<Block = Block>
            + BlockByHash<Block = Block, BlockHash = Hash>
            + ReceiptByHash,
    {
        match comit::herc20::watch_for_funded(connector, params, start_of_swap, deployed).await? {
            comit::herc20::Funded::Correctly { transaction, asset } => {
                Ok(CorrectlyFunded { transaction, asset })
            }
            comit::herc20::Funded::Incorrectly { .. } => {
                anyhow::bail!("Ethereum HTLC incorrectly funded")
            }
        }
    }
}

#[derive(Clone, Debug)]
struct WatchOnlyAlice<AC, BC> {
    alpha_connector: Arc<AC>,
    beta_connector: Arc<BC>,
    secret_hash: SecretHash,
    start_of_swap: NaiveDateTime,
}

#[async_trait::async_trait]
impl<AC, BC> hbit::Fund for WatchOnlyAlice<AC, BC>
where
    AC: LatestBlock<Block = bitcoin::Block>
        + BlockByHash<Block = bitcoin::Block, BlockHash = bitcoin::BlockHash>,
    BC: LatestBlock<Block = ethereum::Block>
        + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
        + ReceiptByHash,
{
    async fn fund(&self, params: &hbit::Params) -> anyhow::Result<hbit::CorrectlyFunded> {
        let event =
            hbit::watch_for_funded(self.alpha_connector.as_ref(), &params, self.start_of_swap)
                .await?;

        Ok(event)
    }
}

#[async_trait::async_trait]
impl<AC, BC> herc20::RedeemAsAlice for WatchOnlyAlice<AC, BC>
where
    AC: LatestBlock<Block = bitcoin::Block>
        + BlockByHash<Block = bitcoin::Block, BlockHash = bitcoin::BlockHash>,
    BC: LatestBlock<Block = ethereum::Block>
        + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
        + ReceiptByHash,
{
    async fn redeem(
        &self,
        _params: &herc20::Params,
        deploy_event: herc20::Deployed,
    ) -> anyhow::Result<herc20::Redeemed> {
        let event = herc20::watch_for_redeemed(
            self.beta_connector.as_ref(),
            self.start_of_swap,
            deploy_event,
        )
        .await?;

        Ok(event)
    }
}

#[async_trait::async_trait]
impl<AC, BC> hbit::Refund for WatchOnlyAlice<AC, BC>
where
    AC: LatestBlock<Block = bitcoin::Block>
        + BlockByHash<Block = bitcoin::Block, BlockHash = bitcoin::BlockHash>,
    BC: LatestBlock<Block = ethereum::Block>
        + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
        + ReceiptByHash,
{
    /// Refund for a watch-only actor is a no-op. The protocol is
    /// finished, so there is verifying that the other party has
    /// refunded
    async fn refund<SC>(
        &self,
        _params: &hbit::Params,
        _fund_event: hbit::CorrectlyFunded,
        _secp: &bitcoin::secp256k1::Secp256k1<SC>,
    ) -> anyhow::Result<()>
    where
        SC: bitcoin::secp256k1::Signing,
    {
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct WalletBob<AW, BW, E> {
    alpha_wallet: AW,
    beta_wallet: BW,
    secret_hash: SecretHash,
    private_protocol_details: E,
}

#[async_trait::async_trait]
impl herc20::Deploy for WalletBob<BitcoinWallet, EthereumWallet, hbit::PrivateDetailsRedeemer> {
    async fn deploy(&self, params: &herc20::Params) -> anyhow::Result<herc20::Deployed> {
        let deploy_action = params.build_deploy_action();
        let event = self.beta_wallet.deploy(deploy_action).await?;

        Ok(event)
    }
}

#[async_trait::async_trait]
impl herc20::Fund for WalletBob<BitcoinWallet, EthereumWallet, hbit::PrivateDetailsRedeemer> {
    async fn fund(
        &self,
        params: &herc20::Params,
        deploy_event: herc20::Deployed,
    ) -> anyhow::Result<herc20::CorrectlyFunded> {
        let fund_action = params.build_fund_action(deploy_event.location)?;
        let event = self.beta_wallet.fund(fund_action).await?;

        Ok(event)
    }
}

#[async_trait::async_trait]
impl hbit::RedeemAsBob for WalletBob<BitcoinWallet, EthereumWallet, hbit::PrivateDetailsRedeemer> {
    async fn redeem<SC>(
        &self,
        params: &hbit::Params,
        fund_event: hbit::CorrectlyFunded,
        secret: Secret,
        secp: &bitcoin::secp256k1::Secp256k1<SC>,
    ) -> anyhow::Result<hbit::Redeemed>
    where
        SC: bitcoin::secp256k1::Signing,
    {
        let redeem_action = params.build_redeem_action(
            &secp,
            fund_event.asset,
            fund_event.location,
            self.private_protocol_details.clone().transient_redeem_sk,
            self.private_protocol_details.clone().final_redeem_identity,
            secret,
        )?;
        let event = self.alpha_wallet.redeem(redeem_action).await?;

        Ok(event)
    }
}

#[async_trait::async_trait]
impl herc20::Refund for WalletBob<BitcoinWallet, EthereumWallet, hbit::PrivateDetailsRedeemer> {
    async fn refund(
        &self,
        params: &herc20::Params,
        deploy_event: herc20::Deployed,
    ) -> anyhow::Result<()> {
        loop {
            let ethereum_time = ethereum_latest_time(self.beta_wallet.connector.as_ref()).await?;

            if ethereum_time >= params.expiry {
                break;
            }

            tokio::time::delay_for(Duration::from_secs(1)).await;
        }

        let refund_action = params.build_refund_action(deploy_event.location)?;
        let _event = self.beta_wallet.refund(refund_action).await?;

        Ok(())
    }
}

/// Determine whether funding a smart contract is safe.
///
/// Implementations should decide based on blockchain time and
/// Beta expiry, the shorter of the two expiries.
#[async_trait::async_trait]
pub trait SafeToFund {
    async fn is_safe_to_fund(&self, beta_expiry: Timestamp) -> anyhow::Result<bool>;
}

/// Determine whether redeeming a smart contract is safe.
///
/// Implementations should decide based on blockchain time and
/// expiries.
#[async_trait::async_trait]
pub trait SafeToRedeem {
    async fn is_safe_to_redeem(&self, beta_expiry: Timestamp) -> anyhow::Result<bool>;
}

#[async_trait::async_trait]
impl<AC, BC> SafeToFund for WatchOnlyAlice<AC, BC>
where
    BC: LatestBlock<Block = ethereum::Block>,
    AC: Send + Sync,
{
    async fn is_safe_to_fund(&self, beta_expiry: Timestamp) -> anyhow::Result<bool> {
        let ethereum_time = ethereum_latest_time(self.beta_connector.as_ref()).await?;
        // TODO: Apply a buffer depending on the blocktime and how
        // safe we want to be

        Ok(beta_expiry > ethereum_time)
    }
}

#[async_trait::async_trait]
impl<AW, BW, E> SafeToFund for WalletBob<AW, BW, E>
where
    AW: Send + Sync,
    BW: LatestBlock<Block = ethereum::Block>,
    E: Send + Sync,
{
    async fn is_safe_to_fund(&self, beta_expiry: Timestamp) -> anyhow::Result<bool> {
        let ethereum_time = ethereum_latest_time(&self.beta_wallet).await?;
        // TODO: Apply a buffer depending on the blocktime and how
        // safe we want to be.

        Ok(beta_expiry > ethereum_time)
    }
}

#[async_trait::async_trait]
impl<AC, BC> SafeToRedeem for WatchOnlyAlice<AC, BC>
where
    BC: LatestBlock<Block = ethereum::Block>,
    AC: Send + Sync,
{
    async fn is_safe_to_redeem(&self, beta_expiry: Timestamp) -> anyhow::Result<bool> {
        let ethereum_time = ethereum_latest_time(self.beta_connector.as_ref()).await?;
        // TODO: Apply a buffer depending on the blocktime and how
        // safe we want to be.

        Ok(beta_expiry > ethereum_time)
    }
}

async fn hbit_herc20<A, B, SC>(
    alice: A,
    bob: B,
    hbit_params: hbit::Params,
    herc20_params: herc20::Params,
    secp: &bitcoin::secp256k1::Secp256k1<SC>,
) -> anyhow::Result<()>
where
    A: hbit::Fund + herc20::RedeemAsAlice + hbit::Refund + SafeToFund + SafeToRedeem,
    B: herc20::Deploy + herc20::Fund + hbit::RedeemAsBob + herc20::Refund + SafeToFund,
    SC: bitcoin::secp256k1::Signing,
{
    if !alice.is_safe_to_fund(herc20_params.expiry).await? {
        return Ok(());
    }

    let hbit_funded = alice.fund(&hbit_params).await?;

    if !bob.is_safe_to_fund(herc20_params.expiry).await? {
        alice.refund(&hbit_params, hbit_funded, secp).await?;

        return Ok(());
    }

    let herc20_deployed = bob.deploy(&herc20_params).await?;

    if !bob.is_safe_to_fund(herc20_params.expiry).await? {
        alice.refund(&hbit_params, hbit_funded, secp).await?;

        return Ok(());
    }

    let _herc20_funded = bob.fund(&herc20_params, herc20_deployed.clone()).await?;

    if !alice.is_safe_to_redeem(herc20_params.expiry).await? {
        alice.refund(&hbit_params, hbit_funded, secp).await?;
        bob.refund(&herc20_params, herc20_deployed.clone()).await?;

        return Ok(());
    }

    let herc20_redeemed = alice.redeem(&herc20_params, herc20_deployed).await?;

    let hbit_redeem = bob.redeem(&hbit_params, hbit_funded, herc20_redeemed.secret, &secp);
    let hbit_refund = alice.refund(&hbit_params, hbit_funded, secp);

    // It's always safe for Bob to redeem, he just has to do it before
    // Alice refunds
    match future::try_select(hbit_redeem, hbit_refund).await {
        Ok(Either::Left((_hbit_redeemed, _))) => Ok(()),
        Ok(Either::Right((_hbit_refunded, _))) => Ok(()),
        Err(either) => {
            let (error, _other_future) = either.factor_first();
            Err(error)
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct WalletAlice<AW, BW, E> {
    pub alpha_wallet: AW,
    pub beta_wallet: BW,
    pub private_protocol_details: E,
    pub secret: comit::Secret,
}

#[async_trait::async_trait]
impl hbit::Fund for WalletAlice<BitcoinWallet, EthereumWallet, hbit::PrivateDetailsFunder> {
    async fn fund(&self, params: &hbit::Params) -> anyhow::Result<hbit::CorrectlyFunded> {
        let fund_action = params.build_fund_action();
        let event = self.alpha_wallet.fund(fund_action).await?;

        Ok(event)
    }
}

#[async_trait::async_trait]
impl herc20::RedeemAsAlice
    for WalletAlice<BitcoinWallet, EthereumWallet, hbit::PrivateDetailsFunder>
{
    async fn redeem(
        &self,
        params: &herc20::Params,
        deploy_event: herc20::Deployed,
    ) -> anyhow::Result<herc20::Redeemed> {
        let redeem_action = params.build_redeem_action(deploy_event.location, self.secret)?;
        let event = self.beta_wallet.redeem(redeem_action).await?;

        Ok(event)
    }
}

#[async_trait::async_trait]
impl hbit::Refund for WalletAlice<BitcoinWallet, EthereumWallet, hbit::PrivateDetailsFunder> {
    async fn refund<SC>(
        &self,
        params: &hbit::Params,
        fund_event: hbit::CorrectlyFunded,
        secp: &bitcoin::secp256k1::Secp256k1<SC>,
    ) -> anyhow::Result<()>
    where
        SC: bitcoin::secp256k1::Signing,
    {
        loop {
            let bitcoin_time =
                comit::bitcoin::median_time_past(self.alpha_wallet.connector.as_ref()).await?;

            if bitcoin_time >= params.expiry {
                break;
            }

            tokio::time::delay_for(Duration::from_secs(1)).await;
        }

        let refund_action = params.build_refund_action(
            secp,
            fund_event.asset,
            fund_event.location,
            self.private_protocol_details.transient_refund_sk,
            self.private_protocol_details.final_refund_identity.clone(),
        )?;
        let _event = self.alpha_wallet.refund(refund_action).await?;

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct WatchOnlyBob<AC, BC> {
    alpha_connector: Arc<AC>,
    beta_connector: Arc<BC>,
    secret_hash: SecretHash,
    start_of_swap: NaiveDateTime,
}

#[async_trait::async_trait]
impl<AC, BC> herc20::Deploy for WatchOnlyBob<AC, BC>
where
    AC: LatestBlock<Block = bitcoin::Block>
        + BlockByHash<Block = bitcoin::Block, BlockHash = bitcoin::BlockHash>,
    BC: LatestBlock<Block = ethereum::Block>
        + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
        + ReceiptByHash,
{
    async fn deploy(&self, params: &herc20::Params) -> anyhow::Result<herc20::Deployed> {
        let event = herc20::watch_for_deployed(
            self.beta_connector.as_ref(),
            params.clone(),
            self.start_of_swap,
        )
        .await?;

        Ok(event)
    }
}

#[async_trait::async_trait]
impl<AC, BC> herc20::Fund for WatchOnlyBob<AC, BC>
where
    AC: LatestBlock<Block = bitcoin::Block>
        + BlockByHash<Block = bitcoin::Block, BlockHash = bitcoin::BlockHash>,
    BC: LatestBlock<Block = ethereum::Block>
        + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
        + ReceiptByHash,
{
    async fn fund(
        &self,
        params: &herc20::Params,
        deploy_event: herc20::Deployed,
    ) -> anyhow::Result<herc20::CorrectlyFunded> {
        let event = herc20::watch_for_funded(
            self.beta_connector.as_ref(),
            params.clone(),
            self.start_of_swap,
            deploy_event,
        )
        .await?;

        Ok(event)
    }
}

#[async_trait::async_trait]
impl<AC, BC> hbit::RedeemAsBob for WatchOnlyBob<AC, BC>
where
    AC: LatestBlock<Block = bitcoin::Block>
        + BlockByHash<Block = bitcoin::Block, BlockHash = bitcoin::BlockHash>,
    BC: LatestBlock<Block = ethereum::Block>
        + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
        + ReceiptByHash,
{
    async fn redeem<SC>(
        &self,
        params: &hbit::Params,
        fund_event: hbit::CorrectlyFunded,
        _secret: Secret,
        _secp: &bitcoin::secp256k1::Secp256k1<SC>,
    ) -> anyhow::Result<hbit::Redeemed>
    where
        SC: bitcoin::secp256k1::Signing,
    {
        let event = hbit::watch_for_redeemed(
            self.alpha_connector.as_ref(),
            &params,
            fund_event.location,
            self.start_of_swap,
        )
        .await?;

        Ok(event)
    }
}

#[async_trait::async_trait]
impl<AC, BC> herc20::Refund for WatchOnlyBob<AC, BC>
where
    AC: LatestBlock<Block = bitcoin::Block>
        + BlockByHash<Block = bitcoin::Block, BlockHash = bitcoin::BlockHash>,
    BC: LatestBlock<Block = ethereum::Block>
        + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
        + ReceiptByHash,
{
    /// Refund for a watch-only actor is a no-op. The protocol is
    /// finished, so there is verifying that the other party has
    /// refunded
    async fn refund(
        &self,
        _params: &comit::herc20::Params,
        _deploy_event: comit::herc20::Deployed,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl<AW, BW, E> SafeToFund for WalletAlice<AW, BW, E>
where
    AW: Send + Sync,
    BW: LatestBlock<Block = ethereum::Block>,
    E: Send + Sync,
{
    async fn is_safe_to_fund(&self, beta_expiry: Timestamp) -> anyhow::Result<bool> {
        let ethereum_time = ethereum_latest_time(&self.beta_wallet).await?;
        // TODO: Apply a buffer depending on the blocktime and how
        // safe we want to be

        Ok(beta_expiry > ethereum_time)
    }
}

#[async_trait::async_trait]
impl<AC, BC> SafeToFund for WatchOnlyBob<AC, BC>
where
    AC: Send + Sync,
    BC: LatestBlock<Block = ethereum::Block>,
{
    async fn is_safe_to_fund(&self, beta_expiry: Timestamp) -> anyhow::Result<bool> {
        let ethereum_time = ethereum_latest_time(self.beta_connector.as_ref()).await?;
        // TODO: Apply a buffer depending on the blocktime and how
        // safe we want to be.

        Ok(beta_expiry > ethereum_time)
    }
}

#[async_trait::async_trait]
impl<AW, BW, E> SafeToRedeem for WalletAlice<AW, BW, E>
where
    AW: Send + Sync,
    BW: LatestBlock<Block = ethereum::Block>,
    E: Send + Sync,
{
    async fn is_safe_to_redeem(&self, beta_expiry: Timestamp) -> anyhow::Result<bool> {
        let ethereum_time = ethereum_latest_time(&self.beta_wallet).await?;
        // TODO: Apply a buffer depending on the blocktime and how
        // safe we want to be

        Ok(beta_expiry > ethereum_time)
    }
}

#[derive(Debug)]
pub struct BitcoinWallet {
    connector: Arc<comit::btsieve::bitcoin::BitcoindConnector>,
}

impl BitcoinWallet {
    pub async fn fund(
        &self,
        _action: hbit::SendToAddress,
    ) -> anyhow::Result<hbit::CorrectlyFunded> {
        todo!()
    }

    pub async fn redeem(
        &self,
        _action: hbit::BroadcastSignedTransaction,
    ) -> anyhow::Result<hbit::Redeemed> {
        todo!()
    }

    pub async fn refund(
        &self,
        _action: hbit::BroadcastSignedTransaction,
    ) -> anyhow::Result<hbit::Refunded> {
        todo!()
    }
}

#[async_trait::async_trait]
impl LatestBlock for BitcoinWallet {
    type Block = bitcoin::Block;
    async fn latest_block(&self) -> anyhow::Result<Self::Block> {
        self.connector.as_ref().latest_block().await
    }
}

#[derive(Debug)]
pub struct EthereumWallet {
    connector: Arc<comit::btsieve::ethereum::Web3Connector>,
}

impl EthereumWallet {
    pub async fn deploy(
        &self,
        _action: herc20::DeployContract,
    ) -> anyhow::Result<herc20::Deployed> {
        todo!()
    }

    pub async fn fund(
        &self,
        _action: herc20::CallContract,
    ) -> anyhow::Result<herc20::CorrectlyFunded> {
        todo!()
    }

    pub async fn redeem(&self, _action: herc20::CallContract) -> anyhow::Result<herc20::Redeemed> {
        todo!()
    }

    pub async fn refund(&self, _action: herc20::CallContract) -> anyhow::Result<herc20::Refunded> {
        todo!()
    }
}

#[async_trait::async_trait]
impl LatestBlock for EthereumWallet {
    type Block = ethereum::Block;
    async fn latest_block(&self) -> anyhow::Result<Self::Block> {
        self.connector.latest_block().await
    }
}

async fn ethereum_latest_time<C>(connector: &C) -> anyhow::Result<Timestamp>
where
    C: LatestBlock<Block = ethereum::Block>,
{
    let timestamp = connector.latest_block().await?.timestamp.into();

    Ok(timestamp)
}

#[cfg(all(test, feature = "test-docker"))]
mod tests {
    use super::*;
    use crate::test_harness::{BitcoinBlockchain, EthereumBlockchain};
    use bitcoin::{secp256k1, Network};
    use chrono::Utc;
    use comit::{
        asset::{
            self,
            ethereum::{Erc20Quantity, FromWei},
        },
        btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector},
        ethereum, identity, Secret, SecretHash, Timestamp,
    };
    use std::{str::FromStr, sync::Arc};
    use testcontainers::clients;

    fn hbit_params<C>(
        secret_hash: SecretHash,
        secp: &bitcoin::secp256k1::Secp256k1<C>,
    ) -> (
        hbit::Params,
        hbit::PrivateDetailsFunder,
        hbit::PrivateDetailsRedeemer,
    )
    where
        C: secp256k1::Signing,
    {
        let asset = asset::Bitcoin::from_sat(100_000_000);
        let network = Network::Regtest;
        let expiry = Timestamp::from(0);

        let (private_details_funder, transient_refund_pk) = {
            let transient_refund_sk = secp256k1::SecretKey::from_str(
                "01010101010101010001020304050607ffff0000ffff00006363636363636363",
            )
            .unwrap();
            // FIXME: Get final_refund_identity from funder wallet
            let final_refund_identity =
                bitcoin::Address::from_str("bcrt1q2nfxmhd4n3c8834pj72xagvyr9gl57n5r94fsl").unwrap();
            let private_details_funder = hbit::PrivateDetailsFunder {
                transient_refund_sk,
                final_refund_identity,
            };

            let transient_refund_pk =
                identity::Bitcoin::from_secret_key(&secp, &transient_refund_sk);

            (private_details_funder, transient_refund_pk)
        };

        let (private_details_redeemer, transient_redeem_pk) = {
            let transient_redeem_sk = secp256k1::SecretKey::from_str(
                "01010101010101010001020304050607ffff0000ffff00006363636363636363",
            )
            .unwrap();
            // FIXME: Get final_redeem_identity from funder wallet
            let final_redeem_identity =
                bitcoin::Address::from_str("bcrt1q2nfxmhd4n3c8834pj72xagvyr9gl57n5r94fsl").unwrap();
            let private_details_redeemer = hbit::PrivateDetailsRedeemer {
                transient_redeem_sk,
                final_redeem_identity,
            };

            let transient_redeem_pk =
                identity::Bitcoin::from_secret_key(&secp, &transient_redeem_sk);

            (private_details_redeemer, transient_redeem_pk)
        };

        let params = hbit::Params {
            network,
            asset,
            redeem_identity: transient_redeem_pk,
            refund_identity: transient_refund_pk,
            expiry,
            secret_hash,
        };

        (params, private_details_funder, private_details_redeemer)
    }

    fn herc20_params(secret_hash: SecretHash) -> herc20::Params {
        let token_contract =
            ethereum::Address::from_str("6b175474e89094c44da98b954eedeac495271d0f").unwrap();
        let quantity = Erc20Quantity::from_wei(1_000u32);
        let asset = asset::Erc20::new(token_contract, quantity);

        // FIXME: Obtain identities from user wallets
        let identity =
            identity::Ethereum::from_str("c5549e335b2786520f4c5d706c76c9ee69d0a028").unwrap();

        herc20::Params {
            asset,
            redeem_identity: identity,
            refund_identity: identity,
            expiry: Timestamp::from(0),
            chain_id: ethereum::ChainId::regtest(),
            secret_hash,
        }
    }

    fn secret() -> Secret {
        let bytes = b"hello world, you are beautiful!!";
        Secret::from(*bytes)
    }

    #[tokio::test]
    async fn execute_alice_hbit_herc20_swap() {
        let secp: bitcoin::secp256k1::Secp256k1<bitcoin::secp256k1::All> =
            bitcoin::secp256k1::Secp256k1::new();

        let bitcoin_connector = {
            let client = clients::Cli::default();
            let blockchain = BitcoinBlockchain::new(&client).unwrap();

            Arc::new(BitcoindConnector::new(blockchain.node_url, Network::Regtest).unwrap())
        };
        let ethereum_connector = {
            let client = clients::Cli::default();
            let blockchain = EthereumBlockchain::new(&client).unwrap();

            Arc::new(Web3Connector::new(blockchain.node_url))
        };

        let secret = secret();
        let secret_hash = SecretHash::new(secret);

        let start_of_swap = Utc::now().naive_local();

        let (hbit_params, private_details_funder, private_details_redeemer) =
            hbit_params(secret_hash, &secp);

        let herc20_params = herc20_params(secret_hash);

        let _alice_swap = {
            let alice = WalletAlice {
                alpha_wallet: BitcoinWallet {
                    connector: Arc::clone(&bitcoin_connector),
                },
                beta_wallet: EthereumWallet {
                    connector: Arc::clone(&ethereum_connector),
                },
                private_protocol_details: private_details_funder,
                secret,
            };
            let bob = WatchOnlyBob {
                alpha_connector: Arc::clone(&bitcoin_connector),
                beta_connector: Arc::clone(&ethereum_connector),
                secret_hash,
                start_of_swap,
            };

            hbit_herc20(alice, bob, hbit_params, herc20_params.clone(), &secp)
        };

        let _bob_swap = {
            let alice = WatchOnlyAlice {
                alpha_connector: Arc::clone(&bitcoin_connector),
                beta_connector: Arc::clone(&ethereum_connector),
                secret_hash,
                start_of_swap,
            };
            let bob = WalletBob {
                alpha_wallet: BitcoinWallet {
                    connector: bitcoin_connector,
                },
                beta_wallet: EthereumWallet {
                    connector: ethereum_connector,
                },
                secret_hash,
                private_protocol_details: private_details_redeemer,
            };

            hbit_herc20(alice, bob, hbit_params, herc20_params, &secp)
        };

        // TODO: Actually spawn both swap executions
        // TODO: Assert that the money moves as expected
    }
}
