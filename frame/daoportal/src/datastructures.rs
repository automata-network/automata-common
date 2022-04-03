// use sp_std::prelude::*;
// use frame_support::{pallet_prelude::*};
// use frame_system::pallet_prelude::*;
// use sp_std::prelude::*;
use codec::{Decode, Encode};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use sp_core::{U256, H160};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

pub type ChainIndex = u32;
pub type ProjectId = u32;
pub type ProposalId = u32;
pub type OptionIndex = u8;
pub type VotingPower = U256;
pub type IpfsHash = Vec<u8>;
pub type EthAddress = H160;

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub enum CrossChainAccount<AccountId> {
    Solidity(EthAddress),
    Substrate(AccountId),
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub enum Protocol {
    Solidity,
    Substrate,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub enum Strategy {
    Solidity(SolidityStrategy),
    Substrate(SubstrateStrategy),
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, Ord, PartialOrd)]
pub enum SolidityStrategy {
    ERC20Balance(EthAddress),
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, Ord, PartialOrd)]
pub enum SubstrateStrategy {
    NativeBalance,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub enum VotingFormat {
    SingleChoice,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct UserGroup<AccountId> {
    pub owner: CrossChainAccount<AccountId>,
    pub admins: Vec<CrossChainAccount<AccountId>>,
    pub maintainers: Vec<CrossChainAccount<AccountId>>,
    pub proposers: Option<Vec<CrossChainAccount<AccountId>>>
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct Project<AccountId> {
    // pub owner: CrossChainAccount<AccountId>,
    pub usergroup: UserGroup<AccountId>,
    pub data: IpfsHash,
    pub workspaces: Vec<Workspace>,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct Chain {
    pub _protocol: Protocol,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct Workspace {
    pub _chain: ChainIndex,
    pub strategies: Vec<Strategy>,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct DAOProposal<AccountId> {
    pub _author: CrossChainAccount<AccountId>,
    pub _voting_format: VotingFormat,
    pub _option_count: OptionIndex,
    pub _data: IpfsHash,
    pub _privacy: PrivacyLevel,
    pub _start: u64,
    pub _end: u64,
    pub _frequency: Option<u64>,
    pub state: DAOProposalState,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub enum PrivacyLevel {
    Opaque(u8, bool),
    Private,
    Public,
    Mixed,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct DAOProposalState {
    // pub status: ProposalStatus,
    pub finalized: bool,
    pub snapshots: Vec<U256>,
    pub blacklisted: bool,
    pub votes: Vec<VotingPower>,
    pub pub_voters: Option<IpfsHash>,
    pub updates: u32,
}

impl Default for DAOProposalState {
    fn default() -> Self {
        DAOProposalState {
            finalized: false,
            snapshots: Vec::new(),
            blacklisted: false,
            votes: Vec::new(),
            pub_voters: None,
            updates: 0,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct VoteUpdate {
    pub project: ProjectId,
    pub proposal: ProposalId,
    pub votes: Vec<VotingPower>,
    pub pub_voters: Option<IpfsHash>,
}
