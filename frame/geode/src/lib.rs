#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{ensure, pallet_prelude::{StorageMap, OptionQuery}, Blake2_128Concat};
    use std::collections::BTreeMap;

    use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::{StorageMap, ValueQuery}, Blake2_128Concat};
    use frame_system::{pallet_prelude::OriginFor, ensure_signed};
    use sp_std::{collections::btree_map::BTreeMap};

    #[cfg(feature = "std")]
    use serde::{Deserialize, Serialize};


    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, Default)]
    pub enum HealthyState {
        /// Less than required attestors approved that the geode is healthy
        Unhealthy,
        /// More than required attestors approved that the geode is healthy
        Healthy,
    }

    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, Default)]
    pub enum WorkingState {
        /// The geode is not on service and ready to be dispatched an order
        Idle,
        /// The geode has been dispatched an order and is doing the preparation work
        Pending(session_index),
        /// The geode is on service now
        Working(block_height),
        /// The geode is on the progress of finishing the service, maybe doing something like backup the data, uninstall the binary...
        Finalizing(session_index),
        /// The geode is trying to exit, we should not dispatch orders to it
        Exting,
        /// The geode has exited successfully, and it can shutdown at any time
        Exited(block_height),
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

    pub type GeodeOf<T> = Geode<<T as frame_system::Config>::AccountId, <T as frame_system::Config>::Hash>;

    #[pallet::storage]
    #[pallet::getter(fn geodes)]
    pub type Geodes<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, GeodeOf<T>, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn offline_requests)]
    pub type OfflineRequests<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, T::AccountId, OptionQuery>;
    
    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::event]
    pub enum Event<T: Config> {
    }

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
            let who = ensure_signed(origin);

            if Geode<T>::contains_key(&geode.id) {
                // If there is already a geode record, should check its working state
                // Only `Exited` state geode is allowed to register again
                let mut existing_geode = Geodes<T>::get(&geode.id);
                ensure!(
                    existing_geode.provider_id == who, 
                    Error::<T>::DuplicateGeodeId
                );
                if let WorkingState::Exited(x) = existing_geode.working_state {
                    existing_geode.working_state = WorkingState::Idle;
                    Geodes<T>::insert(geode.id, existing_geode);
                } else {
                    Err(Error::<T>::StateNotExited)
                }
            } else {
                // Register a new geode instance
                let mut geode_record = geode.clone();
                geode_record.working_state = WorkingState::Idle;
                geode_record.healthy_state = HealthyState::Unhealthy;
                geode_record.order_id = None;
                Geodes<T>::insert(geode_record.id, geode_record);
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
            let who = ensure_signed(origin);

            if Geode<T>::contains_key(&geode_id) {
                // Just need to record the offline request which will be processed during the offline phase
                let geode = Geodes<T>::get(&geode_id);
                ensure!(
                    geode.provider_id == who,
                    Error::<T>::NotOwner
                );
                OfflineRequests<T>::insert(who, geode_id);
            } else {
                // The geode instance does not exist
                Err(Error::<T>::NonexistentGeode)
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
            let who = ensure_signed(origin);

            if Geode<T>::contains_key(&geode_id) {
                let mut geode = Geodes<T>::get(&geode_id);
                ensure!(
                    geode.provider_id == who,
                    Error::<T>::NotOwner
                );
                geode.props.insert(prop_name, prop_value);
                Geodes<T>::insert(geode_id, geode);
            } else {
                // The geode instance does not exist
                Err(Error::<T>::NonexistentGeode)
            }
        }

        /// Update dns of the geode.
        #[pallet::weight(0)]
        pub fn update_geode_dns(
            origin: OriginFor<T>,
            geode_id: T::AccountId,
            dns: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin);

            if Geode<T>::contains_key(&geode_id) {
                let mut geode = Geodes<T>::get(&geode_id);
                ensure!(
                    geode.provider_id == who,
                    Error::<T>::NotOwner
                );
                geode.dns = dns;
                Geodes<T>::insert(geode_id, geode);
            } else {
                // The geode instance does not exist
                Err(Error::<T>::NonexistentGeode)
            }
        }
        
        /// Called when geode finish the data loading, binary loading and etc.
        /// And is ready to process the order.
        #[pallet::weight(0)]
        pub fn geode_ready(
            origin: OriginFor<T>,
            geode_id: T::AccountId,
            order_id: T::Hash,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin);

            if Geode<T>::contains_key(&geode_id) {
                let mut geode = Geodes<T>::get(&geode_id);
                ensure!(
                    geode.provider_id == who,
                    Error::<T>::NotOwner
                );
                if let WorkingState::Pending(x) = geode.working_state {
                    let block_number = <frame_system::Pallet<T>>::block_number();
                    geode.working_state = WorkingState::Working(block_number);
                    Geodes<T>::insert(geode_id, geode);
                } else {
                    Err(Error::<T>::NotPendingState)
                }
                Geodes<T>::insert(geode_id, geode);
            } else {
                // The geode instance does not exist
                Err(Error::<T>::NonexistentGeode)
            }
        }

        /// Called when geode failed to initialize(load data, load binary...).
        #[pallet::weight(0)]
        pub fn geode_initialize_failed(
            origin: OriginFor<T>,
            geode_id: T::AccountId,
            order_id: T::Hash,
        ) -> DispatchResultWithPostInfo {

        }

        /// Called when geode finish the finalization.
        #[pallet::weight(0)]
        pub fn geode_finalized(
            origin: OriginFor<T>,
            geode_id: T::AccountId,
            order_id: T::Hash,
        ) -> DispatchResultWithPostInfo {

        }

        /// Called when geode failed to finalize.
        #[pallet::weight(0)]
        pub fn geode_finalize_failed(
            origin: OriginFor<T>,
            geode_id: T::AccountId,
            order_id: T::Hash,
        ) -> DispatchResultWithPostInfo {

        }
    }
}