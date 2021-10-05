#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

pub trait WeightInfo {
    fn unlock_locked(l: u32, ) -> Weight;
    fn unlock_partial_unlocked(l: u32, ) -> Weight;
    fn unlock_complete_unlocked(l: u32, ) -> Weight;
    fn vested_transfer(l: u32, ) -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    // Storage: Vesting VestingPlans (r:1 w:0)
    // Storage: Timestamp Now (r:1 w:0)
    // Storage: Balances Locks (r:1 w:1)
    fn unlock_locked(l: u32, ) -> Weight {
        (49_650_000 as Weight)
            // Standard Error: 18_000
            .saturating_add((209_000 as Weight).saturating_mul(l as Weight))
            .saturating_add(T::DbWeight::get().reads(3 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    // Storage: Vesting VestingPlans (r:1 w:0)
    // Storage: Timestamp Now (r:1 w:0)
    // Storage: Balances Locks (r:1 w:1)
    fn unlock_partial_unlocked(l: u32, ) -> Weight {
        (49_081_000 as Weight)
            // Standard Error: 18_000
            .saturating_add((199_000 as Weight).saturating_mul(l as Weight))
            .saturating_add(T::DbWeight::get().reads(3 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    // Storage: Vesting VestingPlans (r:1 w:1)
    // Storage: Timestamp Now (r:1 w:0)
    // Storage: Balances Locks (r:1 w:1)
    fn unlock_complete_unlocked(l: u32, ) -> Weight {
        (51_261_000 as Weight)
            // Standard Error: 5_000
            .saturating_add((151_000 as Weight).saturating_mul(l as Weight))
            .saturating_add(T::DbWeight::get().reads(3 as Weight))
            .saturating_add(T::DbWeight::get().writes(2 as Weight))
    }
    // Storage: Vesting VestingPlans (r:1 w:1)
    // Storage: System Account (r:1 w:1)
    // Storage: Timestamp Now (r:1 w:0)
    // Storage: Balances Locks (r:1 w:1)
    fn vested_transfer(l: u32, ) -> Weight {
        (109_050_000 as Weight)
            // Standard Error: 18_000
            .saturating_add((169_000 as Weight).saturating_mul(l as Weight))
            .saturating_add(T::DbWeight::get().reads(4 as Weight))
            .saturating_add(T::DbWeight::get().writes(3 as Weight))
    }
}