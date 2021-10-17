#![cfg(feature = "runtime-benchmarks")]

use super::*;

use super::Pallet as Economics;
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;
use sp_runtime::traits::{Bounded, Saturating};

use frame_support::{
    ensure,
    pallet_prelude::*,
    traits::{Currency, ExistenceRequirement, LockableCurrency, WithdrawReasons},
};
use frame_system::{ensure_signed, pallet_prelude::*};
use sp_std::prelude::*;

benchmarks! {
    burn {
        let caller = whitelisted_caller();
        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
        let amount = BalanceOf::<T>::max_value().saturating_sub(T::Currency::minimum_balance());
        assert_eq!(
            T::Currency::total_issuance(),
            BalanceOf::<T>::max_value(),
            "Total issuance wrong",
        );
    }: burn(RawOrigin::Signed(caller.clone()), amount.into())
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
    crate::mock::ExtBuilder::default()
        .existential_deposit(1000)
        .build(),
    crate::mock::Test,
);
