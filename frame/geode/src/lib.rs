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
    use frame_support::{
        dispatch::DispatchResultWithPostInfo,
        ensure,
        pallet_prelude::{OptionQuery, StorageMap, ValueQuery},
        Blake2_128Concat,
    };
    use frame_support::{pallet_prelude::*, unsigned::ValidateUnsigned};

    use frame_system::RawOrigin;
    use frame_system::{ensure_signed, pallet_prelude::OriginFor};
    use frame_system::{
        offchain::{SendTransactionTypes, SubmitTransaction},
        pallet_prelude::*,
    };
    use sp_core::sr25519::{Public, Signature};
    use sp_runtime::RuntimeDebug;
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
        /// The geode has exited successfully, and it can shutdown at any time
        Exited { session_index: BlockNumber },
    }

    impl<BlockNumber> Default for WorkingState<BlockNumber> {
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
    #[pallet::getter(fn offline_requests)]
    pub type OfflineRequests<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, T::AccountId, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn fail_requests)]
    pub type FailRequests<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, FailReason, OptionQuery>;

    pub const UNSIGNED_TXS_PRIORITY: u64 = 100;

    #[pallet::config]
    pub trait Config: SendTransactionTypes<Call<Self>> + frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type AttestorHandler: AttestorTrait<AccountId = Self::AccountId>;
        type OrderHandler: OrderTrait<BlockNumber = Self::BlockNumber, Hash = Self::Hash>;
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
                    geode_record.healthy_state =
                        if <T::AttestorHandler>::check_healthy(&geode_record.id) {
                            HealthyState::Healthy
                        } else {
                            HealthyState::Unhealthy
                        };
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
            let _ = Self::get_geode_and_check_provider(origin, &geode_id)?;
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
            Self::mut_geode_fn(&geode_id, |geode| {
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
            Self::mut_geode_fn(&geode_id, |geode| {
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
            Self::mut_geode_fn(&who, |geode| {
                match geode.working_state {
                    WorkingState::Pending { session_index } => {
                        ensure!(
                            geode.order_id == Some(order_id),
                            Error::<T>::OrderIdNotMatch
                        );
                        geode.working_state = WorkingState::Working { session_index };
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
            Self::mut_geode_fn(&who, |geode| {
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
            <FailRequests<T>>::insert(
                geode.id.clone(),
                FailReason {
                    reason: "initialize_failed".into(),
                },
            );
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
            Self::mut_geode_fn(&who, |geode| match geode.working_state {
                WorkingState::Finalizing { .. } => {
                    ensure!(
                        geode.order_id == Some(order_id),
                        Error::<T>::OrderIdNotMatch
                    );
                    geode.working_state = WorkingState::Idle;
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
            <FailRequests<T>>::insert(
                geode.id.clone(),
                FailReason {
                    reason: "finalize_failed".into(),
                },
            );
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

        fn get_geode_and_check_provider(
            origin: OriginFor<T>,
            geode_id: &T::AccountId,
        ) -> Result<GeodeOf<T>, sp_runtime::DispatchError> {
            let who = ensure_signed(origin)?;
            let geode = Self::get_geode(&geode_id)?;
            ensure!(geode.provider.eq(&who), Error::<T>::NotOwner);
            Ok(geode)
        }

        fn mut_geode_fn<F>(who: &T::AccountId, f: F) -> DispatchResult
        where
            F: FnOnce(&mut GeodeOf<T>) -> DispatchResult,
        {
            let mut geode = Self::get_geode(who)?;
            f(&mut geode)?;
            <Geodes<T>>::insert(geode.id.clone(), geode);
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
            session_index: T::BlockNumber,
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
        type BlockNumber = T::BlockNumber;

        // Check the working geode, if it has finished the order, transist its working state to Finalizing.
        // Exiting -> Exited: In the beginning of session, if geode is in Exting state, will be transisted to Exited state.
        fn on_new_session(session_index: Self::BlockNumber) {
            for (geode_id, mut geode) in <Geodes<T>>::iter() {
                if geode.working_state == WorkingState::Exiting {
                    geode.working_state = WorkingState::Exited { session_index };
                    <Geodes<T>>::insert(geode_id, geode);
                }
            }
        }

        // it's safe to process offline request
        // for expected states(idle): transited and removed from request list
        // for working states(pending, working, finalizing): ignored
        // for current states(exiting, exited): removed from request list
        fn on_geode_offline(_: Self::BlockNumber) {
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
                        | WorkingState::Finalizing { .. } => {
                            // ignored
                        }
                        WorkingState::Exiting | WorkingState::Exited { .. } => {
                            <OfflineRequests<T>>::remove(acc_id);
                        }
                    },
                    None => {
                        // already offline
                        <OfflineRequests<T>>::remove(acc_id);
                    }
                }
            }
            for (geode_id, _) in <FailRequests<T>>::iter() {
                // pending -> initialized failed
                // finalizing -> finailze failed
                // set to idle
                match <Geodes<T>>::get(&geode_id) {
                    Some(mut geode) => match geode.working_state {
                        WorkingState::Pending { .. } | WorkingState::Finalizing { .. } => {
                            geode.working_state = WorkingState::Idle;
                            <Geodes<T>>::insert(geode_id.clone(), geode);
                            <FailRequests<T>>::remove(geode_id);
                        }
                        WorkingState::Idle
                        | WorkingState::Working { .. }
                        | WorkingState::Exiting
                        | WorkingState::Exited { .. } => {
                            // idle: we already goto target state
                            // working: it look like the geode has already retry after sending a failure message
                            // exiting: sending a failRequest and offline request. ignore
                            // exited: ignore
                            <FailRequests<T>>::remove(geode_id);
                        }
                    },
                    None => {
                        // already offline
                        <FailRequests<T>>::remove(geode_id);
                    }
                }
            }
        }

        // 1. Check if the current attestor state is Attestor Abnormal State, if yes, do nothing.
        // 2. Transist its healthy state to Unhealthy.
        // 3. Stop the geode from serving, and set the order into an emergency status.
        // 4. Calculate the slash amount for this geode instance.
        fn on_geode_unhealthy(geode_id: T::AccountId) {
            if <T::AttestorHandler>::is_abnormal_mode() {
                return;
            }
            Self::set_healthy_state(geode_id, HealthyState::Unhealthy);
        }

        // 1. Check if the geode is available for working.
        // 2. Transist its working state to Pending.
        fn on_order_dispatched(
            geode_id: T::AccountId,
            session_index: T::BlockNumber,
            order_id: T::Hash,
        ) -> DispatchResult {
            Self::receive_order(geode_id, session_index, order_id)?;
            Ok(())
        }

        // 1. Check the Pending geode list, if any geode is expired, we treat this geode as unhealthy and start to
        //    redispatch the order in emergency solution. Maybe we don't slash the geode right now, but we need to
        //    mark the geode and the order, if the similar case happen for this geode in the future, we need to
        //    transist its healthy state to Unhealthy and slash it. And if the similar case happen for the order
        //    in the future, we need to slash the binary provider and prevent the binary from used by other users.
        // 2. Check the Finalizing geode list? But I don't know how to handle the finalize timeout case,
        //    maybe don't process it now.
        fn on_expired_check(session_index: Self::BlockNumber) {
            for (_, geode) in <Geodes<T>>::iter() {
                match geode.working_state {
                    WorkingState::Idle => {}
                    WorkingState::Pending { .. } => {
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
            <Self as automata_traits::geode::GeodeTrait>::on_geode_unhealthy(geode_id);
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
