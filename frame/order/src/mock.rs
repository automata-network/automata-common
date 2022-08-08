use crate as pallet_order;
use codec::Encode;
use frame_support::dispatch::DispatchResult;
use frame_support::dispatch::DispatchResultWithPostInfo;
use frame_support::parameter_types;
use frame_system as system;
use primitives::order::{OrderOf, OrderState};
use primitives::*;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};
pub const INIT_BALANCE: u64 = 100_100_100;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
type AccountId = u64;
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
        OrderModule: pallet_order::{Pallet, Call, Storage, Event<T>},
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

parameter_types! {
    pub const MaxOrderProcessOneBlock: BlockNumber = 1;
}

impl pallet_order::Config for Test {
    type Event = Event;
    type GeodeHandler = Test;
    type MaxOrderProcessOneBlock = MaxOrderProcessOneBlock;
}

impl automata_traits::geode::GeodeTrait for Test {
    type AccountId = AccountId;
    type Hash = Hash;
    type BlockNumber = BlockNumber;
    fn on_new_session(session_index: Self::BlockNumber) {}
    fn on_geode_offline(session_index: Self::BlockNumber) {}
    fn on_order_dispatched(
        session_index: Self::BlockNumber,
        order_id: Self::Hash,
        mut num: u32,
        domain: Vec<u8>,
    ) -> Vec<Self::AccountId> {
        use std::time::SystemTime;
        let ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let start = ts as u32;
        let mut ids = Vec::new();
        if num > 3 {
            num = 3;
        }
        for i in start..(start + num) {
            ids.push(i as _);
        }
        ids
    }
    fn on_expired_check(session_index: Self::BlockNumber) {}
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

pub fn gen_hash(val: u8) -> H256 {
    let mut hash = H256::default();
    hash.0[0] = val;
    hash
}

use automata_traits::order::OrderTrait;
use primitives::geodesession::GeodeSessionPhase;

pub struct GeodeSession {
    pub idx: BlockNumber,
    pub phase: Option<GeodeSessionPhase>,
}

impl GeodeSession {
    pub fn new() -> Self {
        Self {
            idx: 100,
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
            // println!("session: {:?} -> {:?}", self.idx, self.phase.unwrap());
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
                OrderModule::on_new_session(self.idx);
            }
            GeodeSessionPhase::GeodeOffline => {
                OrderModule::on_emergency_order_dispatch(self.idx);
            }
            GeodeSessionPhase::ExpiredCheck => {}
            GeodeSessionPhase::OrderDispatch => {
                OrderModule::on_orders_dispatch(self.idx);
            }
        };
        self.phase = Some(phase);
    }
}

#[macro_export]
macro_rules! assert_state {
    ($order_id:expr, $state:expr) => {
        assert_eq!(
            <pallet_order::Orders<Test>>::get($order_id).unwrap().state,
            $state
        );
    };
}

#[macro_export]
macro_rules! assert_order {
    ($order_id:expr, $field:ident, $state:expr) => {
        assert_eq!(
            <pallet_order::Orders<Test>>::get($order_id).unwrap().$field,
            $state
        );
    };
}

#[macro_export]
macro_rules! set_order_state {
    ($service_idx:expr, $order_id:expr, $state:expr) => {
        OrderModule::on_order_state(
            {
                let services = <pallet_order::OrderServices<Test>>::get($order_id);
                services.get($service_idx).unwrap().0
            },
            $order_id,
            $state,
        )
    };
}

#[macro_export]
macro_rules! assert_service_state {
    ($order_id:expr, $states:expr) => {{
        let order_services = <pallet_order::OrderServices<Test>>::get($order_id);
        assert_eq!($states.len(), order_services.len());
        for idx in 0..$states.len() {
            assert_eq!(
                $states[idx], order_services[idx].1,
                "states not match in {} => want {:?}, got {:?}",
                idx, $states[idx], order_services[idx].1
            );
        }
    }};
    ($order_id:expr, $state:expr, $len:expr) => {{
        let order_services = <pallet_order::OrderServices<Test>>::get($order_id);
        let mut cnt = 0;
        for order_service in order_services {
            if order_service.1 == $state {
                cnt += 1;
            }
        }
        assert_eq!(cnt, $len, "want {} state={:?}, got {}", $len, $state, cnt);
    }};
}

#[macro_export]
macro_rules! create_order {
    ($origin:expr, $order:expr) => {{
        use super::pallet::Event as PalletEvent;
        assert_ok!(OrderModule::create_order($origin, $order));
        let mut last_id = None;
        let events = events();
        for event in events {
            match event {
                Event::OrderModule(PalletEvent::OrderSubmitted(_, b)) => {
                    last_id = Some(b);
                }
                _ => {}
            }
        }
        let order: OrderOf<Test> = <pallet_order::Orders<Test>>::get(&last_id.unwrap()).unwrap();
        order
    }};
}

pub fn events() -> Vec<Event> {
    let evt = System::events()
        .into_iter()
        .map(|evt| evt.event)
        .collect::<Vec<_>>();

    // System::reset_events();

    evt
}
