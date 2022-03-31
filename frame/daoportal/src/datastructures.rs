// use sp_std::prelude::*;
// use frame_support::{pallet_prelude::*};
// use frame_system::pallet_prelude::*;
// use sp_std::prelude::*;
use codec::{Decode, Encode};

use sp_core::U256;
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

pub type ChainIndex = u32;
pub type ProjectId = u32;
pub type ProposalId = u32;
pub type OptionIndex = u8;
pub type VotingPower = U256;
pub type IpfsHash = sp_core::H256;
pub type EthAddress = sp_core::H160;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub enum CrossChainAccount<AccountId> {
    Solidity(EthAddress),
    Substrate(AccountId),
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub enum Protocol {
    Solidity,
    Substrate,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub enum Strategy {
    Solidity(SolidityStrategy),
    Substrate(SubstrateStrategy),
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, Ord, PartialOrd)]
pub enum SolidityStrategy {
    ERC20Balance(EthAddress),
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, Ord, PartialOrd)]
pub enum SubstrateStrategy {
    NativeBalance,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub enum VotingFormat {
    SingleChoice,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct UserGroup<AccountId> {
    pub owner: CrossChainAccount<AccountId>,
    pub admins: Vec<CrossChainAccount<AccountId>>,
    pub proposers: Option<Vec<CrossChainAccount<AccountId>>>
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct Project<AccountId> {
    // pub owner: CrossChainAccount<AccountId>,
    pub usergroup: UserGroup<AccountId>,
    pub data: IpfsHash,
    pub workspaces: Vec<Workspace>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct Chain {
    pub _protocol: Protocol,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct Workspace {
    pub _chain: ChainIndex,
    pub strategies: Vec<Strategy>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct Proposal<AccountId> {
    pub _author: CrossChainAccount<AccountId>,
    pub _voting_format: VotingFormat,
    pub _option_count: OptionIndex,
    pub _data: IpfsHash,
    pub _privacy: PrivacyLevel,
    pub _start: u64,
    pub _end: u64,
    pub _frequency: Option<u64>,
    pub state: ProposalState,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub enum PrivacyLevel {
    Opaque,
    Private,
    Public,
    Mixed,
}

// #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
// pub enum ProposalStatus {
//     Pending,
//     Ongoing,
//     Closed,
// }

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct ProposalState {
    // pub status: ProposalStatus,
    pub finalized: bool,
    pub snapshots: Vec<U256>,
    pub blacklisted: bool,
    pub votes: Vec<VotingPower>,
    pub pub_voters: Option<IpfsHash>,
    pub updates: u32,
}

impl Default for ProposalState {
    fn default() -> Self {
        ProposalState {
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
