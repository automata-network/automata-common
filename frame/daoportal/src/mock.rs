use datastructures::*;
use frame_support::{assert_ok, parameter_types};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

use super::*;
use crate as pallet_daoportal;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        DAOPortal: pallet_daoportal::{Pallet, Call, Storage, Event<T>},
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(1024);
}

impl frame_system::Config for Test {
    type AccountData = pallet_balances::AccountData<u64>;
    type AccountId = u64;
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockHashCount = BlockHashCount;
    type BlockLength = ();
    type BlockNumber = u64;
    type BlockWeights = ();
    type Call = Call;
    type DbWeight = ();
    type Event = Event;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type Header = Header;
    type Index = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type OnKilledAccount = ();
    type OnNewAccount = ();
    type OnSetCode = ();
    type Origin = Origin;
    type PalletInfo = PalletInfo;
    type SS58Prefix = ();
    type SystemWeightInfo = ();
    type Version = ();
}

parameter_types! {
    pub const MaxLocks: u32 = 10;
    pub const ExistentialDeposit: u32 = 10;
}
impl pallet_balances::Config for Test {
    type AccountStore = System;
    type Balance = u64;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type MaxLocks = MaxLocks;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type WeightInfo = ();
}

parameter_types! {
    pub const MinimumPeriod: u64 = 5;
}
impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
    pub const MinDuration: u64 = 1000;
    pub const MaxDuration: u64 = 10000;
    pub const MaxOptionCount: u8 = 3;
    pub const MaxWorkspace: u32 = 100;
    pub const MaxStrategy: u32 = 100;
}

impl Config for Test {
    type Event = Event;
    type Currency = Balances;
    type MinDuration = MinDuration;
    type MaxDuration = MaxDuration;
    type MaxOptionCount = MaxOptionCount;
    type MaxWorkspace = MaxWorkspace;
    type MaxStrategy = MaxStrategy;
    type UnixTime = Timestamp;
    type DAOPortalWeightInfo = ();
}

pub const INIT_BALANCE: u64 = 100_100_100;

pub struct ExtBuilder {}
impl Default for ExtBuilder {
    fn default() -> Self {
        Self {}
    }
}
impl ExtBuilder {
    pub fn build(&self) -> sp_io::TestExternalities {
        let mut t = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();
        pallet_balances::GenesisConfig::<Test> {
            balances: vec![
                (1, INIT_BALANCE), // relayer
                (2, INIT_BALANCE), // native user
            ],
        }
        .assimilate_storage(&mut t)
        .unwrap();
        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| {
            System::set_block_number(1);
            Timestamp::set_timestamp(1000);
        });
        ext
    }

    pub fn install(&self) -> sp_io::TestExternalities {
        let mut t = self.build();
        t.execute_with(|| {
            // set relayer
            assert_ok!(DAOPortal::update_relayer(Origin::root(), 1));
            assert_eq!(DAOPortal::relayer(), 1);
            // register 2 chains
            assert_ok!(DAOPortal::register_chain(
                Origin::root(),
                Chain {
                    _protocol: Protocol::Solidity
                }
            ));
            assert_ok!(DAOPortal::register_chain(
                Origin::root(),
                Chain {
                    _protocol: Protocol::Substrate
                }
            ));
            assert_eq!(DAOPortal::latest_chain_index(), 2);
            // set fee
            assert_ok!(DAOPortal::update_vote_fee(Origin::root(), 100));
        });
        t
    }

    pub fn install_w_project(&self) -> sp_io::TestExternalities {
        let mut t = self.install();
        t.execute_with(|| {
            let valid_workspace = Workspace {
                _chain: 1,
                strategies: vec![Strategy::Solidity(SolidityStrategy::ERC20Balance(
                    EthAddress::default(),
                ))],
            };
            // Adding project
            assert_ok!(DAOPortal::add_project(
                Some(2).into(),
                Project {
                    owner: CrossChainAccount::Substrate(2),
                    data: IpfsHash::default(),
                    workspaces: vec![valid_workspace.clone()]
                }
            ));
            assert_eq!(DAOPortal::latest_project_id(), 1);
        });
        t
    }

    pub fn install_w_proposal(&self) -> sp_io::TestExternalities {
        let mut t = self.install_w_project();
        t.execute_with(|| {
            assert_ok!(DAOPortal::add_proposal(
                Some(2).into(),
                1,
                Proposal {
                    _author: CrossChainAccount::Substrate(2),
                    _voting_format: VotingFormat::SingleChoice,
                    _option_count: 2,
                    _data: IpfsHash::default(),
                    _privacy: PrivacyLevel::Mixed,
                    _start: 2000,
                    _end: 5000,
                    _frequency: Some(1000),
                    state: ProposalState::default()
                }
            ));
            assert_ok!(DAOPortal::add_proposal(
                Some(2).into(),
                1,
                Proposal {
                    _author: CrossChainAccount::Substrate(2),
                    _voting_format: VotingFormat::SingleChoice,
                    _option_count: 2,
                    _data: IpfsHash::default(),
                    _privacy: PrivacyLevel::Private,
                    _start: 2000,
                    _end: 5000,
                    _frequency: Some(1000),
                    state: ProposalState::default()
                }
            ));
            assert_ok!(DAOPortal::add_proposal(
                Some(2).into(),
                1,
                Proposal {
                    _author: CrossChainAccount::Substrate(2),
                    _voting_format: VotingFormat::SingleChoice,
                    _option_count: 2,
                    _data: IpfsHash::default(),
                    _privacy: PrivacyLevel::Opaque,
                    _start: 2000,
                    _end: 5000,
                    _frequency: None,
                    state: ProposalState::default()
                }
            ));
            assert_eq!(DAOPortal::latest_proposal_id(1), 3);
        });
        t
    }
}
