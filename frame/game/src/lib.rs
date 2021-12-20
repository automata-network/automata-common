#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

#[frame_support::pallet]
pub mod pallet {
    pub use crate::weights::WeightInfo;
    use frame_support::{ensure, pallet_prelude::*, traits::Get};
    use frame_system::pallet_prelude::*;
    use sp_std::prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        #[pallet::constant]
        type MaximumAttackCount: Get<u32>;

        #[pallet::constant]
        type MaximumAttackerNum: Get<u32>;

        type WeightInfo: WeightInfo;
    }

    #[pallet::storage]
    #[pallet::getter(fn attack_count)]
    pub type AttackCount<T> = StorageValue<_, u32, ValueQuery>;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId")]
    pub enum Event<T: Config> {
        AttackBoss(Vec<T::AccountId>),
    }

    #[pallet::error]
    pub enum Error<T> {
        BossDied,
        AttackNumExceed,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(
            T::WeightInfo::attack(participants.len())
        )]
        pub fn attack(origin: OriginFor<T>, participants: Vec<T::AccountId>) -> DispatchResult {
            ensure_root(origin.clone())?;
            ensure!(
                participants.len() <= T::MaximumAttackerNum::get() as usize,
                Error::<T>::AttackNumExceed
            );
            //Do we need a switch which will control the start of the game?

            let current_attack_count = AttackCount::<T>::get();
            ensure!(
                current_attack_count < T::MaximumAttackCount::get(),
                Error::<T>::BossDied
            );

            AttackCount::<T>::set(current_attack_count.saturating_add(1));
            Self::deposit_event(Event::<T>::AttackBoss(participants));

            Ok(())
        }
    }
}
