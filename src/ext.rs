use std::any::{Any, TypeId};

use crate::{rpc::Client, ConstructExt};
use sc_client_api::{ChildInfo, StorageKey};

pub(crate) struct RpcExternalities<'a, T: ConstructExt> {
	client: &'a Client<T>,
	extensions: sp_externalities::Extensions,
	at: Option<<T::Runtime as frame_system::Config>::Hash>,
}

impl<'a, T: ConstructExt + Send + Sync> RpcExternalities<'a, T> {
	pub(crate) fn new(
		client: &'a Client<T>,
		at: Option<<T::Runtime as frame_system::Config>::Hash>,
	) -> Self {
		Self { client, at, extensions: sp_externalities::Extensions::new() }
	}

	pub(crate) fn execute_with<R>(&'a mut self, execute: impl FnOnce() -> R) -> R {
		sp_externalities::set_and_run_with_externalities(self, execute)
	}
}

impl<'a, T: ConstructExt> sp_externalities::ExtensionStore for RpcExternalities<'a, T> {
	fn extension_by_type_id(&mut self, type_id: TypeId) -> Option<&mut dyn Any> {
		self.extensions.get_mut(type_id)
	}

	fn register_extension_with_type_id(
		&mut self,
		type_id: TypeId,
		extension: Box<dyn sp_externalities::Extension>,
	) -> Result<(), sp_externalities::Error> {
		self.extensions.register_with_type_id(type_id, extension)
	}

	fn deregister_extension_by_type_id(
		&mut self,
		type_id: TypeId,
	) -> Result<(), sp_externalities::Error> {
		self.extensions
			.deregister(type_id)
			.then(|| ())
			.ok_or(sp_externalities::Error::ExtensionIsNotRegistered(type_id))
	}
}

impl<'a, T: ConstructExt + Send + Sync> sp_externalities::Externalities
	for RpcExternalities<'a, T>
{
	fn set_offchain_storage(&mut self, _key: &[u8], _value: Option<&[u8]>) {
		unimplemented!("set_offchain_storage")
	}

	fn storage(&self, key: &[u8]) -> Option<Vec<u8>> {
		self.client.storage(StorageKey(key.to_vec()), self.at).map(|data| data.0)
	}

	fn storage_hash(&self, _key: &[u8]) -> Option<Vec<u8>> {
		unimplemented!("storage_hash")
	}

	fn child_storage_hash(&self, _child_info: &ChildInfo, _key: &[u8]) -> Option<Vec<u8>> {
		unimplemented!("child_storage_hash")
	}

	fn child_storage(&self, _child_info: &ChildInfo, _key: &[u8]) -> Option<Vec<u8>> {
		unimplemented!("child_storage")
	}

	fn next_storage_key(&self, _key: &[u8]) -> Option<Vec<u8>> {
		unimplemented!("next_storage_key")
	}

	fn next_child_storage_key(&self, _child_info: &ChildInfo, _key: &[u8]) -> Option<Vec<u8>> {
		unimplemented!("next_child_storage_key")
	}

	fn kill_child_storage(&mut self, _child_info: &ChildInfo, _limit: Option<u32>) -> (bool, u32) {
		unimplemented!("kill_child_storage")
	}

	fn clear_prefix(&mut self, _prefix: &[u8], _limit: Option<u32>) -> (bool, u32) {
		unimplemented!("clear_prefix")
	}

	fn clear_child_prefix(
		&mut self,
		_child_info: &ChildInfo,
		_prefix: &[u8],
		_limit: Option<u32>,
	) -> (bool, u32) {
		unimplemented!("clear_child_prefix")
	}

	fn place_storage(&mut self, _key: Vec<u8>, _value: Option<Vec<u8>>) {
		// no-op
	}

	fn place_child_storage(
		&mut self,
		_child_info: &ChildInfo,
		_key: Vec<u8>,
		_value: Option<Vec<u8>>,
	) {
		unimplemented!("place_child_storage")
	}

	fn storage_root(&mut self, _state_version: sp_storage::StateVersion) -> Vec<u8> {
		unimplemented!("storage_root")
	}

	fn child_storage_root(
		&mut self,
		_child_info: &ChildInfo,
		_state_version: sp_storage::StateVersion,
	) -> Vec<u8> {
		unimplemented!("child_storage_root")
	}

	fn storage_append(&mut self, _key: Vec<u8>, _value: Vec<u8>) {
		unimplemented!("storage_append")
	}

	fn storage_start_transaction(&mut self) {
		unimplemented!("storage_start_transaction")
	}

	fn storage_rollback_transaction(&mut self) -> Result<(), ()> {
		unimplemented!("storage_rollback_transaction")
	}

	fn storage_commit_transaction(&mut self) -> Result<(), ()> {
		unimplemented!("storage_commit_transaction")
	}

	fn wipe(&mut self) {
		unimplemented!("wipe")
	}

	fn commit(&mut self) {
		unimplemented!("commit")
	}

	fn read_write_count(&self) -> (u32, u32, u32, u32) {
		unimplemented!("read_write_count")
	}

	fn reset_read_write_count(&mut self) {
		unimplemented!("reset_read_write_count")
	}

	fn get_whitelist(&self) -> Vec<sp_storage::TrackedStorageKey> {
		unimplemented!("get_whitelist")
	}

	fn set_whitelist(&mut self, _new: Vec<sp_storage::TrackedStorageKey>) {
		unimplemented!("set_whitelist")
	}

	fn get_read_and_written_keys(&self) -> Vec<(Vec<u8>, u32, u32, bool)> {
		unimplemented!("get_read_and_written_keys")
	}
}
