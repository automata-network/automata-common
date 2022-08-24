use codec::Codec;
use jsonrpc_core::{Error, ErrorCode, Result};
use jsonrpc_derive::rpc;
pub use pallet_attestor_rpc_runtime_api::AttestorRuntimeApi;
use sp_api::BlockId;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use sp_runtime::traits::MaybeDisplay;
use std::sync::Arc;

const RUNTIME_ERROR: i64 = 1;

#[rpc]
pub trait AttestorApi<BlockHash, AccountId> {
    /// return the registered geode list
    #[rpc(name = "attestor_list")]
    fn attestor_list(&self) -> Result<Vec<(Vec<u8>, Vec<u8>, u32)>>;
    #[rpc(name = "attestor_attested_appids")]
    fn attestor_attested_appids(&self, attestor: [u8; 32]) -> Result<Vec<[u8; 32]>>;
    #[rpc(name = "attestor_heartbeat")]
    fn attestor_heartbeat(&self, message: Vec<u8>, signature_raw_bytes: Vec<u8>) -> Result<bool>;
}

pub struct AttestorClient<C, P> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<P>,
}

impl<C, P> AttestorClient<C, P> {
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}

impl<C, Block, AccountId> AttestorApi<<Block as BlockT>::Hash, AccountId>
    for AttestorClient<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: AttestorRuntimeApi<Block, AccountId>,
    AccountId: Codec + MaybeDisplay + From<[u8; 32]> + Into<[u8; 32]>,
{
    fn attestor_list(&self) -> Result<Vec<(Vec<u8>, Vec<u8>, u32)>> {
        let api = self.client.runtime_api();
        let best = self.client.info().best_hash;
        let at = BlockId::hash(best);
        let attestor_list = api.attestor_list(&at).map_err(|e| Error {
            code: ErrorCode::ServerError(RUNTIME_ERROR),
            message: "Runtime unable to get attestor list.".into(),
            data: Some(format!("{:?}", e).into()),
        })?;
        Ok(attestor_list)
    }

    fn attestor_attested_appids(&self, attestor: [u8; 32]) -> Result<Vec<[u8; 32]>> {
        let api = self.client.runtime_api();
        let best = self.client.info().best_hash;
        let at = BlockId::hash(best);
        let attestor_attested_geodes_list = api
            .attestor_attested_appids(&at, attestor.into())
            .map_err(|e| Error {
                code: ErrorCode::ServerError(RUNTIME_ERROR),
                message: "Runtime unable to get attestor attested app list.".into(),
                data: Some(format!("{:?}", e).into()),
            })?;
        let attestor_attested_geodes_list = attestor_attested_geodes_list
            .into_iter()
            .map(|e| e.into())
            .collect();
        Ok(attestor_attested_geodes_list)
    }

    fn attestor_heartbeat(&self, message: Vec<u8>, signature_raw_bytes: Vec<u8>) -> Result<bool> {
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
            .unsigned_attestor_heartbeat(&at, message, signature)
            .map_err(|e| Error {
                code: ErrorCode::ServerError(RUNTIME_ERROR),
                message: "Runtime unable to send heartbeat.".into(),
                data: Some(format!("{:?}", e).into()),
            })?;
        Ok(result)
    }
}
