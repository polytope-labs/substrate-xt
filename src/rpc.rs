use crate::{
	error::Error as SubstrateXtError, AdrressFor, ConstructExt, ExtrinsicProgress,
	RpcExternalities, TraitPair, UncheckedExtrinsicFor,
};
use futures::TryFutureExt;
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
	MultiSignature, MultiSigner,
};
use std::{marker::PhantomData, sync::Arc};

pub struct Client<T> {
	client: Arc<WsClient>,
	handle: tokio::runtime::Handle,
	_phantom: PhantomData<T>,
}

impl<T: ConstructExt + Send + Sync> Client<T> {
	pub async fn new<S: AsRef<str>>(from: S) -> Result<Client<T>, SubstrateXtError> {
		let handle = tokio::runtime::Handle::current();

		let client = WsClientBuilder::default()
			.max_request_body_size(u32::MAX)
			.build(from.as_ref())
			.map_err(|e| format!("`WsClientBuilder` failed to build: {:?}", e))
			.await
			.map_err(|e| {
				SubstrateXtError::ClientError(format!("Failed to build client: {:?}", e))
			})?;
		Ok(Client { client: Arc::new(client), handle, _phantom: PhantomData })
	}

	pub fn with_rpc_externalities<R>(
		&self,
		at: Option<<T::Runtime as frame_system::Config>::Hash>,
		closure: impl FnOnce() -> R,
	) -> R {
		let mut externalities = RpcExternalities::<T>::new(self, at);
		externalities.execute_with(closure)
	}

	pub fn construct_extrinsic(
		&self,
		call: <<T as ConstructExt>::Runtime as frame_system::Config>::Call,
		pair: T::Pair,
	) -> Result<UncheckedExtrinsicFor<T>, SubstrateXtError>
	where
		<T::Runtime as frame_system::Config>::AccountId: From<AccountId32>,
		<<T as ConstructExt>::Runtime as frame_system::Config>::Call: Encode + Send,
		MultiSigner: From<<<T as ConstructExt>::Pair as sp_core::Pair>::Public>,
		MultiSignature: From<<<T as ConstructExt>::Pair as TraitPair>::Signature>,
		AdrressFor<T>: From<AccountId32>,
	{
		let account_id = MultiSigner::from(pair.public()).into_account();
		let mut externalities = RpcExternalities::<T>::new(self, None);
		let payload = externalities
			.execute_with(|| {
				let extra = T::signed_extras(account_id.into());
				SignedPayload::new(call, extra).map_err::<&'static str, _>(|e| e.into())
			})
			.map_err(|e| {
				SubstrateXtError::ClientError(format!("Failed to construct extrinsic: {:?}", e))
			})?;

		let address = MultiSigner::from(pair.public()).into_account();
		let signature = payload.using_encoded(|encoded| pair.sign(encoded));

		let (call, extra, ..) = payload.deconstruct();

		Ok(UncheckedExtrinsic::new_signed(call, address.into(), signature.into(), extra))
	}

	pub async fn submit_extrinsic(
		&self,
		xt: UncheckedExtrinsicFor<T>,
	) -> Result<<T::Runtime as frame_system::Config>::Hash, SubstrateXtError>
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
			.map_err(|e| {
				SubstrateXtError::ClientError(format!("Failed to submit extrinsic: {:?}", e))
			})?;
		Ok(ext_hash)
	}

	pub async fn submit_and_watch(
		&self,
		xt: UncheckedExtrinsicFor<T>,
	) -> Result<ExtrinsicProgress<T::Runtime>, SubstrateXtError>
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
			.map_err(|e| {
				SubstrateXtError::ClientError(format!("Failed to submit extrinsic: {:?}", e))
			})?;
		Ok(ExtrinsicProgress::new(subscription))
	}

	pub(crate) fn storage(
		&self,
		key: StorageKey,
		at: Option<<T::Runtime as frame_system::Config>::Hash>,
	) -> Option<StorageData> {
		let client = self.client.clone();
		let handle = self.handle.clone();
		let handle = std::thread::spawn(move || {
			let future =
				client.request::<Option<StorageData>>("state_getStorage", rpc_params!(key, at));
			handle.block_on(future).ok().flatten()
		});
		if let Ok(res) = handle.join() {
			res
		} else {
			None
		}
	}
}
