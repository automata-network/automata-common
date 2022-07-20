#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use automata_traits::geode::GeodeTrait;
    use core::convert::TryInto;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        NewSessionId(u32),
    }

    #[pallet::storage]
    #[pallet::getter(fn session_id)]
    pub type SessionId<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type GeodeHandler: GeodeTrait<
            AccountId = <Self as frame_system::Config>::AccountId,
            Hash = <Self as frame_system::Config>::Hash,
            BlockNumber = <Self as frame_system::Config>::BlockNumber,
        >;

        #[pallet::constant]
        type Blocks: Get<Self::BlockNumber>;
    }

    #[pallet::error]
    pub enum Error<T> {}

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(block_height: BlockNumberFor<T>) -> Weight {
            let mut session_index = <SessionId<T>>::get();
            if block_height % T::Blocks::get() == 0u32.into() {
                session_index += 1u32.into();
                <SessionId<T>>::put(session_index);
                T::GeodeHandler::on_new_session(block_height, session_index);

                T::GeodeHandler::on_geode_offline(session_index);
                T::GeodeHandler::on_expired_check(block_height, session_index);
                Self::deposit_event(<Event<T>>::NewSessionId(session_index.try_into().unwrap_or_default()));
            }
            0
        }
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::call]
    impl<T: Config> Pallet<T> {}
}
