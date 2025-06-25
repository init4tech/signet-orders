use crate::provider::TxSenderProvider;
use alloy::{
    eips::Encodable2718,
    network::TransactionBuilder,
    primitives::{Address, Bytes, U256},
    providers::{Provider, SendableTx, WalletProvider},
    rpc::types::{TransactionRequest, mev::EthSendBundle},
};
use eyre::Error;
use init4_bin_base::deps::tracing::{debug, trace};
use signet_bundle::SignetEthBundle;
use signet_constants::SignetConstants;
use signet_tx_cache::client::TxCache;

/// Multiplier for converting gwei to wei.
const GWEI_TO_WEI: u64 = 1_000_000_000;

/// Example code demonstrating API usage and patterns for Signet Fillers.
#[derive(Debug)]
pub struct BundleSender {
    /// The provider to use for building transactions on the Rollup.
    ru_provider: TxSenderProvider,
    /// The transaction cache endpoint.
    tx_cache: TxCache,
}

impl BundleSender {
    /// Create a new Filler with the given signer, provider, and transaction cache endpoint.
    pub fn new(ru_provider: TxSenderProvider, constants: SignetConstants) -> Result<Self, Error> {
        let tx_cache_url = constants
            .environment()
            .transaction_cache()
            .strip_prefix("https://")
            .map(|s| format!("http://{s}:8080"))
            .unwrap();
        debug!(tx_cache_url, "Connecting to transaction cache");
        Ok(Self {
            ru_provider,
            tx_cache: TxCache::new_from_string(&tx_cache_url)?,
        })
    }

    /// Send a dummy Bundle to the transaction cache.
    /// Bundle contains a single, simple rollup transaction sending 1 wei to the zero address.
    pub async fn send_dummy_bundles(&self, num_blocks: u64) -> Result<(), Error> {
        // get a dummy transaction request for the rollup
        let tx_requests = self.dummy_tx_request().await?;
        trace!(?tx_requests, "Transaction requests");

        // sign & encode the transaction for the Bundle
        let txs = self.sign_and_encode_txns(tx_requests).await?;
        trace!(?txs, "Encoded transactions");

        // set the Bundle to only be valid if mined in the next rollup block
        let current_block_number = self.ru_provider.get_block_number().await? + 1;
        debug!(current_block_number, "Lowest block number for Bundle");

        // loop through `num_blocks` block numbers to ensure the Bundle lands in a block
        for i in 0..num_blocks {
            // construct a Bundle for the given block
            let target_block_number = current_block_number + i;
            let bundle = SignetEthBundle {
                host_fills: None, // no Host fills in this example
                bundle: EthSendBundle {
                    txs: txs.clone(),
                    reverting_tx_hashes: vec![],
                    block_number: target_block_number,
                    min_timestamp: None, // sufficiently covered by pinning to next block number
                    max_timestamp: None, // sufficiently covered by pinning to next block number
                    replacement_uuid: None, // optional if implementing strategies that replace or cancel bundles
                },
            };
            debug!(
                target_block_number,
                "Sending bundle for block number to transaction cache"
            );

            // submit the Bundle to the transaction cache
            let response = self.tx_cache.forward_bundle(bundle).await?;

            debug!(
                target_block_number,
                bundle_id = ?response.id,
                "Sent bundle to transaction cache"
            );
        }

        Ok(())
    }

    /// Construct a single dummy Transaction Request.
    /// This is a simple transaction sending 1 wei to the zero address.
    async fn dummy_tx_request(&self) -> Result<Vec<TransactionRequest>, Error> {
        Ok(vec![
            TransactionRequest::default()
                .with_to(Address::default())
                .with_value(U256::from(1)),
        ])
    }

    /// Given an ordered set of Transaction Requests,
    /// Sign them and encode them for inclusion in a Bundle.
    pub async fn sign_and_encode_txns(
        &self,
        tx_requests: Vec<TransactionRequest>,
    ) -> Result<Vec<Bytes>, Error> {
        let mut encoded_txs: Vec<Bytes> = Vec::new();
        for mut tx in tx_requests {
            // fill out the transaction fields
            tx = tx
                .with_from(self.ru_provider.default_signer_address())
                .with_gas_limit(1_000_000)
                .with_max_priority_fee_per_gas((GWEI_TO_WEI * 16) as u128);

            // sign the transaction
            let SendableTx::Envelope(filled) = self.ru_provider.fill(tx).await? else {
                eyre::bail!("Failed to fill transaction")
            };

            // encode it
            let encoded = filled.encoded_2718();

            // add to array
            encoded_txs.push(Bytes::from(encoded));
        }
        Ok(encoded_txs)
    }
}
