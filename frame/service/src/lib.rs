#![cfg_attr(not(feature = "std"), no_std)]
// #![feature(map_first_last)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use codec::{Decode, Encode};
    use core::convert::TryInto;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use primitives::{BlockNumber, DispatchId};
    use sp_core::H256;
    use sp_runtime::{RuntimeDebug, SaturatedConversion};

    use frame_support::ensure;
    use sha2::{Digest, Sha256};
    use sp_std::prelude::*;

    use sp_std::collections::{btree_map::BTreeMap, btree_set::BTreeSet};

    pub const MIN_ORDER_DURATION: BlockNumber = 40;

    #[cfg(feature = "std")]
    use serde::{Deserialize, Serialize};

    /// The service order struct proposed by the user
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, Default)]
    pub struct Order {
        /// Service data.
        pub binary: Vec<u8>,
        /// Service dns.
        pub dns: Vec<u8>,
        /// Service name.
        pub name: Option<Vec<u8>>,
        // token num that users are willing to pay
        pub price: u256,
        pub start_session_id: u32,
        // session num
        pub duration: u32,
        /// maximum number of geodes to serve the order
        pub geode_num: u32,
    }

    /// Geode state
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
    pub enum ServiceState {
        /// Default state, the service not existing
        Null,
        /// Waiting for geode to serve the service.
        Pending,
        /// When the service is being serviced by geode.
        Online,
        /// When no geode is serving after the service online.
        Offline,
        /// When the service is completed or cancelled by user
        Terminated,
    }

    impl Default for ServiceState {
        fn default() -> Self {
            ServiceState::Null
        }
    }

    /// Geode state
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
    pub enum DispatchState {
        /// default state, the dispatch not exist
        None,
        /// Pending to get a geode to query
        Pending,
        /// Waiting confirmation from geode
        Awaiting,
        /// Waiting dispatched geode to put online
        PreOnline,
    }

    impl Default for DispatchState {
        fn default() -> Self {
            DispatchState::None
        }
    }

    /// The full service struct shows its status
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, Default)]
    pub struct Dispatch<AccountId: Ord, Hash> {
        /// DispatchId is incremental from 0 and updated by 1 whenever a new dispatch is generated, it ensures dispatches will be served based on FIFO order.
        pub dispatch_id: DispatchId,
        /// The service_id for which this dispatch is generated
        pub service_id: Hash,
        /// Geode assigned with this dispatch, None if no geode has been queried for this dispatch
        pub geode: Option<AccountId>,
        /// Dispatch state
        pub state: DispatchState,
    }

    /// The full service struct shows its status
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, Default)]
    pub struct Service<AccountId: Ord, Hash> {
        /// Service order id
        pub order_id: Hash,
        /// Current existing dispatch for this service
        pub dispatches: BTreeSet<DispatchId>,
        /// Service owner id.
        pub owner: AccountId,
        /// Geodes serving the service(already put online).
        pub geodes: BTreeSet<AccountId>,
        /// Total weighted uptime the service has been online (num of geode * online block num)
        pub weighted_uptime: u64,
        /// Expected block num for the service to complete
        pub expected_ending: Option<BlockNumber>,
        /// Whether the service has backup
        pub backup_flag: bool,
        /// Indexing for backups, key is the backup service id, value is the backup data hash
        pub backup_map: BTreeMap<AccountId, Hash>,
        /// Current state of the service
        pub state: ServiceState,
    }

    pub type ServiceOf<T> =
        Service<<T as frame_system::Config>::AccountId, <T as frame_system::Config>::Hash>;
    pub type DispatchOf<T> =
        Dispatch<<T as frame_system::Config>::AccountId, <T as frame_system::Config>::Hash>;

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_geode::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(block_number: T::BlockNumber) -> Weight {
            if let Ok(now) = TryInto::<BlockNumber>::try_into(block_number) {
                // process pending service orders
                {
                    // load all the promised geodes into memory
                    let mut avail_geodes = BTreeMap::<BlockNumber, Vec<T::AccountId>>::new();
                    // let mut avail_promises = Vec::<T::BlockNumber>::new();
                    let mut updated_geodes = BTreeMap::<BlockNumber, Vec<T::AccountId>>::new();
                    pallet_geode::PromisedGeodes::<T>::iter()
                        .map(|(promise, geodes)| {
                            avail_geodes.insert(promise.clone().into(), geodes);
                        })
                        .all(|_| true);

                    let mut processed_services = Vec::<u32>::new();
                    for (dispatch, order_id) in <PendingDispatchesQueue<T>>::iter() {
                        if avail_geodes.is_empty() {
                            break;
                        }

                        let order = <Orders<T>>::get(order_id);

                        let geode;

                        let min_promise = now
                            + T::PutOnlineTimeout::get()
                            + T::DispatchConfirmationTimeout::get();
                        let expected_promise = min_promise + order.duration;
                        let promise;
                        if let Some(entry) = avail_geodes.range(expected_promise..).next() {
                            // try to find the smallest larger geode
                            promise = *entry.0;
                        } else if avail_geodes.contains_key(&0) {
                            promise = 0;
                        } else {
                            if let Some(entry) =
                                avail_geodes.range(min_promise..expected_promise).last()
                            {
                                // else find the largest smaller geode
                                promise = *entry.0;
                            } else {
                                break;
                            }
                        }

                        geode = avail_geodes.get_mut(&promise).unwrap().remove(0);
                        updated_geodes
                            .insert(promise.clone(), avail_geodes.get(&promise).unwrap().clone());

                        if avail_geodes.get(&promise).unwrap().is_empty() {
                            avail_geodes.remove(&promise);
                        }

                        // add to AwaitingDispatches
                        <AwaitingDispatches<T>>::insert(&geode, (&order_id, &now, &dispatch));
                        // remove from PendingDispatchesQueue
                        processed_services.push(dispatch);

                        let mut dispatch_use = <Dispatches<T>>::get(&dispatch);
                        dispatch_use.geode = Some(geode.clone());
                        dispatch_use.state = DispatchState::Awaiting;

                        <Dispatches<T>>::insert(&dispatch, dispatch_use);

                        Self::deposit_event(Event::DispatchQueriedGeode(dispatch, geode));
                    }
                    // handling the updated geode maps in batch
                    for (p, v) in updated_geodes.iter() {
                        if v.is_empty() {
                            pallet_geode::PromisedGeodes::<T>::remove(p);
                        } else {
                            pallet_geode::PromisedGeodes::<T>::insert(p, v);
                        }
                    }
                    // remove processed services from PendingDispatchesQueue
                    for p in processed_services.iter() {
                        <PendingDispatchesQueue<T>>::remove(p);
                    }
                }

                // process expired dispatches awaiting for confirmation - no penalty for geode
                {
                    let mut expired = Vec::<T::AccountId>::new();
                    for (geode, (order_id, block_num, dispatch)) in <AwaitingDispatches<T>>::iter()
                    {
                        if block_num + T::DispatchConfirmationTimeout::get() < now {
                            // put the order back to PendingDispatchesQueue
                            <PendingDispatchesQueue<T>>::insert(&dispatch, &order_id);
                            // change the dispatch state to Pending
                            let mut dispatch_use = <Dispatches<T>>::get(&dispatch);
                            dispatch_use.geode = None;
                            dispatch_use.state = DispatchState::Pending;
                            <Dispatches<T>>::insert(&dispatch, &dispatch_use);
                            // transit geode to unknown state
                            let geode_use = pallet_geode::Geodes::<T>::get(&geode);
                            <pallet_geode::Module<T>>::transit_state(
                                &geode_use,
                                pallet_geode::GeodeState::Unknown,
                            );
                            // clean from AwaitingDispatches
                            expired.push(geode);

                            Self::deposit_event(Event::NewPendingDispatch(dispatch, order_id));
                        }
                    }
                    // process expired
                    for p in expired.iter() {
                        <AwaitingDispatches<T>>::remove(p);
                    }
                }

                // process expired dispatches awaiting to be put online - have penalty for geode
                {
                    let mut expired = Vec::<T::AccountId>::new();
                    for (geode, (order_id, block_num, dispatch)) in <PreOnlineDispatches<T>>::iter()
                    {
                        if block_num + T::PutOnlineTimeout::get() < now {
                            // put the order back to PendingDispatchesQueue
                            <PendingDispatchesQueue<T>>::insert(dispatch, &order_id);
                            let mut dispatch_use = <Dispatches<T>>::get(&dispatch);
                            dispatch_use.geode = None;
                            dispatch_use.state = DispatchState::Pending;
                            <Dispatches<T>>::insert(&dispatch, &dispatch_use);
                            // transit geode to unknown state
                            let geode_use = pallet_geode::Geodes::<T>::get(&geode);
                            <pallet_geode::Module<T>>::transit_state(
                                &geode_use,
                                pallet_geode::GeodeState::Unknown,
                            );
                            // TODO: punish geode

                            // clean from AwaitingDispatches
                            expired.push(geode);

                            Self::deposit_event(Event::NewPendingDispatch(dispatch, order_id));
                        }
                    }
                    // process expired
                    for p in expired.iter() {
                        <PreOnlineDispatches<T>>::remove(p);
                    }
                }

                // check expected_endings and end services
                {
                    if <ExpectedEndings<T>>::contains_key(now) {
                        let terminated_services = <ExpectedEndings<T>>::get(now);
                        for service in terminated_services.iter() {
                            let service_record = <Services<T>>::get(service);
                            Self::terminate_service(service_record, now, true);
                        }
                        <TerminatedBatch<T>>::insert(now, terminated_services);
                        <ExpectedEndings<T>>::remove(now);
                    }
                }

                // clean expired terminated services records
            }
            0
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId", T::Hash = "Hash")]
    pub enum Event<T: Config> {
        /// User created service. \[user_id, service_hash\]
        ServiceCreated(T::AccountId, T::Hash),
        /// New dispatch created \[dispatch_id, service_hash\]
        NewPendingDispatch(DispatchId, T::Hash),
        /// Service removed. \[service_hash\]
        ServiceRemoved(T::Hash),
        /// Dispatch confirmed by geode \[dispatch_id, geode_id\]
        DispatchConfirmed(DispatchId, T::AccountId),
        /// Service turns online. \[service_hash\]
        ServiceOnline(T::Hash),
        /// Service gets degraded. \[service_hash\]
        ServiceDegraded(T::Hash),
        /// Service turns offline. \[service_hash\]
        ServiceOffline(T::Hash),
        /// Service gets terminated. \[service_hash\]
        ServiceTerminated(T::Hash),
        /// Dispatch queried geode for dispatching. \[dispatch_id, geode_id\]
        DispatchQueriedGeode(DispatchId, T::AccountId),
        /// Dispatched geode put service online \[dispatch_id, geode_id\]
        DispatchPutOnline(DispatchId, T::AccountId),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Use an invalid service id.
        InvalidService,
        /// The ServiceState can't allow you to do something now.
        InvalidServiceState,
        /// You doesn't have the right to do what you want.
        NoRight,
        /// Not allowed to change duration for a service without indicating duration at creation
        InvalidDuration,
        /// Insecure execution operated such as type overflow etc.
        InsecureExecution,
        /// Invalid operation
        InvalidOperation,
        /// Invalid dispatch
        WrongDispatch,
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn orders)]
    pub type Orders<T: Config> = StorageMap<_, Blake2_128Concat, T::Hash, Order, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn services)]
    pub type Services<T: Config> =
        StorageMap<_, Blake2_128Concat, T::Hash, ServiceOf<T>, ValueQuery>;

    // #[pallet::storage]
    // #[pallet::getter(fn pending_services)]
    // pub type PendingServices<T: Config> =
    //     StorageMap<_, Blake2_128Concat, T::Hash, BlockNumber, ValueQuery>;

    /// Value: the block number of when weighted_uptime updated last time
    #[pallet::storage]
    #[pallet::getter(fn online_services)]
    pub type OnlineServices<T: Config> =
        StorageMap<_, Blake2_128Concat, T::Hash, BlockNumber, ValueQuery>;

    // #[pallet::storage]
    // #[pallet::getter(fn offline_services)]
    // pub type OfflineServices<T: Config> =
    //     StorageMap<_, Blake2_128Concat, T::Hash, BlockNumber, ValueQuery>;

    // #[pallet::storage]
    // #[pallet::getter(fn terminated_services)]
    // pub type TerminatedServices<T: Config> =
    //     StorageMap<_, Blake2_128Concat, T::Hash, BlockNumber, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn teminated_batch)]
    pub type TerminatedBatch<T: Config> =
        StorageMap<_, Blake2_128Concat, BlockNumber, BTreeSet<T::Hash>, ValueQuery>;

    #[pallet::type_value]
    pub fn DefaultDispatchId<T: Config>() -> DispatchId {
        0
    }

    #[pallet::storage]
    #[pallet::getter(fn latest_dispatch_id)]
    pub type LatestDispatchId<T: Config> =
        StorageValue<_, DispatchId, ValueQuery, DefaultDispatchId<T>>;

    #[pallet::storage]
    #[pallet::getter(fn dispatch_states)]
    pub type Dispatches<T: Config> =
        StorageMap<_, Blake2_128Concat, DispatchId, DispatchOf<T>, ValueQuery>;

    /// Dispatches haven't been assigned to any geode
    #[pallet::storage]
    #[pallet::getter(fn pending_dispatches)]
    pub type PendingDispatchesQueue<T: Config> =
        StorageMap<_, Blake2_128Concat, DispatchId, T::Hash, ValueQuery>;

    /// Dispatches waiting for geode's confirmation
    #[pallet::storage]
    #[pallet::getter(fn awaiting_dispatch)]
    pub type AwaitingDispatches<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        (T::Hash, BlockNumber, DispatchId),
        ValueQuery,
    >;

    /// Dispatches waiting for geode to put online
    #[pallet::storage]
    #[pallet::getter(fn pre_online_dispatch)]
    pub type PreOnlineDispatches<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        (T::Hash, BlockNumber, DispatchId),
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn expected_endings)]
    pub type ExpectedEndings<T: Config> =
        StorageMap<_, Blake2_128Concat, BlockNumber, BTreeSet<T::Hash>, ValueQuery>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Called by user to create a service order.
        #[pallet::weight(0)]
        pub fn user_create_service(
            origin: OriginFor<T>,
            service_order: Order,
        ) -> DispatchResultWithPostInfo {
            ensure!(service_order.geode_num >= 1, Error::<T>::InvalidService);
            ensure!(
                service_order.duration >= MIN_ORDER_DURATION,
                Error::<T>::InvalidDuration
            );

            let who = ensure_signed(origin)?;
            let nonce = <frame_system::Module<T>>::account_nonce(&who);

            // TODO: calculate fee

            let mut data: Vec<u8> = Vec::new();
            data.extend_from_slice(&who.using_encoded(Self::to_ascii_hex));
            data.extend_from_slice(&nonce.encode().as_slice());

            let mut hasher = Sha256::new();
            hasher.update(data);
            let result = H256::from_slice(hasher.finalize().as_slice());
            let order_id: T::Hash = sp_core::hash::convert_hash(&result);

            let dispatches =
                Self::create_dispatches(service_order.geode_num, order_id.clone()).unwrap();

            let service = Service {
                order_id: order_id.clone(),
                dispatches: dispatches,
                owner: who.clone(),
                geodes: BTreeSet::new(),
                weighted_uptime: 0,
                expected_ending: None,
                backup_flag: false,
                backup_map: BTreeMap::new(),
                state: ServiceState::Pending,
            };

            <Orders<T>>::insert(&order_id, service_order);
            <Services<T>>::insert(&order_id, service);

            // let block_number =
            //     <frame_system::Module<T>>::block_number().saturated_into::<BlockNumber>();
            // <PendingServices<T>>::insert(&order_id, block_number);

            Self::deposit_event(Event::ServiceCreated(who, order_id.clone()));

            Ok(().into())
        }

        /// Called by user to remove a service order.
        #[pallet::weight(0)]
        pub fn user_remove_service(
            origin: OriginFor<T>,
            service_id: T::Hash,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(
                <Orders<T>>::contains_key(&service_id),
                Error::<T>::InvalidService
            );
            let service = <Services<T>>::get(&service_id);
            ensure!(service.owner == who, Error::<T>::NoRight);
            ensure!(
                service.state != ServiceState::Terminated,
                Error::<T>::InvalidServiceState
            );

            Self::terminate_service(
                service,
                <frame_system::Module<T>>::block_number().saturated_into::<BlockNumber>(),
                false,
            );

            Ok(().into())
        }

        /// Called by user to increase the duration of a service order, extended BlockNumber will be rounded up by SLOT_LENGTH
        #[pallet::weight(0)]
        pub fn user_extend_duration(
            origin: OriginFor<T>,
            service_id: T::Hash,
            extend: BlockNumber,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let mut service = <Services<T>>::get(&service_id);
            ensure!(service.owner == who, Error::<T>::NoRight);
            ensure!(
                service.state != ServiceState::Terminated,
                Error::<T>::InvalidServiceState
            );
            let mut order = <Orders<T>>::get(&service_id);
            // TODO: calculate fee

            order.duration = match order.duration.checked_add(extend) {
                Some(v) => v,
                None => {
                    return Err(Error::<T>::InsecureExecution.into());
                }
            };

            // TODO: update expected ending
            match service.expected_ending {
                Some(v) => {
                    let new_expected_ending = Self::get_expected_ending(
                        order.geode_num,
                        order.duration,
                        service.weighted_uptime,
                        service.geodes.len() as u32,
                    );
                    Self::update_expected_ending(service_id, Some(v), new_expected_ending);
                    service.expected_ending = Some(new_expected_ending);
                    <Services<T>>::insert(service_id, service);
                }
                None => {}
            }

            <Orders<T>>::insert(service_id, order);

            Ok(().into())
        }

        /// Called by geode to confirm an order
        #[pallet::weight(0)]
        pub fn provider_confirm_dispatch(
            origin: OriginFor<T>,
            geode: T::AccountId,
            service_id: T::Hash,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            ensure!(
                pallet_geode::Geodes::<T>::contains_key(&geode),
                pallet_geode::Error::<T>::InvalidGeode
            );
            let geode_use = pallet_geode::Geodes::<T>::get(&geode);
            ensure!(geode_use.provider == who, pallet_geode::Error::<T>::NoRight);
            ensure!(
                geode_use.state == pallet_geode::GeodeState::Attested,
                pallet_geode::Error::<T>::InvalidGeodeState
            );

            ensure!(
                <Services<T>>::contains_key(&service_id),
                Error::<T>::InvalidService
            );
            let service_use = <Services<T>>::get(&service_id);
            ensure!(
                service_use.state != ServiceState::Terminated,
                Error::<T>::InvalidServiceState
            );

            ensure!(
                <AwaitingDispatches<T>>::contains_key(&geode),
                Error::<T>::InvalidOperation
            );

            // load the dispatch info
            let (order_hash, _block_num, dispatch) = <AwaitingDispatches<T>>::get(&geode);

            ensure!(order_hash == service_id, Error::<T>::WrongDispatch);

            let mut geode_use = geode_use;
            geode_use.order = Some((service_id, None));
            ensure!(
                <pallet_geode::Module<T>>::transit_state(
                    &geode_use,
                    pallet_geode::GeodeState::Instantiated
                ),
                pallet_geode::Error::<T>::InvalidTransition
            );

            <PreOnlineDispatches<T>>::insert(
                &geode,
                (
                    order_hash,
                    <frame_system::Module<T>>::block_number().saturated_into::<BlockNumber>(),
                    &dispatch,
                ),
            );
            <AwaitingDispatches<T>>::remove(&geode);

            let mut dispatch_use = <Dispatches<T>>::get(&dispatch);
            dispatch_use.state = DispatchState::PreOnline;
            <Dispatches<T>>::insert(&dispatch, dispatch_use);

            Self::deposit_event(Event::DispatchConfirmed(dispatch, geode));

            Ok(().into())
        }

        /// Called by geode to start serving an order
        #[pallet::weight(0)]
        pub fn provider_start_serving(
            origin: OriginFor<T>,
            geode: T::AccountId,
            service_id: T::Hash,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            ensure!(
                pallet_geode::Geodes::<T>::contains_key(&geode),
                pallet_geode::Error::<T>::InvalidGeode
            );
            let geode_use = pallet_geode::Geodes::<T>::get(&geode);
            ensure!(geode_use.provider == who, pallet_geode::Error::<T>::NoRight);
            ensure!(
                geode_use.state == pallet_geode::GeodeState::Instantiated
                    || geode_use.state == pallet_geode::GeodeState::Degraded,
                pallet_geode::Error::<T>::InvalidGeodeState
            );

            ensure!(
                <Services<T>>::contains_key(&service_id),
                Error::<T>::InvalidService
            );
            let service_use = <Services<T>>::get(&service_id);
            ensure!(
                service_use.state != ServiceState::Terminated,
                Error::<T>::InvalidServiceState
            );

            ensure!(
                <PreOnlineDispatches<T>>::contains_key(&geode),
                Error::<T>::InvalidOperation
            );
            // load the dispatch info
            let (order_hash, _block_num, dispatch) = <PreOnlineDispatches<T>>::get(&geode);
            ensure!(service_id == order_hash, Error::<T>::WrongDispatch);

            let order_record = <Orders<T>>::get(order_hash);
            let mut service_use = service_use;
            service_use.dispatches.remove(&dispatch);

            let now = <frame_system::Module<T>>::block_number().saturated_into::<BlockNumber>();

            <PreOnlineDispatches<T>>::remove(&geode);
            <Dispatches<T>>::remove(&dispatch);

            match service_use.state {
                ServiceState::Pending => {
                    <OnlineServices<T>>::insert(order_hash, now);
                    // <PendingServices<T>>::remove(&order_hash);
                    service_use.state = ServiceState::Online;
                    Self::deposit_event(Event::ServiceOnline(order_hash));
                }
                ServiceState::Offline => {
                    <OnlineServices<T>>::insert(order_hash, now);
                    // <OfflineServices<T>>::remove(&order_hash);
                    service_use.state = ServiceState::Online;
                    Self::deposit_event(Event::ServiceOnline(order_hash));
                }
                ServiceState::Online => {
                    // update weighted_uptime
                    let last_update = <OnlineServices<T>>::get(order_hash);
                    let updated_weighted_uptime = Self::get_updated_weighted_uptime(
                        service_use.weighted_uptime,
                        last_update,
                        service_use.geodes.len() as u32,
                    );
                    service_use.weighted_uptime = updated_weighted_uptime;
                    <OnlineServices<T>>::insert(order_hash, now);
                }
                _ => {}
            }

            service_use.geodes.insert(geode.clone());

            let new_expected_ending = Self::get_expected_ending(
                order_record.geode_num,
                order_record.duration,
                service_use.weighted_uptime,
                service_use.geodes.len() as u32,
            );
            Self::update_expected_ending(
                order_hash,
                service_use.expected_ending,
                new_expected_ending,
            );
            service_use.expected_ending = Some(new_expected_ending);

            <Services<T>>::insert(order_hash, service_use);

            // update geode struct
            let mut geode_use = geode_use;
            geode_use.order = Some((order_hash, Some(now)));
            pallet_geode::Geodes::<T>::insert(&geode, geode_use);

            Self::deposit_event(Event::DispatchPutOnline(dispatch, geode));

            Ok(().into())
        }

        /// Called by provider to exit from Instantiated state
        #[pallet::weight(0)]
        pub fn provider_uninstantiate_geode(
            origin: OriginFor<T>,
            geode: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(
                pallet_geode::Geodes::<T>::contains_key(&geode),
                pallet_geode::Error::<T>::InvalidGeode
            );
            let geode_use = pallet_geode::Geodes::<T>::get(geode);
            ensure!(geode_use.provider == who, Error::<T>::NoRight);
            ensure!(
                geode_use.state == pallet_geode::GeodeState::Instantiated
                    || geode_use.state == pallet_geode::GeodeState::Degraded,
                pallet_geode::Error::<T>::InvalidGeodeState
            );

            let order_id = geode_use.order.unwrap().0;
            ensure!(
                !<Services<T>>::contains_key(order_id)
                    || <Services<T>>::get(order_id).state == ServiceState::Terminated,
                Error::<T>::InvalidOperation
            );

            // TODO: Backup logics
            let ret;
            match geode_use.state {
                pallet_geode::GeodeState::Instantiated => {
                    ret = <pallet_geode::Module<T>>::transit_state(
                        &geode_use,
                        pallet_geode::GeodeState::Attested,
                    );
                }
                pallet_geode::GeodeState::Degraded => {
                    ret = <pallet_geode::Module<T>>::transit_state(
                        &geode_use,
                        pallet_geode::GeodeState::Registered,
                    );
                }
                _ => {
                    // won't happen
                    ret = false;
                }
            }

            match ret {
                true => Ok(().into()),
                false => Err(pallet_geode::Error::<T>::InvalidTransition.into()),
            }
        }
    }

    impl<T: Config> Pallet<T> {
        fn to_ascii_hex(data: &[u8]) -> Vec<u8> {
            let mut r = Vec::with_capacity(data.len() * 2);
            let mut push_nibble = |n| r.push(if n < 10 { b'0' + n } else { b'a' - 10 + n });
            for &b in data.iter() {
                push_nibble(b / 16);
                push_nibble(b % 16);
            }
            r
        }

        pub fn get_updated_weighted_uptime(
            prev_weighted_uptime: u64,
            last_update: BlockNumber,
            prev_serving_geode_num: u32,
        ) -> u64 {
            prev_weighted_uptime
                .checked_add(last_update as u64 * prev_serving_geode_num as u64)
                .unwrap()
        }

        pub fn update_expected_ending(
            order_id: T::Hash,
            cur_expected_ending: Option<BlockNumber>,
            new_expected_ending: BlockNumber,
        ) {
            match cur_expected_ending {
                Some(v) => {
                    let mut ending_orders = <ExpectedEndings<T>>::get(v);
                    ending_orders.remove(&order_id);
                    if ending_orders.is_empty() {
                        <ExpectedEndings<T>>::remove(v);
                    } else {
                        <ExpectedEndings<T>>::insert(v, ending_orders);
                    }
                }
                None => {}
            }
            let mut ending_orders = <ExpectedEndings<T>>::get(new_expected_ending);
            ending_orders.insert(order_id);
            <ExpectedEndings<T>>::insert(new_expected_ending, ending_orders);
        }

        pub fn get_expected_ending(
            geode_req: u32,
            duration: BlockNumber,
            weighted_uptime: u64,
            geode_num: u32,
        ) -> BlockNumber {
            let total_weighted_duration: u64 = geode_req as u64 * duration as u64;
            let left_weighted_duration = total_weighted_duration - weighted_uptime;
            left_weighted_duration
                .checked_div(geode_num as u64)
                .unwrap() as BlockNumber
        }

        pub fn create_dispatches(num: u32, order_id: T::Hash) -> Option<BTreeSet<DispatchId>> {
            if num == 0 {
                return None;
            }

            let mut dispatch = <LatestDispatchId<T>>::get();

            let mut dispatches = BTreeSet::<DispatchId>::new();

            for _n in 1..num {
                dispatch += 1;
                <PendingDispatchesQueue<T>>::insert(&dispatch, &order_id);
                dispatches.insert(dispatch.clone());
                // change the dispatch state to Pending
                <Dispatches<T>>::insert(
                    &dispatch,
                    Dispatch {
                        dispatch_id: dispatch,
                        service_id: order_id,
                        geode: None,
                        state: DispatchState::Pending,
                    },
                );
                Self::deposit_event(Event::NewPendingDispatch(dispatch, order_id));
            }

            <LatestDispatchId<T>>::put(&dispatch);

            Some(dispatches)
        }

        fn terminate_service(service: ServiceOf<T>, when: BlockNumber, completed: bool) {
            let mut service = service;
            // remove service from state map
            match service.state {
                ServiceState::Pending => {
                    // <PendingServices<T>>::remove(service.order_id);
                }
                ServiceState::Offline => {
                    // <OfflineServices<T>>::remove(service.order_id);
                }
                ServiceState::Online => {
                    <OnlineServices<T>>::remove(service.order_id);
                }
                _ => {}
            }

            // update service state
            service.state = ServiceState::Terminated;
            // <TerminatedServices<T>>::insert(service.order_id, &when);

            if !completed {
                let now = <frame_system::Module<T>>::block_number().saturated_into::<BlockNumber>();
                let mut batch = <TerminatedBatch<T>>::get(&now);
                batch.insert(service.order_id);
                <TerminatedBatch<T>>::insert(now, batch);
            }

            // TODO: how to compensate geode? - flagdown fee for each geode

            // dismiss dispatches if there is any
            for dispatch in service.dispatches.iter() {
                let dispatch_use = <Dispatches<T>>::get(&dispatch);
                match dispatch_use.state {
                    DispatchState::Pending => {
                        <PendingDispatchesQueue<T>>::remove(&dispatch);
                    }
                    DispatchState::Awaiting => {
                        let geode = dispatch_use.geode.unwrap();
                        <AwaitingDispatches<T>>::remove(&geode);
                        // put geode back to priority pool
                        let geode = pallet_geode::Geodes::<T>::get(&geode);
                        <pallet_geode::Module<T>>::add_to_promises(&geode, &when);
                    }
                    DispatchState::PreOnline => {
                        // let geode itself recover from Instantiated/Degraded state
                        <PreOnlineDispatches<T>>::remove(&dispatch_use.geode.unwrap());
                        // if naturally completed - bad luck for geode
                        // TODO: if user terminate - compensate geode with flagdown fee
                    }
                    _ => {}
                }
            }

            <Services<T>>::remove(&service.order_id);
            <Orders<T>>::remove(&service.order_id);

            Self::deposit_event(Event::ServiceRemoved(service.order_id));
            // TODO: how to distribute reward and let user claim back left staking?
        }
    }
}
