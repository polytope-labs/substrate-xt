use jsonrpsee::{
	core::client::{ClientT, Subscription, SubscriptionClientT},
	rpc_params,
	ws_client::{WsClient, WsClientBuilder},
};
use sc_client_api::{StorageData, StorageKey};
use sc_transaction_pool_api::TransactionStatus;

pub struct RpcClient {
	client: WsClient,
	pub rt: tokio::runtime::Runtime,
}

impl RpcClient {
	pub(crate) fn new(client: WsClient, rt: tokio::runtime::Runtime) -> Self {
		RpcClient { client, rt }
	}

	pub(crate) fn storage<T: frame_system::Config>(
		&self,
		key: StorageKey,
		at: Option<T::Hash>,
	) -> Option<StorageData> {
		self.rt
			.block_on(
				self.client
					.request::<Option<StorageData>>("state_getStorage", rpc_params!(key, at)),
			)
			.ok()
			.flatten()
	}

	pub(crate) async fn submit_extrinsic<T: frame_system::Config>(
		&self,
		ext: sp_core::Bytes,
	) -> Result<T::Hash, String> {
		self.client
			.request::<T::Hash>("author_submitExtrinsic", rpc_params!(ext))
			.await
			.map_err(|e| format!("Failed to submit extrinsic: {:?}", e))
	}

	pub(crate) async fn submit_and_watch<T: frame_system::Config>(
		&self,
		ext: sp_core::Bytes,
	) -> Result<Subscription<TransactionStatus<T::Hash, T::Hash>>, String> {
		self.client
			.subscribe(
				"author_submitAndWatchExtrinsic",
				rpc_params!(ext),
				"author_unwatchExtrinsic",
			)
			.await
			.map_err(|e| format!("Failed to submit extrinsic: {:?}", e))
	}
}

/// Build a websocket client that connects to `from`.
pub(crate) async fn build_client<S: AsRef<str>>(from: S) -> Result<WsClient, String> {
	WsClientBuilder::default()
		.max_request_body_size(u32::MAX)
		.build(from.as_ref())
		.await
		.map_err(|e| format!("`WsClientBuilder` failed to build: {:?}", e))
}
