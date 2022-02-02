use std::marker::PhantomData;
use futures::TryFutureExt;
use jsonrpsee::{
    core::client::{ClientT, Subscription, SubscriptionClientT},
    rpc_params,
    ws_client::{WsClient, WsClientBuilder},
};
use parity_scale_codec::Encode;
use sc_client_api::{StorageData, StorageKey};
use sc_transaction_pool_api::TransactionStatus;
use sp_core::crypto::AccountId32;
use sp_runtime::generic::{SignedPayload, UncheckedExtrinsic};
use sp_runtime::{MultiSignature, MultiSigner};
use sp_runtime::traits::IdentifyAccount;
use crate::{ConstructExt, ExtrinsicProgress, RpcExternalities, TraitPair, UncheckedExtrinsicFor};

pub struct Client<T> {
    client: WsClient,
    rt: tokio::runtime::Runtime,
    _phantom: PhantomData<T>,
}

impl<T: ConstructExt> Client<T> {
    fn new<S: AsRef<str>>(from: S)-> Result<Client<T>, &'static str> {
        // TODO: add option to use multithreaded?
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Unable to build tokio runtime");

        let future = WsClientBuilder::default()
                .max_request_body_size(u32::MAX)
                .build(from.as_ref())
                .map_err(|e| format!("`WsClientBuilder` failed to build: {:?}", e));
        let client = rt.block_on(future).map_err(|_| "Failed to build client")?;
        Ok(Client {
            client,
            rt,
            _phantom: PhantomData,
        })
    }

    fn construct_ext(
        &self,
        call: <<Self as ConstructExt>::Runtime as frame_system::Config>::Call,
        pair: Self::Pair,
    ) -> Result<
        UncheckedExtrinsicFor<Self>,
        &'static str,
    >
        where
            <T::Runtime as frame_system::Config>::AccountId: From<AccountId32>,
            <<Self as ConstructExt>::Runtime as frame_system::Config>::Call: Encode + Send,
            MultiSigner: From<<<Self as ConstructExt>::Pair as sp_core::Pair>::Public>,
            MultiSignature: From<<<Self as ConstructExt>::Pair as TraitPair>::Signature>,
    {
        let account_id = MultiSigner::from(pair.public()).into_account();
        let mut externalities = RpcExternalities::<T::Runtime>::new(self);
        let payload = externalities.execute_with(|| {
            let extra = T::signed_extras(account_id.into());
            SignedPayload::new(call, extra).map_err::<&'static str, _>(|e| e.into())
        })?;

        let address = MultiSigner::from(pair.public()).into_account();
        let signature = payload.using_encoded(|encoded| pair.sign(encoded));

        let (call, extra, ..) = payload.deconstruct();

        Ok(UncheckedExtrinsic::new_signed(call, address.into(), signature.into(), extra))
    }

    async fn submit_extrinsic(
        &self,
        xt: UncheckedExtrinsicFor<T>,
    ) -> Result<<Self::Runtime as frame_system::Config>::Hash, String>
        where
            <<Self as ConstructExt>::Runtime as frame_system::Config>::Call: Encode + Send + Sync,
            <<Self as ConstructExt>::Pair as TraitPair>::Signature: Send + Sync + Encode,
            <Self as ConstructExt>::SignedExtra: Send,
    {
        let bytes: sp_core::Bytes = xt.encode().into();
        let ext_hash = self.client
            .request::<T::Hash>("author_submitExtrinsic", rpc_params!(bytes))
            .await
            .map_err(|e| format!("Failed to submit extrinsic: {:?}", e))?;
        Ok(ext_hash)
    }

    async fn submit_and_watch(
        &self,
        xt: UncheckedExtrinsicFor<T>,
    ) -> Result<ExtrinsicProgress<T::Runtime>, String>
        where
            <<Self as ConstructExt>::Runtime as frame_system::Config>::Call: Encode + Send + Sync,
            <<Self as ConstructExt>::Pair as TraitPair>::Signature: Send + Sync + Encode,
            <Self as ConstructExt>::SignedExtra: Encode + Send + Sync,
    {
        let bytes: sp_core::Bytes = xt.encode().into();
        let subscription = self.client
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
        at: Option<<T::Runtime as frame_system>::Hash>,
    ) -> Option<StorageData> {
        self.rt
            .block_on(
                self.client
                    .request::<Option<StorageData>>("state_getStorage", rpc_params!(key, at)),
            )
            .ok()
            .flatten()
    }
}
