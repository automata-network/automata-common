#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use frame_system::{ensure_signed, pallet_prelude::*};
	use frame_support::{
		ensure, pallet_prelude::*,
		traits::{
			Currency, ExistenceRequirement, Get, LockIdentifier, LockableCurrency,
			WithdrawReasons, UnixTime
		},
	};
	use sp_runtime::traits::{StaticLookup, AtLeast32BitUnsigned, SaturatedConversion, Zero, Convert};
	use codec::{Decode, Encode};
	use sp_std::{prelude::*};

	const VESTING_ID: LockIdentifier = *b"avesting";

	pub type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
	pub type MaxLocksOf<T> =
		<<T as Config>::Currency as LockableCurrency<<T as frame_system::Config>::AccountId>>::MaxLocks;

	#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, RuntimeDebug)]
	pub struct VestingPlan<Balance> {
		/// Starting time for unlocking(vesting).
		pub start_time: u64,
		/// Duration of cliff, not allowed to withdraw
		pub cliff_duration: u64,
		/// Total duration of this vesting plan
		pub total_duration: u64,
		/// Vesting interval
		pub interval: u64,
		/// Amount of tokens which will be released at startTime
		pub initial_amount: Balance,
		/// Total locked amount, including the initial_amount
		pub total_amount: Balance,
		/// True if vesting amount is accumulated during cliff duration
		pub vesting_during_cliff: bool,
	}
	
	impl<Balance: AtLeast32BitUnsigned + Copy>
		VestingPlan<Balance>
	{
		/// Amount locked at block `n`.
		pub fn locked_at<U64ToBalance: Convert<u64, Balance>>(
			&self,
			current_time_millis: u64,
		) -> Balance {
			// Before vesting start time, all tokens are locked
			if current_time_millis < self.start_time {
				self.total_amount
			// Before cliff duration ended, only initial amount tokens are not locked
			} else if current_time_millis < self.start_time + self.cliff_duration {
				self.total_amount.saturating_sub(self.initial_amount)
			// After total duration ended, all tokens are not locked
			} else if current_time_millis >= self.start_time + self.total_duration {
				Zero::zero()
			} else {
				let vesting_start_time = if self.vesting_during_cliff {
					self.start_time
				} else {
					self.start_time + self.cliff_duration
				};

				let vesting_duration = if self.vesting_during_cliff {
					self.total_duration
				} else {
					self.total_duration - self.cliff_duration
				};
	
				let total_interval_counts = if vesting_duration % self.interval == 0 {
					vesting_duration / self.interval
				} else {
					vesting_duration / self.interval + 1
				};
				
				let unlocked_amount = self.total_amount.saturating_sub(self.initial_amount)
					.saturating_mul(U64ToBalance::convert(current_time_millis.saturating_sub(vesting_start_time) / self.interval)) / U64ToBalance::convert(total_interval_counts);
				self.total_amount.saturating_sub(self.initial_amount).saturating_sub(unlocked_amount)
			}
		}
	}

    #[pallet::config]
	pub trait Config: frame_system::Config {
        /// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The currency trait.
		type Currency: LockableCurrency<Self::AccountId>;

        /// Time used for calculating vesting
		type UnixTime: UnixTime;

        #[pallet::constant]
		type MinVestedTransfer: Get<BalanceOf<Self>>;

		type U64ToBalance: Convert<u64, BalanceOf<Self>>;
    }

	#[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
	#[pallet::getter(fn vesting_plans)]
	pub type VestingPlans<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, VestingPlan<BalanceOf<T>>>;

    #[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub vesting: Vec<(T::AccountId, u64, u64, u64, u64, BalanceOf<T>, BalanceOf<T>, bool)>,
	}

    #[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { vesting: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			for &(ref who, start_time, cliff_duration, total_duration, interval, initial_amount, total_amount, vesting_during_cliff) in self.vesting.iter() {
				let balance = T::Currency::free_balance(who);
				assert!(balance >= total_amount, "Currencies must be init'd before vesting");

				VestingPlans::<T>::insert(who, VestingPlan { start_time, cliff_duration, total_duration, interval, initial_amount, total_amount, vesting_during_cliff });
				let reasons = WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE;
				T::Currency::set_lock(VESTING_ID, who, total_amount, reasons);
			}
        }
    }

    #[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[pallet::metadata(T::AccountId = "AccountId", BalanceOf<T> = "Balance")]
	pub enum Event<T: Config> {
        VestingUpdated(T::AccountId, BalanceOf<T>),
		VestingCompleted(T::AccountId),
    }

    #[pallet::error]
	pub enum Error<T> {
        ExistingVestingPlan,
        AmountLow,
		InvalidVestingPlan,
		NoVestingPlan,
    }

    #[pallet::call]
	impl<T: Config> Pallet<T> {
        /// Create a vested transfer
        #[pallet::weight(0)]
		pub fn vested_transfer(
            origin: OriginFor<T>,
			target: <T::Lookup as StaticLookup>::Source,
			plan: VestingPlan<BalanceOf<T>>,
        ) -> DispatchResult {
            let transactor = ensure_signed(origin)?;
			ensure!(plan.total_amount >= T::MinVestedTransfer::get(), Error::<T>::AmountLow);
			ensure!(plan.interval != 0, Error::<T>::InvalidVestingPlan);
			ensure!(plan.total_duration >= plan.cliff_duration, Error::<T>::InvalidVestingPlan);
			ensure!(plan.total_amount >= plan.initial_amount, Error::<T>::InvalidVestingPlan);

			let who = T::Lookup::lookup(target)?;
			ensure!(!VestingPlans::<T>::contains_key(&who), Error::<T>::ExistingVestingPlan);

			T::Currency::transfer(
				&transactor,
				&who,
				plan.total_amount,
				ExistenceRequirement::AllowDeath,
			)?;

			VestingPlans::<T>::insert(&who, plan);
			let res = Self::update_lock(who.clone());
			debug_assert!(res.is_ok());

			Ok(())
        }

        /// Unlock vested tokens of sender account
        #[pallet::weight(0)]
		pub fn unlock(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::update_lock(who)
        }
    }

	impl<T: Config> Pallet<T> {
		/// (Re)set or remove the pallet's currency lock on `who`'s account in accordance with their
		/// current unvested amount.
		fn update_lock(who: T::AccountId) -> DispatchResult {
			let vesting = Self::vesting_plans(&who).ok_or(Error::<T>::NoVestingPlan)?;
			let now = T::UnixTime::now().as_millis().saturated_into::<u64>();
			let locked_now = vesting.locked_at::<T::U64ToBalance>(now);
	
			if locked_now.is_zero() {
				T::Currency::remove_lock(VESTING_ID, &who);
				VestingPlans::<T>::remove(&who);
				Self::deposit_event(Event::<T>::VestingCompleted(who));
			} else {
				let reasons = WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE;
				T::Currency::set_lock(VESTING_ID, &who, locked_now, reasons);
				Self::deposit_event(Event::<T>::VestingUpdated(who, locked_now));
			}
			Ok(())
		}

		pub fn vesting_balance(who: &T::AccountId) -> Option<BalanceOf<T>> {
			if let Some(vesting) = Self::vesting_plans(who) {
				let now = T::UnixTime::now().as_millis().saturated_into::<u64>();
				let locked_now = vesting.locked_at::<T::U64ToBalance>(now);
				Some(T::Currency::free_balance(who).min(locked_now))
			} else {
				None
			}
		}

		pub fn add_vesting_plan(who: &T::AccountId, plan: VestingPlan<BalanceOf<T>>) -> DispatchResult {
			if VestingPlans::<T>::contains_key(&who) {
				Err(Error::<T>::ExistingVestingPlan)?
			}
			VestingPlans::<T>::insert(&who, plan);
			Ok(())
		}
	}
}