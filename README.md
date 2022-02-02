## Construct and submit Extrinsics

Contains utilities for constructing and submiting extrinsics to a live substrate node

## Usage

```rust
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
	   pallet_asset_tx_payment::ChargeAssetTxPayment::<Runtime>::from(0, None),);
	let call = Call::System(frame_system::Call::remark {     remark: vec![0;32] });
	let pair = sp_keyring::AccountKeyring::Bob.pair();
	let client = XtConstructor::build_client().unwrap();
        // Construct extrinsic outside of async context
        // Constructing extrinsic inside async context can cause code to
	// panic in cases where externalities read storage
        let ext = XtConstructor::construct_ext(&client, call,  pair, extra)
		.expect("Expected extrinsic to be constructed");
	client.rt.block_on(async {
	  let progress = XtConstructor::submit_and_watch(&client, ext)
		.await
		.expect("Expected extrinsic to be submitted successfully");
		progress.wait_for_in_block().await.unwrap();
	})
}
```

## Testing

1. To run the tests in this folder, download a substrate node binary,  
   build the `substrate` node binary on branch `polkadot-v0.9.16`

2. Run this binary using the default websocket port
3. Run `cargo +nightly test`
