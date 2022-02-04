use crate::error::{Error as SubstrateXtError, ExtrinsicError};
use jsonrpsee::core::client::Subscription;
use sc_transaction_pool_api::TransactionStatus;

pub struct ExtrinsicProgress<T: frame_system::Config> {
	sub: Subscription<TransactionStatus<T::Hash, T::Hash>>,
}

impl<T: frame_system::Config> ExtrinsicProgress<T> {
	/// Instantiate a new [`ExtrinsicProgress`] from a custom subscription.
	pub fn new(sub: Subscription<TransactionStatus<T::Hash, T::Hash>>) -> Self {
		Self { sub }
	}

	/// Return the next transaction status when it's emitted.
	pub async fn next_item(
		&mut self,
	) -> Option<Result<TransactionStatus<T::Hash, T::Hash>, jsonrpsee::core::Error>> {
		self.sub.next().await
	}

	/// Wait for extrinsic to get into block
	pub async fn wait_for_in_block(mut self) -> Result<T::Hash, SubstrateXtError> {
		while let Some(status) = self.next_item().await {
			match status.map_err(|e| ExtrinsicError::Custom(e.to_string()))? {
				// Finalized or otherwise in a block! Return.
				TransactionStatus::InBlock(s) | TransactionStatus::Finalized(s) => return Ok(s),
				// Error scenarios; return the error.
				TransactionStatus::FinalityTimeout(_) =>
					return Err(ExtrinsicError::FinalityTimeout.into()),
				// Ignore anything else and wait for next status event:
				_ => continue,
			}
		}
		Err(SubstrateXtError::RpcError("RPC subscription dropped".into()).into())
	}

	/// Wait for extrinsic to get into a finalized block
	pub async fn wait_for_finalized(mut self) -> Result<T::Hash, SubstrateXtError> {
		while let Some(status) = self.next_item().await {
			match status.map_err(|e| ExtrinsicError::Custom(e.to_string()))? {
				// Finalized! Return.
				TransactionStatus::Finalized(s) => return Ok(s),
				// Error scenarios; return the error.
				TransactionStatus::FinalityTimeout(_) =>
					return Err(ExtrinsicError::FinalityTimeout.into()),
				// Ignore and wait for next status event:
				_ => continue,
			}
		}
		Err(SubstrateXtError::RpcError("RPC subscription dropped".into()).into())
	}
}
