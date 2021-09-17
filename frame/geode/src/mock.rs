use crate as pallet_geode;
use frame_support::parameter_types;
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

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
        System: frame_system::{Module, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Module, Call, Storage, Event<T>},
        AttestorModule: pallet_attestor::{Module, Call, Storage, Event<T>},
        GeodeModule: pallet_geode::{Module, Call, Storage, Event<T>},
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

impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
where
    Call: From<C>,
{
    type Extrinsic = UncheckedExtrinsic;
    type OverarchingCall = Call;
}

impl pallet_attestor::Config for Test {
    type Event = Event;
    type Currency = Balances;
    type Call = Call;
}

impl pallet_geode::Config for Test {
    type Event = Event;
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

pub fn register_attestor(attestor_account: <Test as system::Config>::AccountId) {
    let url = vec![1];
    let pubkey = vec![2];
    let min_stake = 100;
    let attestor_account = 1;

    // set the min stake balance
    AttestorModule::set_att_stake_min(Origin::root(), min_stake);

    // successfully call register
    AttestorModule::attestor_register(
        Origin::signed(attestor_account),
        url.clone(),
        pubkey.clone(),
    );
}

pub fn provider_register_geode(
    provider: <Test as system::Config>::AccountId,
    geode_id: <Test as system::Config>::AccountId,
) {
    let geode: pallet_geode::Geode<
        <Test as system::Config>::AccountId,
        <Test as system::Config>::Hash,
    > = pallet_geode::Geode {
        id: geode_id,
        provider: provider,
        order: None,
        ip: vec![],
        dns: vec![],
        props: Default::default(),
        state: Default::default(),
        promise: Default::default(),
    };

    GeodeModule::provider_register_geode(Origin::signed(provider), geode);
}
