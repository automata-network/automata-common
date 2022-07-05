#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

pub mod datastructures;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {

    use crate::datastructures::*;
    use frame_support::{pallet_prelude::*, traits::UnixTime};
    use frame_system::pallet_prelude::*;
    use sp_core::H256;
    use sp_runtime::SaturatedConversion;
    use sp_std::prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_timestamp::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type UnixTime: UnixTime;

        #[pallet::constant]
        type MaxIndexLength: Get<u32>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn get_namespace_id)]
    pub type NamespaceIdStore<T: Config> =
        StorageMap<_, Blake2_128Concat, GmetadataNamespaceName, u32>;

    #[pallet::storage]
    #[pallet::getter(fn get_namespace)]
    pub type NamespaceStore<T: Config> =
        StorageMap<_, Blake2_128Concat, u32, GmetadataNamespaceInfo<T::AccountId>>;

    #[pallet::storage]
    #[pallet::getter(fn latest_namespace_id)]
    pub type LatestNamespaceId<T> = StorageValue<_, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn get_value)]
    pub type ValueStore<T: Config> =
        StorageMap<_, Blake2_128Concat, GmetadataKey, GmetadataValueInfo>;

    #[pallet::storage]
    #[pallet::getter(fn get_index)]
    pub type IndexStore<T: Config> =
        StorageMap<_, Blake2_128Concat, GmetadataKey, GmetadataIndexInfo>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        StateUpdate(
            /*req_id*/ H256,
            /*namespace*/ u32,
            /*table*/ Vec<u8>,
            /*pk*/ Vec<u8>,
        ),
        IndexUpdate(
            /*req_id*/ H256,
            /*namespace*/ u32,
            /*table*/ Vec<u8>,
            /*pk*/ Vec<u8>,
        ),
        ValueUpdate(
            /*req_id*/ H256,
            /*namespace*/ u32,
            /*table*/ Vec<u8>,
            /*pk*/ Vec<u8>,
        ),
    }

    #[pallet::error]
    pub enum Error<T> {
        NamespaceAlreadyExist,
        NamespaceNotFound,
        NamespaceOwnerAlreadyExists,
        InvalidNamespaceName,
        InvalidKey,
        IndexLengthTooLong,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(0)]
        pub fn namespace_add_owner(
            origin: OriginFor<T>,
            id: u32,
            account: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            NamespaceStore::<T>::try_mutate(id, |ns| -> DispatchResult {
                match ns {
                    Some(ns) => {
                        Self::check_owner_or_root(&origin, &ns.owners)?;
                        if !ns.owners.contains(&account) {
                            ns.owners.push(account);
                            Ok(())
                        } else {
                            Err(Error::<T>::NamespaceOwnerAlreadyExists.into())
                        }
                    }
                    None => Err(Error::<T>::NamespaceNotFound.into()),
                }
            })?;
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn namespace_remove_owner(
            origin: OriginFor<T>,
            id: u32,
            account: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            let mut ns = match NamespaceStore::<T>::get(id) {
                Some(ns) => ns,
                None => return Err(Error::<T>::NamespaceNotFound.into()),
            };
            Self::check_owner_or_root(&origin, &ns.owners)?;
            match ns.owners.iter().position(|a| a.eq(&account)) {
                Some(idx) => {
                    ns.owners.remove(idx);
                    NamespaceStore::<T>::insert(id, ns);
                }
                None => {}
            };
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn create_namespace(
            origin: OriginFor<T>,
            name: GmetadataNamespaceName,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin.clone())?;
            if name.len() == 0 {
                return Err(Error::<T>::InvalidNamespaceName.into());
            }

            let namespace_id = NamespaceIdStore::<T>::get(&name);
            if namespace_id.is_some() {
                return Err(Error::<T>::NamespaceAlreadyExist.into());
            }
            let namespace_id = Self::latest_namespace_id().saturating_add(1);
            let owners = Vec::<T::AccountId>::new();
            NamespaceStore::<T>::insert(
                namespace_id,
                GmetadataNamespaceInfo {
                    id: namespace_id,
                    name: name.clone(),
                    owners,
                },
            );

            NamespaceIdStore::<T>::insert(name, namespace_id);
            LatestNamespaceId::<T>::put(namespace_id);
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn batch_write(
            origin: OriginFor<T>,
            ops: Vec<GmetadataWriteOp>,
            req_id: H256,
        ) -> DispatchResultWithPostInfo {
            for op in ops {
                match op {
                    GmetadataWriteOp::SetValue(key, value) => {
                        Self::set_value(origin.clone(), key, value, req_id)?;
                    }
                    GmetadataWriteOp::RemoveValue(key) => {
                        Self::remove_value(origin.clone(), key, req_id)?;
                    }
                    GmetadataWriteOp::AddIndex(key, value) => {
                        Self::add_index(origin.clone(), key, value, req_id)?;
                    }
                    GmetadataWriteOp::RemoveIndex(key, value) => {
                        Self::remove_index(origin.clone(), key, value, req_id)?;
                    }
                }
            }
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn set_value(
            origin: OriginFor<T>,
            key: GmetadataKey,
            value: Vec<u8>,
            req_id: H256,
        ) -> DispatchResultWithPostInfo {
            Self::check_namespace(origin, key.ns)?;
            Self::check_key(&key)?;
            ValueStore::<T>::insert(
                key.clone(),
                GmetadataValueInfo {
                    data: value,
                    update_time: T::UnixTime::now().as_millis().saturated_into::<u64>(),
                },
            );
            Self::deposit_event(Event::StateUpdate(
                req_id,
                key.ns,
                key.table.clone().into(),
                key.pk.clone().into(),
            ));
            Self::deposit_event(Event::ValueUpdate(
                req_id,
                key.ns,
                key.table.into(),
                key.pk.into(),
            ));
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn add_index(
            origin: OriginFor<T>,
            key: GmetadataKey,
            value: Vec<u8>,
            req_id: H256,
        ) -> DispatchResultWithPostInfo {
            Self::check_namespace(origin, key.ns)?;
            Self::check_key(&key)?;
            let mut old_value = IndexStore::<T>::get(&key);
            match &mut old_value {
                Some(old_value) => {
                    if !old_value.data.contains(&value) {
                        if old_value.data.len() as u32 >= T::MaxIndexLength::get() {
                            return Err(<Error<T>>::IndexLengthTooLong.into());
                        }
                        old_value.data.push(value);
                        old_value.data.sort();
                        old_value.update_time =
                            T::UnixTime::now().as_millis().saturated_into::<u64>();
                        IndexStore::<T>::insert(key.clone(), old_value);
                    }
                }
                None => IndexStore::<T>::insert(
                    key.clone(),
                    GmetadataIndexInfo {
                        data: sp_std::vec![value],
                        update_time: T::UnixTime::now().as_millis().saturated_into::<u64>(),
                    },
                ),
            }
            Self::deposit_event(Event::StateUpdate(
                req_id,
                key.ns,
                key.table.clone().into(),
                key.pk.clone().into(),
            ));
            Self::deposit_event(Event::IndexUpdate(
                req_id,
                key.ns,
                key.table.into(),
                key.pk.into(),
            ));
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn remove_value(
            origin: OriginFor<T>,
            key: GmetadataKey,
            req_id: H256,
        ) -> DispatchResultWithPostInfo {
            Self::check_namespace(origin, key.ns)?;
            ValueStore::<T>::remove(key.clone());
            Self::deposit_event(Event::StateUpdate(
                req_id,
                key.ns,
                key.table.clone().into(),
                key.pk.clone().into(),
            ));
            Self::deposit_event(Event::ValueUpdate(
                req_id,
                key.ns,
                key.table.into(),
                key.pk.into(),
            ));
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn remove_index(
            origin: OriginFor<T>,
            key: GmetadataKey,
            value: Vec<u8>,
            req_id: H256,
        ) -> DispatchResultWithPostInfo {
            Self::check_namespace(origin, key.ns)?;
            let mut old_value = IndexStore::<T>::get(&key);
            match &mut old_value {
                Some(old_value) => match old_value.data.iter().position(|v| v == &value) {
                    Some(idx) => {
                        old_value.data.remove(idx);
                        old_value.data.sort();
                        old_value.update_time =
                            T::UnixTime::now().as_millis().saturated_into::<u64>();
                        IndexStore::<T>::insert(key.clone(), old_value);
                    }
                    None => {}
                },
                None => {}
            }
            Self::deposit_event(Event::StateUpdate(
                req_id,
                key.ns,
                key.table.clone().into(),
                key.pk.clone().into(),
            ));
            Self::deposit_event(Event::IndexUpdate(
                req_id,
                key.ns,
                key.table.into(),
                key.pk.into(),
            ));
            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        pub fn value(key: GmetadataKey) -> Option<Vec<u8>> {
            match ValueStore::<T>::get(key) {
                Some(value) => Some(value.data),
                None => None,
            }
        }

        fn check_owner_or_root(
            origin: &OriginFor<T>,
            owners: &Vec<T::AccountId>,
        ) -> DispatchResult {
            match ensure_signed(origin.clone()) {
                Ok(who) => {
                    if !owners.contains(&who) {
                        return Err(DispatchError::BadOrigin);
                    }
                }
                Err(_) => ensure_root(origin.clone())?,
            }
            Ok(().into())
        }

        fn check_namespace(origin: OriginFor<T>, ns_id: u32) -> DispatchResultWithPostInfo {
            let ns = match NamespaceStore::<T>::get(ns_id) {
                Some(ns) => ns,
                None => return Err(Error::<T>::NamespaceNotFound.into()),
            };
            Self::check_owner_or_root(&origin, &ns.owners)?;
            Ok(().into())
        }

        fn check_key(key: &GmetadataKey) -> DispatchResultWithPostInfo {
            if key.table.len() > 100 {
                return Err(Error::<T>::InvalidKey.into());
            }
            if key.pk.len() > 500 {
                return Err(Error::<T>::InvalidKey.into());
            }
            Ok(().into())
        }

        pub fn query_with_index(
            index_key: GmetadataKey,
            value_key: GmetadataKey,
            start: HexBytes,
            limit: u64,
        ) -> GmetadataQueryResult {
            Self::query_with_indexes(sp_std::vec![index_key], value_key, start, limit)
        }

        pub fn query_with_indexes(
            index_keys: Vec<GmetadataKey>,
            mut value_key: GmetadataKey,
            start: HexBytes,
            limit: u64,
        ) -> GmetadataQueryResult {
            let mut result = Vec::new();
            let mut cursor = None;
            let mut skip = true;
            let max_index_length = T::MaxIndexLength::get() as usize;
            for index_key in index_keys {
                match Self::get_index(index_key) {
                    Some(index_info) => {
                        let list = if index_info.data.len() > max_index_length {
                            &index_info.data[..max_index_length]
                        } else {
                            &index_info.data
                        };
                        for key in list {
                            if skip {
                                if start.len() == 0 {
                                    skip = false;
                                } else if start.eq(key) {
                                    skip = false;
                                    continue;
                                }
                            }
                            if skip {
                                continue;
                            }
                            value_key.pk = key.clone().into();
                            match Self::get_value(value_key.clone()) {
                                Some(val) => {
                                    result.push(val.data.into());
                                    if result.len() >= limit as _ {
                                        if Some(key) != index_info.data.last() {
                                            cursor = Some(key.clone().into());
                                        }
                                        break;
                                    }
                                }
                                None => {}
                            };
                        }
                    }
                    None => {}
                }
            }

            let cursor = match cursor {
                Some(cursor) => cursor,
                None => HexBytes::new(),
            };
            GmetadataQueryResult {
                list: result,
                cursor: cursor.into(),
            }
        }
    }
}
