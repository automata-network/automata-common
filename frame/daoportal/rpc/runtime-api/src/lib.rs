#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
pub use pallet_daoportal::datastructures::{DAOProposal, Project, ProjectId, ProposalId};
use sp_runtime::traits::MaybeDisplay;
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
    pub trait DAOPortalRuntimeApi<AccountId> where
        AccountId: Codec + MaybeDisplay
    {
        fn get_projects() -> Vec<(ProjectId, Project<AccountId>)>;
        fn get_proposals(project_id: ProjectId) -> Vec<(ProposalId, DAOProposal<AccountId>)>;
        fn get_all_proposals() -> Vec<(ProjectId, ProposalId, DAOProposal<AccountId>)>;
    }
}
