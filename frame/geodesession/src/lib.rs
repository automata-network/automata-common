#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use automata_traits::geode::GeodeTrait;
    use automata_traits::order::OrderTrait;
    use core::convert::TryInto;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use primitives::geodesession::GeodeSessionPhase;
    use sp_std::collections::btree_map::BTreeMap;
    use sp_std::vec::Vec;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        NewSessionId(u32),
    }

    #[pallet::storage]
    #[pallet::getter(fn session_id)]
    pub type SessionId<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn phase_block)]
    pub type PhaseBlock<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        GeodeSessionPhase,
        T::BlockNumber,
        ValueQuery,
        DefaultPhaseBlock<T>,
    >;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type GeodeHandler: GeodeTrait<
            AccountId = <Self as frame_system::Config>::AccountId,
            Hash = <Self as frame_system::Config>::Hash,
            BlockNumber = <Self as frame_system::Config>::BlockNumber,
        >;
        type OrderHandler: OrderTrait<
            Hash = <Self as frame_system::Config>::Hash,
            BlockNumber = <Self as frame_system::Config>::BlockNumber,
        >;
    }

    #[pallet::type_value]
    pub fn DefaultPhaseBlock<T: Config>() -> T::BlockNumber {
        1u32.into()
    }

    #[pallet::error]
    pub enum Error<T> {}

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(block_height: BlockNumberFor<T>) -> Weight {
            let mut session_index = <SessionId<T>>::get();
            let (phase, is_new_session) = Self::get_phase(block_height);
            if is_new_session {
                session_index += 1u32.into();
                <SessionId<T>>::put(session_index);
                Self::deposit_event(<Event<T>>::NewSessionId(
                    session_index.try_into().unwrap_or_default(),
                ));
            }
            match phase {
                GeodeSessionPhase::SessionInitialize => {
                    T::GeodeHandler::on_new_session(session_index);
                    T::OrderHandler::on_new_session(session_index);
                }
                GeodeSessionPhase::GeodeOffline => {
                    T::GeodeHandler::on_geode_offline(session_index);
                    T::OrderHandler::on_emergency_order_dispatch(session_index);
                }
                GeodeSessionPhase::OrderDispatch => {
                    T::OrderHandler::on_orders_dispatch(session_index);
                }
                GeodeSessionPhase::ExpiredCheck => {
                    T::GeodeHandler::on_expired_check(session_index);
                }
            }
            0
        }
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(0)]
        pub fn set_phase_block(
            origin: OriginFor<T>,
            phase_blocks: Vec<(GeodeSessionPhase, T::BlockNumber)>,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            for (phase, block) in phase_blocks {
                <PhaseBlock<T>>::insert(phase, block);
            }
            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        fn get_phase(block_height: T::BlockNumber) -> (GeodeSessionPhase, bool) {
            let phases = GeodeSessionPhase::all();
            let mut blocks = BTreeMap::new();
            let mut total_block = 0u32.into();
            for key in &phases {
                let block_of_phase = <PhaseBlock<T>>::get(key);
                total_block += block_of_phase;
                blocks.insert(key.clone(), block_of_phase);
            }
            let mut block_height = block_height % total_block;
            let is_new_session = block_height == 0u32.into();

            for phase in &phases {
                let block_of_phase = blocks.get(phase).unwrap().clone();
                if block_height < block_of_phase {
                    return (phase.clone(), is_new_session);
                }
                block_height -= block_of_phase;
            }
            unreachable!();
        }
    }
}
