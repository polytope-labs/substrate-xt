use ext::MockExternalities;
use parity_scale_codec::Encode;
use rpc::{build_client, Client};
use sp_runtime::{generic::{SignedPayload, UncheckedExtrinsic}, traits::{IdentifyAccount, SignedExtension}, AccountId32, MultiSigner, MultiSignature, MultiAddress};

use sp_core::Pair as TraitPair;

mod ext;
mod extrinsic;
mod rpc;

use extrinsic::ExtrinsicProgress;

pub use async_trait;
use crate::ext::RpcExternalities;

pub type AdrressFor<T> = MultiAddress<<<T as ConstructExt>::Runtime as frame_system::Config>::AccountId, <<T as ConstructExt>::Runtime as frame_system::Config>::Index>
pub type UncheckedExtrinsicFor<T> = UncheckedExtrinsic<
    AdrressFor<T>,
    <T::Runtime as frame_system::Config>::Call,
    MultiSignature,
    T::SignedExtra,
>;


#[async_trait::async_trait]
pub trait ConstructExt {
    /// SignedExtra
    type SignedExtra: SignedExtension;
    /// Runtime type
    type Runtime: frame_system::Config;
    /// Signer
    type Pair: TraitPair;

    fn signed_extras(account_id: <Self::Runtime as frame_system::Config>::AccountId) -> Self::SignedExtra;
}

#[cfg(test)]
mod tests {
    use super::*;
    use node_runtime::{Address, Call, Runtime, Signature, SignedExtra};
    use sp_runtime::generic::Era;

    pub struct XtConstructor;

    #[async_trait::async_trait]
    impl ConstructExt for XtConstructor {
        type Runtime = Runtime;
        type Pair = sp_core::sr25519::Pair;
        type SignedExtra = SignedExtra;

        fn signed_extras() -> Self::SignedExtra {
            (
                frame_system::CheckNonZeroSender::<Runtime>::new(),
                frame_system::CheckSpecVersion::<Runtime>::new(),
                frame_system::CheckTxVersion::<Runtime>::new(),
                frame_system::CheckGenesis::<Runtime>::new(),
                frame_system::CheckEra::<Runtime>::from(Era::Immortal),
                // todo: get nonce from frame_system
                frame_system::CheckNonce::<Runtime>::from(0),
                frame_system::CheckWeight::<Runtime>::new(),
                pallet_asset_tx_payment::ChargeAssetTxPayment::<Runtime>::from(0, None),
            )
        }
    }

    #[test]
    fn should_submit_and_watch_extrinsic() {
        let call = Call::System(frame_system::Call::remark { remark: vec![0; 32] });
        let pair = sp_keyring::AccountKeyring::Bob.pair();
        let client = XtConstructor::build_client().unwrap();
        // Construct extrinsic outside of async context
        // Constructing extrinsic inside async context can cause code to
        // panic in cases where externalities read storage
        let ext = XtConstructor::construct_ext(&client, call, pair, extra)
            .expect("Expected extrinsic to be constructed");
        client.rt.block_on(async {
            let progress = XtConstructor::submit_and_watch(&client, ext)
                .await
                .expect("Expected extrinsic to be submitted successfully");
            progress.wait_for_in_block().await.unwrap();
        })
    }
}
