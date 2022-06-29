//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]

use std::sync::Arc;

use jsonrpc_core::{Error, ErrorCode, Result};
use jsonrpc_derive::rpc;
use jsonrpc_pubsub::manager::{SubscriptionManager, RandomStringIdProvider};
use node_template_runtime::{opaque::Block, AccountId, Balance, Index};
use sc_rpc::SubscriptionTaskExecutor;
pub use sc_rpc_api::DenyUnsafe;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sc_client_api::client::BlockchainEvents;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_runtime::{traits::Block as BlockT, RuntimeDebug};
use sp_std::prelude::*;

const RUNTIME_ERROR: i64 = 1;

/// Full client dependencies.
pub struct FullDeps<C, P> {
    /// The client instance to use.
    pub client: Arc<C>,
    /// Transaction pool instance.
    pub pool: Arc<P>,
    /// Whether to deny unsafe calls
    pub deny_unsafe: DenyUnsafe,
}

/// Instantiate all full RPC extensions.
pub fn create_full<C, P>(
    deps: FullDeps<C, P>,
    subscription_task_executor: SubscriptionTaskExecutor,
) -> jsonrpc_core::IoHandler<sc_rpc::Metadata>
where
    C: ProvideRuntimeApi<Block>,
    C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError> + 'static,
    C: Send + Sync + 'static,
    C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
    C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
    C::Api: BlockBuilder<Block>,
    // C::Api: fp_rpc::EthereumRuntimeRPCApi<Block>,
    C::Api: node_template_runtime::AttestorApi<Block>,
    C: BlockchainEvents<Block>,
    P: TransactionPool + 'static,
{
    use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApi};
    use substrate_frame_rpc_system::{FullSystem, SystemApi};
    // use fc_rpc::{
    //     EthApi, EthApiServer, EthDevSigner, EthPubSubApi, EthPubSubApiServer, EthSigner,
    //     HexEncodedIdProvider, NetApi, NetApiServer, Web3Api, Web3ApiServer,
    // };

    let mut io = jsonrpc_core::IoHandler::default();
    let FullDeps {
        client,
        pool,
        deny_unsafe,
    } = deps;

    io.extend_with(SystemApi::to_delegate(FullSystem::new(
        client.clone(),
        pool,
        deny_unsafe,
    )));

    io.extend_with(TransactionPaymentApi::to_delegate(TransactionPayment::new(
        client.clone(),
    )));

    // Extend this RPC with a custom API by using the following syntax.
    // `YourRpcStruct` should have a reference to a client, which is needed
    // to call into the runtime.
    // `io.extend_with(YourRpcTrait::to_delegate(YourRpcStruct::new(ReferenceToClient, ...)));`

    use super::rpc_attestor::{AttestorApi, AttestorServer};
    io.extend_with(AttestorServer::to_delegate(AttestorApi::new(
        client.clone(),
    )));

    use super::rpc_geode::{GeodeApi, GeodeServer};
    io.extend_with(GeodeServer::to_delegate(GeodeApi::new(
        client.clone(),
        SubscriptionManager::with_id_provider(
            RandomStringIdProvider::default(),
            Arc::new(subscription_task_executor),
        ),
    )));

    io
}
