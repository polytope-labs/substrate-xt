use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
	#[error("{0}")]
	ClientError(String),
	#[error("{0}")]
	RpcError(String),
	#[error("{0}")]
	TransactionError(ExtrinsicError),
}

#[derive(Error, Debug)]
pub enum ExtrinsicError {
	#[error("Extrinsic was not finalized before timeout")]
	FinalityTimeout,
	#[error("{0}")]
	Custom(String),
}

impl From<ExtrinsicError> for Error {
	fn from(e: ExtrinsicError) -> Self {
		Self::TransactionError(e)
	}
}
