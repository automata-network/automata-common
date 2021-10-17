#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

pub trait WeightInfo {
    fn burn() -> Weight;
}

/// Weight functions for pallet_economics.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn burn() -> Weight {
        (45_189_000 as Weight)
    }
}

impl WeightInfo for () {
    fn burn() -> Weight {
        (45_189_000 as Weight)
    }
}