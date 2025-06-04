use alloy::{
    primitives::{U256, uint},
    signers::Signer,
};
use chrono::Utc;
use init4_bin_base::utils::{from_env::FromEnv, signer::LocalOrAws};
use orders::{
    filler::{Filler, FillerConfig},
    order::SendOrder,
    provider::{TxSenderProvider, connect_provider},
};
use signet_types::SignedOrder;
use signet_zenith::RollupOrders::{Input, Order, Output};
use tokio::time::{Duration, sleep};

const ONE_USDC: U256 = uint!(1_000_000_U256);

/// Construct, sign, and send a Signet Order, then Fill the same Order.
#[tokio::main(flavor = "multi_thread")]
async fn main() -> eyre::Result<()> {
    // load config from environment variables
    let config = FillerConfig::from_env()?;

    // connect signer and provider
    let signer = config.signer_config.connect().await?;
    let provider = connect_provider(signer.clone(), config.ru_rpc_url.clone()).await?;

    // create an example order swapping 1 rollup USDC for 1 host USDC
    let example_order = Order {
        inputs: vec![Input {
            token: config.constants.rollup().tokens().usdc(),
            amount: ONE_USDC,
        }],
        outputs: vec![Output {
            token: config.constants.host().tokens().usdc(),
            amount: ONE_USDC,
            chainId: config.constants.host().chain_id() as u32,
            recipient: signer.address(),
        }],
        deadline: U256::from(Utc::now().timestamp() + 60), // 60 seconds from now
    };

    // sign & send the order to the transaction cache
    let signed = send_order(example_order, &signer, &config).await?;

    // wait ~1 sec to ensure order is in cache
    sleep(Duration::from_secs(1)).await;

    // fill the order from the transaction cache
    fill_orders(&signed, signer, provider, config).await?;

    Ok(())
}

/// Sign and send an order to the transaction cache.
async fn send_order(
    order: Order,
    signer: &LocalOrAws,
    config: &FillerConfig,
) -> eyre::Result<SignedOrder> {
    let send_order = SendOrder::new(signer.clone(), config.constants.clone())?;

    // sign the order, return it back for comparison
    let signed = send_order.sign_order(order).await?;

    // send the signed order to the transaction cache
    send_order.send_order(signed.clone()).await?;

    Ok(signed)
}

/// Fill example orders from the transaction cache.
async fn fill_orders(
    target_order: &SignedOrder,
    signer: LocalOrAws,
    provider: TxSenderProvider,
    config: FillerConfig,
) -> eyre::Result<()> {
    let filler = Filler::new(signer, provider, config.constants)?;

    // get all SignedOrders from tx cache
    let orders: Vec<SignedOrder> = filler.get_orders().await?;

    // filter orders into a Vec<SignedOrder> of only orders that match the target order
    let fillable_orders: Vec<SignedOrder> =
        orders.into_iter().filter(|o| o == target_order).collect();

    // fill each individually
    filler.fill_individually(fillable_orders.as_slice()).await?;

    Ok(())
}
