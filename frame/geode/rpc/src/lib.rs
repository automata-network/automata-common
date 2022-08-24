use jsonrpc_core::{Error, ErrorCode, Result};
use jsonrpc_derive::rpc;
pub use pallet_geode_rpc_runtime_api::GeodeRuntimeApi;
use sp_api::BlockId;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;

const RUNTIME_ERROR: i64 = 1;

#[rpc]
pub trait GeodeApi<BlockHash> {
    #[rpc(name = "geode_ready")]
    fn geode_ready(&self, message: Vec<u8>, signature_raw_bytes: Vec<u8>) -> Result<bool>;
    #[rpc(name = "geode_finalizing")]
    fn geode_finalizing(&self, message: Vec<u8>, signature_raw_bytes: Vec<u8>) -> Result<bool>;
    #[rpc(name = "geode_finalized")]
    fn geode_finalized(&self, message: Vec<u8>, signature_raw_bytes: Vec<u8>) -> Result<bool>;
    #[rpc(name = "geode_initialize_failed")]
    fn geode_initialize_failed(
        &self,
        message: Vec<u8>,
        signature_raw_bytes: Vec<u8>,
    ) -> Result<bool>;
}

pub struct GeodeClient<C, P> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<P>,
}

impl<C, P> GeodeClient<C, P> {
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}

impl<C, Block> GeodeApi<<Block as BlockT>::Hash> for GeodeClient<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: GeodeRuntimeApi<Block>,
{
    fn geode_ready(&self, message: Vec<u8>, signature_raw_bytes: Vec<u8>) -> Result<bool> {
        let api = self.client.runtime_api();
        let best = self.client.info().best_hash;
        let at = BlockId::hash(best);
        let mut signature = [0_u8; 64];
        if signature_raw_bytes.len() != signature.len() {
            return Err(Error {
                code: ErrorCode::ServerError(RUNTIME_ERROR),
                message: "Runtime unable to send heartbeat.".into(),
                data: Some("invalid signature".into()),
            });
        }
        signature.copy_from_slice(&signature_raw_bytes);
        let result = api
            .unsigned_geode_ready(&at, message, signature)
            .map_err(|e| Error {
                code: ErrorCode::ServerError(RUNTIME_ERROR),
                message: "Runtime unable to call geode_ready.".into(),
                data: Some(format!("{:?}", e).into()),
            })?;
        Ok(result)
    }

    fn geode_finalizing(&self, message: Vec<u8>, signature_raw_bytes: Vec<u8>) -> Result<bool> {
        let api = self.client.runtime_api();
        let best = self.client.info().best_hash;
        let at = BlockId::hash(best);
        let mut signature = [0_u8; 64];
        if signature_raw_bytes.len() != signature.len() {
            return Err(Error {
                code: ErrorCode::ServerError(RUNTIME_ERROR),
                message: "Runtime unable to send heartbeat.".into(),
                data: Some("invalid signature".into()),
            });
        }
        signature.copy_from_slice(&signature_raw_bytes);
        let result = api
            .unsigned_geode_finalizing(&at, message, signature)
            .map_err(|e| Error {
                code: ErrorCode::ServerError(RUNTIME_ERROR),
                message: "Runtime unable to call geode_finalizing.".into(),
                data: Some(format!("{:?}", e).into()),
            })?;
        Ok(result)
    }

    fn geode_finalized(&self, message: Vec<u8>, signature_raw_bytes: Vec<u8>) -> Result<bool> {
        let api = self.client.runtime_api();
        let best = self.client.info().best_hash;
        let at = BlockId::hash(best);
        let mut signature = [0_u8; 64];
        if signature_raw_bytes.len() != signature.len() {
            return Err(Error {
                code: ErrorCode::ServerError(RUNTIME_ERROR),
                message: "Runtime unable to send heartbeat.".into(),
                data: Some("invalid signature".into()),
            });
        }
        signature.copy_from_slice(&signature_raw_bytes);
        let result = api
            .unsigned_geode_finalized(&at, message, signature)
            .map_err(|e| Error {
                code: ErrorCode::ServerError(RUNTIME_ERROR),
                message: "Runtime unable to geode_finalized.".into(),
                data: Some(format!("{:?}", e).into()),
            })?;
        Ok(result)
    }

    fn geode_initialize_failed(
        &self,
        message: Vec<u8>,
        signature_raw_bytes: Vec<u8>,
    ) -> Result<bool> {
        let api = self.client.runtime_api();
        let best = self.client.info().best_hash;
        let at = BlockId::hash(best);
        let mut signature = [0_u8; 64];
        if signature_raw_bytes.len() != signature.len() {
            return Err(Error {
                code: ErrorCode::ServerError(RUNTIME_ERROR),
                message: "Runtime unable to send heartbeat.".into(),
                data: Some("invalid signature".into()),
            });
        }
        signature.copy_from_slice(&signature_raw_bytes);
        let result = api
            .unsigned_geode_initialize_failed(&at, message, signature)
            .map_err(|e| Error {
                code: ErrorCode::ServerError(RUNTIME_ERROR),
                message: "Runtime unable to geode_initialize_failed.".into(),
                data: Some(format!("{:?}", e).into()),
            })?;
        Ok(result)
    }
}
