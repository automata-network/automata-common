use jsonrpc_core::{Error, ErrorCode, Result};
use jsonrpc_derive::rpc;
use jsonrpc_pubsub::{typed::Subscriber, SubscriptionId};
use node_template_runtime::{opaque::Block, AccountId};
use pallet_geode::Geode;
use sc_client_api::client::BlockchainEvents;
use sc_client_api::notifications::StorageChangeSet;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;

#[rpc]
pub trait GeodeServer<BlockHash> {
    type Metadata;
    #[pubsub(subscription = "geode", subscribe, name = "geode_subscribeState")]
    fn subscribe_state(&self, _: Self::Metadata, _: Subscriber<u8>, _: [u8; 32]);
    #[pubsub(subscription = "geode", unsubscribe, name = "geode_unsubscribeState")]
    fn unsubscribe_state(
        &self,
        _: Option<Self::Metadata>,
        _: SubscriptionId,
    ) -> jsonrpc_core::Result<bool>;
}
pub struct GeodeApi<C> {
    client: Arc<C>,
    manager: jsonrpc_pubsub::manager::SubscriptionManager,
}

impl<C> GeodeApi<C> {
    pub fn new(client: Arc<C>, manager: jsonrpc_pubsub::manager::SubscriptionManager) -> Self {
        GeodeApi { client, manager }
    }

    fn get_custom_obj(changes: StorageChangeSet) -> Result<u8> {
        log::info!("get custom obj");
        Ok(123u8)
        // for (_, _, data) in changes.iter() {
        //     match data {
        //          Some(data) => {
        //             let mut value: &[u8] = &data.0.clone();
        //             match CustomObj::decode(&mut value) {
        //                 Ok(obj) => return Ok(obj),
        //                 Err(_) => warn!("unable to decode object"),
        //             }
        //          }
        //          None => warn!("empty change set"),
        //     };
        // }
        // Err(Error::internal_error())
    }
}

impl<C> GeodeServer<<Block as BlockT>::Hash> for GeodeApi<C>
where
    C: Send + Sync + 'static,
    C: ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: node_template_runtime::AttestorApi<Block>,
    C: BlockchainEvents<Block>,
{
    type Metadata = sc_rpc::Metadata;

    fn subscribe_state(&self, _: Self::Metadata, subscriber: Subscriber<u8>, geode_id: [u8; 32]) {
        log::info!("subscribe: geode_id: {:?}", geode_id);
        let mut bytes = sp_core::twox_128(b"Geode").to_vec();
        bytes.extend(&sp_core::twox_128(b"Geodes")[..]);

        // let my_module = sp_core::twox_128(b"Geode");
        // let obj = sp_core::twox_128(b"Geodes");
        // let mut key = vec![];
        // key.extend(my_module);
        // key.extend(obj);

        use sp_core::storage::StorageKey;
        let key: StorageKey = StorageKey(bytes);
        let keys = Into::<Option<Vec<_>>>::into(vec![key]);

        use futures::SinkExt;
        use futures::StreamExt as _;
        use futures::TryStreamExt as _;
        use jsonrpc_core::futures::Future;
        use jsonrpc_core::futures::Sink;

        let stream = match self.client.storage_changes_notification_stream(None, None) {
            Ok(stream) => stream,
            Err(err) => {
                let _ = subscriber.reject(Error::invalid_params(format!("{:?}", err)));
                return;
            }
        };
        let stream = stream
            .filter_map(move |(_, changes)| {
                log::info!("hello stream");
                match Self::get_custom_obj(changes) {
                    Ok(state) => futures::future::ready(Some(Ok::<_, ()>(Ok(state)))),
                    Err(_) => futures::future::ready(None),
                }
            })
            .compat();
        // let stream = self.client.import_notification_stream().filter_map(move |a| {
        //     log::info!("hello stream");
        //     futures::future::ready(Some(Ok::<_, ()>(Ok(1u8))))
        // }).compat();

        self.manager.add(subscriber, |sink| {
            sink.sink_map_err(|e| log::warn!("Error sending notifications: {:?}", e))
                .send_all(stream)
                .map(|_| ())
        });
    }

    fn unsubscribe_state(
        &self,
        _: Option<Self::Metadata>,
        id: SubscriptionId,
    ) -> jsonrpc_core::Result<bool> {
        Ok(self.manager.cancel(id))
    }
}
