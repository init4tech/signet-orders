use alloy::{
    consensus::constants::GWEI_TO_WEI,
    primitives::{Address, U256},
    signers::Signer,
};
use chrono::Utc;
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

const TX_CACHE_WAIT_TIME: Duration = Duration::from_millis(500);

#[derive(Debug, FromEnv)]
struct OrdersArgs {
    /// If true, the order will be filled on the rollup chain.
    /// else, it will be filled on the host chain.
    #[from_env(
        var = "SEND_TO_ROLLUP",
        desc = "Whether to send the order to rollup or host. If true, it will be a RU-RU order. Else, it'll be a RU-HOST order."
    )]
    pub send_to_rollup: bool,
    #[from_env(var = "SLEEP_TIME", desc = "Time to sleep between transactions, in ms")]
    sleep_time: u64,
}

/// Construct, sign, and send a Signet Order, then Fill the same Order.
#[tokio::main]
async fn main() -> eyre::Result<()> {
    init_tracing();

    let config = FillerConfig::from_env()?;
    let OrdersArgs {
        send_to_rollup,
        sleep_time,
    } = OrdersArgs::from_env()?;

    let signer = config.signer_config.connect().await?;
    let provider = connect_provider(signer.clone(), config.ru_rpc_url.clone()).await?;
    info!(signer_address = %signer.address(), "Connected to Signer and Provider");

    loop {
        let example_order = get_example_order(&config, signer.address(), send_to_rollup);

        let signed = send_order(example_order, &signer, &config).await?;
        debug!(?signed, "Order contents");

        sleep(TX_CACHE_WAIT_TIME).await;

        fill_orders(&signed, signer.clone(), provider.clone(), &config).await?;

        sleep(Duration::from_millis(sleep_time)).await;
    }
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
#[instrument(skip(order, signer, config), fields(signer_address = %signer.address()))]
async fn send_order(
    order: UnsignedOrder<'_>,
    signer: &LocalOrAws,
    config: &FillerConfig,
) -> eyre::Result<SignedOrder> {
    info!("signing and sending order");

    let send_order = SendOrder::new(signer.clone(), config.constants.clone())?;

    // sign the order, return it back for comparison
    let signed = order.sign(signer).await?;

    tracing::Span::current().record("signed_order_signature", signed.order_hash().to_string());
    debug!(?signed, "Signed order contents");

    // send the signed order to the transaction cache
    send_order.send_order(signed.clone()).await?;
    info!("Order signed and sent to transaction cache");

    Ok(signed)
}

/// Fill example [`SignedOrder`]s from the transaction cache.
#[instrument(skip(target_order, signer, provider, config), fields(target_order_signature = %target_order.permit.signature, target_order_owner = %target_order.permit.owner))]
async fn fill_orders(
    target_order: &SignedOrder,
    signer: LocalOrAws,
    provider: TxSenderProvider,
    config: &FillerConfig,
) -> eyre::Result<()> {
    info!("filling orders from transaction cache");
    let filler = Filler::new(signer, provider, config.constants.clone())?;

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

    info!("Order filled successfully");

    Ok(())
}
