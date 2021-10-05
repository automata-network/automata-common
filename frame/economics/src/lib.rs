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
    use frame_support::{
        ensure,
        pallet_prelude::*,
        traits::{Currency, ExistenceRequirement, LockableCurrency, WithdrawReasons},
    };
    use frame_system::{ensure_signed, pallet_prelude::*};
    use sp_std::prelude::*;

    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The currency trait.
        type Currency: Currency<Self::AccountId> + LockableCurrency<Self::AccountId>;

        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::error]
    pub enum Error<T> {
        InsufficientLiquidity,
        KillAcount,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId", BalanceOf<T> = "Balance")]
    pub enum Event<T: Config> {
        EconomicsBurnt(T::AccountId, BalanceOf<T>),
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(T::WeightInfo::burn())]
        pub fn burn(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(
                T::Currency::free_balance(&who) - T::Currency::minimum_balance() >= amount,
                Error::<T>::KillAcount
            );
            // Locked balances are not allowed to burn
            T::Currency::ensure_can_withdraw(
                &who,
                amount,
                WithdrawReasons::TRANSFER,
                T::Currency::free_balance(&who) - amount,
            )
            .map_err(|_| Error::<T>::InsufficientLiquidity)?;

            let imbalance = T::Currency::burn(amount);
            if let Err(_e) = T::Currency::settle(
                &who,
                imbalance,
                WithdrawReasons::TRANSFER,
                ExistenceRequirement::KeepAlive,
            ) {
                // Will not fail because we have check before
            }

            Self::deposit_event(Event::<T>::EconomicsBurnt(who, amount));
            Ok(())
        }
    }
}
