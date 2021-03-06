#![cfg(feature = "runtime-benchmarks")]

use super::Pallet as Game;
use super::*;
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller, Vec};
use frame_support::{ensure, pallet_prelude::*, traits::Get};
use frame_system::RawOrigin;

benchmarks! {
    attack {
        let l in 0 .. T::MaximumAttackerNum::get();

        let mut participants = Vec::<T::AccountId>::new();
        let mut i: u64 = 0;
        while i < l.into() {
            let participant: T::AccountId = account("caller", 0, 0);
            participants.push(participant);
            i = i + 1;
        }
    }: attack(RawOrigin::Root, participants)
    verify {
        // frame_system::Pallet::<T>::assert_last_event(<T as pallet::Config>::Event::AttackBoss(participates).into());
    }
}

impl_benchmark_test_suite!(
    Game,
    crate::mock::ExtBuilder::default().build(),
    crate::mock::Test,
);
