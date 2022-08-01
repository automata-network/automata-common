use codec::Codec;
use jsonrpc_core::{Error, ErrorCode, Result};
use jsonrpc_derive::rpc;
use pallet_daoportal::datastructures::{DAOProposal, Project, ProjectId, ProposalId};
pub use pallet_daoportal_rpc_runtime_api::DAOPortalRuntimeApi;
use sp_api::BlockId;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use sp_runtime::traits::MaybeDisplay;
use std::sync::Arc;

const RUNTIME_ERROR: i64 = 1;

#[rpc]
pub trait DAOPortalApi<BlockHash, AccountId> {
    //transfer to substrate address
    #[rpc(name = "daoportal_getProjects")]
    fn get_projects(&self) -> Result<Vec<(ProjectId, Project<AccountId>)>>;

    #[rpc(name = "daoportal_getProposals")]
    fn get_proposals(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<(ProposalId, DAOProposal<AccountId>)>>;

    #[rpc(name = "daoportal_getAllProposals")]
    fn get_all_proposals(&self) -> Result<Vec<(ProjectId, ProposalId, DAOProposal<AccountId>)>>;
}

pub struct DAOPortalClient<C, P> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<P>,
}

impl<C, P> DAOPortalClient<C, P> {
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}

impl<C, Block, AccountId> DAOPortalApi<<Block as BlockT>::Hash, AccountId>
    for DAOPortalClient<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: DAOPortalRuntimeApi<Block, AccountId>,
    AccountId: Codec + MaybeDisplay,
{
    /// get projects list
    fn get_projects(&self) -> Result<Vec<(ProjectId, Project<AccountId>)>> {
        let api = self.client.runtime_api();
        let best = self.client.info().best_hash;
        let at = BlockId::hash(best);

        let projects_list = api.get_projects(&at).map_err(|e| Error {
            code: ErrorCode::ServerError(RUNTIME_ERROR),
            message: "Runtime unable to get projects list.".into(),
            data: Some(format!("{:?}", e).into()),
        })?;

        Ok(projects_list)
    }

    /// get proposals for a project
    fn get_proposals(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<(ProposalId, DAOProposal<AccountId>)>> {
        let api = self.client.runtime_api();
        let best = self.client.info().best_hash;
        let at = BlockId::hash(best);

        let proposals_list = api.get_proposals(&at, project_id).map_err(|e| Error {
            code: ErrorCode::ServerError(RUNTIME_ERROR),
            message: "Runtime unable to get projects list.".into(),
            data: Some(format!("{:?}", e).into()),
        })?;

        Ok(proposals_list)
    }

    /// get all projects
    fn get_all_proposals(&self) -> Result<Vec<(ProjectId, ProposalId, DAOProposal<AccountId>)>> {
        let api = self.client.runtime_api();
        let best = self.client.info().best_hash;
        let at = BlockId::hash(best);

        let proposals_list = api.get_all_proposals(&at).map_err(|e| Error {
            code: ErrorCode::ServerError(RUNTIME_ERROR),
            message: "Runtime unable to get projects list.".into(),
            data: Some(format!("{:?}", e).into()),
        })?;

        Ok(proposals_list)
    }
}
