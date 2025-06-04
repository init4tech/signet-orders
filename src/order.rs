use alloy::signers::Signer;
use eyre::Error;
use signet_constants::SignetConstants;
use signet_tx_cache::client::TxCache;
use signet_types::{SignedOrder, UnsignedOrder};
use signet_zenith::RollupOrders::Order;

/// Example code demonstrating API usage and patterns for signing an Order.
#[derive(Debug)]
pub struct SendOrder<S: Signer> {
    /// The signer to use for signing the order.
    signer: S,
    /// The transaction cache endpoint.
    tx_cache: TxCache,
    /// The system constants.
    constants: SignetConstants,
}

impl<S> SendOrder<S>
where
    S: Signer,
{
    /// Create a new SendOrder instance.
    pub fn new(signer: S, constants: SignetConstants) -> Result<Self, Error> {
        Ok(Self {
            signer,
            tx_cache: TxCache::new_from_string(constants.environment().transaction_cache())?,
            constants,
        })
    }

    /// Sign an Order and forward it to the transaction cache to be Filled.
    pub async fn sign_and_send_order(&self, order: Order) -> Result<(), Error> {
        let signed = self.sign_order(order).await?;
        self.send_order(signed).await
    }

    /// Sign an Order.
    pub async fn sign_order(&self, order: Order) -> Result<SignedOrder, Error> {
        // make an UnsignedOrder from the Order
        let unsigned = UnsignedOrder::from(&order);

        // sign it
        unsigned
            .with_chain(
                self.constants.rollup().chain_id(),
                self.constants.rollup().orders(),
            )
            .sign(&self.signer)
            .await
            .map_err(Into::into)
    }

    /// Forward a SignedOrder to the transaction cache.
    pub async fn send_order(&self, signed: SignedOrder) -> Result<(), Error> {
        // send the SignedOrder to the transaction cache
        self.tx_cache.forward_order(signed).await
    }
}
