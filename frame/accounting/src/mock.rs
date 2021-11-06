use crate as accounting;
use frame_support::parameter_types;
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};
use primitives::*;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

pub const INIT_BALANCE: u128 = 100_100_100;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
        AttestorModule: pallet_attestor::{Pallet, Call, Storage, Event<T>},
        AccountingModule: accounting::{Pallet, Call, Storage, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

impl system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u128>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
}

parameter_types! {
    pub const ExistentialDeposit: u128 = 500;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ();
    /// The type for recording an account's balance.
    type Balance = u128;
    /// The ubiquitous event type.
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
}

// All parameters for accounting
pub const ATTESTOR_STAKING_AMOUNT: Balance = 1 * CENTS;
pub const GEODE_STAKING_AMOUNT: Balance = 1 * CENTS;
pub const ATTESTOR_TOTAL_REWARD: Balance = 1 * CENTS;
pub const GEODE_TOTAL_REWARD: Balance = 1 * CENTS;
pub const ATTESTOR_REWARD_TIMESPAN: u128 = YEARS as u128 * 5 as u128;
pub const GEODE_REWARD_TIMESPAN: u128 = YEARS as u128 * 5 as u128;
pub const ATTESTOR_BASIC_REWARD_RATIO: u8 = 1_u8;
pub const GEODE_TERMINATE_PENALTY: Balance = 1 * CENTS;
pub const GEODE_MISCONDUCT_FOR_ATTESTOR: Balance = 1 * CENTS;
pub const GEODE_MISCONDUCT_FOR_SERVICE_USER: Balance = 1 * CENTS;
pub const COMMISSION_RATE_FOR_SERVICE: u8 = 1_u8;
pub const COMMISSION_RATE_FOR_ON_DEMAND: u8 = 1_u8;
pub const SLOT_LENGTH: BlockNumber = 1;

parameter_types! {
    pub const AttestorStakingAmount: Balance = ATTESTOR_STAKING_AMOUNT;
    pub const GeodeStakingAmount: Balance = GEODE_STAKING_AMOUNT;
    pub const AttestorTotalReward: Balance = ATTESTOR_TOTAL_REWARD;
    pub const GeodeTotalReward: Balance = GEODE_TOTAL_REWARD;

    pub const GeodeTerminatePenalty: Balance = GEODE_TERMINATE_PENALTY;
    pub const GeodeMisconductForAttestor: Balance = GEODE_MISCONDUCT_FOR_ATTESTOR;
    pub const GeodeMisconductForServiceUser: Balance = GEODE_MISCONDUCT_FOR_SERVICE_USER;  

    pub const SlotLength: BlockNumber = SLOT_LENGTH;

    pub const AttestorBasicRewardRatio: u8 = ATTESTOR_BASIC_REWARD_RATIO;
    pub const CommissionRateForService: u8 = COMMISSION_RATE_FOR_SERVICE;
    pub const CommissionRateForOnDemand: u8 = COMMISSION_RATE_FOR_ON_DEMAND;

    pub const AttestorRewardEachSlot: Balance = ATTESTOR_TOTAL_REWARD * (SLOT_LENGTH as u128) / ATTESTOR_REWARD_TIMESPAN;
    pub const GeodeRewardEachSlot: Balance = GEODE_TOTAL_REWARD * (SLOT_LENGTH as u128) / GEODE_REWARD_TIMESPAN;
}

impl accounting::Config for Test {
    type Event = Event;
    type Currency = Balances;

    type GetAttestors = AttestorModule;
    type GetGeodes = GeodeModule;

    type AttestorStakingAmount = AttestorStakingAmount;
    type GeodeStakingAmount = GeodeStakingAmount;
    type AttestorTotalReward = AttestorTotalReward;
    type GeodeTotalReward = GeodeTotalReward;

    type GeodeTerminatePenalty = GeodeTerminatePenalty;
    type GeodeMisconductForAttestor = GeodeMisconductForAttestor;
    type GeodeMisconductForServiceUser = GeodeMisconductForServiceUser;  

    type SlotLength = SlotLength;

    type AttestorBasicRewardRatio = AttestorBasicRewardRatio;
    type CommissionRateForService = CommissionRateForService;
    type CommissionRateForOnDemand = CommissionRateForOnDemand;

    type AttestorRewardEachSlot = AttestorRewardEachSlot;
    type GeodeRewardEachSlot = GeodeRewardEachSlot;
}

impl pallet_attestor::Config for Test {
    type Event = Event;
    type Currency = Balances;
    type AttestorAccounting = AccountingModule;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(1, INIT_BALANCE), (2, INIT_BALANCE)],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

pub fn events() -> Vec<Event> {
    let evt = System::events()
        .into_iter()
        .map(|evt| evt.event)
        .collect::<Vec<_>>();

    System::reset_events();

    evt
}