#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use automata_traits::geode::GeodeTrait;
    use core::convert::TryInto;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_std::vec::Vec;
    use sp_core::U256;

    #[pallet::event]
    pub enum Event<T: Config> {}

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::error]
    pub enum Error<T> {}

    pub type OrderOf<T> = Order<<T as frame_system::Config>::BlockNumber>;

    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, Default)]
    pub struct Order<BlockNumber> {
        pub binary: sp_std::vec::Vec<u8>,
        pub dns: Vec<u8>,
        pub name: Vec<u8>,
        // token num that users are willing to pay
        pub price: U256,
        pub start_session_id: BlockNumber,
        // session num
        pub duration: BlockNumber,
        pub geode_num: u32,
        pub state: OrderState,
        // price - refund_unit / duration * geode_num * price
        pub refund_unit: u32,
    }

    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
    pub enum OrderState {
        Submitted,
        Pending,
        Processing,
        Emergency,
        Done,
    }

    impl Default for OrderState {
        fn default() -> Self {
            Self::Submitted
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_block_number: BlockNumberFor<T>) -> Weight {
            0
        }
    }

    impl<T: Config> automata_traits::order::OrderTrait for Pallet<T> {
        type Hash = T::Hash;
        type BlockNumber = T::BlockNumber;
        fn is_order_expired(
            _order_id: Self::Hash,
            _block_height: Self::BlockNumber,
            _session_index: Self::BlockNumber,
        ) -> bool {
            false
        }
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::call]
    impl<T: Config> Pallet<T> {}
}
