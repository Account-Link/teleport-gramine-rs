use alloy::{
    network::{Ethereum, EthereumWallet},
    providers::{
        fillers::{
            BlobGasFiller, CachedNonceManager, ChainIdFiller, FillProvider, GasFiller, JoinFill,
            NonceFiller, WalletFiller,
        },
        Identity, ProviderBuilder, RootProvider,
    },
    transports::http::{Client, Http},
};

pub type WalletProvider = FillProvider<
    JoinFill<
        JoinFill<
            Identity,
            JoinFill<
                GasFiller,
                JoinFill<BlobGasFiller, JoinFill<NonceFiller<CachedNonceManager>, ChainIdFiller>>,
            >,
        >,
        WalletFiller<EthereumWallet>,
    >,
    RootProvider<Http<Client>>,
    Http<Client>,
    Ethereum,
>;

pub fn get_provider(rpc_url: String, wallet: EthereumWallet) -> WalletProvider {
    let provider = ProviderBuilder::new()
        .filler(JoinFill::new(
            GasFiller,
            JoinFill::new(
                BlobGasFiller,
                JoinFill::new(NonceFiller::default(), ChainIdFiller::default()),
            ),
        ))
        .wallet(wallet)
        .on_http(rpc_url.parse().unwrap());
    provider
}

// pub fn gen_sk() -> eyre::Result<String> {
//     let mut buf = [0u8; 32];
//     getrandom::getrandom(&mut buf)?;
//     let sk = alloy::hex::encode(buf);
//     Ok(sk)
// }

// #[cfg(test)]
// mod tests {
//     use std::str::FromStr;

//     use alloy::signers::local::PrivateKeySigner;

//     #[test]
//     fn test_gen_sk() -> eyre::Result<()> {
//         let mut buf = [0u8; 32];
//         getrandom::getrandom(&mut buf)?;
//         let signer = PrivateKeySigner::from_bytes(&buf.into())?;
//         let hex_key = alloy::hex::encode(buf);
//         let signer_1 = PrivateKeySigner::from_str(&hex_key)?;
//         assert_eq!(signer.address().to_string(), signer_1.address().to_string());
//         Ok(())
//     }
// }
