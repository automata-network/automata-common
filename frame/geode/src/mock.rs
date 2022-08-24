use crate as pallet_geode;
use automata_traits::{AttestorAccounting, GeodeAccounting};
use codec::Encode;
use frame_support::dispatch::DispatchResult;
use frame_support::dispatch::DispatchResultWithPostInfo;
use frame_support::parameter_types;
use frame_system as system;
use primitives::order::OrderState;
use primitives::*;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};
pub const INIT_BALANCE: u64 = 100_100_100;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
pub type AccountId = u64;
type BlockNumber = u64;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
        GeodeModule: pallet_geode::{Pallet, Call, Storage, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

impl system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type PalletInfo = PalletInfo;
    type BlockWeights = ();
    type BlockLength = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 500;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ();
    /// The type for recording an account's balance.
    type Balance = u64;
    /// The ubiquitous event type.
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
where
    Call: From<C>,
{
    type Extrinsic = UncheckedExtrinsic;
    type OverarchingCall = Call;
}

impl automata_traits::attestor::AttestorTrait for Test {
    type AccountId = AccountId;
    fn is_abnormal_mode() -> bool {
        false
    }
    fn check_healthy(app_id: &Self::AccountId) -> bool {
        false
    }
}

impl automata_traits::order::OrderTrait for Test {
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type AccountId = AccountId;
    fn is_order_expired(_order_id: Self::Hash, _session_index: Self::BlockNumber) -> bool {
        false
    }
    fn on_new_session(session_index: Self::BlockNumber) {}
    fn on_orders_dispatch(session_index: Self::BlockNumber) {}
    fn on_emergency_order_dispatch(session_index: Self::BlockNumber) {}
    fn on_order_state(
        geode_id: Self::AccountId,
        order_id: Self::Hash,
        target_state: OrderState,
    ) -> DispatchResult {
        Ok(())
    }
}

parameter_types! {
    pub const MaxGeodeProcessOneBlock: u32 = 1;
}

impl pallet_geode::Config for Test {
    type Event = Event;
    type AttestorHandler = Test;
    type OrderHandler = Test;
    type MaxGeodeProcessOneBlock = MaxGeodeProcessOneBlock;
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

pub fn gen_hash(val: u8) -> H256 {
    let mut hash = H256::default();
    hash.0[0] = val;
    hash
}

use automata_traits::geode::GeodeTrait;
use primitives::geodesession::GeodeSessionPhase;

pub struct GeodeSession {
    pub idx: BlockNumber,
    pub phase: Option<GeodeSessionPhase>,
}

impl GeodeSession {
    pub fn new() -> Self {
        Self {
            idx: 0,
            phase: None,
        }
    }
    pub fn next_session(&mut self) {
        self.idx += 1;
        self.phase = None;
        self.next_phase();
    }

    pub fn next_phase_to(&mut self, phase: GeodeSessionPhase) {
        loop {
            self.next_phase();
            println!("session: {:?} -> {:?}", self.idx, self.phase.unwrap());
            if self.phase == Some(phase) {
                break;
            }
        }
    }

    pub fn next_phase(&mut self) {
        let phase = match self.phase {
            Some(phase) => {
                let phase = phase.next();
                if phase == GeodeSessionPhase::all()[0] {
                    self.idx += 1;
                }
                phase
            }
            None => GeodeSessionPhase::all()[0].clone(),
        };
        match phase {
            GeodeSessionPhase::SessionInitialize => {
                GeodeModule::on_new_session(self.idx);
            }
            GeodeSessionPhase::ExpiredCheck => {
                GeodeModule::on_expired_check(self.idx);
            }
            GeodeSessionPhase::GeodeOffline => {
                GeodeModule::on_geode_offline(self.idx);
            }
            GeodeSessionPhase::OrderDispatch => {
                // GeodeModule::on_orders_dispatch(self.idx);
            }
        };
        self.phase = Some(phase);
    }
}

#[macro_export]
macro_rules! assert_state {
    ($order_id:expr, $state:expr) => {
        assert_eq!(
            <pallet_geode::Geodes<Test>>::get($order_id)
                .unwrap()
                .working_state,
            $state
        );
    };
}

#[macro_export]
macro_rules! assert_geode {
    ($order_id:expr, $field:ident, $state:expr) => {
        assert_eq!(
            <pallet_geode::Geodes<Test>>::get($order_id).unwrap().$field,
            $state
        );
    };
}
