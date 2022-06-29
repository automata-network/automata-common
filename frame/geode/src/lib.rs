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
        Working {
            session_index: u64,
            block_number: BlockNumber,
        },
        /// The geode is on the progress of finishing the service
        /// maybe doing something like backup the data, uninstall the binary...
        Finalizing { session_index: u64 },
        /// The geode is trying to exit, we should not dispatch orders to it
        Exiting,
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
    pub struct FailReason {
        pub reason: Vec<u8>,
    }

    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, Default)]
    pub struct Geode<AccountId, Hash> {
        /// Id of geode, the key pair will be generate in enclave
        pub id: AccountId,
        /// Account of the machain provider
        pub provider: AccountId,
        pub ip: Vec<u8>,
        pub domain: Vec<u8>,
        /// Extra properties
        pub props: BTreeMap<Vec<u8>, Vec<u8>>,
        // mark by attestor
        pub healthy_state: HealthyState,
        // mark by geode session
        pub working_state: WorkingState,
        /// Hash of the current order, set on pending
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

    #[pallet::storage]
    #[pallet::getter(fn fail_requests)]
    pub type FailRequests<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, FailReason, OptionQuery>;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        RegisterGeode(T::AccountId),
        RemoveGeode(T::AccountId),
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
        GeodeNotHealthy,
        NotFinalizingState,
        NotPendingState,
        NotWorkingState,
        OrderIdNotMatch,
        // invalid state
        InvalidState,
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Called when geode want to register itself on chain.
        /// working: not_exist | exited
        /// healthy: unlimited
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
                    if let WorkingState::Exited { .. } = existing_geode.working_state {
                        existing_geode.working_state = WorkingState::Idle;
                        <Geodes<T>>::insert(geode.id.clone(), existing_geode);
                    } else {
                        return Err(Error::<T>::StateNotExited.into());
                    }
                }
                None => {
                    // Register a new geode instance
                    let mut geode_record = geode.clone();
                    geode_record.working_state = WorkingState::Idle;
                    geode_record.healthy_state = HealthyState::Unhealthy;
                    geode_record.provider = who;
                    geode_record.order_id = None;
                    <Geodes<T>>::insert(geode_record.id.clone(), geode_record);
                }
            }
            Self::deposit_event(Event::RegisterGeode(geode.id));
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
            let _ = Self::get_geode_and_check_owner(&who, &geode_id)?;
            <OfflineRequests<T>>::insert(geode_id.clone(), geode_id.clone());
            Self::deposit_event(Event::RemoveGeode(geode_id));
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
            Self::mut_geode_fn(&who, &geode_id, |geode| {
                geode.props.insert(prop_name, prop_value);
                Ok(())
            })?;
            Ok(().into())
        }

        /// Update dns of the geode.
        #[pallet::weight(0)]
        pub fn update_geode_domain(
            origin: OriginFor<T>,
            geode_id: T::AccountId,
            domain: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::mut_geode_fn(&who, &geode_id, |g| {
                g.domain = domain;
                Ok(())
            })?;
            Ok(().into())
        }

        /// Called when geode finish the data loading, binary loading and etc.
        /// And is ready to process the order.
        /// states: Pending -> Working
        #[pallet::weight(0)]
        pub fn geode_ready(
            origin: OriginFor<T>,
            geode_id: T::AccountId,
            order_id: T::Hash,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::mut_geode_fn(&who, &geode_id.clone(), |geode| {
                match geode.working_state {
                    WorkingState::Pending { session_index } => {
                        ensure!(
                            geode.order_id == Some(order_id),
                            Error::<T>::OrderIdNotMatch
                        );
                        let block_number = <frame_system::Pallet<T>>::block_number()
                            .saturated_into::<BlockNumber>();
                        geode.working_state = WorkingState::Working {
                            session_index,
                            block_number,
                        };
                    }
                    _ => {
                        return Err(Error::<T>::NotPendingState.into());
                    }
                }
                Ok(())
            })?;
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn geode_finalizing_failed(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            Ok(().into())
        }

        /// Called when geode finish its order and working on the finalizing work
        #[pallet::weight(0)]
        pub fn geode_finalizing(
            origin: OriginFor<T>,
            geode_id: T::AccountId,
            order_id: T::Hash,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::mut_geode_fn(&who, &geode_id, |geode| {
                ensure!(
                    geode.order_id == Some(order_id),
                    Error::<T>::OrderIdNotMatch
                );
                if let WorkingState::Working { session_index, .. } = geode.working_state {
                    geode.working_state = WorkingState::Finalizing { session_index };
                } else {
                    return Err(Error::<T>::NotWorkingState.into());
                }
                Ok(())
            })?;
            Ok(().into())
        }

        /// Called when geode failed to initialize(load data, load binary...).
        #[pallet::weight(0)]
        pub fn geode_initialize_failed(
            origin: OriginFor<T>,
            geode_id: T::AccountId,
            order_id: T::Hash,
            reason: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let geode = Self::get_geode_and_check_owner(&who, &geode_id)?;
            ensure!(
                geode.order_id == Some(order_id),
                Error::<T>::OrderIdNotMatch
            );
            <FailRequests<T>>::insert(geode_id, FailReason { reason });
            Ok(().into())
        }

        /// Called when geode finish the finalization.
        /// state: finalizing -> idle
        /// healthy: any?
        #[pallet::weight(0)]
        pub fn geode_finalized(
            origin: OriginFor<T>,
            geode_id: T::AccountId,
            order_id: T::Hash,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::mut_geode_fn(&who, &geode_id.clone(), |geode| match geode.working_state {
                WorkingState::Finalizing { .. } => {
                    ensure!(
                        geode.order_id == Some(order_id),
                        Error::<T>::OrderIdNotMatch
                    );
                    geode.working_state = WorkingState::Idle;
                    Ok(())
                }
                _ => return Err(Error::<T>::NotFinalizingState.into()),
            })?;
            Ok(().into())
        }

        /// Called when geode failed to finalize.
        /// state: finalizing -> await idle
        /// healthy: ?
        #[pallet::weight(0)]
        pub fn geode_finalize_failed(
            origin: OriginFor<T>,
            geode_id: T::AccountId,
            order_id: T::Hash,
            reason: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let geode = Self::get_geode_and_check_owner(&who, &geode_id)?;
            ensure!(
                geode.order_id == Some(order_id),
                Error::<T>::OrderIdNotMatch
            );
            <FailRequests<T>>::insert(geode_id, FailReason { reason });
            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        fn get_all_geodes() -> Vec<T::AccountId> {
            <Geodes<T>>::iter()
                .map(|(account_id, _)| account_id)
                .collect::<Vec<T::AccountId>>()
        }

        fn set_healthy_state(geode_id: T::AccountId, state: HealthyState) -> Option<HealthyState> {
            match <Geodes<T>>::get(&geode_id) {
                Some(mut geode) => {
                    let old = Some(geode.healthy_state);
                    geode.healthy_state = state;
                    <Geodes<T>>::insert(geode_id, geode);
                    old
                }
                None => None,
            }
        }

        fn get_geode_and_check_owner(
            who: &T::AccountId,
            geode_id: &T::AccountId,
        ) -> Result<GeodeOf<T>, sp_runtime::DispatchError> {
            let geode = Self::get_geode(&geode_id)?;
            ensure!(geode.provider.eq(who), Error::<T>::NotOwner);
            Ok(geode)
        }

        fn mut_geode_fn<F>(who: &T::AccountId, geode_id: &T::AccountId, f: F) -> DispatchResult
        where
            F: FnOnce(&mut GeodeOf<T>) -> DispatchResult,
        {
            let mut geode = Self::get_geode_and_check_owner(who, geode_id)?;
            f(&mut geode)?;
            <Geodes<T>>::insert(geode_id.clone(), geode);
            Ok(())
        }

        fn get_geode(geode_id: &T::AccountId) -> Result<GeodeOf<T>, sp_runtime::DispatchError> {
            match <Geodes<T>>::get(&geode_id) {
                Some(geode) => Ok(geode),
                None => Err(Error::<T>::NonexistentGeode.into()),
            }
        }

        // only healthy and idle instance can receive the order
        fn receive_order(
            geode_id: T::AccountId,
            session_index: u64,
            order_id: T::Hash,
        ) -> DispatchResult {
            let mut geode = Self::get_geode(&geode_id)?;
            match geode.working_state {
                WorkingState::Idle => match geode.healthy_state {
                    HealthyState::Healthy => {
                        geode.working_state = WorkingState::Pending { session_index };
                        geode.order_id = Some(order_id);
                        <Geodes<T>>::insert(geode_id, geode);
                    }
                    HealthyState::Unhealthy => {
                        return Err(Error::<T>::GeodeNotHealthy.into());
                    }
                },
                _ => return Err(Error::<T>::NotPendingState.into()),
            };
            Ok(().into())
        }
    }

    impl<T: Config> automata_traits::geode::GeodeTrait for Pallet<T> {
        type AccountId = T::AccountId;
        type Hash = T::Hash;

        // Check the working geode, if it has finished the order, transist its working state to Finalizing.
        fn on_new_session(session_index: u32) -> Result<(), DispatchError> {
            for (geode_id, geode) in <Geodes<T>>::iter() {}
            todo!()
        }

        // TODO: handle geode fail request

        // it's safe to process offline request
        // for expected states(idle): transited and removed from request list
        // for working states(pending, working, finalizing): ignored
        // for current states(exiting, exited): removed from request list
        fn on_geode_offline(session_index: u32) -> DispatchResult {
            for (acc_id, geode_id) in <OfflineRequests<T>>::iter() {
                match <Geodes<T>>::get(&geode_id) {
                    Some(mut geode) => match geode.working_state {
                        WorkingState::Idle => {
                            geode.working_state = WorkingState::Exiting;
                            <Geodes<T>>::insert(geode_id, geode);
                            <OfflineRequests<T>>::remove(acc_id);
                        }
                        WorkingState::Pending { .. }
                        | WorkingState::Working { .. }
                        | WorkingState::Finalizing { .. } => {}
                        WorkingState::Exiting | WorkingState::Exited { .. } => {
                            <OfflineRequests<T>>::remove(acc_id);
                        }
                    },
                    None => {}
                }
            }
            Ok(().into())
        }

        fn on_geode_unhealthy(geode_id: T::AccountId) -> DispatchResult {
            Self::set_healthy_state(geode_id, HealthyState::Unhealthy);
            Ok(().into())
        }

        fn on_order_dispatched(
            geode_id: T::AccountId,
            session_index: u64,
            order_id: T::Hash,
        ) -> DispatchResult {
            Self::receive_order(geode_id, session_index, order_id)?;
            Ok(())
        }

        fn on_expired_check() {
            for (id, geode) in <Geodes<T>>::iter() {
                match geode.working_state {
                    WorkingState::Idle => {}
                    WorkingState::Pending { session_index } => {
                        // check whether it spend too much time in pending phase
                        // get the timeout duration from order
                        //

                        // get the timeout the order
                        // check the session_idx changed?
                        // mark unhealthy
                        // redispatch as an emergency order
                    }
                    WorkingState::Working { .. } => {}
                    WorkingState::Finalizing { .. } => {}
                    WorkingState::Exiting => {}
                    WorkingState::Exited { .. } => {}
                }
            }
        }
    }

    impl<T: Config> automata_traits::attestor::ApplicationTrait for Pallet<T> {
        type AccountId = T::AccountId;
        /// Currently we will only report a simple `unhealthy` state, but there might be more status in the future.
        /// E.g maybe something wrong with the application binary
        fn application_unhealthy(geode_id: Self::AccountId) -> DispatchResult {
            Self::set_healthy_state(geode_id, HealthyState::Unhealthy);
            Ok(().into())
        }

        /// Application are attested by several attestors, and reach healthy state
        fn application_healthy(geode_id: Self::AccountId) -> DispatchResult {
            Self::set_healthy_state(geode_id, HealthyState::Healthy);
            Ok(().into())
        }
    }

    impl<T: Config> Get<Vec<T::AccountId>> for Pallet<T> {
        fn get() -> Vec<T::AccountId> {
            Self::get_all_geodes()
        }
    }
}
