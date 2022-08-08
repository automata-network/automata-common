#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use automata_traits::geode::GeodeTrait;
    use frame_support::pallet_prelude::*;
    use frame_support::storage::{IterableStorageMap, PrefixIterator, StorageMap as StorageMapT};
    use frame_system::pallet_prelude::*;
    use primitives::order::{OrderOf, OrderState};
    use sha2::{Digest, Sha256};
    use sp_core::H256;
    use sp_std::convert::TryInto;
    use sp_std::vec::Vec;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId", T::Hash = "Hash")]
    pub enum Event<T: Config> {
        /// User created order. \[user_id, service_hash\]
        OrderSubmitted(T::AccountId, T::Hash),
    }

    #[pallet::storage]
    #[pallet::getter(fn orders)]
    pub type Orders<T: Config> = StorageMap<_, Blake2_128Concat, T::Hash, OrderOf<T>>;

    #[pallet::storage]
    #[pallet::getter(fn order_services)]
    pub type OrderServices<T: Config> =
        StorageMap<_, Blake2_128Concat, T::Hash, Vec<(T::AccountId, OrderState)>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn submitted_order_ids)]
    pub type SubmittedOrderIds<T: Config> = StorageMap<_, Blake2_128Concat, T::Hash, ()>;

    #[pallet::storage]
    #[pallet::getter(fn processing_order_ids)]
    pub type ProcessingOrderIds<T: Config> = StorageMap<_, Blake2_128Concat, T::Hash, ()>;

    #[pallet::storage]
    #[pallet::getter(fn emergency_order_ids)]
    pub type EmergencyOrderIds<T: Config> = StorageMap<_, Blake2_128Concat, T::Hash, ()>;

    #[pallet::storage]
    #[pallet::getter(fn canceled_order_ids)]
    pub type CanceledOrderIds<T: Config> = StorageMap<_, Blake2_128Concat, T::Hash, ()>;

    #[pallet::storage]
    #[pallet::getter(fn on_new_session_previous_key)]
    pub type OnNewSessionPreviousKey<T: Config> = StorageValue<_, (T::BlockNumber, Vec<u8>)>;

    #[pallet::storage]
    #[pallet::getter(fn on_new_session_cancel_previous_key)]
    pub type OnNewSessionCancelPreviousKey<T: Config> = StorageValue<_, (T::BlockNumber, Vec<u8>)>;

    #[pallet::storage]
    #[pallet::getter(fn on_emergency_order_dispatch_key)]
    pub type OnEmergencyOrderDispatchPreviousKey<T: Config> =
        StorageValue<_, (T::BlockNumber, Vec<u8>)>;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type GeodeHandler: GeodeTrait<
            AccountId = <Self as frame_system::Config>::AccountId,
            Hash = <Self as frame_system::Config>::Hash,
            BlockNumber = <Self as frame_system::Config>::BlockNumber,
        >;
        #[pallet::constant]
        type MaxOrderProcessOneBlock: Get<Self::BlockNumber>;
    }

    #[pallet::error]
    pub enum Error<T> {
        InvalidOrder,
        OrderIdDuplicated,
        InvalidDuration,
        InvalidOrderOwner,
        InvalidNotPending,
        InvalidState,
        InvalidService,
        InternalLogicError,
    }

    impl<T: Config> automata_traits::order::OrderTrait for Pallet<T> {
        type AccountId = T::AccountId;
        type Hash = T::Hash;
        type BlockNumber = T::BlockNumber;
        fn is_order_expired(order_id: Self::Hash, session_index: Self::BlockNumber) -> bool {
            match <Orders<T>>::get(order_id) {
                Some(order) => session_index > order.start_session_id + order.duration,
                None => false,
            }
        }

        /// Check the state of orders, if its state is Processing and should be finished before
        /// this session, transist its state to Done.
        ///
        /// We can call `on_new_session` servial times in one session
        fn on_new_session(session_index: Self::BlockNumber) {
            let limit = T::MaxOrderProcessOneBlock::get();
            let from_key = Self::get_previous_key::<OnNewSessionPreviousKey<T>>(session_index);
            let (order_list, last_key) = Self::get_list::<ProcessingOrderIds<T>>(from_key, limit);
            for order in order_list {
                if session_index > order.start_session_id + order.duration {
                    let _ = Self::mut_order(order, |order| {
                        order.state = OrderState::Done;
                        let mut order_services = <OrderServices<T>>::get(order.order_id);
                        for order_service in &mut order_services {
                            order_service.1 = OrderState::Done;
                        }
                        <OrderServices<T>>::insert(order.order_id.clone(), order_services);
                        return Ok(());
                    });
                }
            }
            Self::save_previous_key::<OnNewSessionPreviousKey<T>>(session_index, last_key);

            let from_key =
                Self::get_previous_key::<OnNewSessionCancelPreviousKey<T>>(session_index);
            let (canceled_list, last_key) = Self::get_list::<CanceledOrderIds<T>>(from_key, limit);
            for order in canceled_list {
                <CanceledOrderIds<T>>::remove(order.order_id);
                let _ = Self::mut_order(order, |order| {
                    order.state = OrderState::Done;
                    let mut order_services = <OrderServices<T>>::get(order.order_id);
                    for order_service in &mut order_services {
                        order_service.1 = OrderState::Done;
                    }
                    <OrderServices<T>>::insert(order.order_id.clone(), order_services);
                    Ok(())
                });
            }
            Self::save_previous_key::<OnNewSessionCancelPreviousKey<T>>(session_index, last_key);
        }

        /// Dispatch submitted orders to the geode
        ///
        /// 1. Process with the Submitted orders, select a proper geode and assign the order to it.
        /// 2. Call GeodeHandler::on_order_dispatched().
        /// 3. Transist the order state to Pending.
        fn on_orders_dispatch(session_index: Self::BlockNumber) {
            let limit = T::MaxOrderProcessOneBlock::get();
            let (order_list, _) = Self::get_list::<SubmittedOrderIds<T>>(None, limit);
            for order in order_list {
                if order.state != OrderState::Submitted {
                    <SubmittedOrderIds<T>>::remove(&order.order_id);
                    continue;
                }
                let mut order_services = <OrderServices<T>>::get(&order.order_id);
                let num = order.num - order_services.len() as u32;
                let service_ids = T::GeodeHandler::on_order_dispatched(
                    session_index,
                    order.order_id,
                    num,
                    order.domain.clone(),
                );
                for service_id in service_ids {
                    order_services.push((service_id, OrderState::Pending));
                }
                let is_enough_service = order_services.len() >= order.num as usize;
                <OrderServices<T>>::insert(order.order_id.clone(), order_services);

                let _ = Self::mut_order(order, |order| {
                    order.start_session_id = session_index;
                    order.state = if is_enough_service {
                        OrderState::Pending
                    } else {
                        OrderState::Emergency
                    };
                    Ok(())
                });
            }
        }

        /// 1. Process orders in emergency order list, choose a proper geode and assign the order to it.
        /// 2. Call GeodeHandler::on_order_dispatched().
        /// 3. Transist the order state to Pending.
        fn on_emergency_order_dispatch(session_index: Self::BlockNumber) {
            let limit = T::MaxOrderProcessOneBlock::get();
            let from_key =
                Self::get_previous_key::<OnEmergencyOrderDispatchPreviousKey<T>>(session_index);
            let (order_list, last_key) = Self::get_list::<EmergencyOrderIds<T>>(from_key, limit);
            for order in order_list {
                let mut order_services = <OrderServices<T>>::get(&order.order_id);
                let num = order.num - order_services.len() as u32;
                let service_ids = T::GeodeHandler::on_order_dispatched(
                    session_index,
                    order.order_id,
                    num,
                    order.domain.clone(),
                );
                for service_id in service_ids {
                    order_services.push((service_id, OrderState::Pending));
                }
                let target_order_state = Self::current_state(&order, &order_services);
                <OrderServices<T>>::insert(order.order_id.clone(), order_services);

                if target_order_state != order.state {
                    let _ = Self::mut_order(order, |order| {
                        order.state = target_order_state;
                        Ok(())
                    });
                }
            }
            Self::save_previous_key::<OnEmergencyOrderDispatchPreviousKey<T>>(
                session_index,
                last_key,
            );
        }

        fn on_order_state(
            service_id: Self::AccountId,
            order_id: Self::Hash,
            target_state: OrderState,
        ) -> DispatchResult {
            ensure!(
                ![OrderState::Submitted, OrderState::Pending].contains(&target_state),
                <Error<T>>::InvalidState
            );

            let order = match <Orders<T>>::get(&order_id) {
                Some(order) => order,
                None => return Err(<Error<T>>::InvalidOrder.into()),
            };

            let mut services = <OrderServices<T>>::get(&order_id);
            let idx = match services.iter().position(|x| x.0 == service_id) {
                Some(idx) => idx,
                None => return Err(<Error<T>>::InvalidService.into()),
            };

            match target_state {
                OrderState::Submitted | OrderState::Pending => unreachable!(),
                OrderState::Processing => {
                    ensure!(
                        services[idx].1 == OrderState::Pending,
                        Error::<T>::InvalidNotPending
                    );
                    services[idx].1 = OrderState::Processing;
                }
                OrderState::Emergency => {
                    ensure!(
                        [OrderState::Pending, OrderState::Processing].contains(&services[idx].1),
                        Error::<T>::InvalidState
                    );
                    services.remove(idx);
                }
                OrderState::Done => {
                    ensure!(
                        [OrderState::Pending, OrderState::Processing,].contains(&services[idx].1),
                        Error::<T>::InvalidState
                    );
                    services[idx].1 = OrderState::Done;
                }
            }
            <OrderServices<T>>::insert(order.order_id.clone(), services.clone());

            // detemine the order state base on OrderService's state
            let target_order_state = Self::current_state(&order, &services);
            if target_order_state == order.state {
                return Ok(());
            }
            ensure!(
                order.state.check_next(target_order_state),
                <Error<T>>::InternalLogicError
            );

            Self::mut_order(order, |order| {
                order.state = target_order_state;
                Ok(())
            })
        }
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(0)]
        pub fn create_order(
            origin: OriginFor<T>,
            mut order: OrderOf<T>,
        ) -> DispatchResultWithPostInfo {
            ensure!(order.num >= 1, Error::<T>::InvalidOrder);
            ensure!(order.duration >= 1u32.into(), Error::<T>::InvalidDuration);

            let who = ensure_signed(origin)?;
            let nonce = <frame_system::Pallet<T>>::account_nonce(&who);

            let mut hasher = Sha256::new();
            hasher.update({
                let mut data: Vec<u8> = Vec::new();
                data.extend_from_slice(&who.using_encoded(Self::to_ascii_hex));
                data.extend_from_slice(&nonce.encode().as_slice());
                data
            });
            let result = H256::from_slice(hasher.finalize().as_slice());
            order.order_id = sp_core::hash::convert_hash(&result);
            ensure!(
                !<Orders<T>>::contains_key(&order.order_id),
                <Error<T>>::OrderIdDuplicated
            );

            order.provider = who.clone();
            <Orders<T>>::insert(order.order_id.clone(), order.clone());
            <SubmittedOrderIds<T>>::insert(&order.order_id, ());

            Self::deposit_event(Event::OrderSubmitted(who, order.order_id));
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn cancel_order(origin: OriginFor<T>, order_id: T::Hash) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let order = <Orders<T>>::get(&order_id);
            ensure!(order.is_some(), Error::<T>::InvalidOrder);
            let order: OrderOf<T> = order.unwrap();
            ensure!(order.provider == who, Error::<T>::InvalidOrderOwner);
            <CanceledOrderIds<T>>::insert(&order_id, ());
            Ok(().into())
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

        /// Collect a list of orders with the index S
        ///
        /// It will remove the order_id from the order if it's not exists
        fn get_list<S>(
            from_key: Option<Vec<u8>>,
            limit: T::BlockNumber,
        ) -> (Vec<OrderOf<T>>, Vec<u8>)
        where
            S: IterableStorageMap<T::Hash, (), Iterator = PrefixIterator<(T::Hash, ())>>,
            S: StorageMapT<T::Hash, ()>,
        {
            let limit: usize = limit.try_into().unwrap_or_default();
            let mut list = Vec::new();
            let mut iter = match from_key {
                Some(key) => S::iter_from(key),
                None => S::iter(),
            };
            loop {
                let order_id = match iter.next() {
                    Some((order_id, _)) => order_id,
                    None => break,
                };
                match <Orders<T>>::get(&order_id) {
                    Some(order) => {
                        list.push(order);
                        if list.len() >= limit {
                            break;
                        }
                    }
                    None => {
                        S::remove(&order_id);
                    }
                }
            }

            (list, iter.last_raw_key().into())
        }

        fn current_state(
            order: &OrderOf<T>,
            services: &[(T::AccountId, OrderState)],
        ) -> OrderState {
            let mut output = None;
            for (_, service_state) in services {
                if output.is_none() {
                    output = Some(service_state.clone());
                    continue;
                }
                match service_state {
                    OrderState::Submitted | OrderState::Emergency => unreachable!(),
                    OrderState::Pending => {
                        // still pending
                        output = Some(OrderState::Pending);
                    }
                    OrderState::Processing => {
                        if output.unwrap() != OrderState::Pending {
                            output = Some(OrderState::Processing);
                        }
                    }
                    OrderState::Done => {}
                }
            }
            if services.len() < order.num as usize {
                output = Some(OrderState::Emergency);
            }
            output.unwrap() // it's safe because of the emergency checking
        }

        fn mut_order<F>(mut order: OrderOf<T>, f: F) -> DispatchResult
        where
            F: FnOnce(&mut OrderOf<T>) -> DispatchResult,
        {
            let old_state = order.state.clone();
            f(&mut order)?;
            if old_state.ne(&order.state) {
                match old_state {
                    OrderState::Submitted => <SubmittedOrderIds<T>>::remove(&order.order_id),
                    OrderState::Pending => (),
                    OrderState::Processing => <ProcessingOrderIds<T>>::remove(&order.order_id),
                    OrderState::Emergency => <EmergencyOrderIds<T>>::remove(&order.order_id),
                    OrderState::Done => (),
                }
                match &order.state {
                    OrderState::Submitted => <SubmittedOrderIds<T>>::insert(order.order_id, ()),
                    OrderState::Pending => (),
                    OrderState::Processing => <ProcessingOrderIds<T>>::insert(order.order_id, ()),
                    OrderState::Emergency => <EmergencyOrderIds<T>>::insert(order.order_id, ()),
                    OrderState::Done => (),
                }
            }
            <Orders<T>>::insert(order.order_id.clone(), order);
            Ok(())
        }
    }
}
