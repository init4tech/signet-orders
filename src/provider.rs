use alloy::{
    network::{Ethereum, EthereumWallet},
    providers::{
        Identity, ProviderBuilder, RootProvider,
        fillers::{
            BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller,
            WalletFiller,
        },
    },
};
use init4_bin_base::utils::signer::LocalOrAws;

/// Type alias for the provider used to sign transactions on the rollup.
pub type TxSenderProvider = FillProvider<
    JoinFill<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        WalletFiller<EthereumWallet>,
    >,
    RootProvider,
    Ethereum,
>;

/// Connect a provider capable of filling and sending transactions to a given chain.
pub async fn connect_provider(
    signer: LocalOrAws,
    rpc_url: String,
) -> eyre::Result<TxSenderProvider> {
    ProviderBuilder::new()
        .wallet(EthereumWallet::from(signer))
        .connect(&rpc_url)
        .await
        .map_err(Into::into)
}
