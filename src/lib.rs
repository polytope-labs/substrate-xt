use ext::MockExternalities;
use parity_scale_codec::Encode;
use rpc::{build_client, RpcClient};
use sp_runtime::{
	generic::{SignedPayload, UncheckedExtrinsic},
	traits::{IdentifyAccount, SignedExtension},
	AccountId32, MultiSigner,
};

use sp_core::Pair as TraitPair;

mod ext;
mod extrinsic;
mod rpc;
use extrinsic::ExtrinsicProgress;

pub use async_trait;

#[async_trait::async_trait]
pub trait ConstructExt {
	type SignedExtra: SignedExtension;
	type Runtime: frame_system::Config;
	type Pair: TraitPair;
	type Signature;
	type Address;
	const WS_URL: &'static str;

	fn build_client() -> Result<RpcClient, &'static str> {
		let rt = tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("Unable to build tokio runtime");
		let client =
			rt.block_on(build_client(Self::WS_URL)).map_err(|_| "Failed to build client")?;
		Ok(RpcClient::new(client, rt))
	}

	fn construct_ext(
		client: &RpcClient,
		call: <<Self as ConstructExt>::Runtime as frame_system::Config>::Call,
		pair: Self::Pair,
		extra: Self::SignedExtra,
	) -> Result<
		UncheckedExtrinsic<
			Self::Address,
			<<Self as ConstructExt>::Runtime as frame_system::Config>::Call,
			Self::Signature,
			Self::SignedExtra,
		>,
		&'static str,
	>
	where
		<<Self as ConstructExt>::Runtime as frame_system::Config>::Call: Encode + Send,
		MultiSigner: From<<<Self as ConstructExt>::Pair as sp_core::Pair>::Public>,
		<Self as ConstructExt>::Address: From<AccountId32>,
		<Self as ConstructExt>::Signature:
			From<<<Self as ConstructExt>::Pair as TraitPair>::Signature>,
	{
		let mut externalities = MockExternalities::<Self::Runtime>::new(client);
		let payload = externalities.execute_with(|| {
			SignedPayload::new(call, extra).map_err::<&'static str, _>(|e| e.into())
		})?;

		let address = MultiSigner::from(pair.public()).into_account();
		let signature = payload.using_encoded(|encoded| pair.sign(encoded));

		let (call, extra, ..) = payload.deconstruct();

		Ok(UncheckedExtrinsic::new_signed(call, address.into(), signature.into(), extra))
	}

	async fn submit_extrinsic(
		client: &RpcClient,
		xt: UncheckedExtrinsic<
			Self::Address,
			<<Self as ConstructExt>::Runtime as frame_system::Config>::Call,
			Self::Signature,
			Self::SignedExtra,
		>,
	) -> Result<<Self::Runtime as frame_system::Config>::Hash, String>
	where
		<<Self as ConstructExt>::Runtime as frame_system::Config>::Call: Encode + Send + Sync,
		<Self as ConstructExt>::Address: Send + Sync + Encode,
		<Self as ConstructExt>::Signature: Send + Sync + Encode,
		<Self as ConstructExt>::SignedExtra: Send,
	{
		let ext_hash = client.submit_extrinsic::<Self::Runtime>(xt.encode().into()).await?;
		Ok(ext_hash)
	}

	async fn submit_and_watch(
		client: &RpcClient,
		xt: UncheckedExtrinsic<
			Self::Address,
			<<Self as ConstructExt>::Runtime as frame_system::Config>::Call,
			Self::Signature,
			Self::SignedExtra,
		>,
	) -> Result<ExtrinsicProgress<Self::Runtime>, String>
	where
		<<Self as ConstructExt>::Runtime as frame_system::Config>::Call: Encode + Send + Sync,
		<Self as ConstructExt>::Address: Encode + Send + Sync,
		<Self as ConstructExt>::Signature: Send + Sync + Encode,
		<Self as ConstructExt>::SignedExtra: Encode + Send + Sync,
	{
		let subscription = client.submit_and_watch::<Self::Runtime>(xt.encode().into()).await?;
		Ok(ExtrinsicProgress::new(subscription))
	}
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
		type Address = Address;
		type Signature = Signature;
		type Pair = sp_core::sr25519::Pair;
		type SignedExtra = SignedExtra;
		const WS_URL: &'static str = "ws://127.0.0.1:9944";
	}

	#[test]
	fn should_submit_and_watch_extrinsic() {
		let extra = (
			frame_system::CheckNonZeroSender::<Runtime>::new(),
			frame_system::CheckSpecVersion::<Runtime>::new(),
			frame_system::CheckTxVersion::<Runtime>::new(),
			frame_system::CheckGenesis::<Runtime>::new(),
			frame_system::CheckEra::<Runtime>::from(Era::Immortal),
			frame_system::CheckNonce::<Runtime>::from(0),
			frame_system::CheckWeight::<Runtime>::new(),
			pallet_asset_tx_payment::ChargeAssetTxPayment::<Runtime>::from(0, None),
		);

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
