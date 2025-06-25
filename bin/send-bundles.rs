use init4_bin_base::{
    deps::tracing::debug,
    utils::{from_env::FromEnv, tracing::init_tracing},
};
use orders::{bundle::BundleSender, filler::FillerConfig, provider::connect_provider};

/// Construct, sign, and send a Signet Order, then Fill the same Order.
#[tokio::main(flavor = "multi_thread")]
async fn main() -> eyre::Result<()> {
    // initialize tracing
    init_tracing();

    // load config from environment variables
    let config = FillerConfig::from_env()?;

    // connect signer and provider
    debug!("Connecting signer and provider...");
    let signer = config.signer_config.connect().await?;
    let provider = connect_provider(signer.clone(), config.ru_rpc_url.clone()).await?;

    let bundle_sender = BundleSender::new(provider, config.constants)?;

    bundle_sender.send_dummy_bundles(10).await?;

    Ok(())
}
