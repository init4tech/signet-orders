use alloy::{
    consensus::constants::GWEI_TO_WEI,
    primitives::{Address, U256},
    signers::Signer,
};
use chrono::Utc;
use clap::Parser;
use init4_bin_base::{
    deps::tracing::{debug, info, instrument},
    utils::{from_env::FromEnv, signer::LocalOrAws, tracing::init_tracing},
};
use orders::{
    filler::{Filler, FillerConfig},
    order::SendOrder,
    provider::{TxSenderProvider, connect_provider},
};
use signet_types::{SignedOrder, UnsignedOrder};
use tokio::time::{Duration, sleep};

#[derive(Parser, Debug)]
struct OrdersArgs {
    /// If present, the order will be filled on the rollup chain.
    /// If absent, the order will be filled on the host chain.
    #[arg(long, default_value_t = false)]
    pub rollup: bool,
}

/// Construct, sign, and send a Signet Order, then Fill the same Order.
#[tokio::main]
async fn main() -> eyre::Result<()> {
    // initialize tracing
    init_tracing();

    // load config from environment variables
    let config = FillerConfig::from_env()?;
    let args = OrdersArgs::parse();

    // connect signer and provider
    let signer = config.signer_config.connect().await?;
    let provider = connect_provider(signer.clone(), config.ru_rpc_url.clone()).await?;
    info!(signer_address = %signer.address(), "Connected to Signer and Provider");

    // create an example order
    let example_order = get_example_order(&config, signer.address(), args.rollup);

    // sign & send the order to the transaction cache
    let signed = send_order(example_order, &signer, &config).await?;
    debug!(?signed, "Order contents");
    info!("Order signed and sent to transaction cache");

    // wait ~1 sec to ensure order is in cache
    sleep(Duration::from_secs(1)).await;

    // fill the order from the transaction cache
    fill_orders(&signed, signer, provider, config).await?;
    info!("Bundle sent to tx cache successfully; wait for bundle to mine.");

    Ok(())
}

/// Constructs an example [`Order`] based on the provided configuration and recipient address.
/// If `rollup` is true, it creates an order that targets the rollup; otherwise, it creates an order that targets the host chain.
fn get_example_order(
    config: &FillerConfig,
    recipient: Address,
    rollup: bool,
) -> UnsignedOrder<'static> {
    let unsigned = UnsignedOrder::default()
        .with_input(
            config.constants.rollup().tokens().weth(),
            U256::from(GWEI_TO_WEI),
        )
        .with_deadline(Utc::now().timestamp() as u64 + (60 * 10));

    if rollup {
        unsigned.with_output(
            config.constants.rollup().tokens().weth(),
            U256::from(GWEI_TO_WEI),
            recipient,
            config.constants.rollup().chain_id() as u32,
        )
    } else {
        unsigned.with_output(
            config.constants.host().tokens().weth(),
            U256::from(GWEI_TO_WEI),
            recipient,
            config.constants.host().chain_id() as u32,
        )
    }
}

/// Sign and send an order to the transaction cache.
#[instrument(skip(order, signer, config), level = "debug", fields(signer_address = %signer.address()))]
async fn send_order(
    order: UnsignedOrder<'_>,
    signer: &LocalOrAws,
    config: &FillerConfig,
) -> eyre::Result<SignedOrder> {
    info!("signing and sending order");

    let send_order = SendOrder::new(signer.clone(), config.constants.clone())?;

    // sign the order, return it back for comparison
    let signed = order
        .with_chain(config.constants.system())
        .sign(signer)
        .await?;

    // send the signed order to the transaction cache
    send_order.send_order(signed.clone()).await?;

    Ok(signed)
}

/// Fill example [`SignedOrder`]s from the transaction cache.
#[instrument(skip(target_order, signer, provider, config), level = "debug")]
async fn fill_orders(
    target_order: &SignedOrder,
    signer: LocalOrAws,
    provider: TxSenderProvider,
    config: FillerConfig,
) -> eyre::Result<()> {
    info!("filling orders from transaction cache");
    let filler = Filler::new(signer, provider, config.constants)?;

    // get all the [`SignedOrder`]s from tx cache
    let mut orders: Vec<SignedOrder> = filler.get_orders().await?;
    debug!(
        orders = ?orders,
        "Queried order contents from transaction cache"
    );

    // Retain only the orders that match the target order
    orders.retain(|o| o == target_order);

    // fill each individually
    filler.fill_individually(orders.as_slice()).await?;

    Ok(())
}
