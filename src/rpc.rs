use crate::{ConstructExt, ExtrinsicProgress, RpcExternalities, TraitPair, UncheckedExtrinsicFor};
use futures::{Future, TryFutureExt};
use jsonrpsee::{
	core::client::{ClientT, SubscriptionClientT},
	rpc_params,
	ws_client::{WsClient, WsClientBuilder},
};
use parity_scale_codec::Encode;
use sc_client_api::{StorageData, StorageKey};
use sp_core::crypto::AccountId32;
use sp_runtime::{
	generic::{SignedPayload, UncheckedExtrinsic},
	traits::IdentifyAccount,
	MultiAddress, MultiSignature, MultiSigner,
};
use std::marker::PhantomData;

pub struct Client<T> {
	client: WsClient,
	rt: tokio::runtime::Runtime,
	_phantom: PhantomData<T>,
}

impl<T: ConstructExt + Send + Sync> Client<T> {
	pub fn new<S: AsRef<str>>(
		from: S,
		enable_multithreaded: bool,
	) -> Result<Client<T>, &'static str> {
		let rt = if enable_multithreaded {
			tokio::runtime::Builder::new_multi_thread()
				.enable_all()
				.build()
				.expect("Unable to build tokio runtime")
		} else {
			tokio::runtime::Builder::new_current_thread()
				.enable_all()
				.build()
				.expect("Unable to build tokio runtime")
		};

		let future = WsClientBuilder::default()
			.max_request_body_size(u32::MAX)
			.build(from.as_ref())
			.map_err(|e| format!("`WsClientBuilder` failed to build: {:?}", e));
		let client = rt.block_on(future).map_err(|_| "Failed to build client")?;
		Ok(Client { client, rt, _phantom: PhantomData })
	}

	pub fn block_on<O>(&self, execute: impl Future<Output = O>) {
		self.rt.block_on(execute);
	}

	pub fn construct_ext(
		&self,
		call: <<T as ConstructExt>::Runtime as frame_system::Config>::Call,
		pair: T::Pair,
	) -> Result<UncheckedExtrinsicFor<T>, &'static str>
	where
		<T::Runtime as frame_system::Config>::AccountId: From<AccountId32>,
		<<T as ConstructExt>::Runtime as frame_system::Config>::Call: Encode + Send,
		MultiSigner: From<<<T as ConstructExt>::Pair as sp_core::Pair>::Public>,
		MultiSignature: From<<<T as ConstructExt>::Pair as TraitPair>::Signature>,
		MultiAddress<
			<<T as ConstructExt>::Runtime as frame_system::Config>::AccountId,
			<<T as ConstructExt>::Runtime as frame_system::Config>::Index,
		>: From<AccountId32>,
	{
		let account_id = MultiSigner::from(pair.public()).into_account();
		let mut externalities = RpcExternalities::<T>::new(self);
		let payload = externalities.execute_with(|| {
			let extra = T::signed_extras(account_id.into());
			SignedPayload::new(call, extra).map_err::<&'static str, _>(|e| e.into())
		})?;

		let address = MultiSigner::from(pair.public()).into_account();
		let signature = payload.using_encoded(|encoded| pair.sign(encoded));

		let (call, extra, ..) = payload.deconstruct();

		Ok(UncheckedExtrinsic::new_signed(call, address.into(), signature.into(), extra))
	}

	pub async fn submit_extrinsic(
		&self,
		xt: UncheckedExtrinsicFor<T>,
	) -> Result<<T::Runtime as frame_system::Config>::Hash, String>
	where
		<<T as ConstructExt>::Runtime as frame_system::Config>::Call: Encode + Send + Sync,
		<<T as ConstructExt>::Pair as TraitPair>::Signature: Send + Sync + Encode,
		<T as ConstructExt>::SignedExtra: Encode + Send + Sync,
	{
		let bytes: sp_core::Bytes = xt.encode().into();
		let ext_hash = self
			.client
			.request::<<T::Runtime as frame_system::Config>::Hash>(
				"author_submitExtrinsic",
				rpc_params!(bytes),
			)
			.await
			.map_err(|e| format!("Failed to submit extrinsic: {:?}", e))?;
		Ok(ext_hash)
	}

	pub async fn submit_and_watch(
		&self,
		xt: UncheckedExtrinsicFor<T>,
	) -> Result<ExtrinsicProgress<T::Runtime>, String>
	where
		<<T as ConstructExt>::Runtime as frame_system::Config>::Call: Encode + Send + Sync,
		<<T as ConstructExt>::Pair as TraitPair>::Signature: Send + Sync + Encode,
		<T as ConstructExt>::SignedExtra: Encode + Send + Sync,
	{
		let bytes: sp_core::Bytes = xt.encode().into();
		let subscription = self
			.client
			.subscribe(
				"author_submitAndWatchExtrinsic",
				rpc_params!(bytes),
				"author_unwatchExtrinsic",
			)
			.await
			.map_err(|e| format!("Failed to submit extrinsic: {:?}", e))?;
		Ok(ExtrinsicProgress::new(subscription))
	}

	pub(crate) fn storage(
		&self,
		key: StorageKey,
		at: Option<<T::Runtime as frame_system::Config>::Hash>,
	) -> Option<StorageData> {
		let future = self
			.client
			.request::<Option<StorageData>>("state_getStorage", rpc_params!(key, at));
		self.rt.block_on(future).ok().flatten()
	}
}
