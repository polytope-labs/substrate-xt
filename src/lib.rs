use sp_runtime::{
	generic::UncheckedExtrinsic, traits::SignedExtension, MultiAddress, MultiSignature,
};

use sp_core::Pair as TraitPair;

mod ext;
mod extrinsic;
mod rpc;

pub use rpc::Client;

use extrinsic::ExtrinsicProgress;

use crate::ext::RpcExternalities;

pub type AdrressFor<T> = MultiAddress<
	<<T as ConstructExt>::Runtime as frame_system::Config>::AccountId,
	<<T as ConstructExt>::Runtime as frame_system::Config>::Index,
>;
pub type UncheckedExtrinsicFor<T> = UncheckedExtrinsic<
	AdrressFor<T>,
	<<T as ConstructExt>::Runtime as frame_system::Config>::Call,
	MultiSignature,
	<T as ConstructExt>::SignedExtra,
>;

pub trait ConstructExt {
	/// SignedExtra
	type SignedExtra: SignedExtension;
	/// Runtime type
	type Runtime: frame_system::Config;
	/// Signer
	type Pair: TraitPair;

	fn signed_extras(
		account_id: <Self::Runtime as frame_system::Config>::AccountId,
	) -> Self::SignedExtra;
}

#[cfg(test)]
mod tests {
	use super::*;
	use node_runtime::{Call, Runtime, SignedExtra};
	use sp_runtime::generic::Era;

	pub struct XtConstructor;

	const WS_URL: &'static str = "ws://127.0.0.1:9944";

	impl ConstructExt for XtConstructor {
		type Runtime = Runtime;
		type Pair = sp_core::sr25519::Pair;
		type SignedExtra = SignedExtra;

		fn signed_extras(
			account_id: <Self::Runtime as frame_system::Config>::AccountId,
		) -> Self::SignedExtra {
			let nonce = frame_system::Pallet::<Self::Runtime>::account_nonce(account_id);
			(
				frame_system::CheckNonZeroSender::<Runtime>::new(),
				frame_system::CheckSpecVersion::<Runtime>::new(),
				frame_system::CheckTxVersion::<Runtime>::new(),
				frame_system::CheckGenesis::<Runtime>::new(),
				frame_system::CheckEra::<Runtime>::from(Era::Immortal),
				frame_system::CheckNonce::<Runtime>::from(nonce),
				frame_system::CheckWeight::<Runtime>::new(),
				pallet_asset_tx_payment::ChargeAssetTxPayment::<Runtime>::from(0, None),
			)
		}
	}

	#[test]
	fn should_submit_and_watch_extrinsic() {
		let call = Call::System(frame_system::Call::remark { remark: vec![0; 32] });
		let pair = sp_keyring::AccountKeyring::Bob.pair();
		let client = Client::<XtConstructor>::new(WS_URL).unwrap();
		// Construct extrinsic outside of async context
		// Constructing extrinsic inside async context can cause code to
		// panic in cases where externalities read storage
		let ext = client
			.construct_extrinsic(call, pair)
			.expect("Expected extrinsic to be constructed");
		client.block_on(async {
			let progress = client
				.submit_and_watch(ext)
				.await
				.expect("Expected extrinsic to be submitted successfully");
			progress.wait_for_in_block().await.unwrap();
		})
	}
}
