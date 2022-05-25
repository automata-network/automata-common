#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use automata_traits::geode::GeodeTrait;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_runtime::print;

    #[pallet::event]
    pub enum Event<T: Config> {}

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type GeodeHandler: GeodeTrait<
            AccountId = <Self as frame_system::Config>::AccountId,
            Hash = <Self as frame_system::Config>::Hash,
        >;
    }

    #[pallet::error]
    pub enum Error<T> {}

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
            T::GeodeHandler::on_expired_check();
            log::info!("on initialize");
            print("hello");
            0
        }
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::call]
    impl<T: Config> Pallet<T> {}
}
