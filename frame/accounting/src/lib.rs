#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

// #[cfg(test)]
// mod mock;

// #[cfg(test)]
// mod tests;

#[frame_support::pallet]
pub mod pallet {
    use automata_traits::{AttestorAccounting, GeodeAccounting};
    use core::convert::TryInto;
    use frame_support::traits::{Currency, ReservableCurrency};
    use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
    use frame_system::pallet_prelude::*;
    use pallet_geode::GeodeOf;
    use sp_std::collections::{btree_map::BTreeMap, btree_set::BTreeSet};
    use sp_std::prelude::*;

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// The currency in which fees are paid and contract balances are held.
        type Currency: ReservableCurrency<Self::AccountId> + Currency<Self::AccountId>;

        type GetAttestors: Get<BTreeMap<Self::AccountId, usize>>;
        type GetGeodes: Get<Vec<Self::AccountId>>;

        type AttestorStakingAmount: Get<BalanceOf<Self>>;
        type GeodeStakingAmount: Get<BalanceOf<Self>>;
        type AttestorTotalReward: Get<BalanceOf<Self>>;
        type GeodeTotalReward: Get<BalanceOf<Self>>;

        type GeodeTerminatePenalty: Get<BalanceOf<Self>>;
        type GeodeMisconductForAttestor: Get<BalanceOf<Self>>;
        type GeodeMisconductForServiceUser: Get<BalanceOf<Self>>;

        type SlotLength: Get<Self::BlockNumber>;

        type AttestorBasicRewardRatio: Get<u8>;
        type CommissionRateForService: Get<u8>;
        type CommissionRateForOnDemand: Get<u8>;

        type AttestorRewardEachSlot: Get<BalanceOf<Self>>;
        type GeodeRewardEachSlot: Get<BalanceOf<Self>>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn total_attestor_distributed_reward)]
    pub type TotalAttestorDistributedReward<T: Config> = StorageValue<_, BalanceOf<T>>;

    #[pallet::storage]
    #[pallet::getter(fn total_geode_distributed_reward)]
    pub type TotalGeodeDistributedReward<T: Config> = StorageValue<_, BalanceOf<T>>;

    #[pallet::event]
    #[pallet::metadata(T::BlockNumber = "BlockNumber")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Attestor rewarded. \[attestor_id\]
        AttestorRewarded(T::BlockNumber),
        /// Geode rewarded. \[geode_id\]
        GeodeRewarded(T::BlockNumber),
        /// No reward left for attestor
        AttestorRewardRanOut(),
        /// No reward left for geode
        GeodeRewardRanOut(),
    }

    #[pallet::error]
    pub enum Error<T> {
        InvalidAttestor,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(block_number: T::BlockNumber) -> Weight {
            if let Some(value) = Self::total_attestor_distributed_reward() {
                if value >= T::AttestorTotalReward::get() {
                    Self::deposit_event(Event::AttestorRewardRanOut());
                    return 0;
                }
            }

            if let Some(value) = Self::total_geode_distributed_reward() {
                if value >= T::GeodeTotalReward::get() {
                    Self::deposit_event(Event::GeodeRewardRanOut());
                    return 0;
                }
            }

            let slot_length = T::SlotLength::get();
            let index_in_slot = block_number % slot_length;

            /// Reward at the begin of each slot
            if index_in_slot == T::BlockNumber::from(0_u32) {
                Self::reward_attestors();
                Self::reward_geodes();
                Self::deposit_event(Event::AttestorRewarded(block_number));
                Self::deposit_event(Event::GeodeRewarded(block_number));
            }

            10000
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(0)]
        pub fn attestor_register(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Reward attestors
        pub fn reward_attestors() {
            return;
            /// Get all attestors and its verified geodes number
            let attestors = T::GetAttestors::get();

            let attestors_length = attestors.len();
            let geodes_length: usize = attestors.iter().map(|(_, geodes)| geodes).sum();
            let reward_each_slot = T::AttestorRewardEachSlot::get();

            /// Compute basic reward and commission reward
            let basic_reward = reward_each_slot
                * BalanceOf::<T>::from(T::AttestorBasicRewardRatio::get())
                / BalanceOf::<T>::from(100_u32);
            let basic_reward_per_attestor =
                basic_reward / BalanceOf::<T>::from(attestors_length as u32);
            let commission_reward = reward_each_slot - basic_reward;
            let commission_reward_per_geode =
                commission_reward / BalanceOf::<T>::from(geodes_length as u32);

            /// Reward each attestor
            attestors.iter().map(|(accountId, geodes)| {
                let reward = basic_reward_per_attestor
                    + commission_reward_per_geode * BalanceOf::<T>::from(*geodes as u32);
                <T as Config>::Currency::deposit_into_existing(accountId, reward);
            });

            match Self::total_attestor_distributed_reward() {
                Some(value) => TotalAttestorDistributedReward::<T>::put(value + reward_each_slot),
                None => TotalAttestorDistributedReward::<T>::put(reward_each_slot),
            }
        }

        pub fn reward_geodes() {
            return;
            let geodes = T::GetGeodes::get();

            let geodes_len = geodes.len();
            let reward_each_slot = T::GeodeRewardEachSlot::get();
            let reward = reward_each_slot / BalanceOf::<T>::from(geodes_len as u32);

            geodes.iter().map(|geode| {
                <T as Config>::Currency::deposit_into_existing(&geode, reward);
            });

            match Self::total_geode_distributed_reward() {
                Some(value) => TotalGeodeDistributedReward::<T>::put(value + reward_each_slot),
                None => TotalGeodeDistributedReward::<T>::put(reward_each_slot),
            }
        }
    }

    impl<T: Config> AttestorAccounting for Pallet<T> {
        type AccountId = <T as frame_system::Config>::AccountId;
        fn attestor_staking(who: T::AccountId) -> DispatchResultWithPostInfo {
            <T as Config>::Currency::reserve(&who, T::AttestorStakingAmount::get())?;
            Ok(().into())
        }

        fn attestor_unreserve(who: T::AccountId) -> DispatchResultWithPostInfo {
            <T as Config>::Currency::unreserve(&who, T::AttestorStakingAmount::get());
            Ok(().into())
        }
    }

    impl<T: Config> GeodeAccounting for Pallet<T> {
        type AccountId = <T as frame_system::Config>::AccountId;
        fn geode_staking(who: T::AccountId) -> DispatchResultWithPostInfo {
            <T as Config>::Currency::reserve(&who, T::GeodeStakingAmount::get())?;
            Ok(().into())
        }

        fn geode_unreserve(who: T::AccountId) -> DispatchResultWithPostInfo {
            <T as Config>::Currency::unreserve(&who, T::GeodeStakingAmount::get());
            Ok(().into())
        }
    }
}
