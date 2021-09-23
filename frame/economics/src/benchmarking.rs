#![cfg(feature = "runtime-benchmarks")]

use super::*;

use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;
use sp_runtime::traits::{Bounded, Saturating};
use super::Pallet as Economics;

use frame_system::{ensure_signed, pallet_prelude::*};
use frame_support::{
    ensure, pallet_prelude::*,
    traits::{
        Currency, WithdrawReasons, ExistenceRequirement, LockableCurrency
    },
};
use sp_std::{prelude::*};

pub type MaxLocksOf<T> =
		<<T as Config>::Currency as LockableCurrency<<T as frame_system::Config>::AccountId>>::MaxLocks;

fn add_locks<T: Config>(who: &T::AccountId, n: u8) {
    for id in 0..n {
        let lock_id = [id; 8];
        let locked = T::Currency::minimum_balance();
        let reasons = WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE;
        T::Currency::set_lock(lock_id, who, locked, reasons);
    }
}

benchmarks!{
    burn_no_lock {
        let caller = whitelisted_caller();
        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
        let amount = BalanceOf::<T>::max_value().saturating_sub(T::Currency::minimum_balance());
        assert_eq!(
            T::Currency::total_issuance(),
            BalanceOf::<T>::max_value(),
            "Total issuance wrong",
        );
    }: burn(RawOrigin::Signed(caller.clone()), amount)
    verify {
        assert_eq!(
            T::Currency::total_issuance(),
            BalanceOf::<T>::max_value().saturating_sub(amount),
            "Total issuance not changed",
        );
        assert_eq!(
            T::Currency::free_balance(&caller),
            BalanceOf::<T>::max_value().saturating_sub(amount),
            "Free balance not changed",
        );
    }

    burn_with_lock {
        let l in 0 .. MaxLocksOf::<T>::get();

        let caller = whitelisted_caller();
        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
        add_locks::<T>(&caller, l as u8);
        let amount = BalanceOf::<T>::max_value().saturating_sub(T::Currency::minimum_balance());
        assert_eq!(
            T::Currency::total_issuance(),
            BalanceOf::<T>::max_value(),
            "Total issuance wrong",
        );
    }: burn(RawOrigin::Signed(caller.clone()), amount)
    verify {
        assert_eq!(
            T::Currency::total_issuance(),
            BalanceOf::<T>::max_value().saturating_sub(amount),
            "Total issuance not changed",
        );
        assert_eq!(
            T::Currency::free_balance(&caller),
            BalanceOf::<T>::max_value().saturating_sub(amount),
            "Free balance not changed",
        );
    }
}

impl_benchmark_test_suite!(
    Economics,
    crate::mock::ExtBuilder::default().existential_deposit(100).build(),
    crate::mock::Test,
);