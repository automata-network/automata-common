#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use automata_traits::attestor::AttestorTrait;
    use automata_traits::order::OrderTrait;
    use frame_support::storage::{IterableStorageMap, PrefixIterator, StorageMap as StorageMapT};
    use frame_support::{
        dispatch::DispatchResultWithPostInfo, ensure, pallet_prelude::*,
        unsigned::ValidateUnsigned, Blake2_128Concat,
    };
    use frame_system::offchain::{SendTransactionTypes, SubmitTransaction};
    use frame_system::RawOrigin;
    use frame_system::{ensure_signed, pallet_prelude::OriginFor};
    use primitives::order::OrderState;
    use sp_core::sr25519::{Public, Signature};
    use sp_runtime::RuntimeDebug;
    use sp_std::collections::btree_map::BTreeMap;
    use sp_std::prelude::*;

    #[cfg(feature = "full_crypto")]
    use sp_core::{crypto::Pair, sr25519::Pair as Sr25519Pair};

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
    pub enum WorkingState<BlockNumber> {
        /// The geode is not on service and ready to be dispatched an order
        Idle,
        /// The geode has been dispatched an order and is doing the preparation work
        Pending { session_index: BlockNumber },
        /// The geode is on service now
        Working { session_index: BlockNumber },
        /// The geode is on the progress of finishing the service
        /// maybe doing something like backup the data, uninstall the binary...
        Finalizing { session_index: BlockNumber },
        /// The geode is trying to exit, we should not dispatch orders to it
        Exiting,
    }

    impl<BlockNumber> Default for WorkingState<BlockNumber> {
        fn default() -> Self {
            Self::Idle
        }
    }

    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, Default)]
    pub struct Geode<AccountId, Hash, BlockNumber> {
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
        pub working_state: WorkingState<BlockNumber>,
        /// Hash of the current order, set on pending
        pub order_id: Option<Hash>,
    }

    pub type GeodeOf<T> = Geode<
        <T as frame_system::Config>::AccountId,
        <T as frame_system::Config>::Hash,
        <T as frame_system::Config>::BlockNumber,
    >;

    #[pallet::storage]
    #[pallet::getter(fn geodes)]
    pub type Geodes<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, GeodeOf<T>>;

    #[pallet::storage]
    #[pallet::getter(fn idle_geodes)]
    pub type IdleGeodes<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, ()>;

    #[pallet::storage]
    #[pallet::getter(fn pending_geodes)]
    pub type PendingGeodes<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, ()>;

    #[pallet::storage]
    #[pallet::getter(fn on_expired_check_previous_key)]
    pub type OnExpiredCheckPreviousKey<T: Config> = StorageValue<_, (T::BlockNumber, Vec<u8>)>;

    #[pallet::storage]
    #[pallet::getter(fn exiting_geodes)]
    pub type ExitingGeodes<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, ()>;

    #[pallet::storage]
    #[pallet::getter(fn on_new_session_previous_key)]
    pub type OnNewSessionPreviousKey<T: Config> = StorageValue<_, (T::BlockNumber, Vec<u8>)>;

    #[pallet::storage]
    #[pallet::getter(fn on_geode_offline_previous_key)]
    pub type OnGeodeOfflinePreviousKey<T: Config> = StorageValue<_, (T::BlockNumber, Vec<u8>)>;

    #[pallet::storage]
    #[pallet::getter(fn on_geode_failed_previous_key)]
    pub type OnGeodeFailedPreviousKey<T: Config> = StorageValue<_, (T::BlockNumber, Vec<u8>)>;

    #[pallet::storage]
    #[pallet::getter(fn offline_requests)]
    pub type OfflineRequests<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, ()>;

    #[pallet::storage]
    #[pallet::getter(fn fail_requests)]
    pub type FailRequests<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, ()>;

    pub const UNSIGNED_TXS_PRIORITY: u64 = 100;

    #[pallet::config]
    pub trait Config: SendTransactionTypes<Call<Self>> + frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type AttestorHandler: AttestorTrait<AccountId = Self::AccountId>;
        type OrderHandler: OrderTrait<
            BlockNumber = Self::BlockNumber,
            Hash = Self::Hash,
            AccountId = Self::AccountId,
        >;
        #[pallet::constant]
        type MaxGeodeProcessOneBlock: Get<u32>;
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
        InvalidSignature,
        InvalidMessage,
        NotSaveGeode,
        WaitingForOffline,
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
            mut geode: GeodeOf<T>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(
                !<Geodes<T>>::contains_key(&geode.id),
                Error::<T>::DuplicateGeodeId
            );
            // Register a new geode instance
            geode.working_state = WorkingState::Idle;
            geode.healthy_state = if <T::AttestorHandler>::check_healthy(&geode.id) {
                HealthyState::Healthy
            } else {
                HealthyState::Unhealthy
            };
            geode.provider = who;
            geode.order_id = None;
            <IdleGeodes<T>>::insert(geode.id.clone(), ());
            <Geodes<T>>::insert(geode.id.clone(), geode.clone());

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
            let geode = Self::get_geode_and_check_provider(origin, &geode_id)?;
            let _ = Self::mut_geode(geode, |geode| {
                if geode.working_state == WorkingState::Idle {
                    // it's safe to to exiting directly
                    geode.working_state = WorkingState::Exiting;
                    return Ok(());
                }
                <OfflineRequests<T>>::insert(geode_id.clone(), ());
                Err(<Error<T>>::NotSaveGeode.into())
            });
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
            Self::mut_geode_by_id(&geode_id, |geode| {
                ensure!(geode.provider == who, <Error<T>>::NotOwner);
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
            Self::mut_geode_by_id(&geode_id, |geode| {
                ensure!(geode.provider == who, <Error<T>>::NotOwner);
                geode.domain = domain;
                Ok(())
            })?;
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn unsigned_geode_ready(
            _: OriginFor<T>,
            message: Vec<u8>,
            signature_raw_bytes: [u8; 64],
        ) -> DispatchResultWithPostInfo {
            let (acc, order_id) =
                Self::decode_message(&message, &signature_raw_bytes, |mut data| {
                    ensure!(data.len() == 8, Error::<T>::InvalidMessage);
                    Ok(<T::Hash>::decode(&mut data).unwrap_or_default())
                })?;
            Self::geode_ready(RawOrigin::Signed(acc).into(), order_id)
        }

        /// Called when geode finish the data loading, binary loading and etc.
        /// And is ready to process the order.
        /// states: Pending -> Working
        #[pallet::weight(0)]
        pub fn geode_ready(origin: OriginFor<T>, order_id: T::Hash) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::mut_geode_by_id(&who, |geode| {
                match geode.working_state {
                    WorkingState::Pending { session_index } => {
                        ensure!(
                            geode.order_id == Some(order_id),
                            Error::<T>::OrderIdNotMatch
                        );
                        geode.working_state = WorkingState::Working { session_index };
                        T::OrderHandler::on_order_state(
                            geode.id.clone(),
                            order_id,
                            OrderState::Processing,
                        )?;
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
        pub fn unsigned_geode_finalizing(
            _: OriginFor<T>,
            message: Vec<u8>,
            signature_raw_bytes: [u8; 64],
        ) -> DispatchResultWithPostInfo {
            let (acc, order_id) =
                Self::decode_message(&message, &signature_raw_bytes, |mut data| {
                    ensure!(data.len() == 8, Error::<T>::InvalidMessage);
                    Ok(<T::Hash>::decode(&mut data).unwrap_or_default())
                })?;
            Self::geode_finalizing(RawOrigin::Signed(acc).into(), order_id)
        }

        /// Called when geode finish its order and working on the finalizing work
        #[pallet::weight(0)]
        pub fn geode_finalizing(
            origin: OriginFor<T>,
            order_id: T::Hash,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::mut_geode_by_id(&who, |geode| {
                ensure!(
                    geode.order_id == Some(order_id),
                    Error::<T>::OrderIdNotMatch
                );
                if let WorkingState::Working { session_index } = geode.working_state {
                    geode.working_state = WorkingState::Finalizing { session_index };
                } else {
                    return Err(Error::<T>::NotWorkingState.into());
                }
                Ok(())
            })?;
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn unsigned_geode_initialize_failed(
            _: OriginFor<T>,
            message: Vec<u8>,
            signature_raw_bytes: [u8; 64],
        ) -> DispatchResultWithPostInfo {
            let (acc, order_id) =
                Self::decode_message(&message, &signature_raw_bytes, |mut data| {
                    ensure!(data.len() == 8, Error::<T>::InvalidMessage);
                    Ok(<T::Hash>::decode(&mut data).unwrap_or_default())
                })?;
            Self::geode_initialize_failed(RawOrigin::Signed(acc).into(), order_id)
        }

        /// Called when geode failed to initialize(load data, load binary...).
        #[pallet::weight(0)]
        pub fn geode_initialize_failed(
            origin: OriginFor<T>,
            order_id: T::Hash,
            // reason: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let geode = Self::get_geode(&who)?;
            ensure!(
                geode.order_id == Some(order_id),
                Error::<T>::OrderIdNotMatch
            );
            <FailRequests<T>>::insert(geode.id.clone(), ());
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn unsigned_geode_finalized(
            _: OriginFor<T>,
            message: Vec<u8>,
            signature_raw_bytes: [u8; 64],
        ) -> DispatchResultWithPostInfo {
            let (acc, order_id) =
                Self::decode_message(&message, &signature_raw_bytes, |mut data| {
                    ensure!(data.len() == 8, Error::<T>::InvalidMessage);
                    Ok(<T::Hash>::decode(&mut data).unwrap_or_default())
                })?;
            Self::geode_finalized(RawOrigin::Signed(acc).into(), order_id)
        }

        /// Called when geode finish the finalization.
        /// state: finalizing -> idle
        /// healthy: any?
        #[pallet::weight(0)]
        pub fn geode_finalized(
            origin: OriginFor<T>,
            order_id: T::Hash,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::mut_geode_by_id(&who, |geode| match geode.working_state {
                WorkingState::Finalizing { .. } => {
                    ensure!(
                        geode.order_id == Some(order_id),
                        Error::<T>::OrderIdNotMatch
                    );
                    geode.working_state = WorkingState::Idle;
                    T::OrderHandler::on_order_state(who.clone(), order_id, OrderState::Done)?;
                    geode.order_id = None;
                    Ok(())
                }
                _ => return Err(Error::<T>::NotFinalizingState.into()),
            })?;
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn unsigned_geode_finalize_failed(
            _: OriginFor<T>,
            message: Vec<u8>,
            signature_raw_bytes: [u8; 64],
        ) -> DispatchResultWithPostInfo {
            let (acc, order_id) =
                Self::decode_message(&message, &signature_raw_bytes, |mut data| {
                    ensure!(data.len() == 8, Error::<T>::InvalidMessage);
                    Ok(<T::Hash>::decode(&mut data).unwrap_or_default())
                })?;
            Self::geode_finalize_failed(RawOrigin::Signed(acc).into(), order_id)
        }

        /// Called when geode failed to finalize.
        /// state: finalizing -> await idle
        /// healthy: ?
        #[pallet::weight(0)]
        pub fn geode_finalize_failed(
            origin: OriginFor<T>,
            order_id: T::Hash,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let geode = Self::get_geode(&who)?;
            ensure!(
                geode.order_id == Some(order_id),
                Error::<T>::OrderIdNotMatch
            );
            <FailRequests<T>>::insert(geode.id.clone(), ());
            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        fn get_all_geodes() -> Vec<T::AccountId> {
            <Geodes<T>>::iter()
                .map(|(account_id, _)| account_id)
                .collect::<Vec<T::AccountId>>()
        }

        fn get_geode_and_check_provider(
            origin: OriginFor<T>,
            geode_id: &T::AccountId,
        ) -> Result<GeodeOf<T>, sp_runtime::DispatchError> {
            let who = ensure_signed(origin)?;
            let geode = Self::get_geode(&geode_id)?;
            ensure!(geode.provider.eq(&who), Error::<T>::NotOwner);
            Ok(geode)
        }

        fn mut_geode_by_id<F>(who: &T::AccountId, f: F) -> DispatchResult
        where
            F: FnOnce(&mut GeodeOf<T>) -> DispatchResult,
        {
            let geode = Self::get_geode(who)?;
            Self::mut_geode(geode, f)?;
            Ok(())
        }

        fn mut_geode<F>(mut geode: GeodeOf<T>, f: F) -> DispatchResult
        where
            F: FnOnce(&mut GeodeOf<T>) -> DispatchResult,
        {
            let origin_working_state = geode.working_state.clone();
            f(&mut geode)?;
            if origin_working_state != geode.working_state {
                match geode.working_state {
                    WorkingState::Idle => {
                        <IdleGeodes<T>>::insert(geode.id.clone(), ());
                    }
                    WorkingState::Pending { .. } => {
                        <PendingGeodes<T>>::insert(geode.id.clone(), ());
                    }
                    WorkingState::Exiting { .. } => {
                        <ExitingGeodes<T>>::insert(geode.id.clone(), ());
                    }
                    _ => {}
                }
                match origin_working_state {
                    WorkingState::Idle => {
                        <IdleGeodes<T>>::remove(geode.id.clone());
                    }
                    WorkingState::Pending { .. } => {
                        <PendingGeodes<T>>::remove(geode.id.clone());
                    }
                    WorkingState::Exiting { .. } => {
                        <ExitingGeodes<T>>::remove(geode.id.clone());
                    }
                    _ => {}
                }
            }
            <Geodes<T>>::insert(geode.id.clone(), geode);
            Ok(())
        }

        fn get_geode(geode_id: &T::AccountId) -> Result<GeodeOf<T>, sp_runtime::DispatchError> {
            match <Geodes<T>>::get(&geode_id) {
                Some(geode) => Ok(geode),
                None => Err(Error::<T>::NonexistentGeode.into()),
            }
        }

        /// Collect a list of geode with the index S
        ///
        /// It will remove the order_id from the order if it's not exists
        fn get_list<S>(from_key: Option<Vec<u8>>, limit: u32) -> (Vec<GeodeOf<T>>, Vec<u8>)
        where
            S: IterableStorageMap<T::AccountId, (), Iterator = PrefixIterator<(T::AccountId, ())>>,
            S: StorageMapT<T::AccountId, ()>,
        {
            let limit: usize = limit as _;
            let mut list = Vec::new();
            let mut iter = match from_key {
                Some(key) => S::iter_from(key),
                None => S::iter(),
            };
            loop {
                let id = match iter.next() {
                    Some((id, _)) => id,
                    None => break,
                };
                match <Geodes<T>>::get(&id) {
                    Some(geode) => {
                        list.push(geode);
                        if list.len() >= limit {
                            break;
                        }
                    }
                    None => {
                        S::remove(&id);
                    }
                }
            }

            (list, iter.last_raw_key().into())
        }

        /// Only healthy and idle instance can receive the order
        ///
        /// 1. Check if the geode is available for working.
        /// 2. Transist its working state to Pending.
        fn receive_order(
            geode_id: T::AccountId,
            session_index: T::BlockNumber,
            order_id: T::Hash,
            domain: Vec<u8>,
        ) -> DispatchResult {
            ensure!(
                !<OfflineRequests<T>>::contains_key(&geode_id),
                <Error<T>>::WaitingForOffline
            );
            Self::mut_geode_by_id(&geode_id, |geode| {
                match geode.working_state {
                    WorkingState::Idle => match geode.healthy_state {
                        HealthyState::Healthy => {
                            geode.working_state = WorkingState::Pending { session_index };
                            geode.order_id = Some(order_id);
                            geode.domain = domain;
                        }
                        HealthyState::Unhealthy => {
                            return Err(Error::<T>::GeodeNotHealthy.into());
                        }
                    },
                    _ => return Err(Error::<T>::NotPendingState.into()),
                }
                Ok(())
            })?;
            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        fn get_previous_key<S>(session_index: T::BlockNumber) -> Option<Vec<u8>>
        where
            S: frame_support::storage::StorageValue<(T::BlockNumber, Vec<u8>)>,
        {
            match S::try_get() {
                Ok((session, key)) => {
                    if session == session_index {
                        Some(key)
                    } else {
                        None
                    }
                }
                Err(_) => None,
            }
        }

        fn save_previous_key<S>(session_index: T::BlockNumber, data: Vec<u8>)
        where
            S: frame_support::storage::StorageValue<(T::BlockNumber, Vec<u8>)>,
        {
            S::put((session_index, data))
        }
    }

    impl<T: Config> automata_traits::geode::GeodeTrait for Pallet<T> {
        type AccountId = T::AccountId;
        type Hash = T::Hash;
        type BlockNumber = T::BlockNumber;

        // Check the working geode, if it has finished the order, transist its working state to Finalizing.
        // Exiting -> Exited: In the beginning of session, if geode is in Exting state, will be transisted to Exited state.
        fn on_new_session(session_index: Self::BlockNumber) {
            let limit = T::MaxGeodeProcessOneBlock::get();
            let from_key = Self::get_previous_key::<OnNewSessionPreviousKey<T>>(session_index);
            let (geodes, last_key) = Self::get_list::<ExitingGeodes<T>>(from_key, limit);
            for geode in geodes {
                if geode.working_state == WorkingState::Exiting {
                    <ExitingGeodes<T>>::remove(&geode.id);
                    <Geodes<T>>::remove(&geode.id);
                }
            }
            Self::save_previous_key::<OnNewSessionPreviousKey<T>>(session_index, last_key);
        }

        // it's safe to process offline request
        // for expected states(idle): transited and removed from request list
        // for working states(pending, working, finalizing): ignored
        // for current states(exiting, exited): removed from request list
        fn on_geode_offline(session_index: Self::BlockNumber) {
            let limit = T::MaxGeodeProcessOneBlock::get();
            let from_key = Self::get_previous_key::<OnGeodeOfflinePreviousKey<T>>(session_index);
            let (geodes, last_key) = Self::get_list::<OfflineRequests<T>>(from_key, limit);
            let fail_request_limit = limit - geodes.len() as u32;
            for geode in geodes {
                match geode.working_state {
                    WorkingState::Idle => {
                        <OfflineRequests<T>>::remove(&geode.id);
                        let _ = Self::mut_geode(geode, |geode| {
                            geode.working_state = WorkingState::Exiting;
                            Ok(())
                        });
                    }
                    WorkingState::Pending { .. }
                    | WorkingState::Working { .. }
                    | WorkingState::Finalizing { .. } => {
                        // ignored
                    }
                    WorkingState::Exiting => {
                        <OfflineRequests<T>>::remove(&geode.id);
                    }
                }
            }
            Self::save_previous_key::<OnGeodeOfflinePreviousKey<T>>(session_index, last_key);

            let from_key = Self::get_previous_key::<OnGeodeFailedPreviousKey<T>>(session_index);
            let (geodes, last_key) =
                Self::get_list::<FailRequests<T>>(from_key, fail_request_limit);
            for geode in geodes {
                // pending -> initialized failed
                // finalizing -> finailze failed
                // set to idle
                match geode.working_state {
                    WorkingState::Pending { .. } | WorkingState::Finalizing { .. } => {
                        <FailRequests<T>>::remove(&geode.id);
                        let _ = Self::mut_geode(geode, |geode| {
                            geode.working_state = WorkingState::Idle;
                            Ok(())
                        });
                    }
                    WorkingState::Idle | WorkingState::Working { .. } | WorkingState::Exiting => {
                        // idle: we already goto target state
                        // working: it look like the geode has already retry after sending a failure message
                        // exiting: sending a failRequest and offline request. ignore
                        // exited: ignore
                        <FailRequests<T>>::remove(&geode.id);
                    }
                }
            }
            Self::save_previous_key::<OnGeodeFailedPreviousKey<T>>(session_index, last_key);
        }

        /// Dispatch an order to any numbers of geode
        fn on_order_dispatched(
            session_index: T::BlockNumber,
            order_id: T::Hash,
            num: u32,
            domain: Vec<u8>,
        ) -> Vec<T::AccountId> {
            let mut geode_ids = Vec::new();
            for (geode_id, _) in <IdleGeodes<T>>::iter() {
                // remove IdleGeodes(geode_id) in receive_order
                match Self::receive_order(geode_id.clone(), session_index, order_id, domain.clone())
                {
                    Ok(_) => geode_ids.push(geode_id.clone()),
                    Err(_) => {}
                }
                if geode_ids.len() >= num as usize {
                    break;
                }
            }
            geode_ids
        }

        // 1. Check the Pending geode list, if any geode is expired, we treat this geode as unhealthy and start to
        //    redispatch the order in emergency solution. Maybe we don't slash the geode right now, but we need to
        //    mark the geode and the order, if the similar case happen for this geode in the future, we need to
        //    transist its healthy state to Unhealthy and slash it. And if the similar case happen for the order
        //    in the future, we need to slash the binary provider and prevent the binary from used by other users.
        // 2. Check the Finalizing geode list? But I don't know how to handle the finalize timeout case,
        //    maybe don't process it now.
        fn on_expired_check(session_index: Self::BlockNumber) {
            let limit = T::MaxGeodeProcessOneBlock::get();
            let from_key = Self::get_previous_key::<OnExpiredCheckPreviousKey<T>>(session_index);
            let (geodes, _) = Self::get_list::<PendingGeodes<T>>(from_key, limit);
            for geode in geodes {
                match geode.order_id {
                    Some(order_id) => {
                        // check whether it spend too much time in pending phase
                        // get the timeout duration from order
                        if <T::OrderHandler>::is_order_expired(order_id, session_index) {
                            // mark unhealthy
                            // redispatch as an emergency order
                        }
                    }
                    None => {
                        // should not happen
                    }
                };
            }
        }
    }

    impl<T: Config> automata_traits::attestor::ApplicationTrait for Pallet<T> {
        type AccountId = T::AccountId;
        /// Currently we will only report a simple `unhealthy` state, but there might be more status in the future.
        /// E.g maybe something wrong with the application binary
        ///
        /// 1. Check if the current attestor state is Attestor Abnormal State, if yes, do nothing.
        /// 2. Transist its healthy state to Unhealthy.
        /// 3. Stop the geode from serving, and set the order into an emergency status.
        /// 4. Calculate the slash amount for this geode instance.
        fn application_unhealthy(
            geode_id: Self::AccountId,
            _is_attestor_down: bool,
        ) -> DispatchResult {
            if <T::AttestorHandler>::is_abnormal_mode() {
                return Ok(());
            }
            Self::mut_geode_by_id(&geode_id, |geode| {
                geode.healthy_state = HealthyState::Unhealthy;
                match geode.order_id {
                    Some(order_id) => {
                        T::OrderHandler::on_order_state(
                            geode.id.clone(),
                            order_id,
                            OrderState::Emergency,
                        )?;
                        geode.order_id = None;
                    }
                    None => (),
                }
                Ok(().into())
            })?;

            // TODO: slash geode if is_attestor_down is false
            Ok(().into())
        }

        /// Application are attested by several attestors, and reach healthy state
        fn application_healthy(geode_id: Self::AccountId) -> DispatchResult {
            Self::mut_geode_by_id(&geode_id, |geode| {
                geode.healthy_state = HealthyState::Healthy;
                Ok(())
            })?;
            Ok(().into())
        }
    }

    impl<T: Config> Get<Vec<T::AccountId>> for Pallet<T> {
        fn get() -> Vec<T::AccountId> {
            Self::get_all_geodes()
        }
    }

    macro_rules! unsigned_rpc {
        ( $rpc_name:ident, $name:ident ) => {
            pub fn $rpc_name(message: Vec<u8>, signature_raw_bytes: [u8; 64]) -> Result<(), ()> {
                let call = Call::$name(message, signature_raw_bytes);
                SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
            }
        };
    }

    // unsigned implementation
    impl<T: Config> Pallet<T> {
        fn decode_message_in_validate<F>(
            msg: &Vec<u8>,
            signature: &[u8; 64],
            f: F,
        ) -> Result<T::AccountId, InvalidTransaction>
        where
            F: FnOnce(&[u8]) -> sp_runtime::DispatchResult,
        {
            let acc = match Self::decode_message(msg, signature, f) {
                Ok((acc, _)) => acc,
                Err(_) => return Err(InvalidTransaction::Call),
            };
            Ok(acc)
        }

        fn decode_message<F, N>(
            message: &Vec<u8>,
            signature_raw_bytes: &[u8; 64],
            f: F,
        ) -> Result<(T::AccountId, N), sp_runtime::DispatchError>
        where
            F: FnOnce(&[u8]) -> Result<N, sp_runtime::DispatchError>,
        {
            let mut geode_id = [0u8; 32];
            geode_id.copy_from_slice(&message[0..32]);
            let pubkey = Public::from_raw(geode_id.clone());
            let signature = Signature::from_raw(signature_raw_bytes.clone());

            #[cfg(feature = "full_crypto")]
            ensure!(
                Sr25519Pair::verify(&signature, message, &pubkey),
                Error::<T>::InvalidSignature
            );
            let acc = T::AccountId::decode(&mut &geode_id[..]).unwrap_or_default();
            let args = f(&message[32..])?;
            Ok((acc, args))
        }

        unsigned_rpc! {rpc_unsigned_geode_ready, unsigned_geode_ready}
        unsigned_rpc! {rpc_unsigned_geode_finalizing, unsigned_geode_finalizing}
        unsigned_rpc! {rpc_unsigned_geode_finalize_failed, unsigned_geode_finalize_failed}
        unsigned_rpc! {rpc_unsigned_geode_finalized, unsigned_geode_finalized}
    }

    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            let (tag, account) = match call {
                Call::unsigned_geode_ready(msg, signature) => (
                    "Automata/geode/unsigned_geode_ready",
                    Self::decode_message_in_validate(&msg, &signature, |data| {
                        ensure!(data.len() == 8, Error::<T>::InvalidMessage);
                        Ok(())
                    })?,
                ),
                Call::unsigned_geode_finalizing(msg, signature) => (
                    "Automata/geode/unsigned_geode_finalizing",
                    Self::decode_message_in_validate(&msg, &signature, |data| {
                        ensure!(data.len() == 8, <Error<T>>::InvalidMessage);
                        Ok(())
                    })?,
                ),
                Call::unsigned_geode_finalized(msg, signature) => (
                    "Automata/geode/unsigned_geode_finalized",
                    Self::decode_message_in_validate(&msg, &signature, |data| {
                        ensure!(data.len() == 8, <Error<T>>::InvalidMessage);
                        Ok(())
                    })?,
                ),
                Call::unsigned_geode_finalize_failed(msg, signature) => (
                    "Automata/geode/unsigned_geode_finalize_failed",
                    Self::decode_message_in_validate(&msg, &signature, |data| {
                        ensure!(data.len() > 8, <Error<T>>::InvalidMessage);
                        Ok(())
                    })?,
                ),
                _ => return InvalidTransaction::Call.into(),
            };

            ValidTransaction::with_tag_prefix(tag)
                .priority(UNSIGNED_TXS_PRIORITY)
                .and_provides((account, <frame_system::Pallet<T>>::block_number()))
                .longevity(3)
                .propagate(true)
                .build()
        }
    }
}
