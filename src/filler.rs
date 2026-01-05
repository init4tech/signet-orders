use crate::provider::TxSenderProvider;
use alloy::{
    consensus::constants::GWEI_TO_WEI,
    eips::Encodable2718,
    network::TransactionBuilder,
    primitives::{Bytes, TxHash},
    providers::{PendingTransactionConfig, Provider, SendableTx},
    rpc::types::{TransactionRequest, mev::EthSendBundle},
    signers::Signer,
};
use eyre::{Error, eyre};
use futures::future::try_join_all;
use init4_bin_base::{
    deps::tracing::{debug, info, instrument},
    utils::{from_env::FromEnv, signer::LocalOrAwsConfig},
};
use signet_bundle::SignetEthBundle;
use signet_constants::SignetConstants;
use signet_tx_cache::client::TxCache;
use signet_types::{AggregateOrders, SignedFill, SignedOrder, UnsignedFill};
use std::{collections::HashMap, slice::from_ref, time::Duration};

/// Default gas limit for transactions.
const DEFAULT_GAS_LIMIT: u64 = 1_000_000;
/// Default priority fee multiplier for transactions.
const DEFAULT_PRIORITY_FEE_MULTIPLIER: u64 = 16;

/// Configuration for the Filler application.
#[derive(Debug, FromEnv)]
pub struct FillerConfig {
    /// The Rollup RPC URL.
    #[from_env(var = "RU_RPC_URL", desc = "RPC URL for the Rollup")]
    pub ru_rpc_url: String,
    /// The Host RPC URL.
    #[from_env(var = "HOST_RPC_URL", desc = "RPC URL for the Host")]
    pub host_rpc_url: String,
    /// The signer to use for signing transactions on the Host and Rollup.
    /// NOTE: For the example, this key must be funded with gas on both the Host and Rollup, as well as Input/Output tokens for the Orders on the Host/Rollup.
    /// .env var: SIGNER_KEY
    pub signer_config: LocalOrAwsConfig,
    /// The Signet constants.
    /// .env var: CHAIN_NAME
    #[from_env(var = "CHAIN_NAME", desc = "Signet chain name")]
    pub constants: SignetConstants,
}

/// Example code demonstrating API usage and patterns for Signet Fillers.
#[derive(Debug)]
pub struct Filler<S: Signer> {
    /// The signer to use for signing transactions.
    signer: S,
    /// The provider to use for building transactions on the Rollup.
    ru_provider: TxSenderProvider,
    /// The provider to use for building transactions on the Host.
    host_provider: TxSenderProvider,
    /// The transaction cache endpoint.
    tx_cache: TxCache,
    /// The system constants.
    constants: SignetConstants,
}

impl<S> Filler<S>
where
    S: Signer,
{
    /// Create a new Filler with the given signer, provider, and transaction cache endpoint.
    pub fn new(
        signer: S,
        ru_provider: TxSenderProvider,
        host_provider: TxSenderProvider,
        constants: SignetConstants,
    ) -> Result<Self, Error> {
        let tx_cache_url: reqwest::Url = constants.environment().transaction_cache().parse()?;
        let client = reqwest::ClientBuilder::new().use_rustls_tls().build()?;

        info!(
            tx_cache_url = tx_cache_url.as_str(),
            "Connecting to transaction cache"
        );

        Ok(Self {
            signer,
            ru_provider,
            host_provider,
            tx_cache: TxCache::new_with_client(tx_cache_url, client),
            constants,
        })
    }

    /// Query the transaction cache to get all possible orders.
    pub async fn get_orders(&self) -> Result<Vec<SignedOrder>, Error> {
        debug!("Querying transaction cache for orders");
        let resp = self.tx_cache.get_orders(None).await?;
        let orders = resp.into_inner().orders.clone();
        info!(orders_count = orders.len(), "Retrieved orders from cache");
        Ok(orders)
    }

    /// Fills Orders individually, by submitting a separate Bundle for each Order.
    ///
    /// Filling Orders individually ensures that even if some Orders are not fillable, others may still mine;
    /// however, it is less gas efficient.
    ///
    /// A nice feature of filling Orders individually is that Fillers could be less concerned
    /// about carefully simulating Orders onchain before attempting to fill them.
    /// As long as an Order is economically a "good deal" for the Filler, they can attempt to fill it
    /// without simulating to check whether it has already been filled, because they can rely on Builder simulation.
    /// Order `initiate` transactions will revert if the Order has already been filled,
    /// in which case the entire Bundle would simply be discarded by the Builder.
    #[instrument(skip_all)]
    pub async fn fill_individually(&self, orders: &[SignedOrder]) -> Result<(), Error> {
        debug!(orders_count = orders.len(), "Filling orders individually");

        // submit one bundle per individual order
        for order in orders {
            self.fill(from_ref(order)).await?;
        }

        Ok(())
    }

    /// Fills one or more Order(s) in a single, atomic Bundle.
    /// - Signs Fill(s) for the Order(s)
    /// - Constructs a Bundle of transactions to fill & initiate the Order(s)
    /// - Sends the Bundle to the transaction cache to be mined by Builders
    ///
    /// If more than one Order is passed to this fn,
    /// Filling them in aggregate means that Fills are batched and more gas efficient;
    /// however, if a single Order cannot be filled, then the entire Bundle will not mine.
    /// For example, using this strategy, if one Order is filled by another Filler first, then all other Orders will also not be filled.
    ///
    /// If a single Order is passed to this fn,
    /// Filling Orders individually ensures that even if some Orders are not fillable, others may still mine;
    /// however, it is less gas efficient.
    #[instrument(skip_all)]
    pub async fn fill(&self, orders: &[SignedOrder]) -> Result<(), Error> {
        info!(orders_count = orders.len(), "Filling orders in bundle");

        // if orders is empty, error out
        if orders.is_empty() {
            eyre::bail!("no orders to fill")
        }

        // sign a SignedFill for the orders
        let signed_fills: HashMap<u64, SignedFill> = self.sign_fills(orders).await?;
        debug!(?signed_fills, "Signed fills for orders");
        info!("Successfully signed fills");

        // get the transaction requests for the rollup
        let tx_requests = self.rollup_txn_requests(&signed_fills, orders).await?;
        debug!(?tx_requests, "Rollup transaction requests");

        // sign & encode the rollup transactions for the Bundle
        let rollup_signed = self
            .sign_and_encode_txns(&self.ru_provider, tx_requests)
            .await?;
        let ru_hashes: Vec<TxHash> = rollup_signed.iter().map(|tx| tx.hash).collect();
        let txs: Vec<Bytes> = rollup_signed.into_iter().map(|tx| tx.encoded).collect();
        debug!(?txs, ?ru_hashes, "Rollup encoded transactions");

        // get the transaction requests for the host
        let host_tx_requests = self.host_txn_requests(&signed_fills).await?;
        debug!(?host_tx_requests, "Host transaction requests");

        // sign & encode the host transactions for the Bundle
        let host_signed = self
            .sign_and_encode_txns(&self.host_provider, host_tx_requests)
            .await?;
        let host_hashes: Vec<TxHash> = host_signed.iter().map(|tx| tx.hash).collect();
        let host_txs = host_signed
            .into_iter()
            .map(|tx| tx.encoded)
            .collect::<Vec<_>>();
        debug!(?host_txs, ?host_hashes, "Host encoded transactions");

        // get current rollup block to determine the subsequent target block(s) for Bundle
        let latest_ru_block_number = self.ru_provider.get_block_number().await?;
        info!(latest_ru_block_number, "latest rollup block number");

        let target_block_number = latest_ru_block_number + 1;
        info!(target_block_number, "target rollup block number");
        self.send_bundle(txs, host_txs, target_block_number).await?;

        self.watch_confirmations(&ru_hashes, &host_hashes).await?;
        info!(
            orders_count = orders.len(),
            "All bundle transactions confirmed"
        );

        Ok(())
    }

    async fn send_bundle(
        &self,
        ru_txs: Vec<Bytes>,
        host_txs: Vec<Bytes>,
        target_ru_block_number: u64,
    ) -> Result<(), Error> {
        // construct a Bundle containing the Rollup transactions and the Host fill (if any)
        let bundle = SignetEthBundle {
            host_txs,
            bundle: EthSendBundle {
                txs: ru_txs,
                block_number: target_ru_block_number,
                ..Default::default()
            },
        };

        info!(
            ru_tx_count = bundle.bundle.txs.len(),
            host_tx_count = bundle.host_txs.len(),
            target_ru_block_number,
            "forwarding bundle to transaction cache"
        );

        // submit the Bundle to the transaction cache
        let response = self.tx_cache.forward_bundle(bundle).await?;
        info!(bundle_id = response.id.to_string(), "Bundle sent to cache");

        Ok(())
    }

    /// Aggregate the given orders into a SignedFill, sign it, and
    /// return a HashMap of SignedFills for each destination chain.
    ///
    /// This is the simplest, minimally viable way to turn a set of SignedOrders into a single Aggregated Fill on each chain;
    /// Fillers may wish to implement more complex setups.
    ///
    /// For example, if utilizing different signers for each chain, they may use `UnsignedFill.sign_for(chain_id)` instead of `sign()`.
    ///
    /// If filling multiple Orders, they may wish to utilize one Order's Outputs to provide another Order's rollup Inputs.
    /// In this case, the Filler would wish to split up the Fills for each Order,
    /// rather than signing a single, aggregate a Fill for each chain, as is done here.
    #[instrument(skip_all, fields(orders_count = orders.len()))]
    async fn sign_fills(&self, orders: &[SignedOrder]) -> Result<HashMap<u64, SignedFill>, Error> {
        if orders.is_empty() {
            eyre::bail!("no orders to fill");
        }
        let deadline = orders[0]
            .permit()
            .permit
            .deadline
            .to_string()
            .parse::<u64>()
            .map_err(|e| eyre!("invalid deadline in orders: {e}"))?;
        //  create an AggregateOrder from the SignedOrders they want to fill
        let agg: AggregateOrders = orders.iter().collect();
        debug!(?agg, "Aggregated orders for fill");
        info!("Aggregating orders for fill");
        // produce an UnsignedFill from the AggregateOrder
        let mut unsigned_fill = UnsignedFill::from(&agg);
        unsigned_fill = unsigned_fill
            .with_deadline(deadline)
            .with_chain(self.constants.system().clone());
        debug!(?unsigned_fill, "Unsigned fill created");
        // sign the UnsignedFill, producing a SignedFill for each target chain
        Ok(unsigned_fill.sign(&self.signer).await?)
    }

    /// Construct a set of transaction requests to be submitted on the rollup.
    ///
    /// Perform a single, aggregate Fill upfront, then Initiate each Order.
    /// Transaction requests look like [`fill_aggregate`, `initiate_1`, `initiate_2`].
    ///
    /// This is the simplest, minimally viable way to get a set of Orders mined;
    /// Fillers may wish to implement more complex strategies.
    ///
    /// For example, Fillers might utilize one Order's Inputs to fill subsequent Orders' Outputs.
    /// In this case, the rollup transactions should look like [`fill_1`, `inititate_1`, `fill_2`, `initiate_2`].
    #[instrument(skip_all)]
    async fn rollup_txn_requests(
        &self,
        signed_fills: &HashMap<u64, SignedFill>,
        orders: &[SignedOrder],
    ) -> Result<Vec<TransactionRequest>, Error> {
        // construct the transactions to be submitted to the Rollup
        let mut tx_requests = Vec::new();

        // first, if there is a SignedFill for the Rollup, add a transaction to submit the fill
        // Note that `fill` transactions MUST be mined *before* the corresponding Order(s) `initiate` transactions in order to count
        // Host `fill` transactions are always considered to be mined "before" the rollup block is processed,
        // but Rollup `fill` transactions MUST take care to be ordered before the Orders are `initiate`d
        if let Some(rollup_fill) = signed_fills.get(&self.constants.rollup().chain_id()) {
            debug!(?rollup_fill, "Rollup fill");
            // add the fill tx to the rollup txns
            let ru_fill_tx = rollup_fill.to_fill_tx(self.constants.rollup().orders());
            tx_requests.push(ru_fill_tx);
        }

        // next, add a transaction to initiate each SignedOrder
        for signed_order in orders {
            // add the initiate tx to the rollup txns
            let ru_initiate_tx = signed_order
                .to_initiate_tx(self.signer.address(), self.constants.rollup().orders());
            tx_requests.push(ru_initiate_tx);
        }

        Ok(tx_requests)
    }

    /// Construct a set of transaction requests to be submitted on the host.
    ///
    /// This example only includes one Host transaction,
    /// which performs a single, aggregate Fill on the Host chain.
    ///
    /// This is the simplest, minimally viable way to get a set of Orders mined;
    /// Fillers may wish to implement more complex strategies.
    ///
    /// For example, Fillers might wish to include swaps on Host AMMs to source liquidity as part of their filling strategy.
    #[instrument(skip_all)]
    async fn host_txn_requests(
        &self,
        signed_fills: &HashMap<u64, SignedFill>,
    ) -> Result<Vec<TransactionRequest>, Error> {
        // If there is a SignedFill for the Host, add a transaction to submit the fill
        if let Some(host_fill) = signed_fills.get(&self.constants.host().chain_id()) {
            debug!(?host_fill, "Host fill");
            // add the fill tx to the host txns
            let host_fill_tx = host_fill.to_fill_tx(self.constants.host().orders());
            Ok(vec![host_fill_tx])
        } else {
            Ok(vec![])
        }
    }

    /// Given an ordered set of Transaction Requests,
    /// Sign them and encode them for inclusion in a Bundle.
    #[instrument(skip_all)]
    async fn sign_and_encode_txns(
        &self,
        provider: &TxSenderProvider,
        tx_requests: Vec<TransactionRequest>,
    ) -> Result<Vec<SignedTx>, Error> {
        let mut encoded_txs: Vec<SignedTx> = Vec::new();
        for mut tx in tx_requests {
            // fill out the transaction fields
            tx = tx
                .with_from(self.signer.address())
                .with_gas_limit(DEFAULT_GAS_LIMIT)
                .with_max_priority_fee_per_gas(
                    (GWEI_TO_WEI * DEFAULT_PRIORITY_FEE_MULTIPLIER) as u128,
                );

            // sign the transaction
            let SendableTx::Envelope(filled) = provider.fill(tx).await? else {
                eyre::bail!("Failed to fill transaction")
            };

            // encode it
            let encoded = filled.encoded_2718();
            let tx_hash = *filled.hash();
            info!(
                ?tx_hash,
                chain_id = provider.get_chain_id().await?,
                "Transaction signed and encoded"
            );

            // add to array
            encoded_txs.push(SignedTx {
                encoded: Bytes::from(encoded),
                hash: tx_hash,
            });
        }
        Ok(encoded_txs)
    }

    async fn watch_confirmations(
        &self,
        ru_hashes: &[TxHash],
        host_hashes: &[TxHash],
    ) -> Result<(), Error> {
        let mut watchers = Vec::new();

        for hash in ru_hashes {
            watchers.push(self.watch_single(&self.ru_provider, *hash, "rollup"));
        }

        for hash in host_hashes {
            watchers.push(self.watch_single(&self.host_provider, *hash, "host"));
        }

        try_join_all(watchers).await?;
        Ok(())
    }

    async fn watch_single(
        &self,
        provider: &TxSenderProvider,
        tx_hash: TxHash,
        chain_label: &'static str,
    ) -> Result<(), Error> {
        info!(
            ?tx_hash,
            chain = chain_label,
            "Waiting for transaction confirmation"
        );

        let pending = provider
            .watch_pending_transaction(
                PendingTransactionConfig::new(tx_hash)
                    .with_required_confirmations(1)
                    .with_timeout(Some(Duration::from_secs(300))),
            )
            .await?;

        let confirmed_hash = pending
            .await
            .map_err(|err| eyre!("failed waiting for {chain_label} tx {tx_hash:?}: {err}"))?;

        info!(
            ?confirmed_hash,
            chain = chain_label,
            "Transaction confirmed"
        );
        Ok(())
    }
}

#[derive(Debug)]
struct SignedTx {
    encoded: Bytes,
    hash: TxHash,
}
