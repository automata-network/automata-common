use jsonrpc_core::{Error, ErrorCode, Result};
use jsonrpc_derive::rpc;
use pallet_gmetadata::datastructures::{GmetadataKey, GmetadataQueryResult, HexBytes};
pub use pallet_gmetadata_rpc_runtime_api::GmetadataRuntimeApi;
use sp_api::BlockId;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;

const RUNTIME_ERROR: i64 = 1;

#[rpc]
pub trait GmetadataApi<BlockHash> {
    //transfer to substrate address
    #[rpc(name = "gmetadata_queryWithIndex")]
    fn query_with_index(
        &self,
        index_key: Vec<GmetadataKey>,
        value_key: GmetadataKey,
        cursor: HexBytes,
        limit: u64,
    ) -> Result<GmetadataQueryResult>;
}

pub struct GmetadataClient<C, P> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<P>,
}

impl<C, P> GmetadataClient<C, P> {
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}

impl<C, Block> GmetadataApi<<Block as BlockT>::Hash> for GmetadataClient<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: GmetadataRuntimeApi<Block>,
{
    fn query_with_index(
        &self,
        index_key: Vec<GmetadataKey>,
        value_key: GmetadataKey,
        cursor: HexBytes,
        limit: u64,
    ) -> Result<GmetadataQueryResult> {
        let api = self.client.runtime_api();
        let best = self.client.info().best_hash;
        let at = BlockId::hash(best);

        let result = api
            .query_with_index(&at, index_key, value_key, cursor, limit)
            .map_err(|e| Error {
                code: ErrorCode::ServerError(RUNTIME_ERROR),
                message: "Runtime unable to call query_with_index.".into(),
                data: Some(format!("{:?}", e).into()),
            })?;

        Ok(result)
    }
}
