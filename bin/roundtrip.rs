use alloy::{
    primitives::{Address, U256, uint},
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
use signet_types::SignedOrder;
use signet_zenith::RollupOrders::{Input, Order, Output};
use tokio::time::{Duration, sleep};

const ONE_USDC: U256 = uint!(1_000_000_U256);

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

    // create an example order swapping 1 rollup USDC for 1 host USDC
    let example_order = get_example_order(&config, signer.address(), args.rollup);

    // sign & send the order to the transaction cache
    let signed = send_order(example_order, &signer, &config).await?;
    debug!(?signed, "Order contents");
    info!("Order signed and sent to transaction cache");

    // wait ~1 sec to ensure order is in cache
    sleep(Duration::from_secs(1)).await;

    // fill the order from the transaction cache
    fill_orders(&signed, signer, provider, config).await?;
    info!("Order filled successfully");

    Ok(())
}

/// Constructs an example [`Order`] based on the provided configuration and recipient address.
/// If `rollup` is true, it creates an order that targets the rollup; otherwise, it creates an order that targets the host chain.
fn get_example_order(config: &FillerConfig, recipient: Address, rollup: bool) -> Order {
    if rollup {
        Order {
            inputs: vec![Input {
                token: config.constants.rollup().tokens().usdc(),
                amount: ONE_USDC,
            }],
            outputs: vec![Output {
                token: config.constants.rollup().tokens().usdc(),
                amount: ONE_USDC,
                chainId: config.constants.rollup().chain_id() as u32,
                recipient,
            }],
            deadline: U256::from(Utc::now().timestamp() + (60 * 10)), // 10 minutes from now
        }
    } else {
        Order {
            inputs: vec![Input {
                token: config.constants.rollup().tokens().usdc(),
                amount: ONE_USDC,
            }],
            outputs: vec![Output {
                token: config.constants.host().tokens().usdc(),
                amount: ONE_USDC,
                chainId: config.constants.host().chain_id() as u32,
                recipient,
            }],
            deadline: U256::from(Utc::now().timestamp() + (60 * 10)), // 10 minutes from now
        }
    }
}

/// Sign and send an order to the transaction cache.
#[instrument(skip(order, signer, config), level = "debug", fields(signer_address = %signer.address()))]
async fn send_order(
    order: Order,
    signer: &LocalOrAws,
    config: &FillerConfig,
) -> eyre::Result<SignedOrder> {
    info!("signing and sending order");

    let send_order = SendOrder::new(signer.clone(), config.constants.clone())?;

    // sign the order, return it back for comparison
    let signed = send_order.sign_order(order).await?;

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
