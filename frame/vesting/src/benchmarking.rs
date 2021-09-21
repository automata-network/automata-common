#[cfg(feature = "runtime-benchmarks")]
mod benchmarking {
    use crate::{*, Module as PalletModule};
    use frame_benchmarking::{benchmarks, account, impl_benchmark_test_suite, whitelisted_caller};
    use frame_system::RawOrigin;

    fn add_locks<T: Config>(who: &T::AccountId, n: u8) {
        for id in 0..n {
            let lock_id = [id; 8];
            let locked = 100u32;
            let reasons = WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE;
            T::Currency::set_lock(lock_id, who, locked.into(), reasons);
        }
    }

    fn add_vesting_plan<T: Config>(who: &T::AccountId) -> Result<(), &'static str> {
        Timestamp::<T>::set_timestamp(0);
        let plan = VestingPlan {
            start_time: 400u64,
            cliff_duration: 20u64,
            total_duration: 100u64,
            interval: 10u64,
            initial_amount: 20u32.into(),
            total_amount: 100u32.into(),
            vesting_during_cliff: false,
        };
    
        // Add schedule to avoid `NotVesting` error.
        Vesting::<T>::VestingPlans::insert(&who, plan)?;
        Ok(())
    }

    benchmarks!{
        unlock_locked {
            let l in 0 .. MaxLocksOf::<T>::get();

            let caller = whitelisted_caller();
            T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
            add_locks::<T>(&caller, l as u8);
            add_vesting_plan::<T>(&caller)?;
            // At time zero, everything is vested.
            Timestamp::<T>::set_timestamp(0);
            assert_eq!(
                Vesting::<T>::vesting_balance(&caller),
                Some(100u32.into()),
                "Vesting plan not added",
            );
        }: unlock(RawOrigin::Signed(caller.clone()))
        verify {
            // Nothing happened since everything is still vested.
            assert_eq!(
                Vesting::<T>::vesting_balance(&caller),
                Some(100u32.into()),
                "Vesting plan was removed",
            );
        }

        unlock_partial_unlocked {
            let l in 0 .. MaxLocksOf::<T>::get();

            let caller = whitelisted_caller();
            T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
            add_locks::<T>(&caller, l as u8);
            add_vesting_plan::<T>(&caller)?;

            // This is the worst case in partial unlocked
            Timestamp::<T>::set_timestamp(490);
            assert_eq!(
                Vesting::<T>::vesting_balance(&caller),
                Some(10u32.into()),
                "Vesting amount incorrect",
            );
        }: unlock(RawOrigin::Signed(caller.clone()))
        verify {
            // Vesting schedule is removed!
            assert_eq!(
                Vesting::<T>::vesting_balance(&caller),
                Some(10u32.into()),
                "Vesting amount incorrect",
            );
        }

        unlock_complete_unlocked {
            let l in 0 .. MaxLocksOf::<T>::get();

            let caller = whitelisted_caller();
            T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
            add_locks::<T>(&caller, l as u8);
            add_vesting_plan::<T>(&caller)?;

            // This is the worst case in partial unlocked
            Timestamp::<T>::set_timestamp(500);
            assert_eq!(
                Vesting::<T>::vesting_balance(&caller),
                Some(BalanceOf::<T>::zero()),
                "Vesting schedule still active",
            );
        }: unlock(RawOrigin::Signed(caller.clone()))
        verify {
            // Vesting schedule is removed!
            assert_eq!(
                Vesting::<T>::vesting_balance(&caller),
                None,
                "Vesting schedule was not removed",
            );
        }

        vested_transfer {
            let l in 0 .. MaxLocksOf::<T>::get();

            let caller: T::AccountId = whitelisted_caller();
            T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
            let target: T::AccountId = account("target", 0, SEED);
            let target_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(target.clone());
            // Give target existing locks
            add_locks::<T>(&target, l as u8);

            let transfer_amount = T::MinVestedTransfer::get();

            let plan = VestingPlan {
                start_time: 400,
                cliff_duration: 20,
                total_duration: 10,
                interval: 0,
                initial_amount: 256 * 3,
                total_amount: 256 * 2,
                vesting_during_cliff: false,
            };
        }: _(RawOrigin::Signed(caller), target_lookup, plan)
        verify {
            assert_eq!(
                T::MinVestedTransfer::get(),
                T::Currency::free_balance(&target),
                "Transfer didn't happen",
            );
            assert_eq!(
                Vesting::<T>::vesting_balance(&target),
                Some(T::MinVestedTransfer::get()),
                "Lock not created",
            );
        }
    }
}

impl_benchmark_test_suite!(
	Vesting,
	crate::mock::ExtBuilder::default().existential_deposit(256).build(),
	crate::mock::Test,
);