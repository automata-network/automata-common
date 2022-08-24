//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]

use std::sync::Arc;

use node_template_runtime::{opaque::Block, AccountId, Balance, Index};
use sc_rpc::SubscriptionTaskExecutor;
pub use sc_rpc_api::DenyUnsafe;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_std::prelude::*;

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
    _subscription_task_executor: SubscriptionTaskExecutor,
) -> jsonrpc_core::IoHandler<sc_rpc::Metadata>
where
    C: ProvideRuntimeApi<Block>,
    C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError> + 'static,
    C: Send + Sync + 'static,
    C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
    C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
    C::Api: BlockBuilder<Block>,
    C::Api: pallet_attestor_rpc::AttestorRuntimeApi<Block, AccountId>,
    C::Api: pallet_geode_rpc::GeodeRuntimeApi<Block>,
    C::Api: pallet_daoportal_rpc::DAOPortalRuntimeApi<Block, AccountId>,
    C::Api: pallet_gmetadata_rpc::GmetadataRuntimeApi<Block>,
    P: TransactionPool + 'static,
{
    use pallet_attestor_rpc::{AttestorApi, AttestorClient};
    use pallet_daoportal_rpc::{DAOPortalApi, DAOPortalClient};
    use pallet_geode_rpc::{GeodeApi, GeodeClient};
    use pallet_gmetadata_rpc::{GmetadataApi, GmetadataClient};
    use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApi};
    use substrate_frame_rpc_system::{FullSystem, SystemApi};

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

    io.extend_with(DAOPortalApi::to_delegate(DAOPortalClient::new(
        client.clone(),
    )));

    io.extend_with(GmetadataApi::to_delegate(GmetadataClient::new(
        client.clone(),
    )));

    io.extend_with(AttestorApi::to_delegate(AttestorClient::new(
        client.clone(),
    )));

    io.extend_with(GeodeApi::to_delegate(GeodeClient::new(client.clone())));

    io
}
