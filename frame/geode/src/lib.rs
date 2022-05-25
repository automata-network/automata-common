#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        dispatch::DispatchResultWithPostInfo,
        ensure,
        pallet_prelude::{OptionQuery, StorageMap, ValueQuery},
        Blake2_128Concat,
    };
    use frame_support::{pallet_prelude::*, unsigned::ValidateUnsigned};
    use frame_system::{ensure_signed, pallet_prelude::OriginFor};
    use frame_system::{
        offchain::{SendTransactionTypes, SubmitTransaction},
        pallet_prelude::*,
    };
    use primitives::BlockNumber;
    use sp_runtime::{Percent, RuntimeDebug, SaturatedConversion};
    use sp_std::collections::btree_map::BTreeMap;
    use sp_std::prelude::*;

    #[cfg(feature = "std")]
    use serde::{Deserialize, Serialize};

    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
    pub enum HealthyState {
        /// Less than required attestors approved that the geode is healthy
        Unhealthy,
        /// More than required attestors approved that the geode is healthy
        Healthy,
    }

    impl Default for HealthyState {
        fn default() -> Self {
            Self::Unhealthy
        }
    }

    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
    pub enum WorkingState {
        /// The geode is not on service and ready to be dispatched an order
        Idle,
        /// The geode has been dispatched an order and is doing the preparation work
        Pending { session_index: u64 },
        /// The geode is on service now
        Working { block_height: BlockNumber },
        /// The geode is on the progress of finishing the service, maybe doing something like backup the data, uninstall the binary...
        Finalizing { session_index: u64 },
        /// The geode is trying to exit, we should not dispatch orders to it
        Exting,
        /// The geode has exited successfully, and it can shutdown at any time
        Exited { block_height: BlockNumber },
    }

    impl Default for WorkingState {
        fn default() -> Self {
            Self::Idle
        }
    }

    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, Default)]
    pub struct Geode<AccountId, Hash> {
        /// Id of geode, the key pair will be generate in enclave
        pub id: AccountId,
        /// Account of the machain provider
        pub provider: AccountId,
        pub ip: Vec<u8>,
        pub dns: Vec<u8>,
        /// Extra properties
        pub props: BTreeMap<Vec<u8>, Vec<u8>>,
        pub healthy_state: HealthyState,
        pub working_state: WorkingState,
        /// Hash of the current order
        pub order_id: Option<Hash>,
    }

    pub type GeodeOf<T> =
        Geode<<T as frame_system::Config>::AccountId, <T as frame_system::Config>::Hash>;

    #[pallet::storage]
    #[pallet::getter(fn geodes)]
    pub type Geodes<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, GeodeOf<T>>;

    #[pallet::storage]
    #[pallet::getter(fn offline_requests)]
    pub type OfflineRequests<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, T::AccountId, OptionQuery>;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::event]
    pub enum Event<T: Config> {}

    #[pallet::error]
    pub enum Error<T> {
        // Another geode provider tried to register a geode whose id is the same with an registered geode
        DuplicateGeodeId,
        // A existing geode tried to register again, but the previous working state is not `Exited`.
        StateNotExited,
        // Someone try to remove an nonexistent geode
        NonexistentGeode,
        // The origin is not owner of this geode
        NotOwner,
        NotPendingState,
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Called when geode want to register itself on chain.
        #[pallet::weight(0)]
        pub fn register_geode(
            origin: OriginFor<T>,
            geode: GeodeOf<T>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            match <Geodes<T>>::get(&geode.id) {
                Some(mut existing_geode) => {
                    // If there is already a geode record, should check its working state
                    // Only `Exited` state geode is allowed to register again
                    ensure!(existing_geode.provider == who, Error::<T>::DuplicateGeodeId);
                    if let WorkingState::Exited { block_height: x } = existing_geode.working_state {
                        existing_geode.working_state = WorkingState::Idle;
                        <Geodes<T>>::insert(geode.id, existing_geode);
                    } else {
                        return Err(Error::<T>::StateNotExited.into());
                    }
                }
                None => {
                    // Register a new geode instance
                    let mut geode_record = geode.clone();
                    geode_record.working_state = WorkingState::Idle;
                    geode_record.healthy_state = HealthyState::Unhealthy;
                    geode_record.order_id = None;
                    <Geodes<T>>::insert(geode_record.id.clone(), geode_record);
                }
            }

            Ok(().into())
        }

        /// Called when geode want to remove itself.
        /// Once the function is called and its working state is `Idle`, the state will be changed to `Exiting` during the offline phase of next session.
        /// If the current working state is not `Idle`, need to wait until it changes to `Idle`.
        #[pallet::weight(0)]
        pub fn remove_geode(
            origin: OriginFor<T>,
            geode_id: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            match <Geodes<T>>::get(&geode_id) {
                Some(geode) => {
                    ensure!(geode.provider == who, Error::<T>::NotOwner);
                    <OfflineRequests<T>>::insert(who, geode_id);
                }
                None => return Err(Error::<T>::NonexistentGeode.into()),
            }

            Ok(().into())
        }

        /// Update a property of the geode.
        #[pallet::weight(0)]
        pub fn update_geode_props(
            origin: OriginFor<T>,
            geode_id: T::AccountId,
            prop_name: Vec<u8>,
            prop_value: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            match <Geodes<T>>::get(&geode_id) {
                Some(mut geode) => {
                    ensure!(geode.provider == who, Error::<T>::NotOwner);
                    geode.props.insert(prop_name, prop_value);
                    <Geodes<T>>::insert(geode_id, geode);
                }
                None => {
                    // The geode instance does not exist
                    return Err(Error::<T>::NonexistentGeode.into());
                }
            }
            Ok(().into())
        }

        /// Update dns of the geode.
        #[pallet::weight(0)]
        pub fn update_geode_dns(
            origin: OriginFor<T>,
            geode_id: T::AccountId,
            dns: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            match <Geodes<T>>::get(&geode_id) {
                Some(mut geode) => {
                    ensure!(geode.provider == who, Error::<T>::NotOwner);
                    geode.dns = dns;
                    <Geodes<T>>::insert(geode_id, geode);
                }
                None => return Err(Error::<T>::NonexistentGeode.into()),
            }
            Ok(().into())
        }

        /// Called when geode finish the data loading, binary loading and etc.
        /// And is ready to process the order.
        #[pallet::weight(0)]
        pub fn geode_ready(
            origin: OriginFor<T>,
            geode_id: T::AccountId,
            order_id: T::Hash,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            match <Geodes<T>>::get(&geode_id) {
                Some(mut geode) => {
                    ensure!(geode.provider == who, Error::<T>::NotOwner);
                    if let WorkingState::Pending { session_index: x } = geode.working_state {
                        let block_number = <frame_system::Pallet<T>>::block_number()
                            .saturated_into::<BlockNumber>();
                        geode.working_state = WorkingState::Working {
                            block_height: block_number,
                        };
                        <Geodes<T>>::insert(geode_id, geode);
                    } else {
                        return Err(Error::<T>::NotPendingState.into());
                    }
                }
                None => {
                    // The geode instance does not exist
                    return Err(Error::<T>::NonexistentGeode.into());
                }
            }
            Ok(().into())
        }

        /// Called when geode failed to initialize(load data, load binary...).
        #[pallet::weight(0)]
        pub fn geode_initialize_failed(
            origin: OriginFor<T>,
            geode_id: T::AccountId,
            order_id: T::Hash,
        ) -> DispatchResultWithPostInfo {
            Ok(().into())
        }

        /// Called when geode finish the finalization.
        #[pallet::weight(0)]
        pub fn geode_finalized(
            origin: OriginFor<T>,
            geode_id: T::AccountId,
            order_id: T::Hash,
        ) -> DispatchResultWithPostInfo {
            Ok(().into())
        }

        /// Called when geode failed to finalize.
        #[pallet::weight(0)]
        pub fn geode_finalize_failed(
            origin: OriginFor<T>,
            geode_id: T::AccountId,
            order_id: T::Hash,
        ) -> DispatchResultWithPostInfo {
            Ok(().into())
        }
    }

    impl<T: Config> automata_traits::geode::GeodeTrait for Pallet<T> {
        type AccountId = T::AccountId;
        type Hash = T::Hash;
        fn on_new_session(session_index: u32) -> Result<(), DispatchError> {
            todo!()
        }

        fn on_geode_offline(session_index: u32) -> Result<(), DispatchError> {
            todo!()
        }

        fn on_geode_unhealthy(geode_id: T::AccountId) -> Result<(), DispatchError> {
            todo!()
        }

        fn on_order_dispatched(geode_id: T::AccountId, order_id: T::Hash) -> Result<(), DispatchError> {
            todo!()
        }

        fn on_expired_check() {
            log::info!("on_expired_check");
        }
    }

    impl<T: Config> automata_traits::attestor::ApplicationTrait for Pallet<T> {
        type AccountId = T::AccountId;
        /// Currently we will only report a simple `unhealthy` state, but there might be more status in the future.
        /// E.g maybe something wrong with the application binary
        fn application_unhealthy(geode_id: Self::AccountId) -> DispatchResult {
            Ok(().into())
        }

        /// Application are attested by several attestors, and reach healthy state
        fn application_healthy(geode_id: Self::AccountId) -> DispatchResult {
            Ok(().into())
        }
    }
}
