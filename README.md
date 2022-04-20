## Construct and submit Extrinsics

Contains utilities for constructing and submiting extrinsics to a live substrate node

## Usage

```rust
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

	#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
	// #[tokio::test] this won't work, which is also a note on how the api is meant to be used
	// this client needs at least two threads, because of the call to handle.block_on in the
	// call to client.storage() from the `RpcExternalities`
	async fn should_submit_and_watch_extrinsic() {
		let call = Call::System(frame_system::Call::remark { remark: vec![0; 32] });
		let pair = sp_keyring::AccountKeyring::Bob.pair();
		println!("connecting");
		let client = Client::<XtConstructor>::new(WS_URL).await.unwrap();

		let ext = client
			.construct_extrinsic(call, pair)
			.expect("Expected extrinsic to be constructed");
		let progress = client
			.submit_and_watch(ext)
			.await
			.expect("Expected extrinsic to be submitted successfully");
		progress.wait_for_in_block().await.unwrap();
	}

	#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
	async fn should_read_storage_map_and_storage_double_map() {
		let client = Client::<XtConstructor>::new(WS_URL).await.unwrap();
		let mut externalities = RpcExternalities::<XtConstructor>::new(&client);

		externalities.execute_with(|| {
			let pair = sp_keyring::AccountKeyring::Bob.pair();
			let account_id = MultiSigner::from(pair.public()).into_account();
			// Reading storage map should not panic
			frame_system::Pallet::<Runtime>::account_nonce(account_id);

			// Reading storage double map should not panic
			pallet_im_online::Pallet::<Runtime>::is_online(0);
		});
	}

```

## Testing

1. To run the tests in this folder, download a substrate node binary,  
   build the `substrate` node binary on branch `polkadot-v0.9.18`

2. Run this binary using the default websocket port
3. Run `cargo +nightly test`
